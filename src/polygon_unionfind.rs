// SPDX-FileCopyrightText: 2026 polygon_unionfind contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use alloc::vec::Vec;
use core::marker::PhantomData;

#[cfg(feature = "undoredo")]
use alloc::collections::BTreeMap;
use maplike::{
    containers::Container,
    iter::IntoIter,
    ops::{Clear, Get, Insert, Push, Remove, Set},
};
use rstar::{
    RTree, RTreeNum, RTreeObject,
    primitives::{GeomWithData, Rectangle},
};
use rstared::AsRefRTree;
#[cfg(feature = "undoredo")]
use undoredo::{ApplyDelta, Delta, FlushDelta, Recorder};

use crate::bool_ops::Union;
use crate::polygon::rectangle_from_polygon;
use crate::unionfind::UnionFind;
use crate::{Add, Rings};
use crate::{Polygon, PolygonId};

#[derive(Clone, Debug)]
pub struct PolygonUnionFind<
    K,
    P = Polygon<K>,
    PC = Vec<P>,
    PR = AsRefRTree<GeomWithData<Rectangle<[K; 2]>, PolygonId>>,
    UFPC = Vec<usize>,
    UFRC = Vec<usize>,
> {
    polygons: PC,
    rtree: PR,
    unionfind: UnionFind<UFPC, UFRC>,
    scalar_marker: PhantomData<K>,
    polygon_marker: PhantomData<P>,
}

impl<K, P, PC, PR, UFPC, UFRC> PolygonUnionFind<K, P, PC, PR, UFPC, UFRC> {
    /// Returns a reference to the underlying raw polygon collection.
    #[inline]
    pub fn raw_polygons(&self) -> &PC {
        &self.polygons
    }

    /// Returns a reference to the underlying R-tree.
    #[inline]
    pub fn rtree(&self) -> &PR {
        &self.rtree
    }

    /// Returns a reference to the underlying union-find.
    #[inline]
    pub fn unionfind(&self) -> &UnionFind<UFPC, UFRC> {
        &self.unionfind
    }

    /// Dissolve the polygon-union-find, returning its internal polygons,
    /// R-tree, and union-find, ceding ownership over them.
    #[inline]
    pub fn dissolve(self) -> (PC, PR, UnionFind<UFPC, UFRC>) {
        (self.polygons, self.rtree, self.unionfind)
    }
}

impl<K, P, PC, PR, UFPC, UFRC> Container for PolygonUnionFind<K, P, PC, PR, UFPC, UFRC> {
    type Key = PolygonId;
    type Value = P;
}

impl<
    K,
    P,
    PC: Get<usize, Value = P>,
    PR,
    UFPC: Get<usize, Value = usize> + Push<usize> + Set<usize>,
    UFRC: Get<usize, Value = usize> + Push<usize> + Set<usize>,
> Get<PolygonId> for PolygonUnionFind<K, P, PC, PR, UFPC, UFRC>
{
    #[inline]
    fn get(&self, key: &PolygonId) -> Option<&Self::Value> {
        let representative = self.unionfind.find(key.index());
        self.polygons.get(&representative)
    }
}

impl<K: RTreeNum, P, PC, PR: AsRef<RTree<GeomWithData<Rectangle<[K; 2]>, PolygonId>>>, UFPC, UFRC>
    AsRef<RTree<GeomWithData<Rectangle<[K; 2]>, PolygonId>>>
    for PolygonUnionFind<K, P, PC, PR, UFPC, UFRC>
{
    #[inline]
    fn as_ref(&self) -> &RTree<GeomWithData<Rectangle<[K; 2]>, PolygonId>> {
        self.rtree().as_ref()
    }
}

impl<K, P, PC: Default, PR: Default, UFPC: Default, UFRC: Default>
    PolygonUnionFind<K, P, PC, PR, UFPC, UFRC>
{
    /// Create an empty `PolygonUnionFind`.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }
}

impl<K, P, PC: Default, PR: Default, UFPC: Default, UFRC: Default> Default
    for PolygonUnionFind<K, P, PC, PR, UFPC, UFRC>
{
    #[inline]
    fn default() -> Self {
        Self {
            polygons: Default::default(),
            rtree: Default::default(),
            unionfind: UnionFind::new(),
            scalar_marker: PhantomData,
            polygon_marker: PhantomData,
        }
    }
}

impl<
    K,
    P,
    PC,
    PR,
    UFPC: Get<usize, Value = usize> + Push<usize> + Set<usize>,
    UFRC: Get<usize, Value = usize> + Push<usize> + Set<usize>,
