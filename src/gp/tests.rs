use super::*;

/// A guitar track, then a drum track with two bars:
/// - bar 0 (7/8, repeat start, "Intro"): voice 0 is a snare triplet ending
///   in a rim shot, a dotted-quarter kick tied into the next bar, and a grace
///   hi-hat before a quarter rest; voice 1 is a single crash
/// - bar 1 (4/4, repeat end x3): the tied kick as a whole note
const GPIF: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<GPIF>
<Score>
<Title><![CDATA[Test Song]]></Title>
<Artist><![CDATA[Test Band]]></Artist>
</Score>
<MasterTrack>
<Tracks>0 1</Tracks>
<Automations>
<Automation><Type>Tempo</Type><Bar>0</Bar><Position>0</Position><Value>120 2</Value></Automation>
<Automation><Type>Tempo</Type><Bar>1</Bar><Position>0.5</Position><Value>60 3</Value></Automation>
</Automations>
</MasterTrack>
<Tracks>
<Track id="0">
<Name>Guitar</Name>
<InstrumentSet><Type>electricGuitar</Type></InstrumentSet>
</Track>
<Track id="1">
<Name>Drumkit</Name>
<InstrumentSet>
<Type>drumKit</Type>
<Elements>
<Element>
<Name>Snare</Name>
<Articulations>
<Articulation><Name>Snare (hit)</Name><InputMidiNumbers>38</InputMidiNumbers><OutputMidiNumber>38</OutputMidiNumber></Articulation>
<Articulation><Name>Snare (rim shot)</Name><InputMidiNumbers>91</InputMidiNumbers><OutputMidiNumber>38</OutputMidiNumber></Articulation>
</Articulations>
</Element>
<Element>
<Name>Kick Drum</Name>
<Articulations>
<Articulation><Name>Kick (hit)</Name><OutputMidiNumber>36</OutputMidiNumber></Articulation>
</Articulations>
</Element>
</Elements>
</InstrumentSet>
</Track>
</Tracks>
<MasterBars>
<MasterBar>
<Time>7/8</Time>
<Repeat start="true" end="false" count="0" />
<Section><Letter></Letter><Text>
Intro
</Text></Section>
<Bars>0 1</Bars>
</MasterBar>
<MasterBar>
<Time>4/4</Time>
<Repeat start="false" end="true" count="3" />
<Bars>0 2</Bars>
</MasterBar>
</MasterBars>
<Bars>
<Bar id="0"><Voices>-1 -1 -1 -1</Voices></Bar>
<Bar id="1"><Voices>0 1 -1 -1</Voices></Bar>
<Bar id="2"><Voices>2 -1 -1 -1</Voices></Bar>
</Bars>
<Voices>
<Voice id="0"><Beats>0 0 1 2 3 4</Beats></Voice>
<Voice id="1"><Beats>5</Beats></Voice>
<Voice id="2"><Beats>6</Beats></Voice>
</Voices>
<Beats>
<Beat id="0"><Rhythm ref="0" /><Notes>0</Notes></Beat>
<Beat id="1"><Rhythm ref="0" /><Notes>1</Notes></Beat>
<Beat id="2"><Rhythm ref="1" /><Notes>2</Notes></Beat>
<Beat id="3"><GraceNotes>BeforeBeat</GraceNotes><Rhythm ref="3" /><Notes>4</Notes></Beat>
<Beat id="4"><Rhythm ref="5" /></Beat>
<Beat id="5"><Rhythm ref="4" /><Notes>5</Notes></Beat>
<Beat id="6"><Rhythm ref="4" /><Notes>3</Notes></Beat>
</Beats>
<Notes>
<Note id="0"><InstrumentArticulation>0</InstrumentArticulation></Note>
<Note id="1"><InstrumentArticulation>1</InstrumentArticulation></Note>
<Note id="2"><Tie origin="true" destination="false" /><InstrumentArticulation>2</InstrumentArticulation></Note>
<Note id="3"><Tie origin="false" destination="true" /><InstrumentArticulation>2</InstrumentArticulation></Note>
<Note id="4"><Properties><Property name="Midi"><Number>42</Number></Property></Properties></Note>
<Note id="5"><Properties><Property name="Midi"><Number>49</Number></Property></Properties></Note>
</Notes>
<Rhythms>
<Rhythm id="0"><NoteValue>Eighth</NoteValue><PrimaryTuplet num="3" den="2" /></Rhythm>
<Rhythm id="1"><NoteValue>Quarter</NoteValue><AugmentationDot count="1" /></Rhythm>
<Rhythm id="3"><NoteValue>32nd</NoteValue></Rhythm>
<Rhythm id="4"><NoteValue>Whole</NoteValue></Rhythm>
<Rhythm id="5"><NoteValue>Quarter</NoteValue></Rhythm>
</Rhythms>
</GPIF>"#;

fn starts(voice: &GpVoice) -> Vec<Fraction> {
    voice.beats.iter().map(|b| b.start).collect()
}

#[test]
fn reads_score_metadata_and_picks_the_drum_track() {
    let score = parse_gpif(GPIF).unwrap();
    assert_eq!(score.title, "Test Song");
    assert_eq!(score.artist, "Test Band");
    assert_eq!(score.track_name, "Drumkit");
    assert_eq!(score.bars.len(), 2);
}

#[test]
fn reads_tempo_changes_in_quarter_note_bpm() {
    let score = parse_gpif(GPIF).unwrap();
    assert_eq!(
        score.tempo_changes,
        vec![
            TempoChange {
                bar: 0,
                position: 0.0,
                bpm: 120.0
            },
            TempoChange {
                bar: 1,
                position: 0.5,
                bpm: 90.0
            },
        ]
    );
}

