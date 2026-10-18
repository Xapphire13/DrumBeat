//! Sends the score's drums out as MIDI, for a DAW or drum plugin to play.
//! The audio thread decides when each note is due, on the same clock as the
//! built-in drums, and this thread sends it at that moment

use std::{
    cmp::Reverse,
    collections::BinaryHeap,
    sync::{
        Mutex, OnceLock,
        atomic::{AtomicU8, AtomicU64, Ordering},
        mpsc::{self, RecvTimeoutError, Sender},
    },
    time::{Duration, Instant},
};

use midir::{MidiOutput, MidiOutputConnection};

/// The port DrumBeat makes for other apps to listen to, and the name it
/// goes by as a MIDI client
pub const VIRTUAL_PORT: &str = "DrumBeat";

/// Channel 10, which General MIDI keeps for drums, counting from 0
const CHANNEL: u8 = 9;

/// How long each note is held before its note-off. Drum sounds ring out on
/// their own, but some instruments wait for it
const NOTE_LENGTH: Duration = Duration::from_millis(50);

/// The snare, for checking the connection
const TEST_NOTE: u8 = 38;

/// Where notes are going
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Status {
    /// No port is picked
    Off,
    Connected,
    /// The picked port isn't there, e.g. a device that's been unplugged
    Missing,
    /// The system's MIDI couldn't be used
    Failed,
}

static STATUS: AtomicU8 = AtomicU8::new(0);

/// When the last note-on was sent, in milliseconds since `epoch()`, plus one
/// so 0 means never
static LAST_SENT: AtomicU64 = AtomicU64::new(0);

fn epoch() -> Instant {
    static EPOCH: OnceLock<Instant> = OnceLock::new();
    *EPOCH.get_or_init(Instant::now)
}

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

/// How long ago the last note was sent
pub fn since_last_sent() -> Option<Duration> {
    let millis = LAST_SENT.load(Ordering::Relaxed).checked_sub(1)?;
    let sent = epoch() + Duration::from_millis(millis);
    Some(sent.elapsed())
}

/// The ports found by the last `refresh_ports`
static PORTS: Mutex<Vec<String>> = Mutex::new(vec![]);

/// Look for ports again on the MIDI thread, since the system can take a
/// moment to answer. `ports` has them once it has
pub fn refresh_ports() {
    send(Command::RefreshPorts);
}

/// The ports notes can be sent to, as last found, DrumBeat's own first
pub fn ports() -> Vec<String> {
    PORTS.lock().map(|ports| ports.clone()).unwrap_or_default()
}

fn available_ports() -> Vec<String> {
    let mut ports = vec![];
    if cfg!(unix) {
        ports.push(VIRTUAL_PORT.to_string());
    }
    if let Ok(output) = MidiOutput::new(VIRTUAL_PORT) {
        for port in output.ports() {
            if let Ok(name) = output.port_name(&port)
                && !ports.contains(&name)
            {
                ports.push(name);
            }
        }
    }
    ports
}

/// Send notes to the port named `port`, or stop sending with `None`
pub fn connect(port: Option<String>) {
    send(Command::Connect(port));
}

/// Play the snare now, to check notes are getting through
pub fn test_hit() {
    schedule(Instant::now(), TEST_NOTE, 100);
}

/// Send a note-on for `key` at `at`, with its note-off just after
pub(super) fn schedule(at: Instant, key: u8, velocity: u8) {
    send(Command::Schedule(Event {
        at,
        message: [0x90 | CHANNEL, key, velocity.clamp(1, 127)],
    }));
}

/// Forget notes that haven't been sent yet, when playback jumps
pub(super) fn clear_pending() {
    send(Command::ClearPending);
}

/// Forget pending notes and silence anything held, when playback stops
pub(super) fn all_notes_off() {
    send(Command::AllNotesOff);
}

enum Command {
    Connect(Option<String>),
    RefreshPorts,
    Schedule(Event),
    ClearPending,
    AllNotesOff,
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct Event {
    at: Instant,
    message: [u8; 3],
}

impl Event {
    fn is_note_on(&self) -> bool {
        self.message[0] & 0xf0 == 0x90
    }
}

fn send(command: Command) {
    static COMMANDS: OnceLock<Option<Sender<Command>>> = OnceLock::new();
    let commands = COMMANDS.get_or_init(|| {
        let (commands, receiver) = mpsc::channel();
        std::thread::Builder::new()
            .name("midi".into())
            .spawn(move || Outbox::run(receiver))
            .ok()
            .map(|_| commands)
    });
    if let Some(commands) = commands {
        let _ = commands.send(command);
    }
}

/// The MIDI thread's state
struct Outbox {
    connection: Option<MidiOutputConnection>,
    /// Waiting to be sent, soonest first
    pending: BinaryHeap<Reverse<Event>>,
}

impl Outbox {
    fn run(commands: mpsc::Receiver<Command>) {
        let mut outbox = Outbox {
            connection: None,
            pending: BinaryHeap::new(),
        };
        loop {
            let next = match outbox.pending.peek() {
                Some(Reverse(event)) => {
                    let wait = event.at.saturating_duration_since(Instant::now());
                    commands.recv_timeout(wait)
                }
                None => commands.recv().map_err(|_| RecvTimeoutError::Disconnected),
            };
            match next {
                Ok(command) => outbox.handle(command),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return,
            }
            outbox.send_due();
        }
    }