> PolygonUnionFind<K, P, PC, PR, UFPC, UFRC>
where
    PC: Get<usize, Value = P>,
    for<'a> &'a PC: IntoIter<&'a usize, Value = &'a P>,
{
    /// Return unique representative indices for currently merged polygons.
    #[inline]
    pub fn polygon_indices(&self) -> impl Iterator<Item = usize> {
        let mut deduplicating_set = (&self.polygons)
            .into_iter()
            .map(|(i, _)| self.unionfind.find(*i))
            .collect::<Vec<_>>();
        deduplicating_set.sort_unstable();
        deduplicating_set.dedup();
        IntoIterator::into_iter(deduplicating_set)
    }

    /// Iterate representative polygons after all merges.
    ///
    /// If several inserted polygons have been merged into one component,
    /// only the representative polygon for that component is returned.
    #[inline]
    pub fn polygons(&self) -> impl Iterator<Item = &P> {
        self.polygon_indices()
            .map(|i| self.polygons.get(&i).unwrap())
    }
}

impl<
    K: RTreeNum,
    P: Clone + Rings<K> + Union<P>,
    PC: Get<usize, Value = P> + Push<usize> + Set<usize>,
    PR: AsRef<RTree<GeomWithData<Rectangle<[K; 2]>, PolygonId>>>
        + Insert<GeomWithData<Rectangle<[K; 2]>, PolygonId>, Value = ()>
        + Remove<GeomWithData<Rectangle<[K; 2]>, PolygonId>>,
    UFPC: Get<usize, Value = usize> + Push<usize> + Set<usize>,
    UFRC: Get<usize, Value = usize> + Push<usize> + Set<usize>,
> Add<P> for PolygonUnionFind<K, P, PC, PR, UFPC, UFRC>
{
    type Output = PolygonId;

    fn add(&mut self, polygon: P) -> PolygonId {
        self.add(polygon)
    }
}

impl<
    K: RTreeNum,
    P: Clone + Rings<K> + Union<P>,
    PC: Get<usize, Value = P> + Push<usize> + Set<usize>,
    PR: AsRef<RTree<GeomWithData<Rectangle<[K; 2]>, PolygonId>>>
        + Insert<GeomWithData<Rectangle<[K; 2]>, PolygonId>, Value = ()>
        + Remove<GeomWithData<Rectangle<[K; 2]>, PolygonId>>,
    UFPC: Get<usize, Value = usize> + Push<usize> + Set<usize>,
    UFRC: Get<usize, Value = usize> + Push<usize> + Set<usize>,
> PolygonUnionFind<K, P, PC, PR, UFPC, UFRC>
{
    /// Insert a polygon and merge it with any neighboring polygon.
    ///
    /// Returns the polygon's new id even if it was immediately absorbed by
    /// another polygon.
    pub fn add(&mut self, mut polygon: P) -> PolygonId {
        let new_polygon_index = self.unionfind.new_set();

        let bbox = rectangle_from_polygon(&polygon);
        self.rtree.insert(
            GeomWithData::new(bbox, PolygonId::new(new_polygon_index)),
            (),
        );

        let id = self.polygons.push(polygon.clone());
        debug_assert_eq!(new_polygon_index, id);

        let located_ids: Vec<PolygonId> = self
            .rtree
            .as_ref()
            .locate_in_envelope_intersecting(bbox.envelope())
            .map(|neighbor| neighbor.data)
            .collect();

        for located_id in located_ids {
            let located_representative = self.unionfind.find_compress(located_id.index());
            let root_of_new = self.unionfind.find_compress(new_polygon_index);

            if located_representative == root_of_new {
                continue;
            }

            let neighbor = self.polygons.get(&located_representative).unwrap().clone();
            let Some(merged) = P::union(polygon.clone(), neighbor) else {
                continue;
            };

            let root_of_neighbor = located_representative;
            self.unionfind.union(root_of_neighbor, root_of_new);

            let representative = self.unionfind.find_compress(new_polygon_index);
            polygon = merged;
            self.polygons.set(representative, polygon.clone());

            let absorbed = if representative == root_of_neighbor {
                root_of_new
            } else {
                root_of_neighbor
            };

            // Remove the absorbed polygon from R-tree to shorten query
            // times.
            self.remove_polygon_from_rtree(PolygonId::new(absorbed));

            // The bbox changed, so we need to reinsert the polygon in R-tree.
            self.reinsert_polygon_in_rtree(PolygonId::new(representative), &polygon);
        }

        PolygonId::new(id)
    }

    fn remove_polygon_from_rtree(&mut self, polygon_id: PolygonId) {
        let geom_with_data = self
            .rtree
            .as_ref()
            .iter()
            .find(|g| g.data == polygon_id)
            .cloned();
        if let Some(geom_with_data) = geom_with_data {
            self.rtree.remove(&geom_with_data);
        }
    }

    fn reinsert_polygon_in_rtree(&mut self, polygon_id: PolygonId, polygon: &P) {
        self.remove_polygon_from_rtree(polygon_id);
        let bbox = rectangle_from_polygon(polygon);
        self.rtree.insert(GeomWithData::new(bbox, polygon_id), ());
    }

    /// Find the representative polygon for `index` without path compression.
    ///
    /// If you want to path compression to be performed, use [`find_compress()`]
    /// instead.
    #[inline]
    pub fn find(&self, index: usize) -> &P {
        self.polygons.get(&self.unionfind.find(index)).unwrap()
    }

    /// Find the representative polygon for `index`, applying path compression
    /// along the way.
    ///
    /// [https://cp-algorithms.com/data_structures/disjoint_set_union.html#path-compression-optimization](Path compression)
    /// is an optimization that speeds up finding an element by flattening the
    /// tree formed by all the connected nodes.
    pub fn find_compress(&mut self, index: usize) -> &P {
        self.polygons
            .get(&self.unionfind.find_compress(index))
            .unwrap()
    }
}

