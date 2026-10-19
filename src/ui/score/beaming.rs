use crate::data::{
    drum_piece::DrumPiece,
    drum_voice::NoteHead,
    measure::Measure,
    note::{Annotation, Note, NoteValue, TICKS_PER_QUARTER, Tuplet},
};

use super::measure_scale::MeasureScale;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum StemDirection {
    Up,
    Down,
}

/// Notes in one voice that start together and share a stem
#[derive(Clone, PartialEq, Debug)]
pub struct Chord {
    pub timestamp: u32,
    /// The shortest value among the notes, which sets the beams or flags
    pub value: NoteValue,
    /// Length of the shortest note, in ticks
    pub ticks: u32,
    /// Staff line number of each note head
    pub lines: Vec<f32>,
    /// Whether each note head is an X, matching `lines`
    pub cymbals: Vec<bool>,
    /// The tuplet the shortest note is in, if any
    pub tuplet: Option<Tuplet>,
    /// Identifies which tuplet (of those in the voice) the chord is part of,
    /// since consecutive tuplets of the same kind are separate groups
    pub tuplet_group: Option<usize>,
    /// A sign to draw beyond the stem, e.g. an open hi-hat's "o"
    pub annotation: Option<Annotation>,
}

impl Chord {
    /// How many beams (or flags) the chord's stem gets
    pub fn beams(&self) -> usize {
        match self.value {
            NoteValue::Whole | NoteValue::Half | NoteValue::Quarter => 0,
            NoteValue::Eighth => 1,
            NoteValue::Sixteenth => 2,
            NoteValue::ThirtySecond => 3,
            NoteValue::SixtyFourth => 4,
        }
    }

    pub fn has_stem(&self) -> bool {
        self.value != NoteValue::Whole
    }

    fn top_line(&self) -> f32 {
        self.lines.iter().copied().fold(f32::MIN, f32::max)
    }

    fn bottom_line(&self) -> f32 {
        self.lines.iter().copied().fold(f32::MAX, f32::min)
    }
}

/// Chords joined by beams, or a single chord drawn on its own
#[derive(Clone, PartialEq, Debug)]
pub struct BeamGroup {
    /// The voice, and whether it's the hands or feet, as one id
    pub layer: u8,
    pub direction: StemDirection,
    pub chords: Vec<Chord>,
}

impl BeamGroup {
    pub fn max_beams(&self) -> usize {
        self.chords.iter().map(Chord::beams).max().unwrap_or(0)
    }

    /// The staff line the stems end on, where the outermost beam is drawn.
    /// Stems are at least 2 lines long, and longer when there are more
    /// beams to fit, but stay within `(lowest, highest)`
    pub fn stem_end_line(&self, (lowest, highest): (f32, f32)) -> f32 {
        // Keep room beyond the stems for hi-hat signs and tuplet numbers
        let marks_room = [
            self.chords.iter().any(|chord| chord.annotation.is_some()),
            self.chords.iter().any(|chord| chord.tuplet.is_some()),
        ]
        .into_iter()
        .filter(|&has_mark| has_mark)
        .count() as f32
            * MARK_ROOM;
        let (lowest, highest) = (lowest + marks_room, highest - marks_room);
        let extra_beams = self.max_beams().saturating_sub(1) as f32 * BEAM_SPACING;
        match self.direction {
            StemDirection::Up => {
                let top = self
                    .chords
                    .iter()
                    .map(Chord::top_line)
                    .fold(f32::MIN, f32::max);
                ((top + 2.0).max(5.0) + extra_beams).min(highest)
            }
            StemDirection::Down => {
                let bottom = self
                    .chords
                    .iter()
                    .map(Chord::bottom_line)
                    .fold(f32::MAX, f32::min);
                ((bottom - 2.0).min(-2.0) - extra_beams).max(lowest)
            }
        }
    }