#[test]
fn reads_bar_attributes() {
    let score = parse_gpif(GPIF).unwrap();
    let [first, second] = &score.bars[..] else {
        panic!()
    };
    assert_eq!(first.time_signature.bar_length(), Fraction::new(7, 8));
    assert_eq!(first.section.as_deref(), Some("Intro"));
    assert!(first.repeat_start);
    assert_eq!(first.repeat_end, None);
    assert_eq!(second.time_signature.bar_length(), Fraction::new(1, 1));
    assert_eq!(second.section, None);
    assert_eq!(second.repeat_end, Some(3));
}

#[test]
fn places_beats_with_tuplets_dots_and_grace_notes() {
    let score = parse_gpif(GPIF).unwrap();
    let voices = &score.bars[0].voices;
    assert_eq!(voices.len(), 2);
    assert_eq!(
        starts(&voices[0]),
        vec![
            Fraction::new(0, 1),
            Fraction::new(1, 12),
            Fraction::new(1, 6),
            Fraction::new(1, 4),
            // The grace note takes no time, so the rest starts with it
            Fraction::new(5, 8),
            Fraction::new(5, 8),
        ]
    );
    assert_eq!(voices[0].beats[4].grace, Some(Grace::BeforeBeat));
    let last = voices[0].beats.last().unwrap();
    assert!(last.notes.is_empty(), "the last beat is a rest");
    assert_eq!(last.start + last.rhythm.duration(), Fraction::new(7, 8));
}

#[test]
fn resolves_drum_sounds_and_ties() {
    let score = parse_gpif(GPIF).unwrap();
    let beats = &score.bars[0].voices[0].beats;
    let note = |beat: usize| &beats[beat].notes[0];

    assert_eq!(note(0).midi_note, 38);
    assert_eq!(note(0).articulation.as_deref(), Some("Snare (hit)"));
    // A rim shot plays the snare sound, so it maps to the snare's GM number
    assert_eq!(note(2).midi_note, 38);
    assert_eq!(note(2).articulation.as_deref(), Some("Snare (rim shot)"));
    assert_eq!(note(3).midi_note, 36);
    assert!(note(3).tie_origin && !note(3).tie_destination);
    // No articulation: falls back to the note's own MIDI number
    assert_eq!(note(4).midi_note, 42);
    assert_eq!(note(4).articulation, None);

    let tied = &score.bars[1].voices[0].beats[0].notes[0];
    assert_eq!(tied.midi_note, 36);
    assert!(tied.tie_destination);
}

#[test]
fn errors_without_a_drum_track() {
    let xml = GPIF.replace("drumKit", "electricGuitar");
    assert!(matches!(parse_gpif(&xml), Err(GpError::NoDrumTrack)));
}

#[test]
fn rhythm_durations() {
    let rhythm = |value, dots, tuplet: Option<(u32, u32)>| GpRhythm {
        value,
        dots,
        tuplet: tuplet.map(|(notes, in_time_of)| Tuplet { notes, in_time_of }),
        inner_tuplet: None,
    };
    use GpNoteValue::*;
    assert_eq!(rhythm(Quarter, 0, None).duration(), Fraction::new(1, 4));
    assert_eq!(rhythm(Quarter, 1, None).duration(), Fraction::new(3, 8));
    assert_eq!(rhythm(Quarter, 2, None).duration(), Fraction::new(7, 16));
    assert_eq!(
        rhythm(Sixteenth, 0, Some((5, 4))).duration(),
        Fraction::new(1, 20)
    );
    let nested = GpRhythm {
        inner_tuplet: Some(Tuplet {
            notes: 3,
            in_time_of: 2,
        }),
        ..rhythm(Sixteenth, 0, Some((6, 4)))
    };
    assert_eq!(nested.duration(), Fraction::new(1, 36));
}

/// Set DRUMBEAT_GP_FIXTURE to a real .gp file to check it loads and every
/// bar's voices add up to the bar's length
#[test]
fn loads_fixture_file() {
    let Ok(path) = std::env::var("DRUMBEAT_GP_FIXTURE") else {
        return;
    };
    let score = load(&path).unwrap();
    let converted = score.to_score().unwrap();
    assert_eq!(converted.measures.len(), score.bars.len());
    for (index, bar) in score.bars.iter().enumerate() {
        for voice in &bar.voices {
            let Some(last) = voice.beats.last() else {
                continue;
            };
            assert_eq!(
                last.start + last.rhythm.duration(),
                bar.time_signature.bar_length(),
                "bar {index} doesn't add up"
            );
        }
    }
}

#[test]
fn converts_to_the_app_score() {
    let score = parse_gpif(GPIF).unwrap().to_score().unwrap();
    assert_eq!(score.title, "Test Song");
    assert_eq!(score.measures.len(), 2);

    let first = &score.measures[0];
    assert_eq!(first.timestamp, 0);
    assert_eq!(first.get_length(), 3360);
    let timestamps: Vec<u32> = first.notes.iter().map(|n| n.timestamp).collect();
    // The snare triplet, the rim shot and the kick in voice 0 (the grace
    // hi-hat is dropped), then the crash in voice 1
    assert_eq!(timestamps, vec![0, 320, 640, 960, 0]);

    let second = &score.measures[1];
    assert_eq!(second.timestamp, 3360);
    assert!(second.notes[0].tied);

    // 60 BPM in dotted quarters, halfway through the 4/4 bar
    let changes = score.tempo_map.changes();
    assert_eq!(changes.len(), 2);
    assert_eq!(changes[1].timestamp, 3360 + 1920);
    assert!((changes[1].bpm - 90.0).abs() < 1e-9);
}
