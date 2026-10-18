use super::{
    measure::Measure,
    tempo::{TempoChange, TempoMap},
};

/// A measure as it is played, on the performance timeline
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct PlayedMeasure {
    /// Index into the score's measures
    pub measure_index: usize,
    /// Where the measure is played, in ticks since the start of the performance
    pub timestamp: u32,
    /// Where the measure is written, in ticks since the start of the score
    pub written_timestamp: u32,
    pub length: u32,
}

impl PlayedMeasure {
    fn contains(&self, timestamp: u32) -> bool {
        self.timestamp <= timestamp && timestamp < self.timestamp + self.length
    }

    fn contains_written(&self, timestamp: u32) -> bool {
        self.written_timestamp <= timestamp && timestamp < self.written_timestamp + self.length
    }
}

/// The score as it is played, with repeats expanded. It has its own tick
/// timeline, which runs past the written one whenever a passage repeats
#[derive(Clone, PartialEq, Debug)]
pub struct Performance {
    measures: Vec<PlayedMeasure>,
    tempo_map: TempoMap,
}

impl Performance {
    pub fn new(measures: &[Measure], written_tempo_map: &TempoMap) -> Self {
        let mut played = vec![];
        let mut tempo_map = TempoMap::new(written_tempo_map.bpm_at(0));
        let mut timestamp = 0;

        for measure_index in play_order(measures) {
            let measure = &measures[measure_index];
            let length = measure.get_length();

            // The tempo carries over from wherever the previous played measure
            // was, so restate it at the start of every played measure
            tempo_map.add_change(TempoChange {
                timestamp,
                bpm: written_tempo_map.bpm_at(measure.timestamp),
            });
            for change in written_tempo_map.changes() {
                if change.timestamp > measure.timestamp
                    && change.timestamp < measure.end_timestamp()
                {
                    tempo_map.add_change(TempoChange {
                        timestamp: timestamp + change.timestamp - measure.timestamp,
                        bpm: change.bpm,
                    });
                }
            }

            played.push(PlayedMeasure {
                measure_index,
                timestamp,
                written_timestamp: measure.timestamp,
                length,
            });
            timestamp += length;
        }

        Self {
            measures: played,
            tempo_map,
        }
    }

    pub fn measures(&self) -> &[PlayedMeasure] {
        &self.measures
    }

    pub fn tempo_map(&self) -> &TempoMap {
        &self.tempo_map
    }

    pub fn end_timestamp(&self) -> u32 {
        self.measures
            .last()
            .map_or(0, |measure| measure.timestamp + measure.length)
    }

    /// Index into `measures()` of the played measure containing `timestamp`,
    /// or the last one when `timestamp` is past the end
    pub fn played_measure_index(&self, timestamp: u32) -> usize {
        self.measures
            .partition_point(|measure| measure.timestamp <= timestamp)
            .saturating_sub(1)
    }

    /// Where a performance timestamp is written in the score
    pub fn to_written(&self, timestamp: u32) -> u32 {
        let Some(measure) = self.measures.get(self.played_measure_index(timestamp)) else {
            return 0;
        };
        if measure.contains(timestamp) {
            measure.written_timestamp + timestamp - measure.timestamp
        } else {
            measure.written_timestamp + measure.length
        }
    }

    /// Where a written timestamp is played. A repeated passage is played more
    /// than once, so this picks the time closest to `near`
    pub fn to_performance(&self, written_timestamp: u32, near: u32) -> Option<u32> {
        self.measures
            .iter()
            .filter(|measure| measure.contains_written(written_timestamp))
            .map(|measure| measure.timestamp + written_timestamp - measure.written_timestamp)
            .min_by_key(|&timestamp| timestamp.abs_diff(near))
    }

    /// Where a place `offset` ticks into a written measure is played, on the
    /// pass nearest `near`. The offset can be the measure's length, for its
    /// closing bar line
    pub fn locate(&self, measure_index: usize, offset: u32, near: u32) -> Option<u32> {
        self.measures
            .iter()
            .filter(|played| played.measure_index == measure_index && offset <= played.length)
            .map(|played| played.timestamp + offset)
            .min_by_key(|&timestamp| timestamp.abs_diff(near))
    }

    /// Sorted, unique performance timestamps of every note that is played
    /// (tied continuations are skipped)
    pub fn note_timestamps(&self, measures: &[Measure]) -> Vec<u32> {
        let mut timestamps: Vec<u32> = self
            .measures
            .iter()
            .flat_map(|played| {
                measures[played.measure_index]
                    .notes
                    .iter()
                    .filter(|note| !note.tied)
                    .map(move |note| played.timestamp + note.timestamp - played.written_timestamp)
            })
            .collect();

        timestamps.sort_unstable();
        timestamps.dedup();
        timestamps
    }

    /// Sorted, unique performance timestamps the score can be snapped to:
    /// the start of every played measure and every note in it, as the
    /// score's playhead snaps when clicked
    pub fn snap_points(&self, measures: &[Measure]) -> Vec<u32> {
        let mut timestamps: Vec<u32> = self
            .measures
            .iter()
            .flat_map(|played| {
                let notes = measures[played.measure_index]
                    .notes
                    .iter()
                    .map(move |note| played.timestamp + note.timestamp - played.written_timestamp);
                std::iter::once(played.timestamp).chain(notes)
            })
            .collect();

        timestamps.sort_unstable();
        timestamps.dedup();
        timestamps
    }

