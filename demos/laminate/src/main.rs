// SPDX-FileCopyrightText: 2026 polygon_unionfind contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::BTreeSet;

use macroquad::prelude::*;
use macroquad::rand::gen_range;
use polygon_unionfind::{
    Inflated, LaminateDelta, Negated, Paralleled, PolygonId, PolygonSetHalfDelta,
    PolygonUnionFindHalfDelta, PolygonWithData, RecordingInflated, RecordingLaminate,
    RecordingNegated, RecordingPolygonSet,
};
use undoredo::UndoRedo;

type DemoPolygon = PolygonWithData<i32, ()>;
type DemoNegated = RecordingNegated<i32, DemoPolygon>;
type DemoInnerLayer = Paralleled<DemoNegated>;
type DemoLayer = Paralleled<DemoInnerLayer>;
type DemoTransition = Paralleled<RecordingPolygonSet<i32, DemoPolygon>>;
type DemoLaminate = RecordingLaminate<i32, DemoPolygon>;
type DemoLayersDelta = LaminateDelta<
    i32,
    DemoPolygon,
    Paralleled<
        Paralleled<
            Negated<
                PolygonSetHalfDelta<i32, DemoPolygon>,
                Inflated<
                    PolygonUnionFindHalfDelta<i32, DemoPolygon>,
                    core::marker::PhantomData<i32>,
                >,
            >,
        >,
    >,
    Paralleled<PolygonSetHalfDelta<i32, DemoPolygon>>,
>;
const PRIMARY_INFLATION: i32 = 0;
const RAIL_OFFSET: i32 = 5;
const PERIPHERAL_INFLATION: i32 = 20;

fn frame_polygon() -> DemoPolygon {
    DemoPolygon {
        exterior: vec![[-2000, -2000], [2000, -2000], [2000, 2000], [-2000, 2000]],
        interiors: vec![],
        data: (),
    }
}

fn new_negated_layer(offset: i32) -> DemoNegated {
    let mut minuend: RecordingPolygonSet<i32, DemoPolygon> = RecordingPolygonSet::new();
    let _ = minuend.add(frame_polygon());
    RecordingNegated::new(minuend, RecordingInflated::<i32, DemoPolygon>::new(offset))
}

fn new_layer_stack() -> DemoLayer {
    let core = Paralleled::new(
        new_negated_layer(PRIMARY_INFLATION),
        vec![new_negated_layer(PRIMARY_INFLATION + RAIL_OFFSET)],
    );
    let peripheral = Paralleled::new(
        new_negated_layer(PERIPHERAL_INFLATION),
        vec![new_negated_layer(PERIPHERAL_INFLATION + RAIL_OFFSET)],
    );
    Paralleled::new(core, vec![peripheral])
}

fn new_transition_layer() -> DemoTransition {
    let mut transition_set = RecordingPolygonSet::new();
    let _ = transition_set.add(frame_polygon());
    DemoTransition::new(transition_set, vec![])
}

fn new_laminate_demo() -> DemoLaminate {
    DemoLaminate::with_laminas_interlaminas(
        vec![new_layer_stack(), new_layer_stack(), new_layer_stack()],
        vec![new_transition_layer(), new_transition_layer()],
    )
}

/// Monotone-chain convex hull; returns vertices in counter-clockwise order.
fn convex_hull(points: &[[i32; 2]]) -> Vec<[i32; 2]> {
    if points.len() < 3 {
        return points.to_vec();
    }

    let mut pts: Vec<[i32; 2]> = points.to_vec();
    pts.sort_by(|a, b| a[0].cmp(&b[0]).then_with(|| a[1].cmp(&b[1])));

    fn cross(o: [i32; 2], a: [i32; 2], b: [i32; 2]) -> i32 {
        (a[0] as i32 - o[0] as i32) * (b[1] as i32 - o[1] as i32)
            - (a[1] as i32 - o[1] as i32) * (b[0] as i32 - o[0] as i32)
    }

    let mut lower = Vec::new();
    for &p in &pts {
        while lower.len() >= 2 && cross(lower[lower.len() - 2], lower[lower.len() - 1], p) <= 0 {
            lower.pop();
        }
        lower.push(p);
    }

    let mut upper = Vec::new();
    for &p in pts.iter().rev() {
        while upper.len() >= 2 && cross(upper[upper.len() - 2], upper[upper.len() - 1], p) <= 0 {
            upper.pop();
        }
        upper.push(p);
    }

    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower
}

