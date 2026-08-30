// SPDX-FileCopyrightText: 2026 polygon_unionfind contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#![doc(html_root_url = "https://docs.rs/polygon_unionfind")]
#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, doc = "\n## Feature flags\n")]
#![cfg_attr(docsrs, doc = document_features::document_features!())]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![no_std]

// No feature for `alloc` because it would be always enabled anyway.
extern crate alloc;

mod bool_ops;
mod centroid;
mod combinators;
mod inflate;
mod laminate;
mod polygon;
mod polygon_set;
mod polygon_unionfind;
mod unionfind;

pub use bool_ops::{Difference, Intersection, Union};
pub use centroid::Centroid;
pub use combinators::{Inflated, Negated, Paralleled};
#[cfg(feature = "undoredo")]
pub use combinators::{RecordingInflated, RecordingNegated, RecordingParalleled};
pub use inflate::Inflate;
pub use laminate::Laminate;
#[cfg(feature = "undoredo")]
pub use laminate::{LaminateDelta, LaminateHalfDelta, RecordingLaminate};
pub use polygon::{Polygon, PolygonId, PolygonWithData, Rings};
pub use polygon_set::PolygonSet;
#[cfg(feature = "undoredo")]
pub use polygon_set::{PolygonSetDelta, PolygonSetHalfDelta, RecordingPolygonSet};
pub use polygon_unionfind::PolygonUnionFind;
#[cfg(feature = "undoredo")]
pub use polygon_unionfind::{
    PolygonUnionFindDelta, PolygonUnionFindHalfDelta, RecordingPolygonUnionFind,
};
pub use unionfind::UnionFind;

pub trait Add<P> {
    type Output;

    fn add(&mut self, polygon: P) -> Self::Output;
}

pub trait Sub<P> {
    type Output;

    fn sub(&mut self, polygon: P) -> Self::Output;
}

pub trait Clip<P> {
    type Output;

    fn clip(&mut self, polygon: P) -> Self::Output;
}