    /// The 1-based numbers a written measure is played as, counting played
    /// measures from the start of the performance
    pub fn measure_numbers(&self, measure_index: usize) -> Vec<usize> {
        self.measures
            .iter()
            .enumerate()
            .filter(|(_, played)| played.measure_index == measure_index)
            .map(|(i, _)| i + 1)
            .collect()
    }

    /// Which time through its repeat the played measure at `index` is, and
    /// how many times it is played in total
    pub fn pass(&self, index: usize) -> (usize, usize) {
        let Some(current) = self.measures.get(index) else {
            return (1, 1);
        };
        let same_measure =
            |measure: &&PlayedMeasure| measure.measure_index == current.measure_index;
        let pass = self.measures[..=index].iter().filter(same_measure).count();
        let total = self.measures.iter().filter(same_measure).count();
        (pass, total)
    }
}

/// The order measures are played in, following repeat signs. A repeat with
/// no start sign goes back to the beginning, or to just after the last
/// repeat that finished
fn play_order(measures: &[Measure]) -> Vec<usize> {
    let mut order = vec![];
    let mut section_start = 0;
    // How many times each repeat end has been reached
    let mut times_reached = vec![0; measures.len()];
    let mut i = 0;

    while i < measures.len() {
        let measure = &measures[i];
        if measure.repeat_start {
            section_start = i;
        }
        order.push(i);

        if let Some(count) = measure.repeat_end {
            times_reached[i] += 1;
            if times_reached[i] < count {
                i = section_start;
                continue;
            }
            section_start = i + 1;
        }
        i += 1;
    }

    order
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::measure::Fill;
    use crate::data::{note::NoteValue, time_signature::TimeSignature};

    /// 4/4 measures, `(repeat_start, repeat_end)` for each
    fn measures(repeats: &[(bool, Option<u32>)]) -> Vec<Measure> {
        repeats
            .iter()
            .enumerate()
            .map(|(i, &(repeat_start, repeat_end))| Measure {
                timestamp: i as u32 * 3840,
                notes: vec![],
                time_signature: TimeSignature {
                    notes_per_measure: 4,
                    note_value: NoteValue::Quarter,
                },
                repeat_start,
                rests: vec![],
                grace_notes: vec![],
                repeat_end,
                fill: Fill::Full,
            })
            .collect()
    }

    #[test]
    fn plays_straight_through_without_repeats() {
        let measures = measures(&[(false, None); 3]);
        assert_eq!(play_order(&measures), vec![0, 1, 2]);
    }

    #[test]
    fn repeats_a_single_measure() {
        let measures = measures(&[(true, Some(4)), (false, None)]);
        assert_eq!(play_order(&measures), vec![0, 0, 0, 0, 1]);
    }

    #[test]
    fn repeats_a_passage() {
        let measures = measures(&[
            (false, None),
            (true, None),
            (false, Some(2)),
            (true, None),
            (false, Some(3)),
        ]);
        assert_eq!(play_order(&measures), vec![0, 1, 2, 1, 2, 3, 4, 3, 4, 3, 4]);
    }

    #[test]
    fn repeat_without_start_goes_back_to_last_repeat() {
        let measures = measures(&[(false, Some(2)), (false, None), (false, Some(2))]);
        assert_eq!(play_order(&measures), vec![0, 0, 1, 2, 1, 2]);
    }

    #[test]
    fn maps_between_timelines() {
        let mut tempo_map = TempoMap::new(120.0);
        tempo_map.add_change(TempoChange {
            timestamp: 3840 + 1920,
            bpm: 60.0,
        });
        let measures = measures(&[(false, None), (true, Some(2)), (false, None)]);
        let performance = Performance::new(&measures, &tempo_map);

        assert_eq!(performance.end_timestamp(), 4 * 3840);
        // Halfway through the second time through the repeat
        assert_eq!(performance.to_written(2 * 3840 + 1920), 3840 + 1920);
        assert_eq!(
            performance.to_performance(3840 + 1920, 0),
            Some(3840 + 1920)
        );
        assert_eq!(
            performance.to_performance(3840 + 1920, 3 * 3840),
            Some(2 * 3840 + 1920)
        );
        assert_eq!(performance.pass(2), (2, 2));
        assert_eq!(performance.measure_numbers(1), vec![2, 3]);
        assert_eq!(performance.measure_numbers(2), vec![4]);

        // The tempo drops halfway through the repeated measure both times,
        // and is restored when it repeats
        let tempo = performance.tempo_map();
        assert!((tempo.bpm_at(3840) - 120.0).abs() < 1e-9);
        assert!((tempo.bpm_at(3840 + 1920) - 60.0).abs() < 1e-9);
        assert!((tempo.bpm_at(2 * 3840) - 120.0).abs() < 1e-9);
        assert!((tempo.bpm_at(2 * 3840 + 1920) - 60.0).abs() < 1e-9);
    }
}
