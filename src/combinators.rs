// SPDX-FileCopyrightText: 2026 polygon_unionfind contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use alloc::vec::Vec;

use maplike::{
    abc::{Container, Keyed},
    ops::Get,
};

use crate::{Add, Clip, Inflate, PolygonId, Sub};
#[cfg(feature = "undoredo")]
use crate::{Polygon, RecordingPolygonSet, RecordingPolygonUnionFind};

#[cfg(feature = "undoredo")]
use core::marker::PhantomData;
#[cfg(feature = "undoredo")]
use undoredo::{ApplyDelta, Delta, FlushDelta};

#[derive(Clone, Debug)]
pub struct Inflated<I, K> {
    inflatee: I,
    offset: K,
}

impl<I, K> Inflated<I, K> {
    #[inline]
    pub fn inflatee(&self) -> &I {
        &self.inflatee
    }

    #[inline]
    pub fn offset(&self) -> &K {
        &self.offset
    }
}

impl<I: Default, K> Inflated<I, K> {
    #[inline]
    pub fn new(offset: K) -> Self {
        Self {
            inflatee: I::default(),
            offset,
        }
    }
}

impl<K: Clone, P: Clone + Inflate<K>, S: Add<P, Output = PolygonId>> Add<P> for Inflated<S, K> {
    type Output = PolygonId;

    fn add(&mut self, polygon: P) -> PolygonId {
        self.inflatee.add(polygon.inflate(self.offset.clone()))
    }
}

impl<K: Clone, P: Clone + Inflate<K>, S: Sub<P>> Sub<P> for Inflated<S, K> {
    type Output = S::Output;

    fn sub(&mut self, polygon: P) -> S::Output {
        self.inflatee.sub(polygon.inflate(self.offset.clone()))
    }
}

impl<I: Keyed, K> Container for Inflated<I, K> {
    type Value = I::Value;
}

impl<I: Keyed, K> Keyed for Inflated<I, K> {
    type Key = I::Key;
}

impl<I: Get<K2>, K, K2> Get<K2> for Inflated<I, K> {
    #[inline]
    fn get(&self, key: &K2) -> Option<&Self::Value> {
        self.inflatee.get(key)
    }
}

#[derive(Clone, Debug)]
pub struct Negated<M, S> {
    minuend: M,
    subtrahend: S,
}

impl<M, S> Negated<M, S> {
    #[inline]
    pub fn minuend(&self) -> &M {
        &self.minuend
    }

    #[inline]
    pub fn subtrahend(&self) -> &S {
        &self.subtrahend
    }

    #[inline]
    pub fn new(minuend: M, subtrahend: S) -> Self {
        Self {
            minuend,
            subtrahend,
        }
    }
}

impl<M: Default, S: Default> Default for Negated<M, S> {
    #[inline]
    fn default() -> Self {
        Self {
            minuend: M::default(),
            subtrahend: S::default(),
        }
    }
}

impl<
    P: Clone,
    M: Sub<P, Output = (Vec<PolygonId>, Vec<P>)>,
    S: Add<P, Output = PolygonId> + Get<PolygonId, Value = P>,
> Add<P> for Negated<M, S>
{
    type Output = (Vec<PolygonId>, Vec<P>);

    fn add(&mut self, polygon: P) -> (Vec<PolygonId>, Vec<P>) {
        let id = self.subtrahend.add(polygon.clone());

        self.minuend.sub(self.subtrahend.get(&id).unwrap().clone())
    }
}

#[derive(Clone, Debug)]
pub struct Paralleled<S> {
    main: S,
    sidings: Vec<S>,
}

impl<S> Paralleled<S> {
    #[inline]
    pub fn main(&self) -> &S {
        &self.main
    }

    #[inline]
    pub fn sidings(&self) -> &Vec<S> {
        &self.sidings
    }

    #[inline]
    pub fn row(&self, row: usize) -> &S {
        if row >= 1 {
            &self.sidings[row - 1]
        } else {
            &self.main
        }
    }
}

impl<S> Paralleled<S> {
    #[inline]
    pub fn new(main: S, sidings: Vec<S>) -> Self {
        Self {
            main: main,
            sidings: sidings,
        }
    }
}

impl<P: Clone, S: Add<P>> Add<P> for Paralleled<S> {
    type Output = (S::Output, Vec<S::Output>);

    fn add(&mut self, polygon: P) -> (S::Output, Vec<S::Output>) {
        let siding_outputs = self
            .sidings
            .iter_mut()
            .map(|siding| siding.add(polygon.clone()))
            .collect();
        let main_output = self.main.add(polygon);

        (main_output, siding_outputs)
    }
}

impl<P: Clone, S: Sub<P>> Sub<P> for Paralleled<S> {
    type Output = (S::Output, Vec<S::Output>);

    fn sub(&mut self, polygon: P) -> (S::Output, Vec<S::Output>) {
        let siding_outputs = self
            .sidings
            .iter_mut()
            .map(|siding| siding.sub(polygon.clone()))
            .collect();
        let main_output = self.main.sub(polygon);

        (main_output, siding_outputs)
    }
}

impl<P: Clone, S: Clip<P>> Clip<P> for Paralleled<S> {
    type Output = S::Output;

    fn clip(&mut self, polygon: P) -> S::Output {
        for siding in &mut self.sidings {
            siding.clip(polygon.clone());
        }

        self.main.clip(polygon)
    }
}

#[cfg(feature = "undoredo")]
/// `Inflated` that records changes for delta-based Undo/Redo.
pub type RecordingInflated<K, P = Polygon<K>> = Inflated<RecordingPolygonUnionFind<K, P>, K>;

