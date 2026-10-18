use dioxus::prelude::*;

use crate::{
    data::{
        drum_voice::NoteHead,
        note::{GraceNote, NoteValue},
        position::Position,
    },
    ui::score::{
        beaming::StemDirection,
        measure_scale::MeasureScale,
        note::{Note, head_half_width},
    },
};

/// Size of a grace note's glyphs, relative to a normal note's
const GRACE_GLYPH_SCALE: f32 = 0.6;
/// Length of a grace note's stem past its furthest head, in lines
const STEM_LENGTH: f32 = 1.1;

/// Grace notes leading into the same main note, in one voice and on the same
/// side of the stems
#[derive(Clone, PartialEq)]
struct GraceRun {
    direction: StemDirection,
    /// One chord per step before the main note, furthest first. Each is the
    /// drawn x of the stem's side of the heads, and the heads' staff lines
    chords: Vec<(f32, Vec<(f32, bool)>)>,
}

/// Grace notes in the same run lead into the same main note, in the same
/// voice, on the same side of the stems
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct RunKey {
    main_timestamp: u32,
    voice: u8,
    is_feet: bool,
}

/// A grace note head, ready to be grouped into runs
struct Head {
    run: RunKey,
    steps_before_main: u8,
    x: f32,
    line: f32,
    is_x: bool,
}

/// Group a measure's grace notes into runs, each drawn with its own stems
fn grace_runs(scale: &MeasureScale, grace_notes: &[GraceNote]) -> Vec<GraceRun> {
    let mut heads: Vec<Head> = grace_notes
        .iter()
        .filter_map(|grace| {
            let voice = grace.drum_voice()?;
            let drum_piece = voice.piece();
            Some(Head {
                run: RunKey {
                    main_timestamp: grace.main_timestamp,
                    voice: grace.voice,
                    is_feet: drum_piece.is_played_with_feet(),
                },
                steps_before_main: grace.steps_before_main,
                x: scale.get_grace_x(grace),
                line: MeasureScale::get_line_number(drum_piece),
                is_x: voice.head() == NoteHead::X,
            })
        })
        .collect();
    // Furthest from the main note first
    heads.sort_by(|a, b| {
        a.run
            .cmp(&b.run)
            .then(b.steps_before_main.cmp(&a.steps_before_main))
    });

    let mut runs: Vec<(RunKey, u8, GraceRun)> = vec![];
    for head in heads {
        match runs.last_mut() {
            Some((run, step, grace_run)) if *run == head.run => {
                if *step == head.steps_before_main {
                    let (_, chord) = grace_run.chords.last_mut().unwrap();
                    chord.push((head.line, head.is_x));
                } else {
                    grace_run
                        .chords
                        .push((head.x, vec![(head.line, head.is_x)]));
                    *step = head.steps_before_main;
                }
            }
            _ => runs.push((
                head.run,
                head.steps_before_main,
                GraceRun {
                    direction: if head.run.is_feet {
                        StemDirection::Down
                    } else {
                        StemDirection::Up
                    },
                    chords: vec![(head.x, vec![(head.line, head.is_x)])],
                },
            )),
        }
    }
    runs.into_iter().map(|(.., run)| run).collect()
}

/// A measure's grace notes, drawn small just ahead of their main notes. A
/// lone grace note (a flam) gets a slashed flag; a run of them (a drag) is
/// joined by two beams
#[component]
pub fn GraceNotes(scale: MeasureScale, grace_notes: Vec<GraceNote>) -> Element {
    let small = scale.with_glyph_scale(GRACE_GLYPH_SCALE);
    let unit = small.unit;
    let head = head_half_width(unit);
    let heads = grace_notes.iter().filter_map(|grace| {
        let voice = grace.drum_voice()?;
        Some(rsx! {
            Note {
                value: NoteValue::Eighth,
                voice,
                gp_note: grace.midi_note,
                position: Position {
                    x: scale.get_grace_x(grace),
                    y: scale.get_y(MeasureScale::get_line_number(voice.piece())),
                },
                scale: small.clone(),
            }
        })
    });

    let mut lines: Vec<(f32, f32, f32, f32, f32)> = vec![];
    let mut beams: Vec<(f32, f32, f32)> = vec![];
    let thickness = 0.35 * unit;
    for run in grace_runs(&scale, &grace_notes) {
        let up = run.direction == StemDirection::Up;
        let all_lines = run
            .chords
            .iter()
            .flat_map(|(_, heads)| heads.iter().map(|h| h.0));
        let end_line = if up {
            all_lines.fold(f32::MIN, f32::max) + STEM_LENGTH
        } else {
            all_lines.fold(f32::MAX, f32::min) - STEM_LENGTH
        };
        let end_y = scale.get_y(end_line);
        let stem_x = |x: f32| if up { x + head } else { x - head };

        for (x, heads) in &run.chords {
            // The stem starts from the head furthest from its end, at the
            // corner of an X
            let &(start_line, is_x) = heads
                .iter()
                .min_by(|a, b| {
                    let order = a.0.total_cmp(&b.0);
                    if up { order } else { order.reverse() }
                })
                .unwrap();
            let start_y = scale.get_y(start_line)
                + match (is_x, up) {
                    (false, _) => 0.0,
                    (true, true) => -head,
                    (true, false) => head,
                };
            let x = stem_x(*x);
            lines.push((x, start_y, x, end_y, 1.0));
        }

        let sign = if up { 1.0 } else { -1.0 };
        match run.chords.as_slice() {
            [(x, _)] => {
                let x = stem_x(*x);
                // A flag, and a slash through the stem
                lines.push((x, end_y, x + 0.8 * unit, end_y + sign * 1.2 * unit, 1.5));
                lines.push((
                    x - 0.6 * unit,
                    end_y + sign * 1.6 * unit,
                    x + 0.7 * unit,
                    end_y + sign * 0.2 * unit,
                    1.0,
                ));
            }
            [(first, _), .., (last, _)] => {
                let (x1, x2) = (stem_x(*first), stem_x(*last));
                for level in 0..2 {
                    let y = end_y + sign * level as f32 * 0.7 * unit;
                    let top = if up { y } else { y - thickness };
                    beams.push((x1 - 0.5, x2 + 0.5, top));
                }
            }
            [] => {}
        }
    }

    rsx! {
        g { class: "grace-notes",
            {heads}
            for (x1, y1, x2, y2, stroke_width) in lines {
                line {
                    x1,
                    y1,
                    x2,
                    y2,
                    stroke: "black",
                    stroke_width,
                }
            }
            for (x1, x2, y) in beams {
                rect {
                    x: x1,
                    y,
                    width: x2 - x1,
                    height: thickness,
                    fill: "black",
                    stroke: "none",
                }
            }
        }
    }
}
