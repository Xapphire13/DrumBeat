use std::{cell::Cell, rc::Rc};

use dioxus::{html::input_data::MouseButton, prelude::*};

use crate::{
    data::{
        loop_range::{LoopPoint, count_label},
        measure::{Fill, Measure as MeasureData, Section},
        position::Position,
        score::{Hairpin, HairpinKind},
    },
    platform,
    ui::{
        hooks::{MousePositionData, use_mouse_position},
        providers::{
            loop_editor::{LoopDrag, use_loop_editor},
            playback_manager::use_playback_manager,
            score_manager::use_score_manager,
        },
        score::{
            StaffDimensions, StaffLayout,
            measure::Measure,
            measure_scale::MeasureScale,
            note::HoveredNote,
            staff::{
                loop_marks::{LoopGeometry, LoopMarks, closest_loop_point},
                phantom_playhead::{PhantomPlayhead, closest_snap, use_phantom_playhead_data},
                playhead::Playhead,
                staff_lines::StaffLines,
            },
        },
    },
};

mod loop_marks;
mod phantom_playhead;
mod playhead;
mod staff_lines;

/// The highest and lowest line numbers drawn, leaving room above the staff
/// for upward stems, beams and measure numbers, and below it for downward
/// stems and beams. The staff itself spans lines -0.5 to 3.5
const HIGHEST_LINE: f32 = 7.0;
const LOWEST_LINE: f32 = -3.5;
/// Top and bottom staff lines
const TOP_STAFF_LINE: f32 = 3.5;
const BOTTOM_STAFF_LINE: f32 = -0.5;

/// Extra room below a staff with hairpins, in lines
const HAIRPIN_BAND: f32 = 1.5;
/// How wide a hairpin opens, in lines
const HAIRPIN_OPENING: f32 = 0.9;

/// Extra room above a staff where a section starts, clear of the stems,
/// beams and measure numbers, in lines
const SECTION_BAND: f32 = 2.0;

/// Room above the bottom staff line's baseline, in lines
fn top_in_lines(has_sections: bool) -> f32 {
    HIGHEST_LINE + if has_sections { SECTION_BAND } else { 0.0 }
}

/// The staff's height in lines
fn height_in_lines(has_hairpins: bool, has_sections: bool) -> f32 {
    top_in_lines(has_sections) - LOWEST_LINE + if has_hairpins { HAIRPIN_BAND } else { 0.0 }
}

/// Height of a staff drawn with the given line height
pub fn height(line_height: f32, has_hairpins: bool, has_sections: bool) -> f32 {
    height_in_lines(has_hairpins, has_sections) * line_height
}

/// Whether a section starts on any of the measures
pub fn has_sections(measures: &[MeasureData]) -> bool {
    measures.iter().any(|measure| measure.section.is_some())
}