    /// The staff line of the `level`th beam (1 is the outermost)
    pub fn beam_line(&self, stem_end_line: f32, level: usize) -> f32 {
        let offset = (level - 1) as f32 * BEAM_SPACING;
        match self.direction {
            StemDirection::Up => stem_end_line - offset,
            StemDirection::Down => stem_end_line + offset,
        }
    }

    /// For each beam level, the runs of chords it joins as `(first, last)`
    /// indices. A run of one chord gets a short stub beam
    pub fn beam_runs(&self, level: usize) -> Vec<(usize, usize)> {
        let mut runs = vec![];
        let mut start = None;
        for (i, chord) in self.chords.iter().enumerate() {
            match (chord.beams() >= level, start) {
                (true, None) => start = Some(i),
                (false, Some(first)) => {
                    runs.push((first, i - 1));
                    start = None;
                }
                _ => {}
            }
        }
        if let Some(first) = start {
            runs.push((first, self.chords.len() - 1));
        }
        runs
    }
}

/// Room for one row of signs beyond the stems, in staff lines
const MARK_ROOM: f32 = 0.8;

/// Distance between beams, in staff lines
pub const BEAM_SPACING: f32 = 0.4;

/// Group a measure's notes into chords, and the chords into beamed groups.
/// The hands have stems up and the feet stems down, whichever voice they're
/// in. Beams join eighths and shorter notes within a beat, and break at
/// rests and around tuplets. In simple meters, two beats of plain eighths in the same half of the bar are beamed as one group
/// of four
pub fn beam_groups(measure: &Measure) -> Vec<BeamGroup> {
    let beat = beat_ticks(measure);
    // Each voice's hands and feet are beamed separately, except within a
    // tuplet that mixes them, or a beat where they take turns, which stay one
    // group with stems up
    let mut mixed = mixed_tuplet_spans(measure);
    mixed.extend(alternating_beats(measure, beat));
    let is_feet = |note: &Note, drum_piece: DrumPiece| {
        drum_piece.is_played_with_feet()
            && !mixed.iter().any(|&(voice, start, end)| {
                note.voice == voice && start <= note.timestamp && note.timestamp < end
            })
    };
    let mut layers: Vec<(u8, bool)> = measure
        .notes
        .iter()
        .filter_map(|note| Some((note.voice, is_feet(note, note.drum_piece()?))))
        .collect();
    layers.sort_unstable();
    layers.dedup();

    let mut groups = vec![];
    for (voice, feet) in layers {
        let layer = voice * 2 + u8::from(feet);
        let direction = if feet {
            StemDirection::Down
        } else {
            StemDirection::Up
        };

        let mut voice_groups = vec![];
        let mut current: Vec<Chord> = vec![];
        let notes = measure
            .notes
            .iter()
            .filter(|note| note.voice == voice)
            .filter_map(|note| Some((note, note.drum_piece()?)))
            .filter(|&(note, drum_piece)| is_feet(note, drum_piece) == feet)
            .collect();
        for chord in chords(notes) {
            let joins = current.last().is_some_and(|prev| {
                prev.beams() > 0
                    && chord.beams() > 0
                    && (prev.timestamp - measure.timestamp) / beat
                        == (chord.timestamp - measure.timestamp) / beat
                    && prev.timestamp + prev.ticks >= chord.timestamp
                    // A tuplet is beamed on its own
                    && prev.tuplet_group == chord.tuplet_group
            });
            if !joins && !current.is_empty() {
                voice_groups.push(BeamGroup {
                    layer,
                    direction,
                    chords: std::mem::take(&mut current),
                });
            }
            current.push(chord);
        }
        if !current.is_empty() {
            voice_groups.push(BeamGroup {
                layer,
                direction,
                chords: current,
            });
        }
        groups.extend(join_lone_eighths(join_eighth_beats(measure, voice_groups)));
    }

    groups
}

