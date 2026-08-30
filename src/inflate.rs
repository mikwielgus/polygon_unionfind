// SPDX-FileCopyrightText: 2026 polygon_unionfind contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use alloc::vec::Vec;
use i_overlay::mesh::{outline::offset::OutlineOffset, style::OutlineStyle};

use crate::{Polygon, PolygonWithData};

pub trait Inflate<K> {
    fn inflate(self, offset: K) -> Self;
}

fn first_inflated_shape<K>(
    buffered: Vec<Vec<Vec<[K; 2]>>>,
) -> Option<(Vec<[K; 2]>, Vec<Vec<[K; 2]>>)> {
    rings_from_shape(buffered.into_iter().next()?)
}

fn shape_from_rings<K>(exterior: Vec<[K; 2]>, mut interiors: Vec<Vec<[K; 2]>>) -> Vec<Vec<[K; 2]>> {
    interiors.insert(0, exterior);
    interiors
}

fn rings_from_shape<K>(mut shape: Vec<Vec<[K; 2]>>) -> Option<(Vec<[K; 2]>, Vec<Vec<[K; 2]>>)> {
    if shape.is_empty() {
        return None;
    }
    let exterior = shape.remove(0);
    Some((exterior, shape))
}

impl<K, W> Inflate<K> for PolygonWithData<K, W>
where
    Polygon<K>: Inflate<K>,
{
    fn inflate(self, offset: K) -> Self {
        let PolygonWithData {
            exterior,
            interiors,
            data,
        } = self;
        let Polygon {
            exterior,
            interiors,
        } = Polygon {
            exterior,
            interiors,
        }
        .inflate(offset);
        PolygonWithData {
            exterior,
            interiors,
            data,
        }
    }
}

macro_rules! impl_inflate_float {
    ($k:ty) => {
        impl Inflate<$k> for Polygon<$k> {
            fn inflate(self, offset: $k) -> Self {
                let shape = shape_from_rings(self.exterior, self.interiors);
                let buffered = shape.outline(&OutlineStyle::new(offset));

                let (exterior, interiors) = first_inflated_shape(buffered)
                    .or_else(|| rings_from_shape(shape))
                    .unwrap_or_default();

                Polygon {
                    exterior,
                    interiors,
                }
            }
        }
    };
}

fn inflate_int_shape<K>(
    shape: Vec<Vec<[K; 2]>>,
    offset: K,
) -> Option<(Vec<[K; 2]>, Vec<Vec<[K; 2]>>)>
where
    K: Copy + TryFrom<i64>,
    i64: From<K>,
{
    let to_float_ring = |ring: Vec<[K; 2]>| -> Vec<[f64; 2]> {
        ring.into_iter()
            .map(|vertex| [i64::from(vertex[0]) as f64, i64::from(vertex[1]) as f64])
            .collect()
    };
    let from_float_ring = |ring: Vec<[f64; 2]>| -> Option<Vec<[K; 2]>> {
        ring.into_iter()
            .map(|vertex| {
                if !vertex[0].is_finite() || !vertex[1].is_finite() {
                    return None;
                }

                let x = vertex[0].round();
                let y = vertex[1].round();
                if x < i64::MIN as f64
                    || x > i64::MAX as f64
                    || y < i64::MIN as f64
                    || y > i64::MAX as f64
                {
                    return None;
                }

                Some([K::try_from(x as i64).ok()?, K::try_from(y as i64).ok()?])
            })
            .collect()
    };

    let float_shape = shape.into_iter().map(to_float_ring).collect::<Vec<_>>();
    let buffered = float_shape.outline(&OutlineStyle::new(i64::from(offset) as f64));

    let (exterior, interiors) = first_inflated_shape(buffered)?;
    let exterior = from_float_ring(exterior)?;
    let interiors = interiors
        .into_iter()
        .map(from_float_ring)
        .collect::<Option<Vec<_>>>()?;

    Some((exterior, interiors))
}

macro_rules! impl_inflate_int {
    ($k:ty) => {
        impl Inflate<$k> for Polygon<$k> {
            fn inflate(self, offset: $k) -> Self {
                let shape = shape_from_rings(self.exterior, self.interiors);
                let (exterior, interiors) = inflate_int_shape(shape.clone(), offset)
                    .or_else(|| rings_from_shape(shape))
                    .unwrap_or_default();

                Polygon {
                    exterior,
                    interiors,
                }
            }
        }
    };
}

impl_inflate_float!(f32);
impl_inflate_float!(f64);
impl_inflate_int!(i8);
impl_inflate_int!(i16);
impl_inflate_int!(i32);
impl_inflate_int!(i64);
