//! Loader for Guitar Pro 8 `.gp` files.
//!
//! A `.gp` file is a zip archive whose `Content/score.gpif` entry holds the
//! score as XML. This module reads the first drum track out of it into its
//! own types, which are later converted into the app's data model.

mod convert;
mod fraction;
mod gpif;

use std::collections::HashMap;
use std::fmt;
use std::fs::File;
use std::io::{self, Read, Seek};
use std::path::Path;

pub use fraction::Fraction;

use crate::data::measure::Section;

const SCORE_ENTRY: &str = "Content/score.gpif";

#[derive(Clone, PartialEq, Debug)]
pub struct GpScore {
    pub title: String,
    pub artist: String,
    pub track_name: String,
    pub tempo_changes: Vec<TempoChange>,
    pub bars: Vec<GpBar>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct TempoChange {
    /// Index into `GpScore::bars`
    pub bar: usize,
    /// Where in the bar the change happens, from 0 (start) to 1 (end)
    pub position: f64,
    /// Quarter notes per minute
    pub bpm: f64,
}

#[derive(Clone, PartialEq, Debug)]
pub struct TimeSignature {
    pub notes_per_measure: u32,
    pub note_value: u32,
}

impl TimeSignature {
    /// Length of a bar in whole notes
    pub fn bar_length(&self) -> Fraction {
        Fraction::new(self.notes_per_measure, self.note_value)
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct GpBar {
    pub time_signature: TimeSignature,
    pub section: Option<Section>,
    pub repeat_start: bool,
    /// How many times the repeated passage is played, when a repeat ends here
    pub repeat_end: Option<u32>,
    /// Each voice is an independent rhythm through the bar
    pub voices: Vec<GpVoice>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct GpVoice {
    pub beats: Vec<GpBeat>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct GpBeat {
    /// Offset from the start of the bar, in whole notes. Use
    /// `start.to_ticks(4 * TICKS_PER_QUARTER)` for the app's tick timeline
    pub start: Fraction,
    pub rhythm: GpRhythm,
    /// Grace beats take no time; the next beat starts at the same offset
    pub grace: Option<Grace>,
    /// Set while the beat is under a crescendo or decrescendo hairpin
    pub hairpin: Option<Hairpin>,
    /// Empty for a rest
    pub notes: Vec<GpNote>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Hairpin {
    Crescendo,
    Decrescendo,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Grace {
    BeforeBeat,
    OnBeat,
}

#[derive(Clone, PartialEq, Debug)]
pub struct GpRhythm {
    pub value: GpNoteValue,
    pub dots: u8,
    pub tuplet: Option<Tuplet>,
    /// A tuplet nested inside `tuplet`
    pub inner_tuplet: Option<Tuplet>,
}

impl GpRhythm {
    /// Length of the beat in whole notes
    pub fn duration(&self) -> Fraction {
        let mut duration = Fraction::new(1, self.value.denominator());
        if self.dots > 0 {
            // Each dot adds half of the previous addition: 1.5x, 1.75x, ...
            let pow = 1 << self.dots;
            duration = duration * Fraction::new(2 * pow - 1, pow);
        }
        for tuplet in self.tuplet.iter().chain(self.inner_tuplet.iter()) {
            duration = duration * Fraction::new(tuplet.in_time_of, tuplet.notes);
        }
        duration
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum GpNoteValue {
    Whole,
    Half,
    Quarter,
    Eighth,
    Sixteenth,
    ThirtySecond,
    SixtyFourth,
    OneTwentyEighth,
}

impl GpNoteValue {
    fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "Whole" => Self::Whole,
            "Half" => Self::Half,
            "Quarter" => Self::Quarter,
            "Eighth" => Self::Eighth,
            "16th" => Self::Sixteenth,
            "32nd" => Self::ThirtySecond,
            "64th" => Self::SixtyFourth,
            "128th" => Self::OneTwentyEighth,
            _ => return None,
        })
    }

    /// 1 for a whole note, 4 for a quarter note, etc.
    pub fn denominator(&self) -> u32 {
        match self {
            Self::Whole => 1,
            Self::Half => 2,
            Self::Quarter => 4,
            Self::Eighth => 8,
            Self::Sixteenth => 16,
            Self::ThirtySecond => 32,
            Self::SixtyFourth => 64,
            Self::OneTwentyEighth => 128,
        }
    }
}

/// A tuplet ratio, e.g. a triplet is 3 notes in the time of 2
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Tuplet {
    pub notes: u32,
    pub in_time_of: u32,
}

#[derive(Clone, PartialEq, Debug)]
pub struct GpNote {
    /// Guitar Pro's number for the sound, e.g. 91 for a rim shot
    pub midi_note: u8,
    /// Tied into the next note
    pub tie_origin: bool,
    /// Continues a tied note, so it should not be played
    pub tie_destination: bool,
}

#[derive(Debug)]
pub enum GpError {
    Io(io::Error),
    Zip(zip::result::ZipError),
    Xml(quick_xml::DeError),
    NoDrumTrack,
    Invalid(String),
}

impl fmt::Display for GpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GpError::Io(e) => write!(f, "couldn't read file: {e}"),
            GpError::Zip(e) => write!(f, "not a Guitar Pro 8 file: {e}"),
            GpError::Xml(e) => write!(f, "couldn't parse score: {e}"),
            GpError::NoDrumTrack => write!(f, "score has no drum track"),
            GpError::Invalid(msg) => write!(f, "invalid score: {msg}"),
        }
    }
}

impl std::error::Error for GpError {}

impl From<io::Error> for GpError {
    fn from(e: io::Error) -> Self {
        GpError::Io(e)
    }
}

impl From<zip::result::ZipError> for GpError {
    fn from(e: zip::result::ZipError) -> Self {
        GpError::Zip(e)
    }
}

impl From<quick_xml::DeError> for GpError {
    fn from(e: quick_xml::DeError) -> Self {
        GpError::Xml(e)
    }
}

pub fn load(path: impl AsRef<Path>) -> Result<GpScore, GpError> {
    read(File::open(path)?)
}

pub fn read(reader: impl Read + Seek) -> Result<GpScore, GpError> {
    let mut archive = zip::ZipArchive::new(reader)?;
    let mut xml = String::new();
    archive.by_name(SCORE_ENTRY)?.read_to_string(&mut xml)?;
    parse_gpif(&xml)
}

/// Parse the contents of `Content/score.gpif`
pub fn parse_gpif(xml: &str) -> Result<GpScore, GpError> {
    let gpif: gpif::Gpif = quick_xml::de::from_str(xml)?;

    let (track_index, track) = gpif
        .tracks
        .track
        .iter()
        .enumerate()
        .find(|(_, track)| {
            track
                .instrument_set
                .as_ref()
                .is_some_and(|set| set.kind == "drumKit")
        })
        .ok_or(GpError::NoDrumTrack)?;
    let articulations: Vec<&gpif::Articulation> = track
        .instrument_set
        .iter()
        .flat_map(|set| &set.elements.element)
        .flat_map(|element| &element.articulations.articulation)
        .collect();

    let bars_by_id: HashMap<_, _> = gpif.bars.bar.iter().map(|b| (b.id, b)).collect();
    let voices_by_id: HashMap<_, _> = gpif.voices.voice.iter().map(|v| (v.id, v)).collect();
    let beats_by_id: HashMap<_, _> = gpif.beats.beat.iter().map(|b| (b.id, b)).collect();
    let notes_by_id: HashMap<_, _> = gpif.notes.note.iter().map(|n| (n.id, n)).collect();
    let rhythms_by_id: HashMap<_, _> = gpif.rhythms.rhythm.iter().map(|r| (r.id, r)).collect();

    let convert_note = |id: usize| -> Result<GpNote, GpError> {
        let note = lookup(&notes_by_id, id, "note")?;
        let articulation = note
            .instrument_articulation
            .and_then(|index| articulations.get(index));
        let midi_property = note
            .properties
            .property
            .iter()
            .find(|p| p.name == "Midi")
            .and_then(|p| p.number)
            .and_then(|n| u8::try_from(n).ok());
        // Older files have no input numbers, just the note it's played on
        let midi_note = articulation
            .map(|a| {
                a.input_midi_numbers
                    .as_deref()
                    .and_then(|numbers| numbers.split_whitespace().next()?.parse().ok())
                    .unwrap_or(a.output_midi_number)
            })
            .or(midi_property)
            .ok_or_else(|| GpError::Invalid(format!("note {id} has no drum sound")))?;
        Ok(GpNote {
            midi_note,
            tie_origin: note.tie.as_ref().is_some_and(|t| t.origin),
            tie_destination: note.tie.as_ref().is_some_and(|t| t.destination),
        })
    };

    let convert_rhythm = |id: usize| -> Result<GpRhythm, GpError> {
        let rhythm = lookup(&rhythms_by_id, id, "rhythm")?;
        let tuplet = |t: &gpif::Tuplet| Tuplet {
            notes: t.num,
            in_time_of: t.den,
        };
        Ok(GpRhythm {
            value: GpNoteValue::parse(&rhythm.note_value).ok_or_else(|| {
                GpError::Invalid(format!("unknown note value {:?}", rhythm.note_value))
            })?,
            dots: rhythm.augmentation_dot.as_ref().map_or(0, |d| d.count),
            tuplet: rhythm.primary_tuplet.as_ref().map(tuplet),
            inner_tuplet: rhythm.secondary_tuplet.as_ref().map(tuplet),
        })
    };

    let convert_voice = |id: usize| -> Result<GpVoice, GpError> {
        let voice = lookup(&voices_by_id, id, "voice")?;
        let mut start = Fraction::ZERO;
        let mut beats = vec![];
        for beat_id in parse_ids(&voice.beats)? {
            let beat = lookup(&beats_by_id, beat_id, "beat")?;
            let rhythm = convert_rhythm(beat.rhythm.id)?;
            let grace = match beat.grace_notes.as_deref() {
                None => None,
                Some("OnBeat") => Some(Grace::OnBeat),
                Some(_) => Some(Grace::BeforeBeat),
            };
            let notes = match &beat.notes {
                Some(ids) => parse_ids(ids)?
                    .into_iter()
                    .map(convert_note)
                    .collect::<Result<_, _>>()?,
                None => vec![],
            };
            let duration = rhythm.duration();
            let hairpin = match beat.hairpin.as_deref() {
                Some("Crescendo") => Some(Hairpin::Crescendo),
                Some("Decrescendo" | "Diminuendo") => Some(Hairpin::Decrescendo),
                _ => None,
            };
            beats.push(GpBeat {
                start,
                rhythm,
                grace,
                hairpin,
                notes,
            });
            if grace.is_none() {
                start = start + duration;
            }
        }
        Ok(GpVoice { beats })
    };

    let bars = gpif
        .master_bars
        .master_bar
        .iter()
        .map(|master_bar| {
            let bar_id = *parse_ids(&master_bar.bars)?
                .get(track_index)
                .ok_or_else(|| GpError::Invalid("master bar is missing a bar".into()))?;
            let bar = lookup(&bars_by_id, bar_id, "bar")?;
            let voices = bar
                .voices
                .split_whitespace()
                .filter(|id| *id != "-1")
                .map(|id| convert_voice(parse_id(id)?))
                .collect::<Result<_, _>>()?;
            let repeat = master_bar.repeat.as_ref();
            Ok(GpBar {
                time_signature: parse_time_signature(&master_bar.time)?,
                section: master_bar.section.as_ref().and_then(convert_section),
                repeat_start: repeat.is_some_and(|r| r.start),
                repeat_end: repeat.filter(|r| r.end).map(|r| r.count),
                voices,
            })
        })
        .collect::<Result<_, GpError>>()?;

    let tempo_changes = gpif
        .master_track
        .automations
        .iter()
        .flat_map(|a| &a.automation)
        .filter(|a| a.kind == "Tempo")
        .map(|a| {
            Ok(TempoChange {
                bar: a.bar,
                position: a.position,
                bpm: parse_tempo(&a.value)?,
            })
        })
        .collect::<Result<_, GpError>>()?;

    let score_text = |field: fn(&gpif::ScoreInfo) -> &Option<String>| {
        gpif.score
            .as_ref()
            .and_then(|s| field(s).as_deref())
            .unwrap_or_default()
            .trim()
            .to_string()
    };

    Ok(GpScore {
        title: score_text(|s| &s.title),
        artist: score_text(|s| &s.artist),
        track_name: track.name.as_deref().unwrap_or_default().trim().to_string(),
        tempo_changes,
        bars,
    })
}

fn lookup<'a, T>(map: &HashMap<usize, &'a T>, id: usize, kind: &str) -> Result<&'a T, GpError> {
    map.get(&id)
        .copied()
        .ok_or_else(|| GpError::Invalid(format!("missing {kind} {id}")))
}

fn parse_id(id: &str) -> Result<usize, GpError> {
    id.parse()
        .map_err(|_| GpError::Invalid(format!("bad id {id:?}")))
}

fn parse_ids(ids: &str) -> Result<Vec<usize>, GpError> {
    ids.split_whitespace().map(parse_id).collect()
}

/// A bar's section, dropping blank letters and names, and the section itself
/// when both are blank
fn convert_section(section: &gpif::Section) -> Option<Section> {
    let clean = |text: &Option<String>| {
        text.as_deref()
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(String::from)
    };
    let letter = clean(&section.letter);
    let name = clean(&section.text);
    (letter.is_some() || name.is_some()).then_some(Section { letter, name })
}

fn parse_time_signature(time: &str) -> Result<TimeSignature, GpError> {
    let invalid = || GpError::Invalid(format!("bad time signature {time:?}"));
    let (notes_per_measure, note_value) = time.trim().split_once('/').ok_or_else(invalid)?;
    Ok(TimeSignature {
        notes_per_measure: notes_per_measure.parse().map_err(|_| invalid())?,
        note_value: note_value.parse().map_err(|_| invalid())?,
    })
}

/// Tempo values look like "130 2": the BPM, then which note value counts as
/// the beat (1 = eighth, 2 = quarter, 3 = dotted quarter, 4 = half,
/// 5 = dotted half)
fn parse_tempo(value: &str) -> Result<f64, GpError> {
    let invalid = || GpError::Invalid(format!("bad tempo {value:?}"));
    let mut parts = value.split_whitespace();
    let bpm: f64 = parts
        .next()
        .and_then(|bpm| bpm.parse().ok())
        .ok_or_else(invalid)?;
    let quarters_per_beat = match parts.next() {
        None | Some("2") => 1.0,
        Some("1") => 0.5,
        Some("3") => 1.5,
        Some("4") => 2.0,
        Some("5") => 3.0,
        Some(_) => return Err(invalid()),
    };
    Ok(bpm * quarters_per_beat)
}

#[cfg(test)]
mod tests;
