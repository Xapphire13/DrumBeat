use dioxus::prelude::*;

use crate::{
    data::{measure::Measure as MeasureData, note::Annotation},
    ui::score::{
        beaming::{BeamGroup, StemDirection, TupletMark, beam_groups, tuplet_marks},
        grace_note::GraceNotes,
        note::{Note, head_half_width},
        rest::Rest,
    },
};

use super::measure_scale::MeasureScale;

/// Radius of a hi-hat open or close sign, in glyph sizes
const ANNOTATION_RADIUS: f32 = 0.3;
/// Thickness of a beam, in glyph sizes
const BEAM_THICKNESS: f32 = 0.35;
/// Length of a stub beam on a lone short note, in line heights
const STUB_LENGTH: f32 = 0.9;

/// Component that renders the notes of a measure, with their stems and beams
#[component]
pub fn Measure(
    scale: MeasureScale,
    measure: MeasureData,
    /// The lowest and highest staff lines stems may reach
    stem_bounds: (f32, f32),
) -> Element {
    let groups = beam_groups(&measure);
    let tuplets = tuplet_marks(&groups);
    let voices = measure
        .notes
        .iter()
        .map(|note| note.voice)
        .max()
        .unwrap_or(0)
        + 1;
    let rests: Vec<Element> = measure
        .rests
        .iter()
        // Another voice playing at the same time already shows the beat
        .filter(|rest| {
            !measure
                .notes
                .iter()
                .any(|note| note.voice != rest.voice && note.timestamp == rest.timestamp)
        })
        // The same rest in several voices is drawn once, by the first voice
        .filter_map(|rest| {
            let mut shared_with = measure.rests.iter().filter(|other| {
                other.voice != rest.voice
                    && other.timestamp == rest.timestamp
                    && other.duration == rest.duration
            });
            match shared_with.clone().next() {
                None => Some((rest, false)),
                Some(_) if shared_with.all(|other| other.voice > rest.voice) => Some((rest, true)),
                Some(_) => None,
            }
        })
        .map(|(rest, is_shared)| {
            // With hands and feet in separate voices, the hands' rests sit
            // high in the staff and the feet's low. A rest they share sits
            // in the middle
            let center_line = match (voices, rest.voice) {
                _ if is_shared => 1.5,
                (1, _) => 1.5,
                (_, 0) => 2.5,
                _ => 0.5,
            };
            rsx! {
                Rest {
                    scale: scale.clone(),
                    rest: rest.clone(),
                    measure_start: measure.timestamp,
                    measure_length: measure.get_length(),
                    center_line,
                }
            }
        })
        .collect();

    rsx! {
        g {
            {
                measure
                    .notes
                    .iter()
                    .filter_map(|note| {
                        let voice = note.drum_voice()?;
                        Some(rsx! {
                            Note {
                                value: note.duration.value,
                                dots: note.duration.dots,
                                voice,
                                gp_note: note.midi_note,
                                position: scale.get_position(note.timestamp, voice.piece()),
                                scale: scale.clone(),
                            }
                        })
                    })
            }
            {rests.into_iter()}
            if !measure.grace_notes.is_empty() {
                GraceNotes {
                    scale: scale.clone(),
                    grace_notes: measure.grace_notes.clone(),
                }
            }
            for mark in tuplets {
                TupletNumber {
                    scale: scale.clone(),
                    groups: groups.clone(),
                    mark,
                    stem_bounds,
                }
            }
            for group in groups {
                Stems { scale: scale.clone(), group, stem_bounds }
            }
        }
    }
}

/// The x position of a chord's stem
fn stem_x(scale: &MeasureScale, direction: StemDirection, timestamp: u32) -> f32 {
    let x = scale.get_x(timestamp);
    let head = head_half_width(scale.unit);
    match direction {
        StemDirection::Up => x + head,
        StemDirection::Down => x - head,
    }
}

