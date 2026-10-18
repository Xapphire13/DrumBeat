//! Serde mirror of the parts of `Content/score.gpif` the loader reads.
//!
//! Guitar Pro stores each kind of object (bars, voices, beats, notes,
//! rhythms) in its own flat list, deduplicated, and links them together with
//! space-separated id lists. These types keep that shape; `gp.rs` resolves
//! the ids into a tree.

use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct Gpif {
    pub score: Option<ScoreInfo>,
    pub master_track: MasterTrack,
    #[serde(default)]
    pub tracks: Tracks,
    #[serde(default)]
    pub master_bars: MasterBars,
    #[serde(default)]
    pub bars: Bars,
    #[serde(default)]
    pub voices: Voices,
    #[serde(default)]
    pub beats: Beats,
    #[serde(default)]
    pub notes: Notes,
    #[serde(default)]
    pub rhythms: Rhythms,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct ScoreInfo {
    pub title: Option<String>,
    pub artist: Option<String>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct MasterTrack {
    pub automations: Option<Automations>,
}

#[derive(Deserialize, Debug, Default)]
pub struct Automations {
    #[serde(rename = "Automation", default)]
    pub automation: Vec<Automation>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct Automation {
    #[serde(rename = "Type")]
    pub kind: String,
    pub bar: usize,
    /// Fraction of the bar, from 0 to 1
    #[serde(default)]
    pub position: f64,
    /// For tempo, "<bpm> <reference unit>", e.g. "130 2"
    pub value: String,
}

#[derive(Deserialize, Debug, Default)]
pub struct Tracks {
    #[serde(rename = "Track", default)]
    pub track: Vec<Track>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct Track {
    pub name: Option<String>,
    pub instrument_set: Option<InstrumentSet>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct InstrumentSet {
    #[serde(rename = "Type")]
    pub kind: String,
    #[serde(default)]
    pub elements: Elements,
}

#[derive(Deserialize, Debug, Default)]
pub struct Elements {
    #[serde(rename = "Element", default)]
    pub element: Vec<Element>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct Element {
    pub name: String,
    #[serde(default)]
    pub articulations: Articulations,
}

#[derive(Deserialize, Debug, Default)]
pub struct Articulations {
    #[serde(rename = "Articulation", default)]
    pub articulation: Vec<Articulation>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct Articulation {
    /// Guitar Pro's own numbers for the sound, e.g. "91" for a rim shot,
    /// which tell sounds apart that share an output number
    pub input_midi_numbers: Option<String>,
    /// The General MIDI note Guitar Pro plays the sound on
    pub output_midi_number: u8,
}

#[derive(Deserialize, Debug, Default)]
pub struct MasterBars {
    #[serde(rename = "MasterBar", default)]
    pub master_bar: Vec<MasterBar>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct MasterBar {
    /// e.g. "4/4"
    pub time: String,
    pub repeat: Option<Repeat>,
    pub section: Option<Section>,
    /// One bar id per track, in track order
    pub bars: String,
}

#[derive(Deserialize, Debug)]
pub struct Repeat {
    #[serde(rename = "@start", default)]
    pub start: bool,
    #[serde(rename = "@end", default)]
    pub end: bool,
    #[serde(rename = "@count", default)]
    pub count: u32,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct Section {
    pub text: Option<String>,
}

#[derive(Deserialize, Debug, Default)]
pub struct Bars {
    #[serde(rename = "Bar", default)]
    pub bar: Vec<Bar>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct Bar {
    #[serde(rename = "@id")]
    pub id: usize,
    /// Up to four voice ids, -1 for an unused voice
    pub voices: String,
}

#[derive(Deserialize, Debug, Default)]
pub struct Voices {
    #[serde(rename = "Voice", default)]
    pub voice: Vec<Voice>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct Voice {
    #[serde(rename = "@id")]
    pub id: usize,
    #[serde(default)]
    pub beats: String,
}

#[derive(Deserialize, Debug, Default)]
pub struct Beats {
    #[serde(rename = "Beat", default)]
    pub beat: Vec<Beat>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct Beat {
    #[serde(rename = "@id")]
    pub id: usize,
    pub rhythm: RhythmRef,
    /// Absent for a rest
    pub notes: Option<String>,
    /// "BeforeBeat" or "OnBeat"
    pub grace_notes: Option<String>,
    /// "Crescendo" or "Decrescendo" while the beat is under a hairpin
    pub hairpin: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct RhythmRef {
    #[serde(rename = "@ref")]
    pub id: usize,
}

#[derive(Deserialize, Debug, Default)]
pub struct Notes {
    #[serde(rename = "Note", default)]
    pub note: Vec<Note>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct Note {
    #[serde(rename = "@id")]
    pub id: usize,
    /// Index into the track's articulations, flattened across elements
    pub instrument_articulation: Option<usize>,
    pub tie: Option<Tie>,
    #[serde(default)]
    pub properties: Properties,
}

#[derive(Deserialize, Debug)]
pub struct Tie {
    #[serde(rename = "@origin", default)]
    pub origin: bool,
    #[serde(rename = "@destination", default)]
    pub destination: bool,
}

#[derive(Deserialize, Debug, Default)]
pub struct Properties {
    #[serde(rename = "Property", default)]
    pub property: Vec<Property>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct Property {
    #[serde(rename = "@name")]
    pub name: String,
    pub number: Option<i32>,
}

#[derive(Deserialize, Debug, Default)]
pub struct Rhythms {
    #[serde(rename = "Rhythm", default)]
    pub rhythm: Vec<Rhythm>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct Rhythm {
    #[serde(rename = "@id")]
    pub id: usize,
    pub note_value: String,
    pub augmentation_dot: Option<AugmentationDot>,
    pub primary_tuplet: Option<Tuplet>,
    pub secondary_tuplet: Option<Tuplet>,
}

#[derive(Deserialize, Debug)]
pub struct AugmentationDot {
    #[serde(rename = "@count")]
    pub count: u8,
}

#[derive(Deserialize, Debug)]
pub struct Tuplet {
    #[serde(rename = "@num")]
    pub num: u32,
    #[serde(rename = "@den")]
    pub den: u32,
}
