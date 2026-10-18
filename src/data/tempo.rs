use super::note::TICKS_PER_QUARTER;

#[derive(Clone, PartialEq, Debug)]
pub struct TempoChange {
    /// Where the tempo change happens, measured in ticks since the beginning
    pub timestamp: u32,
    /// Quarter notes per minute
    pub bpm: f64,
}

/// The tempo changes in a score, used to convert between ticks and real time
#[derive(Clone, PartialEq, Debug)]
pub struct TempoMap {
    /// Sorted by timestamp, always starting at 0
    changes: Vec<TempoChange>,
}

impl TempoMap {
    pub fn new(initial_bpm: f64) -> Self {
        Self {
            changes: vec![TempoChange {
                timestamp: 0,
                bpm: initial_bpm,
            }],
        }
    }

    /// Add a tempo change, replacing any existing change at the same timestamp
    pub fn add_change(&mut self, change: TempoChange) {
        self.changes.retain(|c| c.timestamp != change.timestamp);
        self.changes.push(change);
        self.changes.sort_by_key(|c| c.timestamp);
    }

    pub fn changes(&self) -> &[TempoChange] {
        &self.changes
    }

    /// The tempo in effect at `timestamp`
    pub fn bpm_at(&self, timestamp: u32) -> f64 {
        self.changes
            .iter()
            .take_while(|change| change.timestamp <= timestamp)
            .last()
            .map_or(self.changes[0].bpm, |change| change.bpm)
    }

    /// Convert a timestamp in ticks to seconds since the beginning of the score
    pub fn ticks_to_seconds(&self, timestamp: u32) -> f64 {
        let mut seconds = 0.0;

        for (i, change) in self.changes.iter().enumerate() {
            if change.timestamp >= timestamp {
                break;
            }

            let segment_end = self
                .changes
                .get(i + 1)
                .map_or(timestamp, |next| next.timestamp.min(timestamp));
            seconds += seconds_per_tick(change.bpm) * f64::from(segment_end - change.timestamp);
        }

        seconds
    }

    /// Convert seconds since the beginning of the score to a (fractional) timestamp in ticks
    pub fn seconds_to_ticks(&self, seconds: f64) -> f64 {
        let mut elapsed = 0.0;

        for (i, change) in self.changes.iter().enumerate() {
            let tick_seconds = seconds_per_tick(change.bpm);

            if let Some(next) = self.changes.get(i + 1) {
                let segment_seconds = tick_seconds * f64::from(next.timestamp - change.timestamp);
                if elapsed + segment_seconds > seconds {
                    return f64::from(change.timestamp) + (seconds - elapsed) / tick_seconds;
                }
                elapsed += segment_seconds;
            } else {
                return f64::from(change.timestamp) + (seconds - elapsed) / tick_seconds;
            }
        }

        0.0
    }
}

fn seconds_per_tick(bpm: f64) -> f64 {
    60.0 / (bpm * f64::from(TICKS_PER_QUARTER))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_tempo() {
        let tempo_map = TempoMap::new(120.0);
        assert!((tempo_map.ticks_to_seconds(960) - 0.5).abs() < 1e-9);
        assert!((tempo_map.seconds_to_ticks(1.0) - 1920.0).abs() < 1e-9);
    }

    #[test]
    fn tempo_change() {
        let mut tempo_map = TempoMap::new(120.0);
        tempo_map.add_change(TempoChange {
            timestamp: 1920,
            bpm: 60.0,
        });

        // Two quarters at 120 bpm, then one quarter at 60 bpm
        assert!((tempo_map.ticks_to_seconds(2880) - 2.0).abs() < 1e-9);
        assert!((tempo_map.seconds_to_ticks(2.0) - 2880.0).abs() < 1e-9);
    }
}