    fn handle(&mut self, command: Command) {
        match command {
            Command::Connect(port) => self.connect(port),
            Command::RefreshPorts => {
                let found = available_ports();
                if let Ok(mut ports) = PORTS.lock() {
                    *ports = found;
                }
            }
            Command::Schedule(event) => self.pending.push(Reverse(event)),
            Command::ClearPending => {
                // Note-offs still go, so nothing is left held
                self.pending.retain(|Reverse(event)| !event.is_note_on());
            }
            Command::AllNotesOff => {
                self.pending.clear();
                // All Notes Off, then All Sound Off for instruments that
                // ignore it
                self.write(&[0xb0 | CHANNEL, 123, 0]);
                self.write(&[0xb0 | CHANNEL, 120, 0]);
            }
        }
    }

    fn connect(&mut self, port: Option<String>) {
        // Close the old connection first, so a virtual port can be remade
        // under the same name
        self.connection = None;
        let Some(port) = port else {
            set_status(Status::Off);
            return;
        };
        let Ok(output) = MidiOutput::new(VIRTUAL_PORT) else {
            set_status(Status::Failed);
            return;
        };

        let connection = if port == VIRTUAL_PORT {
            create_virtual(output)
        } else {
            match output
                .ports()
                .into_iter()
                .find(|p| output.port_name(p).is_ok_and(|name| name == port))
            {
                Some(found) => output.connect(&found, VIRTUAL_PORT).ok(),
                None => {
                    set_status(Status::Missing);
                    return;
                }
            }
        };
        set_status(if connection.is_some() {
            Status::Connected
        } else {
            Status::Failed
        });
        self.connection = connection;
    }

    fn send_due(&mut self) {
        let now = Instant::now();
        while let Some(Reverse(event)) = self.pending.peek()
            && event.at <= now
        {
            let Some(Reverse(event)) = self.pending.pop() else {
                break;
            };
            self.write(&event.message);
            if event.is_note_on() {
                let sent_ms = now.duration_since(epoch()).as_millis() as u64;
                LAST_SENT.store(sent_ms + 1, Ordering::Relaxed);
                self.pending.push(Reverse(Event {
                    at: event.at + NOTE_LENGTH,
                    message: [0x80 | CHANNEL, event.message[1], 0],
                }));
            }
        }
    }

    fn write(&mut self, message: &[u8]) {
        if let Some(connection) = &mut self.connection {
            let _ = connection.send(message);
        }
    }
}

#[cfg(unix)]
fn create_virtual(output: MidiOutput) -> Option<MidiOutputConnection> {
    use midir::os::unix::VirtualOutput;
    output.create_virtual(VIRTUAL_PORT).ok()
}

#[cfg(not(unix))]
fn create_virtual(_output: MidiOutput) -> Option<MidiOutputConnection> {
    None
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use midir::MidiInput;

    use super::*;

    /// Talks to the system's real MIDI, so it's left out of normal runs
    #[test]
    #[ignore]
    fn virtual_port_delivers_notes() {
        connect(Some(VIRTUAL_PORT.to_string()));
        let started = Instant::now();
        while status() == Status::Off && started.elapsed() < Duration::from_secs(5) {
            std::thread::sleep(Duration::from_millis(10));
        }
        eprintln!("connected after {:?}", started.elapsed());
        assert_eq!(status(), Status::Connected);

        let input = MidiInput::new("DrumBeat test").unwrap();
        let port = input
            .ports()
            .into_iter()
            .find(|port| input.port_name(port).is_ok_and(|name| name == VIRTUAL_PORT))
            .expect("the virtual port is listed");
        let received = Arc::new(Mutex::new(vec![]));
        let _connection = input
            .connect(
                &port,
                "listen",
                {
                    let received = received.clone();
                    move |_, message, _| received.lock().unwrap().push(message.to_vec())
                },
                (),
            )
            .unwrap();

        test_hit();
        std::thread::sleep(Duration::from_millis(300));
        let received = received.lock().unwrap();
        assert_eq!(received[0], [0x99, TEST_NOTE, 100]);
        assert_eq!(received[1], [0x89, TEST_NOTE, 0]);
        assert!(since_last_sent().is_some());
    }
}