/// A tuplet's number beyond the ends of its stems, with a bracket when its
/// notes aren't beamed together
#[component]
fn TupletNumber(
    scale: MeasureScale,
    groups: Vec<BeamGroup>,
    mark: TupletMark,
    stem_bounds: (f32, f32),
) -> Element {
    let unit = scale.unit;
    let font_size = 0.75 * unit;
    let direction = mark.direction;
    let x1 = stem_x(
        &scale,
        direction,
        groups[mark.first.0].chords[mark.first.1].timestamp,
    );
    let x2 = stem_x(
        &scale,
        direction,
        groups[mark.last.0].chords[mark.last.1].timestamp,
    );
    let middle = (x1 + x2) / 2.;
    let stem_ends = groups[mark.first.0..=mark.last.0]
        .iter()
        .map(|group| group.stem_end_line(stem_bounds));
    // Make room for hi-hat signs between the stems and the number
    let has_annotations = groups[mark.first.0..=mark.last.0]
        .iter()
        .flat_map(|group| &group.chords)
        .any(|chord| chord.annotation.is_some());
    let gap = if has_annotations {
        (0.6 + 2. * ANNOTATION_RADIUS) * unit
    } else {
        0.3 * unit
    };
    // The bracket line, just beyond the furthest stem end
    let is_beamed = mark.is_beamed(&groups);
    // The number's baseline sits beyond the beam, or centred on the bracket
    let (bracket_y, number_y, hook) = match direction {
        StemDirection::Up => {
            let y = scale.get_y(stem_ends.fold(f32::MIN, f32::max)) - gap;
            let number_y = if is_beamed { y } else { y + font_size * 0.35 };
            (y, number_y.max(font_size * 0.8), gap)
        }
        StemDirection::Down => {
            let y = scale.get_y(stem_ends.fold(f32::MAX, f32::min)) + gap;
            let number_y = if is_beamed {
                y + font_size * 0.8
            } else {
                y + font_size * 0.35
            };
            (y, number_y, -gap)
        }
    };
    // Leave a gap in the bracket for the number
    let number_half_width = 0.4 * unit;

    rsx! {
        g {
            if !is_beamed {
                polyline {
                    points: "{x1},{bracket_y + hook} {x1},{bracket_y} {middle - number_half_width},{bracket_y}",
                    fill: "none",
                    stroke: "black",
                    stroke_width: 1,
                }
                polyline {
                    points: "{middle + number_half_width},{bracket_y} {x2},{bracket_y} {x2},{bracket_y + hook}",
                    fill: "none",
                    stroke: "black",
                    stroke_width: 1,
                }
            }
            text {
                class: "tuplet-number",
                x: middle,
                y: number_y,
                font_size,
                text_anchor: "middle",
                "{mark.notes}"
            }
        }
    }
}