#[component]
pub fn Staff(dimensions: StaffDimensions, layout: StaffLayout) -> Element {
    let StaffDimensions {
        width,
        height,
        zoom,
    } = dimensions.clone();
    let StaffLayout {
        first_measure_number,
        measures,
        widths,
        start,
        left,
        hairpins,
    } = layout;
    // A short last staff doesn't fill the whole width
    let content_width: f32 = widths.iter().sum();
    let mut playback_manager = use_playback_manager();
    let score = use_score_manager();
    let MousePositionData {
        mouse_position,
        handle_on_mouse_move,
        handle_on_mouse_leave,
    } = use_mouse_position();
    let with_sections = has_sections(&measures);
    let line_height = height / height_in_lines(!hairpins.is_empty(), with_sections);
    let baseline = top_in_lines(with_sections) * line_height;
    let scales = use_scales(&measures, &widths, left, baseline, line_height);
    // The mouse is in window pixels, while the staff is laid out unzoomed
    let mouse_position = mouse_position.map(|Position { x, y }| Position {
        x: x / zoom,
        y: y / zoom,
    });
    let phantom_playhead_data = use_phantom_playhead_data(&mouse_position, &scales(), &measures);
    let mut container = use_signal(|| None::<Rc<MountedData>>);
    // Set by the note heads as the pointer moves over them
    use_context_provider(|| Signal::new(None::<HoveredNote>));
    let span = measures
        .first()
        .zip(measures.last())
        .map(|(first, last)| (first.timestamp, last.end_timestamp()));
    use_follow_playhead(container, span);

    let mut loop_editor = use_loop_editor();

    // The note under the pointer, where the playhead snaps, on the pass
    // through a repeat nearest `near`, and where it's written
    let note_at = {
        let measures = measures.clone();
        move |event: &MouseEvent, near: u32| {
            let x = event.coordinates().element().x as f32 / zoom;
            let written = closest_snap(x, &scales.peek(), &measures)?.closest_timestamp;
            let timestamp = score.peek().performance.to_performance(written, near)?;
            Some((timestamp, written))
        }
    };

    // Where the dragged end of the loop goes with the pointer here
    let loop_point_at = {
        let measures = measures.clone();
        move |event: &MouseEvent, by_tab: bool, anchor: u32, anchor_written: u32| {
            let x = event.coordinates().element().x as f32 / zoom;
            closest_loop_point(x, &scales.peek(), &measures, by_tab)?.locate(
                first_measure_number - 1,
                &score.peek().performance,
                anchor,
                anchor_written,
            )
        }
    };

    // Pressing on a note starts a drag, which sets a loop if it moves to
    // another note, or moves the playhead to the note if it doesn't
    let handle_on_mouse_down = {
        let note_at = note_at.clone();
        move |event: MouseEvent| {
            if event.trigger_button() != Some(MouseButton::Primary) {
                return;
            }
            let playhead = playback_manager.performance_timestamp();
            if let Some((anchor, anchor_written)) = note_at(&event, playhead) {
                loop_editor.start_drag(LoopDrag::Score {
                    anchor: LoopPoint::Note(anchor),
                    anchor_written,
                    current: LoopPoint::Note(anchor),
                    moved: false,
                    by_tab: false,
                });
            }
        }
    };

    let handle_on_mouse_move = move |event: MouseEvent| {
        handle_on_mouse_move.call(event.clone());
        let Some(LoopDrag::Score {
            anchor,
            anchor_written,
            by_tab,
            ..
        }) = loop_editor.drag()
        else {
            return;
        };
        // Released outside the window
        if event.held_buttons().is_empty() {
            loop_editor.finish(&mut playback_manager, &score.peek());
        } else if let Some(point) =
            loop_point_at(&event, by_tab, anchor.timestamp(), anchor_written)
        {
            loop_editor.drag_to(point);
        }
    };

    let shown_loop = loop_editor.shown(&playback_manager, &score.read());
    let loop_geometry = shown_loop
        .map(|range| LoopGeometry::new(range, &score.read().performance, &measures, &scales()));
    // The tabs for dragging the loop's ends stay out of the way unless the
    // pointer is over the score. Without hover they always show
    let show_tabs =
        !platform::HAS_HOVER || loop_editor.is_hovering_score() || loop_editor.is_dragging();
    let loop_tabs = shown_loop
        .zip(loop_geometry.clone())
        .filter(|_| show_tabs)
        .map(|(range, geometry)| {
            let score = score.read();
            let performance = &score.performance;
            let label = |timestamp| count_label(timestamp, performance, &score.measures);
            // Each tab names the count at the edge it sits on. The loop goes
            // up to the end tab's count without playing it. Dragging one
            // keeps the other end where it is
            let start_tab = geometry.start_x.map(|x| TabData {
                x: x * zoom,
                label: label(range.start),
                anchor: range.end,
                anchor_written: performance.to_written_end(range.end),
                current: range.start,
            });
            let end_tab = geometry.end_x.map(|x| TabData {
                x: x * zoom,
                label: label(range.end),
                anchor: range.start,
                anchor_written: performance.to_written(range.start),
                current: range.end,
            });
            (start_tab, end_tab)
        });
    // Grabbing a tab drags that end of the loop, keeping the other in place
    let grab_tab = move |tab: &TabData| {
        let (anchor, anchor_written, current) = (tab.anchor, tab.anchor_written, tab.current);
        move |event: MouseEvent| {
            event.stop_propagation();
            loop_editor.start_drag(LoopDrag::Score {
                anchor: LoopPoint::Edge(anchor),
                anchor_written,
                current: LoopPoint::Edge(current),
                moved: true,
                by_tab: true,
            });
        }
    };

    rsx! {
        div {
            class: "staff-container",
            onmousedown: handle_on_mouse_down,
            onmousemove: handle_on_mouse_move,
            onmouseleave: handle_on_mouse_leave,
            onmounted: move |event: MountedEvent| container.set(Some(event.data())),
            style: "height: {height * zoom}px",

            svg {
                class: "staff",
                width: "{width * zoom}px",
                height: "{height * zoom}px",
                view_box: "0 0 {width} {height}",
                for (measure , scale) in measures.iter().zip(scales()) {
                    if measure.fill != Fill::Full {
                        InvalidMeasure { scale, fill: measure.fill, repeat_end: measure.repeat_end }
                    }
                }

                if let Some(geometry) = loop_geometry {
                    LoopMarks { geometry, height, zoom }
                }

                StaffLines { line_height, baseline, x_bounds: (start, left + content_width) }

                // Open the staff with a bar line and a percussion clef
                line {
                    x1: "{start}",
                    y1: "{baseline - BOTTOM_STAFF_LINE * line_height}",
                    x2: "{start}",
                    y2: "{baseline - TOP_STAFF_LINE * line_height}",
                }
                for offset in [0.6, 1.15] {
                    rect {
                        x: start + offset * line_height,
                        y: baseline - 2.5 * line_height,
                        width: 0.3 * line_height,
                        height: 2.0 * line_height,
                        fill: "black",
                        stroke: "none",
                    }
                }

                for hairpin in hairpins {
                    HairpinMark {
                        hairpin,
                        measures: measures.clone(),
                        scales: scales(),
                    }
                }

                Playhead {
                    measures: measures.clone(),
                    scales,
                    staff_dimensions: dimensions.clone(),
                }

                if let Some(data) = phantom_playhead_data {
                    PhantomPlayhead { data, staff_dimensions: dimensions }
                }

                // Measures
                {
                    let number_of_measures = measures.len();
                    let section_starts: Vec<bool> =
                        measures.iter().map(|measure| measure.section.is_some()).collect();
                    let staff_end = left + content_width;
                    let performance = score.read().performance.clone();
                    measures
                        .into_iter()
                        .enumerate()
                        .filter_map(move |(i, measure)| {
                            let scale = scales.get(i)?;
                            let (left, right) = scale.outer_x_bounds;
                            let top = scale.get_y(TOP_STAFF_LINE);
                            let bottom = scale.get_y(BOTTOM_STAFF_LINE);
                            let measure_number = format_measure_numbers(
                                &performance.measure_numbers(first_measure_number - 1 + i),
                            );
                            // A section's name stops short of the next one on
                            // the staff
                            let section_end = scales
                                .iter()
                                .zip(&section_starts)
                                .skip(i + 1)
                                .find(|(_, starts)| **starts)
                                .map_or(staff_end, |(scale, _)| scale.outer_x_bounds.0);
                            Some(rsx! {
                                // Draw vertical line between measures
                                if i > 0 {
                                    line {
                                        x1: "{left}",
                                        y1: "{bottom}",
                                        x2: "{left}",
                                        y2: "{top}",
                                    }
                                }
                                // Close the staff after its last measure
                                if i + 1 == number_of_measures {
                                    line {
                                        x1: "{right - 0.5}",
                                        y1: "{bottom}",
                                        x2: "{right - 0.5}",
                                        y2: "{top}",
                                    }
                                }
                                text {
                                    class: "measure-number-label",
                                    x: left + 2.,
                                    y: top - 0.35 * scale.unit,
                                    font_size: 0.75 * scale.unit,
                                    "{measure_number}"
                                }
                                if let Some(section) = measure.section.clone() {
                                    SectionLabel {
                                        section,
                                        x: left + 2.,
                                        end: section_end,
                                        y: scale.get_y(HIGHEST_LINE + 0.35),
                                        line_height,
                                    }
                                }
                                RepeatSigns {
                                    scale: scale.clone(),
                                    repeat_start: measure.repeat_start,
                                    repeat_end: measure.repeat_end,
                                }
                                Measure {
                                    scale: scale.clone(),
                                    measure,
                                    stem_bounds: (LOWEST_LINE + 0.3, HIGHEST_LINE - 0.3),
                                }
                            })
                        })
                }
            }

            if let Some((start_tab, end_tab)) = loop_tabs {
                if let Some(tab) = start_tab {
                    button {
                        class: "loop-tab loop-tab-start",
                        style: "left: {tab.x - 1.0}px",
                        title: "Drag to move the start of the loop",
                        aria_label: "Loop start, {tab.label}. Drag to move",
                        onmousedown: grab_tab(&tab),
                        // Its own coordinates would throw off the playhead's
                        onmousemove: |event: MouseEvent| event.stop_propagation(),
                        "{tab.label}"
                    }
                }
                if let Some(tab) = end_tab {
                    button {
                        class: "loop-tab loop-tab-end",
                        style: "left: {tab.x + 1.0}px",
                        title: "Drag to move the end of the loop",
                        aria_label: "Loop end, {tab.label}. Drag to move",
                        onmousedown: grab_tab(&tab),
                        // Its own coordinates would throw off the playhead's
                        onmousemove: |event: MouseEvent| event.stop_propagation(),
                        "{tab.label}"
                    }
                }
            }

            NoteTooltip { zoom }
        }
    }
}

