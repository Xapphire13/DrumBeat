use super::{measure::Measure, tempo::TempoMap};

#[derive(Clone, PartialEq)]
pub struct Score {
    pub measures: Vec<Measure>,
    pub tempo_map: TempoMap,
}

impl Score {
    /// Get the sorted, unique timestamps of every note that is played (tied
    /// continuations are skipped)
    pub fn get_note_timestamps(&self) -> Vec<u32> {
        let mut timestamps: Vec<u32> = self
            .measures
            .iter()
            .flat_map(|measure| &measure.notes)
            .filter(|note| !note.tied)
            .map(|note| note.timestamp)
            .collect();

        timestamps.sort_unstable();
        timestamps.dedup();
        timestamps
    }
}
