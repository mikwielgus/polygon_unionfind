// SPDX-FileCopyrightText: 2026 polygon_unionfind contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

// This file was developed by prompting OpenAI Codex 5.3.

use macroquad::prelude::*;
use macroquad::rand::gen_range;
use polygon_unionfind::{
    Inflated, LaminateDelta, Negated, Paralleled, PolygonSetHalfDelta, PolygonUnionFindHalfDelta,
    PolygonWithData, RecordingInflated, RecordingLaminate, RecordingNegated, RecordingPolygonSet,
    RecordingPolygonUnionFind,
};
use undoredo::UndoRedo;

type DemoPolygon = PolygonWithData<i32, ()>;
type DemoNegated = RecordingNegated<i32, DemoPolygon>;
type DemoLayerWithParallels = Paralleled<Paralleled<DemoNegated>>;
type DemoTransitionLayer = Paralleled<RecordingPolygonSet<i32, DemoPolygon>>;
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

struct Layers {
    inner: DemoLaminate,
}

impl Layers {
    fn new() -> Self {
        let new_negated = |offset| {
            let mut minuend: RecordingPolygonSet<i32, DemoPolygon> = RecordingPolygonSet::new();
            let _ = minuend.add(DemoPolygon {
                exterior: vec![[-2000, -2000], [2000, -2000], [2000, 2000], [-2000, 2000]],
                interiors: vec![],
                data: (),
            });

            RecordingNegated::new(minuend, RecordingInflated::<i32, DemoPolygon>::new(offset))
        };
        let new_layer = || {
            DemoLayerWithParallels::new(
                Paralleled::new(new_negated(0), vec![new_negated(50)]),
                vec![],
            )
        };
        let new_transition = || DemoTransitionLayer::new(RecordingPolygonSet::new(), vec![]);

        Self {
            inner: DemoLaminate::with_laminas_interlaminas(
                vec![new_layer(), new_layer()],
                vec![new_transition()],
            ),
        }
    }

    fn include_top(&mut self, polygon: DemoPolygon) {
        self.inner.add_into_lamina(0, polygon);
    }

    fn include_bottom(&mut self, polygon: DemoPolygon) {
        self.inner.add_into_lamina(1, polygon);
    }

    fn top_result(&self) -> &RecordingPolygonSet<i32, DemoPolygon> {
        self.inner.laminas()[0].primary().primary().minuend()
    }

    fn top_subtrahend(&self) -> &RecordingPolygonUnionFind<i32, DemoPolygon> {
        self.inner.laminas()[0]
            .primary()
            .primary()
            .subtrahend()
            .inflatee()
    }

    fn parallel_result(&self) -> &RecordingPolygonSet<i32, DemoPolygon> {
        self.inner.laminas()[0].primary().parallels()[0].minuend()
    }

    fn parallel_subtrahend(&self) -> &RecordingPolygonUnionFind<i32, DemoPolygon> {
        self.inner.laminas()[0].primary().parallels()[0]
            .subtrahend()
            .inflatee()
    }

    fn bottom_result(&self) -> &RecordingPolygonSet<i32, DemoPolygon> {
        self.inner.laminas()[1].primary().primary().minuend()
    }

    fn bottom_subtrahend(&self) -> &RecordingPolygonUnionFind<i32, DemoPolygon> {
        self.inner.laminas()[1]
            .primary()
            .primary()
            .subtrahend()
            .inflatee()
    }

    fn bottom_parallel_result(&self) -> &RecordingPolygonSet<i32, DemoPolygon> {
        self.inner.laminas()[1].primary().parallels()[0].minuend()
    }

