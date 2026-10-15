// SPDX-FileCopyrightText: 2026 polygon_unionfind contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use alloc::vec::Vec;

use i_overlay::core::{fill_rule::FillRule, overlay::Overlay, overlay_rule::OverlayRule};

use crate::{
    Contours, Polygon, PolygonWithData, Scalar,
    polygon::{from_i_overlay_contour, to_i_overlay_shape},
};

pub trait Union<T> {
    fn union(subj: T, clip: T) -> Option<T>;
}

pub trait Intersection<T> {
    fn intersection(subj: T, clip: T) -> Vec<T>;
}

pub trait Difference<T> {
    fn difference(subj: T, clip: T) -> Vec<T>;
}

fn overlay_into_single_polygon<K: Scalar, A: Contours<K>, B: Contours<K>>(
    a: &A,
    b: &B,
    overlay_rule: OverlayRule,
) -> Option<(Vec<[K; 2]>, Vec<Vec<[K; 2]>>)> {
    let shapes = Overlay::from_subj_and_clip(&to_i_overlay_shape(a), &to_i_overlay_shape(b))
        .overlay(overlay_rule, FillRule::EvenOdd);

    if shapes.len() >= 2 {
        return None;
    }

    let shape = shapes.first()?;

    if shape.is_empty() {
        return None;
    }

    Some((
        from_i_overlay_contour(&shape[0]),
        shape[1..]
            .iter()
            .map(|ring| from_i_overlay_contour(ring))
            .collect(),
    ))
}

fn overlay_into_many_polygons<K: Scalar, A: Contours<K>, B: Contours<K>>(
    a: &A,
    b: &B,
    overlay_rule: OverlayRule,
) -> Option<Vec<(Vec<[K; 2]>, Vec<Vec<[K; 2]>>)>> {
    Overlay::from_subj_and_clip(&to_i_overlay_shape(a), &to_i_overlay_shape(b))
        .overlay(overlay_rule, FillRule::EvenOdd)
        .into_iter()
        .map(|shape| {
            if shape.is_empty() {
                return None;
            }

            Some((
                from_i_overlay_contour(&shape[0]),
                shape[1..]
                    .iter()
                    .map(|ring| from_i_overlay_contour(ring))
                    .collect(),
            ))
        })
        .collect()
}

impl<K: Scalar> Union<Polygon<K>> for Polygon<K> {
    fn union(a: Polygon<K>, b: Polygon<K>) -> Option<Polygon<K>> {
        let (exterior, interiors) = overlay_into_single_polygon(&a, &b, OverlayRule::Union)?;

        Some(Polygon {
            exterior,
            interiors,
        })
    }
}

impl<K: Scalar, W> Union<PolygonWithData<K, W>> for PolygonWithData<K, W> {
    fn union(a: PolygonWithData<K, W>, b: PolygonWithData<K, W>) -> Option<PolygonWithData<K, W>> {
        let (exterior, interiors) = overlay_into_single_polygon(&a, &b, OverlayRule::Union)?;

        Some(PolygonWithData {
            exterior,
            interiors,
            data: a.data,
        })
    }
}

impl<K: Scalar> Intersection<Polygon<K>> for Polygon<K> {
    fn intersection(a: Polygon<K>, b: Polygon<K>) -> Vec<Polygon<K>> {
        overlay_into_many_polygons(&a, &b, OverlayRule::Intersect)
            .unwrap_or_default()
            .into_iter()
            .map(|(exterior, interiors)| Polygon {
                exterior,
                interiors,
            })
            .collect()
    }
}

impl<K: Scalar, W: Clone> Intersection<PolygonWithData<K, W>> for PolygonWithData<K, W> {
    fn intersection(
        a: PolygonWithData<K, W>,
        b: PolygonWithData<K, W>,
    ) -> Vec<PolygonWithData<K, W>> {
        overlay_into_many_polygons(&a, &b, OverlayRule::Intersect)
            .unwrap_or_default()
            .into_iter()
            .map(|(exterior, interiors)| PolygonWithData {
                exterior,
                interiors,
                data: a.data.clone(),
            })
            .collect()
    }
}

impl<K: Scalar> Difference<Polygon<K>> for Polygon<K> {
    fn difference(a: Polygon<K>, b: Polygon<K>) -> Vec<Polygon<K>> {
        overlay_into_many_polygons(&a, &b, OverlayRule::Difference)
            .unwrap_or_default()
            .into_iter()
            .map(|(exterior, interiors)| Polygon {
                exterior,
                interiors,
            })
            .collect()
    }
}

impl<K: Scalar, W: Clone> Difference<PolygonWithData<K, W>> for PolygonWithData<K, W> {
    fn difference(
        a: PolygonWithData<K, W>,
        b: PolygonWithData<K, W>,
    ) -> Vec<PolygonWithData<K, W>> {
        overlay_into_many_polygons(&a, &b, OverlayRule::Difference)
            .unwrap_or_default()
            .into_iter()
            .map(|(exterior, interiors)| PolygonWithData {
                exterior,
                interiors,
                data: a.data.clone(),
            })
            .collect()
    }
}
