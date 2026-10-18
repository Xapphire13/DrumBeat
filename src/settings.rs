//! Settings kept between launches, in a TOML file that can be edited by
//! hand. Changes are written with `toml_edit`, so comments and ordering
//! added to the file survive

use std::{
    path::PathBuf,
    sync::{
        Mutex, OnceLock,
        mpsc::{self, RecvTimeoutError, Sender},
    },
    time::Duration,
};

use serde::Deserialize;
use toml_edit::{DocumentMut, Item, Table, Value};

use crate::audio::midi;

/// How long settings have to stay put before they're written, so dragging a
/// slider doesn't write the file at every step
const SAVE_DELAY: Duration = Duration::from_millis(500);

#[derive(Clone, PartialEq, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub audio: AudioSettings,
}

#[derive(Clone, PartialEq, Debug, Deserialize)]
#[serde(default)]
pub struct AudioSettings {
    pub output: DrumsOutput,
    /// The port MIDI out sends to
    pub midi_port: String,
    /// How far ahead of the playhead MIDI notes are sent
    pub midi_offset_ms: u32,
    /// From 0 to 1
    pub volume: f64,
    pub muted: bool,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            output: DrumsOutput::default(),
            midi_port: midi::VIRTUAL_PORT.to_string(),
            midi_offset_ms: 0,
            volume: 1.0,
            muted: false,
        }
    }
}

/// Where the score's drums are played
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DrumsOutput {
    /// DrumBeat's own synthesized drums
    #[default]
    Synth,
    /// Sent out as MIDI notes
    Midi,
}

impl DrumsOutput {
    fn as_str(self) -> &'static str {
        match self {
            DrumsOutput::Synth => "synth",
            DrumsOutput::Midi => "midi",
        }
    }
}

/// The file as it was read, kept so saving changes only the values that
/// changed
fn document() -> &'static Mutex<DocumentMut> {
    static DOCUMENT: OnceLock<Mutex<DocumentMut>> = OnceLock::new();
    DOCUMENT.get_or_init(|| {
        let text = path()
            .and_then(|path| std::fs::read_to_string(path).ok())
            .unwrap_or_default();
        let document = text.parse().unwrap_or_else(|error| {
            eprintln!("Settings file can't be read, using defaults: {error}");
            DocumentMut::new()
        });
        Mutex::new(document)
    })
}

fn path() -> Option<PathBuf> {
    Some(
        dirs::config_dir()?
            .join("com.xapphire13.drumbeat")
            .join("settings.toml"),
    )
}

/// The settings from the file, with defaults for anything missing or
/// unreadable
pub fn load() -> Settings {
    let document = document().lock().unwrap_or_else(|e| e.into_inner());
    parse(&document)
}

fn parse(document: &DocumentMut) -> Settings {
    toml_edit::de::from_document(document.clone()).unwrap_or_else(|error| {
        eprintln!("Settings file has a bad value, using defaults: {error}");
        Settings::default()
    })
}

/// Save the audio settings, a moment after they stop changing
pub fn save_audio(audio: &AudioSettings) {
    let mut document = document().lock().unwrap_or_else(|e| e.into_inner());
    update_audio(&mut document, audio);
    schedule_write(document.to_string());
}

fn update_audio(document: &mut DocumentMut, audio: &AudioSettings) {
    let table = document
        .entry("audio")
        .or_insert_with(|| Item::Table(Table::new()));
    let Some(table) = table.as_table_like_mut() else {
        // Something else is called `audio`, which can't be fixed up without
        // losing it
        return;
    };
    set(table, "output", audio.output.as_str().into());
    set(table, "midi_port", audio.midi_port.as_str().into());
    set(
        table,
        "midi_offset_ms",
        i64::from(audio.midi_offset_ms).into(),
    );
    // Rounded so the file doesn't fill with float noise
    set(
        table,
        "volume",
        ((audio.volume * 100.0).round() / 100.0).into(),
    );
    set(table, "muted", audio.muted.into());
}

/// Set `key` to `value`, keeping any comment on its line
fn set(table: &mut dyn toml_edit::TableLike, key: &str, value: Value) {
    match table.get_mut(key) {
        Some(Item::Value(existing)) => {
            if existing.to_string().trim() == value.to_string().trim() {
                return;
            }
            let decor = existing.decor().clone();
            *existing = value;
            *existing.decor_mut() = decor;
        }
        _ => {
            table.insert(key, Item::Value(value));
        }
    }
}

/// Hand the file's new contents to the writer, which waits for them to
/// settle
fn schedule_write(contents: String) {
    static WRITER: OnceLock<Option<Sender<String>>> = OnceLock::new();
    let writer = WRITER.get_or_init(|| {
        let (writer, contents) = mpsc::channel::<String>();
        std::thread::Builder::new()
            .name("settings".into())
            .spawn(move || {
                while let Ok(mut latest) = contents.recv() {
                    loop {
                        match contents.recv_timeout(SAVE_DELAY) {
                            Ok(newer) => latest = newer,
                            Err(RecvTimeoutError::Timeout) => break,
                            Err(RecvTimeoutError::Disconnected) => break,
                        }
                    }
                    if let Err(error) = write(&latest) {
                        eprintln!("Settings couldn't be saved: {error}");
                    }
                }
            })
            .ok()
            .map(|_| writer)
    });
    if let Some(writer) = writer {
        let _ = writer.send(contents);
    }
}

fn write(contents: &str) -> std::io::Result<()> {
    let path = path().ok_or_else(|| std::io::Error::other("no settings folder"))?;
    if let Some(folder) = path.parent() {
        std::fs::create_dir_all(folder)?;
    }
    // Written alongside then moved over, so a crash can't leave half a file
    let temporary = path.with_extension("toml.tmp");
    std::fs::write(&temporary, contents)?;
    std::fs::rename(temporary, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_values_get_defaults() {
        let document: DocumentMut = "[audio]\nvolume = 0.5\n".parse().unwrap();
        let settings = parse(&document);
        assert_eq!(settings.audio.volume, 0.5);
        assert_eq!(settings.audio.output, DrumsOutput::Synth);
        assert_eq!(settings.audio.midi_port, midi::VIRTUAL_PORT);
    }

    #[test]
    fn saving_keeps_comments() {
        let mut document: DocumentMut =
            "# My settings\n[audio]\noutput = \"synth\" # or midi\nvolume = 1.0\n"
                .parse()
                .unwrap();
        let audio = AudioSettings {
            output: DrumsOutput::Midi,
            volume: 0.8,
            ..AudioSettings::default()
        };
        update_audio(&mut document, &audio);
        let text = document.to_string();
        assert!(text.starts_with("# My settings\n[audio]\n"), "{text}");
        assert!(text.contains("output = \"midi\" # or midi\n"), "{text}");
        assert!(text.contains("volume = 0.8\n"), "{text}");
        assert_eq!(parse(&text.parse().unwrap()).audio, audio);
    }
}
