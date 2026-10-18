use super::{measure::Measure, performance::Performance, tempo::TempoMap};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum HairpinKind {
    Crescendo,
    Decrescendo,
}

/// A crescendo or decrescendo, which can span several measures
#[derive(Clone, PartialEq, Debug)]
pub struct Hairpin {
    pub kind: HairpinKind,
    /// Where the hairpin starts and ends, in ticks since the beginning
    pub start: u32,
    pub end: u32,
}

#[derive(Clone, PartialEq)]
pub struct Score {
    pub title: String,
    pub measures: Vec<Measure>,
    /// Tempo changes on the written timeline
    pub tempo_map: TempoMap,
    /// How the score is played, with repeats expanded
    pub performance: Performance,
    /// Sorted by start
    pub hairpins: Vec<Hairpin>,
}

impl Score {
    pub fn new(title: String, measures: Vec<Measure>, tempo_map: TempoMap) -> Self {
        let performance = Performance::new(&measures, &tempo_map);
        Self {
            title,
            measures,
            tempo_map,
            performance,
            hairpins: vec![],
        }
    }

    pub fn empty() -> Self {
        Self::new(String::new(), vec![], TempoMap::new(120.0))
    }
}