    fn bottom_parallel_subtrahend(&self) -> &RecordingPolygonUnionFind<i32, DemoPolygon> {
        self.inner.laminas()[1].primary().parallels()[0]
            .subtrahend()
            .inflatee()
    }
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

fn darken(color: Color, factor: f32) -> Color {
    Color::new(
        color.r * factor,
        color.g * factor,
        color.b * factor,
        color.a,
    )
}

#[macroquad::main("Combinator Layers Viewer")]
async fn main() {
    let mut undoredo: UndoRedo<DemoLayersDelta> = UndoRedo::new();
    let mut layers = Layers::new();

    let mut zoom = 1.0f32;
    let mut offset = vec2(0.0, 0.0);
    let mut last_mouse_pos: Option<Vec2> = None;
    let mut show_hint = true;

    loop {
        let undo_button = Rect::new(20.0, 20.0, 100.0, 36.0);
        let redo_button = Rect::new(130.0, 20.0, 100.0, 36.0);

        let (mx, my) = mouse_position();
        let mouse = vec2(mx, my);

        let left_pressed = is_mouse_button_pressed(MouseButton::Left);
        let right_pressed = is_mouse_button_pressed(MouseButton::Right);
        if show_hint && (left_pressed || right_pressed) {
            show_hint = false;
        }

        let undo_clicked = left_pressed && undo_button.contains(mouse);
        let redo_clicked = left_pressed && redo_button.contains(mouse);

        let screen_center = vec2(screen_width() * 0.5, screen_height() * 0.5);
        let mut center = screen_center + offset;

        if undo_clicked {
            undoredo.undo(&mut layers.inner);
        } else if redo_clicked {
            undoredo.redo(&mut layers.inner);
        } else if left_pressed {
            layers.include_top(random_polygon_at_screen_click(center, zoom, mx, my));
            undoredo.commit(&mut layers.inner);
        } else if right_pressed {
            layers.include_bottom(random_polygon_at_screen_click(center, zoom, mx, my));
            undoredo.commit(&mut layers.inner);
        }

        let (_, scroll_y) = mouse_wheel();
        if scroll_y != 0.0 {
            let old_zoom = zoom;
            let world_before = vec2((mx - center.x) / old_zoom, -(my - center.y) / old_zoom);
            zoom = (old_zoom * (1.0 + scroll_y * 0.1)).clamp(0.1, 20.0);
            let new_center = vec2(mx - world_before.x * zoom, my + world_before.y * zoom);
            offset = new_center - screen_center;
        }

        if is_mouse_button_down(MouseButton::Middle) {
            if let Some(previous) = last_mouse_pos {
                offset += mouse - previous;
            }
            last_mouse_pos = Some(mouse);
        } else {
            last_mouse_pos = None;
        }
        center = screen_center + offset;

        clear_background(BLACK);

        if show_hint {
            let hint =
                "Left-click to add to TOP, right-click to add to BOTTOM. Middle-drag to pan.";
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

        let polygon_set = layers.top_result();
        for geom_with_data in polygon_set.rtree().as_ref().iter() {
            let [bbox_min_x, bbox_min_y] = geom_with_data.geom().lower();
            let [bbox_max_x, bbox_max_y] = geom_with_data.geom().upper();

            let bbox_origin = center + vec2(bbox_min_x as f32, -bbox_max_y as f32) * zoom;
            let bbox_width = (bbox_max_x as f32 - bbox_min_x as f32) * zoom;
            let bbox_height = (bbox_max_y as f32 - bbox_min_y as f32) * zoom;
            draw_rectangle_lines(
                bbox_origin.x,
                bbox_origin.y,
                bbox_width,
                bbox_height,
                2.0,
                DARKGRAY,
            );
        }

        let bottom_set = layers.bottom_result();
        for geom_with_data in bottom_set.rtree().as_ref().iter() {
            let [bbox_min_x, bbox_min_y] = geom_with_data.geom().lower();
            let [bbox_max_x, bbox_max_y] = geom_with_data.geom().upper();

            let bbox_origin = center + vec2(bbox_min_x as f32, -bbox_max_y as f32) * zoom;
            let bbox_width = (bbox_max_x as f32 - bbox_min_x as f32) * zoom;
            let bbox_height = (bbox_max_y as f32 - bbox_min_y as f32) * zoom;
            draw_rectangle_lines(
                bbox_origin.x,
                bbox_origin.y,
                bbox_width,
                bbox_height,
                2.0,
                GRAY,
            );
        }

        for (i, (_index, polygon)) in polygon_set.polygons().as_ref().iter().enumerate() {
            let _ = i;
            let color = RED;
            let rings: Vec<&[[i32; 2]]> = std::iter::once(polygon.exterior.as_slice())
                .chain(polygon.interiors.iter().map(Vec::as_slice))
                .collect();
            for ring in rings.iter().copied() {
                for window in ring
                    .iter()
                    .zip(ring.iter().cycle().skip(1))
                    .take(ring.len())
                {
                    let (from, to) = window;
                    let start = center + vec2(from[0] as f32, -from[1] as f32) * zoom;
                    let end = center + vec2(to[0] as f32, -to[1] as f32) * zoom;
                    draw_line(start.x, start.y, end.x, end.y, 3.0, color);
                }
            }
        }

        let subtrahend = layers.top_subtrahend();
        let dark_color = darken(RED, 0.45);
        let raw = subtrahend.raw_polygons();
        for (i, polygon_id) in subtrahend
            .rtree()
            .as_ref()
            .iter()
            .map(|geom_with_data| geom_with_data.data)
            .enumerate()
        {
            let _ = i;
            let polygon = raw.get(&polygon_id.index()).unwrap();
            let color = dark_color;
            let rings: Vec<&[[i32; 2]]> = std::iter::once(polygon.exterior.as_slice())
                .chain(polygon.interiors.iter().map(Vec::as_slice))
                .collect();
            for ring in rings.iter().copied() {
                for window in ring
                    .iter()
                    .zip(ring.iter().cycle().skip(1))
                    .take(ring.len())
                {
                    let (from, to) = window;
                    let start = center + vec2(from[0] as f32, -from[1] as f32) * zoom;
                    let end = center + vec2(to[0] as f32, -to[1] as f32) * zoom;
                    draw_line(start.x, start.y, end.x, end.y, 2.0, color);
                }
            }
        }

        let parallel_set = layers.parallel_result();
        for (i, (_index, polygon)) in parallel_set.polygons().as_ref().iter().enumerate() {
            let _ = i;
            let color = ORANGE;
            let rings: Vec<&[[i32; 2]]> = std::iter::once(polygon.exterior.as_slice())
                .chain(polygon.interiors.iter().map(Vec::as_slice))
                .collect();
            for ring in rings.iter().copied() {
                for window in ring
                    .iter()
                    .zip(ring.iter().cycle().skip(1))
                    .take(ring.len())
                {
                    let (from, to) = window;
                    let start = center + vec2(from[0] as f32, -from[1] as f32) * zoom;
                    let end = center + vec2(to[0] as f32, -to[1] as f32) * zoom;
                    draw_line(start.x, start.y, end.x, end.y, 2.0, color);
                }
            }
        }

        let parallel_subtrahend = layers.parallel_subtrahend();
        let parallel_dark = darken(ORANGE, 0.45);
        let raw_parallel = parallel_subtrahend.raw_polygons();
        for (i, polygon_id) in parallel_subtrahend
            .rtree()
            .as_ref()
            .iter()
            .map(|geom_with_data| geom_with_data.data)
            .enumerate()
        {
            let _ = i;
            let polygon = raw_parallel.get(&polygon_id.index()).unwrap();
            let color = parallel_dark;
            let rings: Vec<&[[i32; 2]]> = std::iter::once(polygon.exterior.as_slice())
                .chain(polygon.interiors.iter().map(Vec::as_slice))
                .collect();
            for ring in rings.iter().copied() {
                for window in ring
                    .iter()
                    .zip(ring.iter().cycle().skip(1))
                    .take(ring.len())
                {
                    let (from, to) = window;
                    let start = center + vec2(from[0] as f32, -from[1] as f32) * zoom;
                    let end = center + vec2(to[0] as f32, -to[1] as f32) * zoom;
                    draw_line(start.x, start.y, end.x, end.y, 1.5, color);
                }
            }
        }

        let bottom_color = Color::new(0.9, 0.9, 0.9, 1.0);
        for (i, (_index, polygon)) in bottom_set.polygons().as_ref().iter().enumerate() {
            let _ = i;
            let color = bottom_color;
            let rings: Vec<&[[i32; 2]]> = std::iter::once(polygon.exterior.as_slice())
                .chain(polygon.interiors.iter().map(Vec::as_slice))
                .collect();
            for ring in rings.iter().copied() {
                for window in ring
                    .iter()
                    .zip(ring.iter().cycle().skip(1))
                    .take(ring.len())
                {
                    let (from, to) = window;
                    let start = center + vec2(from[0] as f32, -from[1] as f32) * zoom;
                    let end = center + vec2(to[0] as f32, -to[1] as f32) * zoom;
                    draw_line(start.x, start.y, end.x, end.y, 2.5, color);
                }
            }
        }

        let bottom_subtrahend = layers.bottom_subtrahend();
        let bottom_dark = darken(bottom_color, 0.45);
        let bottom_raw = bottom_subtrahend.raw_polygons();
        for (i, polygon_id) in bottom_subtrahend
            .rtree()
            .as_ref()
            .iter()
            .map(|geom_with_data| geom_with_data.data)
            .enumerate()
        {
            let _ = i;
            let polygon = bottom_raw.get(&polygon_id.index()).unwrap();
            let color = bottom_dark;
            let rings: Vec<&[[i32; 2]]> = std::iter::once(polygon.exterior.as_slice())
                .chain(polygon.interiors.iter().map(Vec::as_slice))
                .collect();
            for ring in rings.iter().copied() {
                for window in ring
                    .iter()
                    .zip(ring.iter().cycle().skip(1))
                    .take(ring.len())
                {
                    let (from, to) = window;
                    let start = center + vec2(from[0] as f32, -from[1] as f32) * zoom;
                    let end = center + vec2(to[0] as f32, -to[1] as f32) * zoom;
                    draw_line(start.x, start.y, end.x, end.y, 1.5, color);
                }
            }
        }

        let bottom_parallel_set = layers.bottom_parallel_result();
        for (i, (_index, polygon)) in bottom_parallel_set.polygons().as_ref().iter().enumerate() {
            let _ = i;
            let color = darken(bottom_color, 0.75);
            let rings: Vec<&[[i32; 2]]> = std::iter::once(polygon.exterior.as_slice())
                .chain(polygon.interiors.iter().map(Vec::as_slice))
                .collect();
            for ring in rings.iter().copied() {
                for window in ring
                    .iter()
                    .zip(ring.iter().cycle().skip(1))
                    .take(ring.len())
                {
                    let (from, to) = window;
                    let start = center + vec2(from[0] as f32, -from[1] as f32) * zoom;
                    let end = center + vec2(to[0] as f32, -to[1] as f32) * zoom;
                    draw_line(start.x, start.y, end.x, end.y, 1.5, color);
                }
            }
        }

        let bottom_parallel_subtrahend = layers.bottom_parallel_subtrahend();
        let bottom_parallel_raw = bottom_parallel_subtrahend.raw_polygons();
        for (i, polygon_id) in bottom_parallel_subtrahend
            .rtree()
            .as_ref()
            .iter()
            .map(|geom_with_data| geom_with_data.data)
            .enumerate()
        {
            let _ = i;
            let polygon = bottom_parallel_raw.get(&polygon_id.index()).unwrap();
            let color = darken(bottom_color, 0.35);
            let rings: Vec<&[[i32; 2]]> = std::iter::once(polygon.exterior.as_slice())
                .chain(polygon.interiors.iter().map(Vec::as_slice))
                .collect();
            for ring in rings.iter().copied() {
                for window in ring
                    .iter()
                    .zip(ring.iter().cycle().skip(1))
                    .take(ring.len())
                {
                    let (from, to) = window;
                    let start = center + vec2(from[0] as f32, -from[1] as f32) * zoom;
                    let end = center + vec2(to[0] as f32, -to[1] as f32) * zoom;
                    draw_line(start.x, start.y, end.x, end.y, 1.0, color);
                }
            }
        }

        next_frame().await;
    }
}