fn random_convex_polygon_at_point(center: [i32; 2], radius: i32, count: usize) -> Vec<[i32; 2]> {
    let mut points = Vec::with_capacity(count);
    for _ in 0..count {
        let angle = gen_range(0.0f32, std::f32::consts::TAU);
        let r = gen_range(radius as f32 * 0.5, radius as f32);
        let x = center[0] + (r * angle.cos()) as i32;
        let y = center[1] + (r * angle.sin()) as i32;
        points.push([x, y]);
    }

    let mut hull = convex_hull(&points);
    if hull.len() < 3 {
        hull = vec![
            [center[0] - radius, center[1] - radius],
            [center[0] + radius, center[1] - radius],
            [center[0], center[1] + radius],
        ];
    }
    hull
}

fn polygon_from_ring_i32(ring: Vec<[i32; 2]>) -> DemoPolygon {
    DemoPolygon {
        exterior: ring
            .into_iter()
            .map(|[x, y]| [i32::from(x), i32::from(y)])
            .collect(),
        interiors: vec![],
        data: (),
    }
}

fn random_polygon_at_screen_click(center: Vec2, zoom: f32, mx: f32, my: f32) -> DemoPolygon {
    let click_world = vec2((mx - center.x) / zoom, -(my - center.y) / zoom);
    let radius = (60.0 / zoom).max(10.0).round() as i32;
    let count = gen_range(3, 10) as usize;
    let ring = random_convex_polygon_at_point(
        [click_world.x.round() as i32, click_world.y.round() as i32],
        radius,
        count,
    );
    polygon_from_ring_i32(ring)
}

fn exterior_centroid(exterior: &[[i32; 2]]) -> Option<[f32; 2]> {
    if exterior.is_empty() {
        return None;
    }

    let mut sum_x = 0.0f32;
    let mut sum_y = 0.0f32;

    for vertex in exterior {
        sum_x += vertex[0] as f32;
        sum_y += vertex[1] as f32;
    }

    let count = exterior.len() as f32;

    Some([sum_x / count, sum_y / count])
}

fn id_label_rect(label: &str, screen: Vec2, font_size: f32) -> Rect {
    let dims = measure_text(label, None, font_size as u16, 1.0);

    Rect::new(
        screen.x - dims.width * 0.5,
        screen.y + dims.offset_y * 0.5 - dims.height,
        dims.width.max(12.0),
        dims.height.max(12.0),
    )
}

fn draw_id_label(label: &str, screen: Vec2, color: Color, is_highlighted: bool) {
    let font_size = if is_highlighted { 28.0 } else { 18.0 };
    let color = if is_highlighted { WHITE } else { color };
    let dims = measure_text(label, None, font_size as u16, 1.0);
    let x = screen.x - dims.width * 0.5;
    let y = screen.y + dims.offset_y * 0.5;

    if is_highlighted {
        draw_text(label, x + 1.0, y, font_size, WHITE);
        draw_text(label, x, y + 1.0, font_size, WHITE);
    }

    draw_text(label, x, y, font_size, color);
}