/// Join two beats of plain eighths into one group of four, when the first
/// beat starts a half of a simple-meter bar
fn join_eighth_beats(measure: &Measure, groups: Vec<BeamGroup>) -> Vec<BeamGroup> {
    let time_signature = &measure.time_signature;
    if time_signature.note_value != NoteValue::Quarter
        || !time_signature.notes_per_measure.is_multiple_of(2)
    {
        return groups;
    }

    // The beat a group fills exactly with plain eighths, if it does
    let eighth_beat = |group: &BeamGroup| {
        let eighth = NoteValue::Eighth.get_ticks();
        let [first, second] = group.chords.as_slice() else {
            return None;
        };
        let plain = |chord: &Chord| chord.ticks == eighth && chord.tuplet.is_none();
        let offset = first.timestamp - measure.timestamp;
        (plain(first)
            && plain(second)
            && offset.is_multiple_of(TICKS_PER_QUARTER)
            && second.timestamp == first.timestamp + eighth)
            .then_some(offset / TICKS_PER_QUARTER)
    };

    let mut joined: Vec<BeamGroup> = vec![];
    for group in groups {
        if let Some(prev) = joined.last_mut()
            && let (Some(prev_beat), Some(beat)) = (eighth_beat(prev), eighth_beat(&group))
            && prev_beat.is_multiple_of(2)
            && beat == prev_beat + 1
        {
            prev.chords.extend(group.chords);
            continue;
        }
        joined.push(group);
    }
    joined
}

/// Beam two lone eighths that follow each other into a pair, even across a
/// beat, so they read as a pair rather than two flagged notes
fn join_lone_eighths(groups: Vec<BeamGroup>) -> Vec<BeamGroup> {
    let eighth = NoteValue::Eighth.get_ticks();
    let lone_eighth = |group: &BeamGroup| match group.chords.as_slice() {
        [chord] if chord.ticks == eighth && chord.tuplet.is_none() => Some(chord.timestamp),
        _ => None,
    };

    let mut joined: Vec<BeamGroup> = vec![];
    for group in groups {
        if let Some(prev) = joined.last_mut()
            && let (Some(prev_timestamp), Some(timestamp)) =
                (lone_eighth(prev), lone_eighth(&group))
            && prev_timestamp + eighth == timestamp
        {
            prev.chords.extend(group.chords);
            continue;
        }
        joined.push(group);
    }
    joined
}

/// A number (and, when its notes aren't beamed together, a bracket) over
/// the notes of a tuplet
#[derive(Clone, PartialEq, Debug)]
pub struct TupletMark {
    pub notes: u32,
    pub direction: StemDirection,
    /// `(group, chord)` indices of the first and last chord in the tuplet
    pub first: (usize, usize),
    pub last: (usize, usize),
}

impl TupletMark {
    /// Whether the tuplet's notes are all joined by one beam, so only the
    /// number is needed
    pub fn is_beamed(&self, groups: &[BeamGroup]) -> bool {
        self.first.0 == self.last.0 && groups[self.first.0].chords.len() > 1
    }
}

/// Find the tuplets among the groups' chords
pub fn tuplet_marks(groups: &[BeamGroup]) -> Vec<TupletMark> {
    let mut marks: Vec<TupletMark> = vec![];
    // The layer and tuplet group of the last mark
    let mut current: Option<(u8, usize)> = None;

    for (group_index, group) in groups.iter().enumerate() {
        for (chord_index, chord) in group.chords.iter().enumerate() {
            let position = (group_index, chord_index);
            let (Some(tuplet), Some(tuplet_group)) = (chord.tuplet, chord.tuplet_group) else {
                current = None;
                continue;
            };
            if current == Some((group.layer, tuplet_group))
                && let Some(mark) = marks.last_mut()
            {
                mark.last = position;
            } else {
                marks.push(TupletMark {
                    notes: tuplet.notes,
                    direction: group.direction,
                    first: position,
                    last: position,
                });
                current = Some((group.layer, tuplet_group));
            }
        }
    }
    marks
}

