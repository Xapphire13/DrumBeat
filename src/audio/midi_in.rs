//! Listens to a MIDI input, usually the drum kit being practised on. Each
//! note played is kept with the moment it arrived, for the UI to show

use std::{
    collections::VecDeque,
    sync::{
        Mutex, OnceLock,
        atomic::{AtomicU8, AtomicU64, Ordering},
        mpsc::{self, RecvTimeoutError, Sender},
    },
    time::{Duration, Instant},
};

use midir::{MidiInput, MidiInputConnection, MidiInputPort};

use super::midi::VIRTUAL_PORT;

/// The name DrumBeat goes by as a MIDI client while listening
const CLIENT_NAME: &str = "DrumBeat input";

/// How often the system's devices are checked, so an unplugged kit is noticed
/// and picked up again when it's back
const WATCH_INTERVAL: Duration = Duration::from_secs(1);

/// How many of the latest hits are kept
const RECENT_HITS: usize = 12;

/// Whether DrumBeat is listening
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Status {
    /// No device is picked
    Off,
    Connected,
    /// The picked device isn't there, e.g. a kit that's been unplugged
    Missing,
    /// The system's MIDI couldn't be used
    Failed,
}

/// A note played on the input
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ReceivedHit {
    pub note: u8,
    pub velocity: u8,
    /// From 1 to 16
    pub channel: u8,
    pub at: Instant,
}

static STATUS: AtomicU8 = AtomicU8::new(0);

/// Counts every hit received, so the UI can tell when there's a new one
static HITS_RECEIVED: AtomicU64 = AtomicU64::new(0);

/// The latest hits, newest first
static RECENT: Mutex<VecDeque<ReceivedHit>> = Mutex::new(VecDeque::new());

/// The inputs found by the last look
static PORTS: Mutex<Vec<String>> = Mutex::new(vec![]);

pub fn status() -> Status {
    match STATUS.load(Ordering::Relaxed) {
        1 => Status::Connected,
        2 => Status::Missing,
        3 => Status::Failed,
        _ => Status::Off,
    }
}

fn set_status(status: Status) {
    let value = match status {
        Status::Off => 0,
        Status::Connected => 1,
        Status::Missing => 2,
        Status::Failed => 3,
    };
    STATUS.store(value, Ordering::Relaxed);
}

/// How many hits have been received since launch
pub fn hits_received() -> u64 {
    HITS_RECEIVED.load(Ordering::Relaxed)
}

/// The latest hits, newest first
pub fn recent_hits() -> Vec<ReceivedHit> {
    RECENT
        .lock()
        .map(|recent| recent.iter().copied().collect())
        .unwrap_or_default()
}

/// The inputs that can be listened to, as last found
pub fn ports() -> Vec<String> {
    PORTS.lock().map(|ports| ports.clone()).unwrap_or_default()
}

/// Look for inputs again on the MIDI input thread, since the system can take
/// a moment to answer. `ports` has them once it has
pub fn refresh_ports() {
    send(Command::RefreshPorts);
}

/// Listen to the input named `port`, or stop listening with `None`
pub fn connect(port: Option<String>) {
    send(Command::Connect(port));
}

enum Command {
    Connect(Option<String>),
    RefreshPorts,
}

fn send(command: Command) {
    static COMMANDS: OnceLock<Option<Sender<Command>>> = OnceLock::new();
    let commands = COMMANDS.get_or_init(|| {
        let (commands, receiver) = mpsc::channel();
        std::thread::Builder::new()
            .name("midi-in".into())
            .spawn(move || Listener::run(&receiver))
            .ok()
            .map(|_| commands)
    });
    if let Some(commands) = commands {
        let _ = commands.send(command);
    }
}

/// The MIDI input thread's state
struct Listener {
    /// The input picked, whether or not it's there
    port: Option<String>,
    connection: Option<MidiInputConnection<()>>,
    /// Kept for looking at the system's inputs, so a client isn't made at
    /// every look
    lister: Option<MidiInput>,
}