/// Names the voice of the note head under the pointer, and Guitar Pro's
/// number for it, just above the head, or below it when the score's scrolled
/// so there's no room above
#[component]
fn NoteTooltip(zoom: f32) -> Element {
    let hovered = use_context::<Signal<Option<HoveredNote>>>();
    use_effect(move || {
        if hovered().is_some() {
            // Only the page knows where the tooltip is within the scrolling
            // view. It fades in after a moment, so moving it isn't seen
            document::eval(PLACE_TOOLTIP);
        }
    });
    let Some(note) = hovered() else {
        return rsx! {};
    };
    rsx! {
        div {
            class: "note-tooltip",
            style: "left: {note.x * zoom}px; top: {note.top * zoom}px; --below: {note.bottom * zoom}px",
            "{note.label}"
        }
    }
}

/// Puts the tooltip below its note when it would be cut off at the top of
/// the scrolling staves
const PLACE_TOOLTIP: &str = r#"
const tooltip = document.querySelector(".note-tooltip");
const view = tooltip && tooltip.closest(".staves");
if (view) {
    tooltip.classList.remove("below");
    const cut = tooltip.getBoundingClientRect().top < view.getBoundingClientRect().top;
    tooltip.classList.toggle("below", cut);
}
"#;

/// One of the tabs for dragging the loop's ends
struct TabData {
    /// Where its edge is, in window pixels
    x: f32,
    label: String,
    /// The loop's other end, which stays put, and where that's written
    anchor: u32,
    anchor_written: u32,
    /// This end
    current: u32,
}