fn draw_polygon_set_lines(
    polygon_set: &RecordingPolygonSet<i32, DemoPolygon>,
    center: Vec2,
    zoom: f32,
    color: Color,
    highlighted_ids: &BTreeSet<usize>,
) {
    for (index, polygon) in polygon_set.polygons().as_ref().iter() {
        let rings: Vec<&[[i32; 2]]> = std::iter::once(polygon.exterior.as_slice())
            .chain(polygon.interiors.iter().map(Vec::as_slice))
            .collect();

        for ring in rings.iter().copied() {
            for (from, to) in ring
                .iter()
                .zip(ring.iter().cycle().skip(1))
                .take(ring.len())
            {
                let start = center + vec2(from[0] as f32, -from[1] as f32) * zoom;
                let end = center + vec2(to[0] as f32, -to[1] as f32) * zoom;
                draw_line(start.x, start.y, end.x, end.y, 2.5, color);
            }
        }

        if let Some([world_x, world_y]) = exterior_centroid(&polygon.exterior) {
            let label = format!("{index}");
            let screen = center + vec2(world_x, -world_y) * zoom;

            draw_id_label(&label, screen, color, highlighted_ids.contains(&index));
        }
    }
}

fn pick_interpolygon_id(
    model: &DemoLaminate,
    interlamina_visible: &[bool],
    center: Vec2,
    zoom: f32,
    mouse: Vec2,
) -> Option<(usize, PolygonId)> {
    for interlamina_index in (0..model.interlaminas().len()).rev() {
        if !interlamina_visible[interlamina_index] {
            continue;
        }

        for (index, polygon) in model.interlaminas()[interlamina_index]
            .main()
            .polygons()
            .as_ref()
            .iter()
        {
            let Some([world_x, world_y]) = exterior_centroid(&polygon.exterior) else {
                continue;
            };

            let label = format!("{index}");
            let screen = center + vec2(world_x, -world_y) * zoom;
            let hit = id_label_rect(&label, screen, 18.0);

            let padded = Rect::new(hit.x - 6.0, hit.y - 6.0, hit.w + 12.0, hit.h + 12.0);

            if padded.contains(mouse) {
                return Some((interlamina_index, PolygonId::new(index)));
            }
        }
    }

    None
}

fn darken(color: Color, factor: f32) -> Color {
    Color::new(
        color.r * factor,
        color.g * factor,
        color.b * factor,
        color.a,
    )
}

const VISIBILITY_ROW_H: f32 = 28.0;
const VISIBILITY_ROW_GAP: f32 = 4.0;
const VISIBILITY_PANEL_X: f32 = 20.0;
const VISIBILITY_PANEL_W: f32 = 210.0;

fn lamina_visibility_row_top(index: usize) -> f32 {
    72.0 + index as f32 * (VISIBILITY_ROW_H + VISIBILITY_ROW_GAP)
}

fn interlamina_visibility_row_top(num_laminas: usize, index: usize) -> f32 {
    lamina_visibility_row_top(num_laminas)
        + 28.0
        + index as f32 * (VISIBILITY_ROW_H + VISIBILITY_ROW_GAP)
}

fn visibility_row_rect(y: f32) -> Rect {
    Rect::new(VISIBILITY_PANEL_X, y, VISIBILITY_PANEL_W, VISIBILITY_ROW_H)
}

fn visibility_checkbox_square(y: f32) -> Rect {
    Rect::new(VISIBILITY_PANEL_X + 8.0, y + 6.0, 16.0, 16.0)
}

fn visibility_panel_bottom(num_laminas: usize, num_interlaminas: usize) -> f32 {
    if num_interlaminas == 0 {
        lamina_visibility_row_top(num_laminas.saturating_sub(1)) + VISIBILITY_ROW_H
    } else {
        interlamina_visibility_row_top(num_laminas, num_interlaminas - 1) + VISIBILITY_ROW_H
    }
}

fn ui_blocks_click(
    mouse: Vec2,
    undo_button: Rect,
    redo_button: Rect,
    num_laminas: usize,
    num_interlaminas: usize,
) -> bool {
    if undo_button.contains(mouse) || redo_button.contains(mouse) {
        return true;
    }

    mouse.x >= VISIBILITY_PANEL_X
        && mouse.x <= VISIBILITY_PANEL_X + VISIBILITY_PANEL_W
        && mouse.y >= lamina_visibility_row_top(0)
        && mouse.y <= visibility_panel_bottom(num_laminas, num_interlaminas)
}

