// SPDX-FileCopyrightText: 2026 polygon_unionfind contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use alloc::vec::Vec;
use core::{iter::Map, slice::Iter};
use i_overlay::i_float::int::point::IntPoint;
use numlike::convert::{CastFrom, CastInto};
use rstared::{AABB, Envelope, primitives::Rectangle};

use crate::Scalar;

pub trait Rings<K> {
    type RingIter<'a>: Iterator<Item = &'a [[K; 2]]>
    where
        Self: 'a,
        K: 'a;

    fn exterior(&self) -> &[[K; 2]];
    fn interiors<'a>(&'a self) -> Self::RingIter<'a>;

    fn rings<'a>(&'a self) -> impl Iterator<Item = &'a [[K; 2]]> + 'a
    where
        K: 'a,
    {
        core::iter::once(self.exterior()).chain(self.interiors())
    }
}

pub(crate) fn rectangle_from_polygon<K: Scalar, P: Rings<K>>(polygon: &P) -> Rectangle<[K; 2]> {
    Rectangle::from_aabb(
        polygon
            .exterior()
            .iter()
            .fold(AABB::new_empty(), |aabb, vertex| {
                aabb.merged(&AABB::from_point([vertex[0].clone(), vertex[1].clone()]))
            }),
    )
}

pub(crate) fn from_i_overlay_ring<K: CastFrom<i32>>(ring: &[IntPoint]) -> Vec<[K; 2]> {
    ring.iter()
        .map(|vertex| [K::cast_from(vertex.x), K::cast_from(vertex.y)])
        .collect()
}

pub(crate) fn to_i_overlay_ring<K: Clone + CastInto<i32>>(ring: &[[K; 2]]) -> Vec<IntPoint> {
    ring.iter()
        .map(|vertex| IntPoint::new(vertex[0].clone().cast_into(), vertex[1].clone().cast_into()))
        .collect()
}

pub(crate) fn to_i_overlay_shape<K: Clone + CastInto<i32>, P: Rings<K>>(
    polygon: &P,
) -> Vec<Vec<IntPoint>> {
    polygon.rings().map(to_i_overlay_ring).collect()
}

/// An index pointing to a polygon.
///
/// This is just a thin newtype wrapper over [`usize`] for clarity and to
/// disambiguate it from other index types. Use the [`index()`] method to access
/// the underlying index.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct PolygonId(usize);

impl PolygonId {
    /// Wrap a vertex index in a newtype struct.
    #[inline]
    pub fn new(id: usize) -> Self {
        Self(id)
    }

    /// Returns the underlying index.
    #[inline]
    pub fn index(self) -> usize {
        self.0
    }
}

#[derive(Clone, Debug)]
pub struct Polygon<K> {
    /// Polygon boundary.
    pub exterior: Vec<[K; 2]>,
    /// Polygon interior rings.
    pub interiors: Vec<Vec<[K; 2]>>,
}

impl<K> Rings<K> for Polygon<K> {
    type RingIter<'a>
        = Map<Iter<'a, Vec<[K; 2]>>, fn(&'a Vec<[K; 2]>) -> &'a [[K; 2]]>
    where
        K: 'a;

    fn exterior(&self) -> &[[K; 2]] {
        self.exterior.as_slice()
    }

    fn interiors<'a>(&'a self) -> Self::RingIter<'a> {
        self.interiors.iter().map(Vec::as_slice)
    }
}

#[derive(Clone, Debug)]
pub struct PolygonWithData<K, W = ()> {
    /// Polygon boundary.
    pub exterior: Vec<[K; 2]>,
    /// Polygon interior rings.
    pub interiors: Vec<Vec<[K; 2]>>,
    /// User-defined payload for this polygon.
    pub data: W,
}

impl<K, W> Rings<K> for PolygonWithData<K, W> {
    type RingIter<'a>
        = Map<Iter<'a, Vec<[K; 2]>>, fn(&'a Vec<[K; 2]>) -> &'a [[K; 2]]>
    where
        K: 'a,
        W: 'a;

    fn exterior(&self) -> &[[K; 2]] {
        self.exterior.as_slice()
    }

    fn interiors<'a>(&'a self) -> Self::RingIter<'a> {
        self.interiors.iter().map(Vec::as_slice)
    }
}