/// Follow the playhead while the score plays. When it plays on to this
/// staff, the staff is brought to the top so the staves after it can be read
/// ahead, or, while looping, just far enough to be seen, so going round a
/// loop over two staves doesn't jump back and forth. Starting to play, or
/// jumping the playhead while playing, brings the staff into view if it's
/// out of it. Nothing scrolls while paused, nor for a click on the score,
/// where the playhead is already in view
fn use_follow_playhead(container: Signal<Option<Rc<MountedData>>>, span: Option<(u32, u32)>) {
    let playback_manager = use_playback_manager();
    // Only changes when the playhead enters or leaves the staff, playback
    // starts or stops, or the playhead is moved by hand, not on every frame
    let state = use_memo(use_reactive!(|span| {
        let timestamp = playback_manager.playhead_timestamp();
        let here = span.is_some_and(|(start, end)| (start..end).contains(&timestamp));
        let looping = playback_manager.loop_range().is_some();
        (
            here,
            playback_manager.is_playing(),
            looping,
            playback_manager.seeks(),
        )
    }));
    let previous = use_hook(|| Rc::new(Cell::new((false, false, false, (0, false)))));

    use_effect(move || {
        let current = state();
        let (here, playing, looping, (seeks, on_score)) = current;
        let (was_here, was_playing, _, (previous_seeks, _)) = previous.replace(current);
        if !here || !playing {
            return;
        }
        let jumped = seeks != previous_seeks;
        let vertical = if !was_playing || (jumped && !on_score) {
            ScrollLogicalPosition::Nearest
        } else if was_here || jumped {
            return;
        } else if looping {
            ScrollLogicalPosition::Nearest
        } else {
            ScrollLogicalPosition::Start
        };
        let Some(container) = container.peek().clone() else {
            return;
        };
        spawn(async move {
            let _ = container
                .scroll_to_with_options(ScrollToOptions {
                    behavior: ScrollBehavior::Smooth,
                    vertical,
                    horizontal: ScrollLogicalPosition::Nearest,
                })
                .await;
        });
    });
}