fn draw_visibility_checkbox(y: f32, is_on: bool, accent: Color, label: &str) {
    let row = visibility_row_rect(y);
    let bg = if is_on {
        Color::new(accent.r * 0.28, accent.g * 0.28, accent.b * 0.28, 0.92)
    } else {
        Color::new(0.12, 0.12, 0.12, 0.92)
    };

    draw_rectangle(row.x, row.y, row.w, row.h, bg);
    draw_rectangle_lines(row.x, row.y, row.w, row.h, 1.5, LIGHTGRAY);

    let sq = visibility_checkbox_square(y);

    draw_rectangle_lines(sq.x, sq.y, sq.w, sq.h, 2.0, LIGHTGRAY);

    if is_on {
        draw_rectangle(sq.x + 3.5, sq.y + 3.5, sq.w - 7.0, sq.h - 7.0, accent);
    }

    draw_text(label, row.x + 32.0, row.y + 19.0, 18.0, WHITE);
}

#[macroquad::main("lamina with interlamina")]
async fn main() {
    let mut undoredo: UndoRedo<DemoLayersDelta> = UndoRedo::new();
    let mut model = new_laminate_demo();
    let mut zoom = 1.0f32;
    let mut offset = vec2(0.0, 0.0);
    let mut show_hint = true;
    let mut last_middle_mouse: Option<Vec2> = None;
    let mut middle_press_start: Option<Vec2> = None;
    let mut middle_panned = false;

    let num_laminas = model.laminas().len();
    let num_interlaminas = model.interlaminas().len();
    let mut lamina_visible = vec![true; num_laminas];
    let mut interlamina_visible = vec![true; num_interlaminas];
    let mut selected_interpolygon: Option<(usize, PolygonId)> = None;

    let lamina_labels = ["TOP (L0)", "MIDDLE (L1)", "BOTTOM (L2)"];
    let lamina_accents = [RED, ORANGE, SKYBLUE];
    let interlamina_labels = ["inter 0 (L0-L1)", "inter 1 (L1-L2)"];
    let interlamina_accents = [YELLOW, MAGENTA];
    let empty_highlights = BTreeSet::new();

    loop {
        let undo_button = Rect::new(20.0, 20.0, 100.0, 36.0);
        let redo_button = Rect::new(130.0, 20.0, 100.0, 36.0);
        let (mx, my) = mouse_position();
        let mouse = vec2(mx, my);
        let screen_center = vec2(screen_width() * 0.5, screen_height() * 0.5);
        let mut center = screen_center + offset;

        let left_pressed = is_mouse_button_pressed(MouseButton::Left);
        let middle_pressed = is_mouse_button_pressed(MouseButton::Middle);
        let right_pressed = is_mouse_button_pressed(MouseButton::Right);
        if show_hint && (left_pressed || middle_pressed || right_pressed) {
            show_hint = false;
        }

        let undo_clicked = left_pressed && undo_button.contains(mouse);
        let redo_clicked = left_pressed && redo_button.contains(mouse);

        if undo_clicked {
            undoredo.undo(&mut model);
            selected_interpolygon = None;
        } else if redo_clicked {
            undoredo.redo(&mut model);
            selected_interpolygon = None;
        } else if left_pressed {
            let mut is_toggled = false;

            for (index, is_visible) in lamina_visible.iter_mut().enumerate() {
                if visibility_row_rect(lamina_visibility_row_top(index)).contains(mouse) {
                    *is_visible = !*is_visible;
                    is_toggled = true;
                    break;
                }
            }

            if !is_toggled {
                for (index, is_visible) in interlamina_visible.iter_mut().enumerate() {
                    if visibility_row_rect(interlamina_visibility_row_top(num_laminas, index))
                        .contains(mouse)
                    {
                        *is_visible = !*is_visible;
                        is_toggled = true;
                        break;
                    }
                }
            }

            if !is_toggled
                && !ui_blocks_click(
                    mouse,
                    undo_button,
                    redo_button,
                    num_laminas,
                    num_interlaminas,
                )
            {
                if let Some(picked) =
                    pick_interpolygon_id(&model, &interlamina_visible, center, zoom, mouse)
                {
                    selected_interpolygon = Some(picked);
                } else {
                    selected_interpolygon = None;
                    model.add_into_lamina(0, random_polygon_at_screen_click(center, zoom, mx, my));
                    undoredo.commit(&mut model);
                }
            }
        } else if right_pressed
            && !ui_blocks_click(
                mouse,
                undo_button,
                redo_button,
                num_laminas,
                num_interlaminas,
            )
        {
            selected_interpolygon = None;
            model.add_into_lamina(2, random_polygon_at_screen_click(center, zoom, mx, my));
            undoredo.commit(&mut model);
        }

        if middle_pressed {
            middle_press_start = Some(mouse);
            middle_panned = false;
            last_middle_mouse = Some(mouse);
        }
        if is_mouse_button_down(MouseButton::Middle) {
            if let Some(previous) = last_middle_mouse {
                let delta = mouse - previous;
                if delta.length_squared() > 0.0 {
                    offset += delta;
                    middle_panned = true;
                }
            }
            last_middle_mouse = Some(mouse);
        } else {
            last_middle_mouse = None;
        }
        if is_mouse_button_released(MouseButton::Middle) {
            if !middle_panned {
                if let Some(click_pos) = middle_press_start {
                    if !ui_blocks_click(
                        click_pos,
                        undo_button,
                        redo_button,
                        num_laminas,
                        num_interlaminas,
                    ) {
                        selected_interpolygon = None;
                        model.add_into_lamina(
                            1,
                            random_polygon_at_screen_click(center, zoom, click_pos.x, click_pos.y),
                        );
                        undoredo.commit(&mut model);
                    }
                }
            }
            middle_press_start = None;
            middle_panned = false;
        }

        let (_, scroll_y) = mouse_wheel();
        if scroll_y != 0.0 {
            center = screen_center + offset;
            let old_zoom = zoom;
            let world_before = vec2((mx - center.x) / old_zoom, -(my - center.y) / old_zoom);
            zoom = (old_zoom * (1.0 + scroll_y * 0.1)).clamp(0.1, 20.0);
            let new_center = vec2(mx - world_before.x * zoom, my + world_before.y * zoom);
            offset = new_center - screen_center;
        }
        center = screen_center + offset;

        clear_background(BLACK);

        let undo_hover = undo_button.contains(mouse);
        let redo_hover = redo_button.contains(mouse);
        let undo_pressed = undo_hover && is_mouse_button_down(MouseButton::Left);
        let redo_pressed = redo_hover && is_mouse_button_down(MouseButton::Left);
        let undo_fill = if undo_pressed {
            LIGHTGRAY
        } else if undo_hover {
            GRAY
        } else {
            DARKGRAY
        };
        let redo_fill = if redo_pressed {
            LIGHTGRAY
        } else if redo_hover {
            GRAY
        } else {
            DARKGRAY
        };

        draw_rectangle(
            undo_button.x,
            undo_button.y,
            undo_button.w,
            undo_button.h,
            undo_fill,
        );
        draw_text(
            "undo",
            undo_button.x + 26.0,
            undo_button.y + 24.0,
            28.0,
            WHITE,
        );
        draw_rectangle(
            redo_button.x,
            redo_button.y,
            redo_button.w,
            redo_button.h,
            redo_fill,
        );
        draw_text(
            "redo",
            redo_button.x + 26.0,
            redo_button.y + 24.0,
            28.0,
            WHITE,
        );

        if show_hint {
            let hint =
                "Left: TOP / click interpolygon id to highlight. Middle: MIDDLE, right: BOTTOM.";
            let hint_size = 22.0;
            let hint_dims = measure_text(hint, None, hint_size as u16, 1.0);
            draw_text(
                hint,
                (screen_width() - hint_dims.width) * 0.5,
                screen_height() - 32.0,
                hint_size,
                GRAY,
            );
        }

        if let Some((interlamina_index, interpolygon_id)) = selected_interpolygon {
            if model.interlaminas()[interlamina_index]
                .main()
                .polygons()
                .as_ref()
                .get(interpolygon_id.index())
                .is_none()
            {
                selected_interpolygon = None;
            }
        }

        let (highlight_interlamina, highlight_above, highlight_below) =
            if let Some((interlamina_index, interpolygon_id)) = selected_interpolygon {
                let above = model
                    .polygons_above(interlamina_index, interpolygon_id)
                    .into_iter()
                    .map(PolygonId::index)
                    .collect::<BTreeSet<_>>();
                let below = model
                    .polygons_below(interlamina_index, interpolygon_id)
                    .into_iter()
                    .map(PolygonId::index)
                    .collect::<BTreeSet<_>>();

                (
                    Some((interlamina_index, interpolygon_id.index())),
                    (interlamina_index, above),
                    (interlamina_index + 1, below),
                )
            } else {
                (
                    None,
                    (usize::MAX, BTreeSet::new()),
                    (usize::MAX, BTreeSet::new()),
                )
            };

        for (layer_i, base_color) in [(2usize, SKYBLUE), (1usize, ORANGE), (0usize, RED)] {
            if !lamina_visible[layer_i] {
                continue;
            }

            let lamina = &model.laminas()[layer_i];
            let core = lamina.main();

            let main = core.main().minuend();
            draw_polygon_set_lines(main, center, zoom, base_color, &empty_highlights);

            let parallel = core
                .sidings()
                .first()
                .expect("each core stack has one rail polygon-set")
                .minuend();
            let rail_highlights = if layer_i == highlight_above.0 {
                &highlight_above.1
            } else if layer_i == highlight_below.0 {
                &highlight_below.1
            } else {
                &empty_highlights
            };
            draw_polygon_set_lines(
                parallel,
                center,
                zoom,
                darken(base_color, 0.55),
                rail_highlights,
            );

            let periph = lamina
                .sidings()
                .first()
                .expect("each lamina has one peripheral stack");
            let periph_main = periph.main().minuend();
            draw_polygon_set_lines(
                periph_main,
                center,
                zoom,
                darken(base_color, 0.72),
                &empty_highlights,
            );
            let periph_rail = periph
                .sidings()
                .first()
                .expect("each peripheral has one rail polygon-set")
                .minuend();
            draw_polygon_set_lines(
                periph_rail,
                center,
                zoom,
                darken(base_color, 0.42),
                &empty_highlights,
            );
        }

        for (inter_i, accent) in [(0usize, YELLOW), (1usize, MAGENTA)] {
            if !interlamina_visible[inter_i] {
                continue;
            }

            let mut inter_highlights = BTreeSet::new();

            if let Some((selected_inter_i, selected_id)) = highlight_interlamina {
                if selected_inter_i == inter_i {
                    inter_highlights.insert(selected_id);
                }
            }

            draw_polygon_set_lines(
                model.interlaminas()[inter_i].main(),
                center,
                zoom,
                accent,
                &inter_highlights,
            );
        }

        draw_text("laminas", VISIBILITY_PANEL_X, 64.0, 16.0, GRAY);

        for (index, &is_on) in lamina_visible.iter().enumerate() {
            draw_visibility_checkbox(
                lamina_visibility_row_top(index),
                is_on,
                lamina_accents[index],
                lamina_labels[index],
            );
        }

        draw_text(
            "interlaminas",
            VISIBILITY_PANEL_X,
            interlamina_visibility_row_top(num_laminas, 0) - 8.0,
            16.0,
            GRAY,
        );

        for (index, &is_on) in interlamina_visible.iter().enumerate() {
            draw_visibility_checkbox(
                interlamina_visibility_row_top(num_laminas, index),
                is_on,
                interlamina_accents[index],
                interlamina_labels[index],
            );
        }

        next_frame().await;
    }
}