/// `(voice, start, end)` of each tuplet with both hands and feet in it. A
/// tuplet lasts `in_time_of` of its first note's value
fn mixed_tuplet_spans(measure: &Measure) -> Vec<(u8, u32, u32)> {
    let mut notes: Vec<_> = measure
        .notes
        .iter()
        .filter_map(|note| Some((note, note.drum_piece()?, note.duration.tuplet?)))
        .collect();
    notes.sort_by_key(|(note, ..)| (note.voice, note.timestamp));

    // (voice, tuplet, start, end, has hands, has feet)
    let mut spans: Vec<(u8, Tuplet, u32, u32, bool, bool)> = vec![];
    for (note, drum_piece, tuplet) in notes {
        let feet = drum_piece.is_played_with_feet();
        match spans.last_mut() {
            Some((voice, current, _, end, has_hands, has_feet))
                if *voice == note.voice && *current == tuplet && note.timestamp < *end =>
            {
                *has_hands |= !feet;
                *has_feet |= feet;
            }
            _ => spans.push((
                note.voice,
                tuplet,
                note.timestamp,
                note.timestamp + tuplet.in_time_of * note.duration.value.get_ticks(),
                !feet,
                feet,
            )),
        }
    }

    spans
        .into_iter()
        .filter(|&(.., has_hands, has_feet)| has_hands && has_feet)
        .map(|(voice, _, start, end, ..)| (voice, start, end))
        .collect()
}

/// `(voice, start, end)` of each beat where the hands and feet take turns
/// playing one continuous run of short notes, never together, e.g. two
/// 32nds on the toms then two on the kick. Splitting them into hands and
/// feet would hide the run
fn alternating_beats(measure: &Measure, beat: u32) -> Vec<(u8, u32, u32)> {
    let mut voices: Vec<u8> = measure.notes.iter().map(|note| note.voice).collect();
    voices.sort_unstable();
    voices.dedup();

    let mut spans = vec![];
    for voice in voices {
        for start in (measure.timestamp..measure.end_timestamp()).step_by(beat as usize) {
            let end = start + beat;
            // (timestamp, shortest length, has hands, has feet) of each onset
            let mut onsets: Vec<(u32, u32, bool, bool)> = vec![];
            let mut all_beamable = true;
            for note in measure.notes.iter().filter(|note| {
                note.voice == voice && start <= note.timestamp && note.timestamp < end
            }) {
                let Some(drum_piece) = note.drum_piece() else {
                    continue;
                };
                // Tuplets are grouped on their own, so a beat with one in it
                // stays split
                all_beamable &= note.duration.value.get_ticks() <= NoteValue::Eighth.get_ticks()
                    && note.duration.tuplet.is_none();
                let feet = drum_piece.is_played_with_feet();
                let ticks = note.duration.get_ticks();
                match onsets.iter_mut().find(|onset| onset.0 == note.timestamp) {
                    Some(onset) => {
                        onset.1 = onset.1.min(ticks);
                        onset.2 |= !feet;
                        onset.3 |= feet;
                    }
                    None => onsets.push((note.timestamp, ticks, !feet, feet)),
                }
            }
            onsets.sort_unstable();

            let has_hands = onsets.iter().any(|onset| onset.2);
            let has_feet = onsets.iter().any(|onset| onset.3);
            let never_together = onsets.iter().all(|onset| !(onset.2 && onset.3));
            let continuous = onsets
                .windows(2)
                .all(|pair| pair[0].0 + pair[0].1 == pair[1].0);
            if all_beamable && has_hands && has_feet && never_together && continuous {
                spans.push((voice, start, end));
            }
        }
    }
    spans
}

/// Whether the note's head is an X, whose stem starts from a corner
fn has_x_head(note: &Note) -> bool {
    note.drum_voice()
        .is_some_and(|voice| voice.head() == NoteHead::X)
}