/// A highlight behind a measure whose beats don't add up to its time
/// signature, with a note above the staff saying what's wrong
#[component]
fn InvalidMeasure(scale: MeasureScale, fill: Fill, repeat_end: Option<u32>) -> Element {
    let unit = scale.unit;
    let (left, right) = scale.outer_x_bounds;
    let top = scale.get_y(TOP_STAFF_LINE + 1.0);
    let bottom = scale.get_y(BOTTOM_STAFF_LINE - 1.0);
    let (label, description) = match fill {
        Fill::Short => (
            "Short bar",
            "The beats in this bar don't fill its time signature, so the rest of it is silent",
        ),
        Fill::Over { .. } | Fill::Full => (
            "Long bar",
            "The beats in this bar run past its time signature, so it has been lengthened to fit them",
        ),
    };
    // Keep clear of a repeat's play count, which sits in the same corner
    let label_right = right
        - 0.9 * unit
        - if repeat_end.is_some_and(|count| count != 2) {
            2.0 * unit
        } else {
            0.0
        };

    rsx! {
        rect {
            class: "invalid-measure",
            x: left,
            y: top,
            width: right - left,
            height: bottom - top,
            title { "{description}" }
        }
        text {
            class: "invalid-measure-label",
            x: label_right,
            y: scale.get_y(TOP_STAFF_LINE) - 0.35 * unit,
            font_size: 0.75 * unit,
            text_anchor: "end",
            title { "{description}" }
            "⚠ {label}"
        }
    }
}