#[cfg(feature = "undoredo")]
/// Half-delta of `Inflated`.
pub type InflatedHalfDelta<IE, K> = Inflated<IE, PhantomData<K>>;

#[cfg(feature = "undoredo")]
/// Delta of `Inflated` for delta-based Undo/Redo.
pub type InflatedDelta<IE, K> = Delta<InflatedHalfDelta<IE, K>>;

#[cfg(feature = "undoredo")]
impl<IE: Clone, IC: Clone + ApplyDelta<IE>, K> ApplyDelta<InflatedHalfDelta<IE, K>>
    for Inflated<IC, K>
{
    fn apply_delta(&mut self, delta: InflatedDelta<IE, K>) {
        let (removed, inserted) = delta.dissolve();

        let inflatee_delta = Delta::with_removed_inserted(removed.inflatee, inserted.inflatee);
        self.inflatee.apply_delta(inflatee_delta);
    }
}

#[cfg(feature = "undoredo")]
impl<IE: Clone, IC: FlushDelta<IE>, K> FlushDelta<InflatedHalfDelta<IE, K>> for Inflated<IC, K> {
    fn flush_delta(&mut self) -> InflatedDelta<IE, K> {
        let (removed_inflatee, inserted_inflatee) = self.inflatee.flush_delta().dissolve();

        Delta::with_removed_inserted(
            Inflated {
                inflatee: removed_inflatee,
                offset: PhantomData,
            },
            Inflated {
                inflatee: inserted_inflatee,
                offset: PhantomData,
            },
        )
    }
}

#[cfg(feature = "undoredo")]
/// `Negated` that records changes for delta-based Undo/Redo.
pub type RecordingNegated<K, P = Polygon<K>> =
    Negated<RecordingPolygonSet<K, P>, RecordingInflated<K, P>>;

#[cfg(feature = "undoredo")]
/// Half-delta of `Negated`.
pub type NegatedHalfDelta<ME, SE> = Negated<ME, SE>;

#[cfg(feature = "undoredo")]
/// Delta of `Negated` for delta-based Undo/Redo.
pub type NegatedDelta<ME, SE> = Delta<NegatedHalfDelta<ME, SE>>;

#[cfg(feature = "undoredo")]
impl<ME: Clone, M: Clone + ApplyDelta<ME>, SE: Clone, S: Clone + ApplyDelta<SE>>
    ApplyDelta<NegatedHalfDelta<ME, SE>> for Negated<M, S>
{
    fn apply_delta(&mut self, delta: NegatedDelta<ME, SE>) {
        let (removed, inserted) = delta.dissolve();

        let minuend_delta = Delta::with_removed_inserted(removed.minuend, inserted.minuend);
        self.minuend.apply_delta(minuend_delta);

        let subtrahend_delta =
            Delta::with_removed_inserted(removed.subtrahend, inserted.subtrahend);
        self.subtrahend.apply_delta(subtrahend_delta);
    }
}

#[cfg(feature = "undoredo")]
impl<ME: Clone, M: FlushDelta<ME>, SE: Clone, S: FlushDelta<SE>>
    FlushDelta<NegatedHalfDelta<ME, SE>> for Negated<M, S>
{
    fn flush_delta(&mut self) -> NegatedDelta<ME, SE> {
        let (removed_minuend, inserted_minuend) = self.minuend.flush_delta().dissolve();
        let (removed_subtrahend, inserted_subtrahend) = self.subtrahend.flush_delta().dissolve();

        Delta::with_removed_inserted(
            Negated {
                minuend: removed_minuend,
                subtrahend: removed_subtrahend,
            },
            Negated {
                minuend: inserted_minuend,
                subtrahend: inserted_subtrahend,
            },
        )
    }
}

#[cfg(feature = "undoredo")]
/// `Paralleled` that records changes for delta-based Undo/Redo.
pub type RecordingParalleled<K, P = Polygon<K>> = Paralleled<RecordingNegated<K, P>>;

#[cfg(feature = "undoredo")]
/// Half-delta of `Paralleled`.
pub type ParalleledHalfDelta<SE> = Paralleled<SE>;

#[cfg(feature = "undoredo")]
/// Delta of `Paralleled` for delta-based Undo/Redo.
pub type ParalleledDelta<SE> = Delta<ParalleledHalfDelta<SE>>;

#[cfg(feature = "undoredo")]
impl<SE: Clone, S: Clone + ApplyDelta<SE>> ApplyDelta<ParalleledHalfDelta<SE>> for Paralleled<S> {
    fn apply_delta(&mut self, delta: ParalleledDelta<SE>) {
        let (removed, inserted) = delta.dissolve();

        let main_delta = Delta::with_removed_inserted(removed.main, inserted.main);
        self.main.apply_delta(main_delta);

        for (siding, (removed_siding, inserted_siding)) in self.sidings.iter_mut().zip(
            removed
                .sidings
                .into_iter()
                .zip(inserted.sidings.into_iter()),
        ) {
            siding.apply_delta(Delta::with_removed_inserted(
                removed_siding,
                inserted_siding,
            ));
        }
    }
}

#[cfg(feature = "undoredo")]
impl<SE: Clone, S: FlushDelta<SE>> FlushDelta<ParalleledHalfDelta<SE>> for Paralleled<S> {
    fn flush_delta(&mut self) -> ParalleledDelta<SE> {
        let (removed_main, inserted_main) = self.main.flush_delta().dissolve();
        let (removed_sidings, inserted_sidings) = self
            .sidings
            .iter_mut()
            .map(|siding| siding.flush_delta().dissolve())
            .unzip();

        Delta::with_removed_inserted(
            Paralleled {
                main: removed_main,
                sidings: removed_sidings,
            },
            Paralleled {
                main: inserted_main,
                sidings: inserted_sidings,
            },
        )
    }
}