impl Listener {
    fn run(commands: &mpsc::Receiver<Command>) {
        let mut listener = Listener {
            port: None,
            connection: None,
            lister: None,
        };
        loop {
            match commands.recv_timeout(WATCH_INTERVAL) {
                Ok(Command::Connect(port)) => {
                    listener.port = port;
                    listener.connect();
                }
                Ok(Command::RefreshPorts) => {
                    let found = listener.available_ports();
                    store_ports(found);
                }
                // Only worth watching for a device that's picked
                Err(RecvTimeoutError::Timeout) if listener.port.is_some() => listener.watch(),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return,
            }
        }
    }

    fn connect(&mut self) {
        self.connection = None;
        let Some(name) = self.port.clone() else {
            set_status(Status::Off);
            return;
        };
        let Ok(input) = MidiInput::new(CLIENT_NAME) else {
            set_status(Status::Failed);
            return;
        };
        let Some(port) = find_port(&input, &name) else {
            set_status(Status::Missing);
            return;
        };
        match input.connect(&port, "listen", |_, message, ()| receive(message), ()) {
            Ok(connection) => {
                self.connection = Some(connection);
                set_status(Status::Connected);
            }
            Err(_) => set_status(Status::Failed),
        }
    }

    /// The system's inputs, leaving out DrumBeat's own virtual port so the
    /// drums it sends out can't come back in
    fn available_ports(&mut self) -> Vec<String> {
        if self.lister.is_none() {
            self.lister = MidiInput::new(CLIENT_NAME).ok();
        }
        let Some(input) = &self.lister else {
            return vec![];
        };
        let mut ports = vec![];
        for port in input.ports() {
            if let Ok(name) = input.port_name(&port)
                && name != VIRTUAL_PORT
                && !ports.contains(&name)
            {
                ports.push(name);
            }
        }
        ports
    }

    /// Notice the picked device going away, and connect to it again when it
    /// comes back
    fn watch(&mut self) {
        let found = self.available_ports();
        let present = self.port.as_ref().is_some_and(|port| found.contains(port));
        store_ports(found);
        match (status(), present) {
            (Status::Connected, false) => {
                self.connection = None;
                set_status(Status::Missing);
            }
            (Status::Missing | Status::Failed, true) => self.connect(),
            _ => {}
        }
    }
}

/// Keep a note-on from the input. Everything else is ignored, including a
/// note-on with no velocity, which some devices send for a note-off
fn receive(message: &[u8]) {
    let &[status, note, velocity, ..] = message else {
        return;
    };
    if status & 0xf0 != 0x90 || velocity == 0 {
        return;
    }
    let hit = ReceivedHit {
        note,
        velocity,
        channel: (status & 0x0f) + 1,
        at: Instant::now(),
    };
    if let Ok(mut recent) = RECENT.lock() {
        recent.push_front(hit);
        recent.truncate(RECENT_HITS);
    }
    HITS_RECEIVED.fetch_add(1, Ordering::Relaxed);
}

fn store_ports(found: Vec<String>) {
    if let Ok(mut ports) = PORTS.lock() {
        *ports = found;
    }
}

fn find_port(input: &MidiInput, name: &str) -> Option<MidiInputPort> {
    input
        .ports()
        .into_iter()
        .find(|port| input.port_name(port).is_ok_and(|found| found == name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_note_ons_newest_first() {
        let before = hits_received();
        receive(&[0x99, 38, 100]);
        // A note-off, and a note-on standing in for one
        receive(&[0x89, 38, 0]);
        receive(&[0x99, 42, 0]);
        receive(&[0x90, 36, 90]);
        assert_eq!(hits_received() - before, 2);
        let recent = recent_hits();
        assert_eq!(
            (recent[0].note, recent[0].velocity, recent[0].channel),
            (36, 90, 1)
        );
        assert_eq!((recent[1].note, recent[1].channel), (38, 10));
    }
}