/// The stems of a group of chords, and the beams joining them (or flags, for
/// a lone eighth or shorter)
#[component]
fn Stems(scale: MeasureScale, group: BeamGroup, stem_bounds: (f32, f32)) -> Element {
    let unit = scale.unit;
    let head = head_half_width(unit);
    let stem_end_line = group.stem_end_line(stem_bounds);
    let stem_end_y = scale.get_y(stem_end_line);
    let thickness = BEAM_THICKNESS * unit;
    // Beams sit on the inside of the stem end, towards the notes
    let beam_top = |level: usize| {
        let y = scale.get_y(group.beam_line(stem_end_line, level));
        match group.direction {
            StemDirection::Up => y,
            StemDirection::Down => y - thickness,
        }
    };
    let stem_x = |timestamp: u32| stem_x(&scale, group.direction, timestamp);

    let stems = group
        .chords
        .iter()
        .filter(|chord| chord.has_stem())
        .map(|chord| {
            // The stem starts at the note furthest from the stem end
            let start_line = match group.direction {
                StemDirection::Up => chord.lines.iter().copied().fold(f32::MAX, f32::min),
                StemDirection::Down => chord.lines.iter().copied().fold(f32::MIN, f32::max),
            };
            // A cymbal's stem starts from the corner of its X, on the stem side
            let starts_on_cymbal = chord
                .lines
                .iter()
                .zip(&chord.cymbals)
                .any(|(&line, &cymbal)| cymbal && line == start_line);
            let start_y = match (starts_on_cymbal, group.direction) {
                (false, _) => scale.get_y(start_line),
                (true, StemDirection::Up) => scale.get_y(start_line) - head,
                (true, StemDirection::Down) => scale.get_y(start_line) + head,
            };
            let x = stem_x(chord.timestamp);
            rsx! {
                line {
                    x1: x,
                    y1: start_y,
                    x2: x,
                    y2: stem_end_y,
                    stroke: "black",
                    stroke_width: 1.5,
                }
            }
        });

    // Hi-hat open and close signs and cymbal chokes, just beyond the stem end
    let annotation_radius = ANNOTATION_RADIUS * unit;
    let annotations = group.chords.iter().filter_map(|chord| {
        let annotation = chord.annotation?;
        let x = scale.get_x(chord.timestamp);
        let offset = 0.3 * unit + annotation_radius;
        let y = match group.direction {
            StemDirection::Up => stem_end_y - offset,
            StemDirection::Down => stem_end_y + offset,
        };
        let r = annotation_radius;
        Some(match annotation {
            Annotation::Open | Annotation::HalfOpen => rsx! {
                circle {
                    cx: x,
                    cy: y,
                    r,
                    fill: "none",
                    stroke: "black",
                    stroke_width: 1.3,
                }
                if annotation == Annotation::HalfOpen {
                    line {
                        x1: x - r * 1.3,
                        y1: y + r * 1.3,
                        x2: x + r * 1.3,
                        y2: y - r * 1.3,
                        stroke: "black",
                        stroke_width: 1.3,
                    }
                }
            },
            Annotation::Choke => rsx! {
                // An apostrophe: a dot with a short tail
                circle {
                    cx: x,
                    cy: y - 0.35 * r,
                    r: 0.5 * r,
                    fill: "black",
                }
                line {
                    x1: x + 0.4 * r,
                    y1: y - 0.2 * r,
                    x2: x - 0.2 * r,
                    y2: y + r,
                    stroke: "black",
                    stroke_width: 1.3,
                    stroke_linecap: "round",
                }
            },
            Annotation::Closed => rsx! {
                line {
                    x1: x - r,
                    y1: y,
                    x2: x + r,
                    y2: y,
                    stroke: "black",
                    stroke_width: 1.5,
                }
                line {
                    x1: x,
                    y1: y - r,
                    x2: x,
                    y2: y + r,
                    stroke: "black",
                    stroke_width: 1.5,
                }
            },
        })
    });

    let mut beams = vec![];
    if group.chords.len() > 1 {
        for level in 1..=group.max_beams() {
            for (first, last) in group.beam_runs(level) {
                let (x1, x2) = if first == last {
                    // A stub towards the neighbouring note, inside the group
                    let x = stem_x(group.chords[first].timestamp);
                    let neighbour = if first == 0 { 1 } else { first - 1 };
                    let neighbour_x = stem_x(group.chords[neighbour].timestamp);
                    let length = (STUB_LENGTH * unit).min((neighbour_x - x).abs() / 2.);
                    if neighbour_x > x {
                        (x, x + length)
                    } else {
                        (x - length, x)
                    }
                } else {
                    (
                        stem_x(group.chords[first].timestamp),
                        stem_x(group.chords[last].timestamp),
                    )
                };
                beams.push((x1 - 0.75, x2 + 0.75, beam_top(level)));
            }
        }
    }

    // A lone note gets flags instead of beams
    let mut flags = vec![];
    if let [chord] = group.chords.as_slice() {
        let x = stem_x(chord.timestamp);
        for level in 1..=chord.beams() {
            let y = scale.get_y(group.beam_line(stem_end_line, level));
            let tip_y = match group.direction {
                StemDirection::Up => y + 1.2 * unit,
                StemDirection::Down => y - 1.2 * unit,
            };
            flags.push((x, y, x + 0.8 * unit, tip_y));
        }
    }

    rsx! {
        g {
            {stems}
            {annotations}
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
            for (x1, y1, x2, y2) in flags {
                line {
                    x1,
                    y1,
                    x2,
                    y2,
                    stroke: "black",
                    stroke_width: 2,
                }
            }
        }
    }
}