/// The notes of one voice that are drawn, grouped by start time
fn chords(mut notes: Vec<(&Note, DrumPiece)>) -> Vec<Chord> {
    notes.sort_by_key(|(note, _)| note.timestamp);

    let mut chords: Vec<Chord> = vec![];
    for (note, drum_piece) in notes {
        let line = MeasureScale::get_line_number(drum_piece);
        let ticks = note.duration.get_ticks();
        match chords.last_mut() {
            Some(chord) if chord.timestamp == note.timestamp => {
                chord.lines.push(line);
                chord.cymbals.push(has_x_head(note));
                chord.annotation = chord.annotation.or(note.annotation);
                if ticks < chord.ticks {
                    chord.ticks = ticks;
                    chord.value = note.duration.value;
                    chord.tuplet = note.duration.tuplet;
                }
            }
            _ => chords.push(Chord {
                timestamp: note.timestamp,
                value: note.duration.value,
                ticks,
                lines: vec![line],
                cymbals: vec![has_x_head(note)],
                tuplet: note.duration.tuplet,
                tuplet_group: None,
                annotation: note.annotation,
            }),
        }
    }

    // A tuplet lasts `in_time_of` of its first note's value, e.g. a
    // sixteenth triplet lasts two sixteenths
    let mut current: Option<(Tuplet, u32, usize)> = None;
    let mut next_group = 0;
    for chord in &mut chords {
        chord.tuplet_group = match (chord.tuplet, current) {
            (Some(tuplet), Some((current_tuplet, end, group)))
                if tuplet == current_tuplet && chord.timestamp < end =>
            {
                Some(group)
            }
            (Some(tuplet), _) => {
                let end = chord.timestamp + tuplet.in_time_of * chord.value.get_ticks();
                current = Some((tuplet, end, next_group));
                next_group += 1;
                Some(next_group - 1)
            }
            (None, _) => {
                current = None;
                None
            }
        };
    }
    chords
}