impl<K: RTreeNum, P, PC: Clear, PR: Clear, UFPC: Clear, UFRC: Clear>
    PolygonUnionFind<K, P, PC, PR, UFPC, UFRC>
{
    /// Remove all polygons and their relations.
    ///
    /// This resets the strucutre's state to as if it was returned freshly from
    /// [`new()`].
    pub fn clear(&mut self) {
        self.polygons.clear();
        self.rtree.clear();
        self.unionfind.clear();
    }
}

#[cfg(feature = "undoredo")]
/// `PolygonUnionFind` that records changes for delta-based Undo/Redo.
pub type RecordingPolygonUnionFind<K, P = Polygon<K>> = PolygonUnionFind<
    K,
    P,
    Recorder<Vec<P>, BTreeMap<usize, P>>,
    Recorder<RTree<GeomWithData<Rectangle<[K; 2]>, PolygonId>>>,
    Recorder<Vec<usize>>,
    Recorder<Vec<usize>>,
>;

#[cfg(feature = "undoredo")]
/// Half-delta of `PolygonUnionFind`.
pub type PolygonUnionFindHalfDelta<K, P = Polygon<K>> = PolygonUnionFind<
    K,
    P,
    BTreeMap<usize, P>,
    BTreeMap<GeomWithData<Rectangle<[K; 2]>, PolygonId>, ()>,
    BTreeMap<usize, usize>,
    BTreeMap<usize, usize>,
>;

#[cfg(feature = "undoredo")]
/// Delta of `PolygonUnionFind` for delta-based Undo/Redo.
pub type PolygonUnionFindDelta<K, P = Polygon<K>> = Delta<PolygonUnionFindHalfDelta<K, P>>;

#[cfg(feature = "undoredo")]
impl<
    K: RTreeNum,
    P: Clone,
    PE: Clone + Container<Value = P>,
    PC: Container<Value = P> + Clone + ApplyDelta<PE>,
    PRE: Clone + Container,
    PR: Container + Clone + ApplyDelta<PRE>,
    UFPCE: Clone + Container,
    UFPC: Clone + ApplyDelta<UFPCE>,
    UFRCE: Clone + Container,
    UFRC: Clone + ApplyDelta<UFRCE>,
> ApplyDelta<PolygonUnionFind<K, P, PE, PRE, UFPCE, UFRCE>>
    for PolygonUnionFind<K, P, PC, PR, UFPC, UFRC>
{
    fn apply_delta(&mut self, delta: Delta<PolygonUnionFind<K, P, PE, PRE, UFPCE, UFRCE>>) {
        let (removed, inserted) = delta.dissolve();

        let polygons_delta = Delta::with_removed_inserted(removed.polygons, inserted.polygons);
        self.polygons.apply_delta(polygons_delta);

        let rtree_delta = Delta::with_removed_inserted(removed.rtree, inserted.rtree);
        self.rtree.apply_delta(rtree_delta);

        let unionfind_delta = Delta::with_removed_inserted(removed.unionfind, inserted.unionfind);
        self.unionfind.apply_delta(unionfind_delta);
    }
}

#[cfg(feature = "undoredo")]
impl<
    K: RTreeNum,
    P: Clone,
    PE: Clone + Container<Value = P>,
    PC: Container<Value = P> + FlushDelta<PE>,
    PRE: Clone + Container,
    PR: Container + FlushDelta<PRE>,
    UFPCE: Clone + Container,
    UFPC: FlushDelta<UFPCE>,
    UFRCE: Clone + Container,
    UFRC: FlushDelta<UFRCE>,
> FlushDelta<PolygonUnionFind<K, P, PE, PRE, UFPCE, UFRCE>>
    for PolygonUnionFind<K, P, PC, PR, UFPC, UFRC>
{
    fn flush_delta(&mut self) -> Delta<PolygonUnionFind<K, P, PE, PRE, UFPCE, UFRCE>> {
        let (removed_polygons, inserted_polygons) = self.polygons.flush_delta().dissolve();
        let (removed_rtree, inserted_rtree) = self.rtree.flush_delta().dissolve();
        let (removed_unionfind, inserted_unionfind) = self.unionfind.flush_delta().dissolve();

        Delta::with_removed_inserted(
            PolygonUnionFind {
                polygons: removed_polygons,
                rtree: removed_rtree,
                unionfind: removed_unionfind,
                scalar_marker: PhantomData,
                polygon_marker: PhantomData,
            },
            PolygonUnionFind {
                polygons: inserted_polygons,
                rtree: inserted_rtree,
                unionfind: inserted_unionfind,
                scalar_marker: PhantomData,
                polygon_marker: PhantomData,
            },
        )
    }
}
