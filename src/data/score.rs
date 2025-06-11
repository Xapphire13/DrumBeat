use super::measure::Measure;

#[derive(Clone, PartialEq)]
pub struct Score {
    pub measures: Vec<Measure>,
}

impl Score {
    pub fn get_note_timestamps(&self) -> Vec<u32> {
        let mut timestamps = vec![];

        for measure in &self.measures {
            for note in &measure.notes {
                if let Some(&last_timestamp) = timestamps.last() {
                    if last_timestamp < note.timestamp {
                        timestamps.push(note.timestamp);
                    }
                } else {
                    timestamps.push(note.timestamp);
                }
            }
        }

        timestamps
    }
}
