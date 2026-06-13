// SPDX-FileCopyrightText: 2026 polygon_unionfind contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use alloc::vec::Vec;

use maplike::{Container, Get};

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

impl<I: Container, K> Container for Inflated<I, K> {
    type Key = I::Key;
    type Value = I::Value;
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
    primary: S,
    parallels: Vec<S>,
}

impl<S> Paralleled<S> {
    #[inline]
    pub fn primary(&self) -> &S {
        &self.primary
    }

    #[inline]
    pub fn parallels(&self) -> &Vec<S> {
        &self.parallels
    }

    #[inline]
    pub fn row(&self, row: usize) -> &S {
        if row >= 1 {
            &self.parallels[row - 1]
        } else {
            &self.primary
        }
    }
}

impl<S> Paralleled<S> {
    #[inline]
    pub fn new(primary: S, parallels: Vec<S>) -> Self {
        Self { primary, parallels }
    }
}

impl<P: Clone, S: Add<P>> Add<P> for Paralleled<S> {
    type Output = (S::Output, Vec<S::Output>);

    fn add(&mut self, polygon: P) -> (S::Output, Vec<S::Output>) {
        let parallel_outputs = self
            .parallels
            .iter_mut()
            .map(|parallel| parallel.add(polygon.clone()))
            .collect();
        let primary_output = self.primary.add(polygon);
        (primary_output, parallel_outputs)
    }
}

impl<P: Clone, S: Sub<P>> Sub<P> for Paralleled<S> {
    type Output = (S::Output, Vec<S::Output>);

    fn sub(&mut self, polygon: P) -> (S::Output, Vec<S::Output>) {
        let parallel_outputs = self
            .parallels
            .iter_mut()
            .map(|parallel| parallel.sub(polygon.clone()))
            .collect();
        let primary_output = self.primary.sub(polygon);
        (primary_output, parallel_outputs)
    }
}

impl<P: Clone, S: Clip<P>> Clip<P> for Paralleled<S> {
    type Output = S::Output;

    fn clip(&mut self, polygon: P) -> S::Output {
        for parallel in &mut self.parallels {
            parallel.clip(polygon.clone());
        }

        self.primary.clip(polygon)
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

        let primary_delta = Delta::with_removed_inserted(removed.primary, inserted.primary);
        self.primary.apply_delta(primary_delta);

        for (parallel, (removed_parallel, inserted_parallel)) in self.parallels.iter_mut().zip(
            removed
                .parallels
                .into_iter()
                .zip(inserted.parallels.into_iter()),
        ) {
            parallel.apply_delta(Delta::with_removed_inserted(
                removed_parallel,
                inserted_parallel,
            ));
        }
    }
}

#[cfg(feature = "undoredo")]
impl<SE: Clone, S: FlushDelta<SE>> FlushDelta<ParalleledHalfDelta<SE>> for Paralleled<S> {
    fn flush_delta(&mut self) -> ParalleledDelta<SE> {
        let (removed_primary, inserted_primary) = self.primary.flush_delta().dissolve();
        let (removed_parallels, inserted_parallels) = self
            .parallels
            .iter_mut()
            .map(|parallel| parallel.flush_delta().dissolve())
            .unzip();

        Delta::with_removed_inserted(
            Paralleled {
                primary: removed_primary,
                parallels: removed_parallels,
            },
            Paralleled {
                primary: inserted_primary,
                parallels: inserted_parallels,
            },
        )
    }
}