/// Show the numbers a measure is played as, e.g. "5" or, when it repeats,
/// "1–4" or "97, 99, 101"
fn format_measure_numbers(numbers: &[usize]) -> String {
    let mut runs: Vec<(usize, usize)> = vec![];
    for &number in numbers {
        match runs.last_mut() {
            Some((_, last)) if *last + 1 == number => *last = number,
            _ => runs.push((number, number)),
        }
    }
    runs.iter()
        .map(|&(first, last)| {
            if first == last {
                format!("{first}")
            } else {
                format!("{first}–{last}")
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn use_scales(
    measures: &Vec<MeasureData>,
    widths: &Vec<f32>,
    left: f32,
    baseline: f32,
    line_height: f32,
) -> Memo<Vec<MeasureScale>> {
    use_memo(use_reactive(
        (measures, widths, &left, &baseline),
        move |(measures, widths, left, baseline)| {
            let mut left_bound = left;
            measures
                .iter()
                .zip(&widths)
                .map(|(measure, width)| {
                    let right_bound = left_bound + width;
                    let scale = MeasureScale::new(
                        (left_bound, right_bound),
                        baseline,
                        line_height,
                        measure,
                    );

                    left_bound = right_bound;
                    scale
                })
                .collect::<Vec<MeasureScale>>()
        },
    ))
}

/// A crescendo or decrescendo wedge below the staff, for the part of it on
/// this staff
#[component]
fn HairpinMark(hairpin: Hairpin, measures: Vec<MeasureData>, scales: Vec<MeasureScale>) -> Element {
    let (Some(first), Some(last), Some(scale)) =
        (measures.first(), measures.last(), scales.first())
    else {
        return rsx! {};
    };
    let x_at = |timestamp: u32| {
        measures
            .iter()
            .zip(&scales)
            .find(|(measure, _)| timestamp <= measure.end_timestamp())
            .map_or(0.0, |(_, scale)| scale.get_x(timestamp))
    };
    let start = hairpin.start.max(first.timestamp);
    let end = hairpin.end.min(last.end_timestamp());
    // How far open the wedge is at a timestamp
    let half_opening = |timestamp: u32| {
        let progress = (timestamp - hairpin.start) as f32 / (hairpin.end - hairpin.start) as f32;
        let open = match hairpin.kind {
            HairpinKind::Crescendo => progress,
            HairpinKind::Decrescendo => 1.0 - progress,
        };
        open * HAIRPIN_OPENING * scale.line_height / 2.
    };
    let (x1, x2) = (x_at(start), x_at(end));
    let (open1, open2) = (half_opening(start), half_opening(end));
    let y = scale.get_y(LOWEST_LINE - HAIRPIN_BAND / 2.);

    rsx! {
        polyline {
            points: "{x2},{y - open2} {x1},{y - open1}",
            fill: "none",
            stroke: "black",
            stroke_width: 1.2,
        }
        polyline {
            points: "{x2},{y + open2} {x1},{y + open1}",
            fill: "none",
            stroke: "black",
            stroke_width: 1.2,
        }
    }
}

/// Where a section of the song starts, as Guitar Pro marks it: the
/// rehearsal letter in a box, then the name in bold. `y` is the bottom of the
/// label, and the name is cut off at `end`
#[component]
fn SectionLabel(section: Section, x: f32, end: f32, y: f32, line_height: f32) -> Element {
    let font_size = 0.9 * line_height;
    let box_size = 1.15 * line_height;
    let text_y = y - 0.3 * line_height;
    // Wide enough for a letter or two
    let letter_width = section.letter.as_ref().map(|letter| {
        box_size.max(0.62 * font_size * letter.chars().count() as f32 + 0.5 * line_height)
    });
    let name_x = letter_width.map_or(x, |width| x + width + 0.35 * line_height);
    // Room for the name, keeping a gap before whatever comes next
    let name_width = (end - name_x - 0.4 * line_height).max(0.);

    rsx! {
        if let Some((letter, width)) = section.letter.zip(letter_width) {
            rect {
                class: "section-letter-box",
                x,
                y: y - box_size,
                width,
                height: box_size,
                rx: 0.12 * line_height,
            }
            text {
                class: "section-label",
                x: x + width / 2.,
                y: text_y,
                font_size,
                text_anchor: "middle",
                "{letter}"
            }
        }
        if let Some(name) = section.name {
            svg {
                x: name_x,
                y: y - 2. * line_height,
                width: name_width,
                height: 2. * line_height,
                overflow: "hidden",
                text {
                    class: "section-label",
                    x: 0,
                    y: text_y - (y - 2. * line_height),
                    font_size,
                    "{name}"
                }
            }
        }
    }
}

/// Thick and thin bar lines with dots for a repeated passage's start and end,
/// and the number of times it is played when that isn't the usual twice
#[component]
fn RepeatSigns(scale: MeasureScale, repeat_start: bool, repeat_end: Option<u32>) -> Element {
    let unit = scale.unit;
    let (left, right) = scale.outer_x_bounds;
    let top = scale.get_y(TOP_STAFF_LINE);
    let bottom = scale.get_y(BOTTOM_STAFF_LINE);
    let dot_radius = 0.2 * unit;
    // The dots sit in the two middle spaces of the staff
    let dot_ys = [scale.get_y(1.0), scale.get_y(2.0)];

    rsx! {
        if repeat_start {
            line {
                x1: left + 2.,
                y1: bottom,
                x2: left + 2.,
                y2: top,
                stroke_width: 3,
            }
            for y in dot_ys {
                circle {
                    cx: left + 0.6 * unit,
                    cy: y,
                    r: dot_radius,
                    fill: "black",
                    stroke: "none",
                }
            }
        }
        if let Some(count) = repeat_end {
            line {
                x1: right - 2.,
                y1: bottom,
                x2: right - 2.,
                y2: top,
                stroke_width: 3,
            }
            for y in dot_ys {
                circle {
                    cx: right - 0.6 * unit,
                    cy: y,
                    r: dot_radius,
                    fill: "black",
                    stroke: "none",
                }
            }
            if count != 2 {
                text {
                    class: "repeat-count",
                    x: right - 0.9 * unit,
                    y: top - 0.35 * unit,
                    font_size: 0.75 * unit,
                    text_anchor: "end",
                    "×{count}"
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measure_numbers_collapse_into_ranges() {
        assert_eq!(format_measure_numbers(&[5]), "5");
        assert_eq!(format_measure_numbers(&[1, 2, 3, 4]), "1–4");
        assert_eq!(format_measure_numbers(&[97, 99, 101]), "97, 99, 101");
    }
}