/// The length of a beat, which beams don't cross. Compound meters like 6/8
/// are beamed in dotted quarters, everything else in quarters
fn beat_ticks(measure: &Measure) -> u32 {
    let time_signature = &measure.time_signature;
    let is_compound = time_signature.note_value == NoteValue::Eighth
        && time_signature.notes_per_measure > 3
        && time_signature.notes_per_measure.is_multiple_of(3);

    if is_compound {
        TICKS_PER_QUARTER * 3 / 2
    } else {
        TICKS_PER_QUARTER
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::measure::Fill;
    use crate::data::{
        note::{Duration, Note},
        time_signature::TimeSignature,
    };

    const SNARE: u8 = 38;
    const HIHAT: u8 = 42;
    const KICK: u8 = 36;

    fn note(timestamp: u32, value: NoteValue, midi_note: u8, voice: u8) -> Note {
        Note {
            duration: Duration::new(value),
            midi_note,
            timestamp,
            tied: false,
            voice,
            annotation: None,
        }
    }

    fn measure(notes: Vec<Note>) -> Measure {
        Measure {
            timestamp: 0,
            notes,
            time_signature: TimeSignature {
                notes_per_measure: 4,
                note_value: NoteValue::Quarter,
            },
            repeat_start: false,
            section: None,
            rests: vec![],
            grace_notes: vec![],
            repeat_end: None,
            fill: Fill::Full,
        }
    }

    fn group_sizes(groups: &[BeamGroup]) -> Vec<usize> {
        groups.iter().map(|group| group.chords.len()).collect()
    }

    #[test]
    fn eighths_are_beamed_per_beat() {
        let notes = (0..8)
            .map(|i| note(i * 480, NoteValue::Eighth, HIHAT, 0))
            .collect();
        let groups = beam_groups(&measure(notes));
        // Each half of the bar is beamed as four
        assert_eq!(group_sizes(&groups), vec![4, 4]);
        assert!(groups.iter().all(|g| g.direction == StemDirection::Up));
    }

    #[test]
    fn eighths_join_only_within_a_half_bar() {
        // Beats 2 and 3 of eighths, with a quarter on beat 1 and 4
        let notes = vec![
            note(0, NoteValue::Quarter, SNARE, 0),
            note(960, NoteValue::Eighth, HIHAT, 0),
            note(1440, NoteValue::Eighth, HIHAT, 0),
            note(1920, NoteValue::Eighth, HIHAT, 0),
            note(2400, NoteValue::Eighth, HIHAT, 0),
            note(2880, NoteValue::Quarter, SNARE, 0),
        ];
        let groups = beam_groups(&measure(notes));
        assert_eq!(group_sizes(&groups), vec![1, 2, 2, 1]);
    }

    #[test]
    fn marks_triplets() {
        let triplet = |timestamp| {
            let mut note = note(timestamp, NoteValue::Sixteenth, SNARE, 0);
            note.duration.tuplet = Some(Tuplet::TRIPLET);
            note
        };
        // An eighth, then a sixteenth triplet beamed on its own, then two
        // triplets in a row, each its own group
        let notes = vec![
            note(0, NoteValue::Eighth, SNARE, 0),
            triplet(480),
            triplet(640),
            triplet(800),
            triplet(960),
            triplet(1120),
            triplet(1280),
            triplet(1440),
            triplet(1600),
            triplet(1760),
        ];
        let groups = beam_groups(&measure(notes));
        assert_eq!(group_sizes(&groups), vec![1, 3, 3, 3]);
        let marks = tuplet_marks(&groups);
        assert_eq!(marks.len(), 3);
        assert_eq!((marks[0].first, marks[0].last), ((1, 0), (1, 2)));
        assert!(marks[0].is_beamed(&groups));
    }

    #[test]
    fn lone_eighths_pair_across_a_beat() {
        // Eighths on 1&, 2, then 3& after a rest: the first two pair up
        let notes = vec![
            note(480, NoteValue::Eighth, HIHAT, 0),
            note(960, NoteValue::Eighth, HIHAT, 0),
            note(2400, NoteValue::Eighth, HIHAT, 0),
        ];
        let groups = beam_groups(&measure(notes));
        assert_eq!(group_sizes(&groups), vec![2, 1]);
    }

    #[test]
    fn notes_starting_together_share_a_stem() {
        let notes = vec![
            note(0, NoteValue::Eighth, HIHAT, 0),
            note(0, NoteValue::Eighth, SNARE, 0),
            note(480, NoteValue::Eighth, HIHAT, 0),
        ];
        let groups = beam_groups(&measure(notes));
        assert_eq!(group_sizes(&groups), vec![2]);
        assert_eq!(groups[0].chords[0].lines.len(), 2);
    }

    #[test]
    fn voices_are_beamed_separately() {
        let notes = vec![
            note(0, NoteValue::Eighth, HIHAT, 0),
            note(480, NoteValue::Eighth, HIHAT, 0),
            note(0, NoteValue::Sixteenth, KICK, 1),
            note(240, NoteValue::Sixteenth, KICK, 1),
        ];
        let groups = beam_groups(&measure(notes));
        assert_eq!(group_sizes(&groups), vec![2, 2]);
        assert_eq!(groups[1].direction, StemDirection::Down);
        assert_eq!(groups[1].max_beams(), 2);
    }

    #[test]
    fn feet_have_stems_down_in_any_voice() {
        let notes = vec![
            note(0, NoteValue::Eighth, HIHAT, 0),
            note(0, NoteValue::Eighth, KICK, 0),
            note(480, NoteValue::Eighth, HIHAT, 0),
            note(480, NoteValue::Eighth, KICK, 0),
        ];
        let groups = beam_groups(&measure(notes));
        let directions: Vec<_> = groups.iter().map(|group| group.direction).collect();
        assert_eq!(directions, vec![StemDirection::Up, StemDirection::Down]);
    }

    #[test]
    fn tuplets_mixing_hands_and_feet_share_a_beam() {
        let sextuplet = |timestamp, midi_note| {
            let mut note = note(timestamp, NoteValue::Sixteenth, midi_note, 0);
            note.duration.tuplet = Some(Tuplet {
                notes: 6,
                in_time_of: 4,
            });
            note
        };
        let notes = vec![
            sextuplet(0, SNARE),
            sextuplet(160, SNARE),
            sextuplet(320, KICK),
            sextuplet(480, KICK),
            sextuplet(640, SNARE),
            sextuplet(800, SNARE),
            // A kick after the tuplet still has its stem down
            note(960, NoteValue::Quarter, KICK, 0),
        ];
        let groups = beam_groups(&measure(notes));
        assert_eq!(group_sizes(&groups), vec![6, 1]);
        assert_eq!(groups[0].direction, StemDirection::Up);
        assert_eq!(groups[1].direction, StemDirection::Down);
    }

    #[test]
    fn hands_and_feet_taking_turns_share_a_beam() {
        let notes = vec![
            note(0, NoteValue::ThirtySecond, SNARE, 0),
            note(120, NoteValue::ThirtySecond, SNARE, 0),
            note(240, NoteValue::ThirtySecond, KICK, 0),
            note(360, NoteValue::ThirtySecond, KICK, 0),
            note(480, NoteValue::Eighth, SNARE, 0),
            // A groove beat, with kick and hi-hat together, still splits
            note(960, NoteValue::Eighth, HIHAT, 0),
            note(960, NoteValue::Eighth, KICK, 0),
            note(1440, NoteValue::Eighth, HIHAT, 0),
        ];
        let mut kick_triplet: Vec<Note> = (0..3)
            .map(|i| {
                let mut note = note(1920 + i * 160, NoteValue::Sixteenth, KICK, 0);
                note.duration.tuplet = Some(Tuplet::TRIPLET);
                note
            })
            .collect();
        // So does a kick triplet followed by a hand
        kick_triplet.push(note(2400, NoteValue::Eighth, SNARE, 0));
        let notes = [notes, kick_triplet].concat();
        let groups = beam_groups(&measure(notes));
        let shapes: Vec<_> = groups
            .iter()
            .map(|group| (group.chords.len(), group.direction))
            .collect();
        assert_eq!(
            shapes,
            vec![
                (5, StemDirection::Up),
                (2, StemDirection::Up),
                (1, StemDirection::Up),
                (1, StemDirection::Down),
                (3, StemDirection::Down),
            ]
        );
    }

    #[test]
    fn rests_and_quarters_break_beams() {
        let notes = vec![
            // An eighth, then an eighth rest
            note(0, NoteValue::Eighth, SNARE, 0),
            note(960, NoteValue::Quarter, SNARE, 0),
            note(1920, NoteValue::Sixteenth, SNARE, 0),
            note(2160, NoteValue::Sixteenth, SNARE, 0),
            note(2400, NoteValue::Eighth, SNARE, 0),
        ];
        let groups = beam_groups(&measure(notes));
        assert_eq!(group_sizes(&groups), vec![1, 1, 3]);
        assert_eq!(groups[2].beam_runs(1), vec![(0, 2)]);
        assert_eq!(groups[2].beam_runs(2), vec![(0, 1)]);
    }

    #[test]
    fn stems_clear_the_notes() {
        let notes = vec![
            note(0, NoteValue::Sixteenth, HIHAT, 0),
            note(240, NoteValue::Sixteenth, HIHAT, 0),
        ];
        let groups = beam_groups(&measure(notes));
        let end = groups[0].stem_end_line((-3.5, 7.0));
        // The hi-hat is on line 4; a second beam lifts the stem end
        assert!((end - 6.4).abs() < 1e-6);
        assert!((groups[0].beam_line(end, 2) - 6.0).abs() < 1e-6);
    }
}
