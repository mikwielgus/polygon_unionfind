// SPDX-FileCopyrightText: 2026 polygon_unionfind contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use alloc::collections::BTreeSet;
use alloc::vec::Vec;
use core::marker::PhantomData;
use maplike::ops::Get;

use rstar::RTreeNum;
use rstar::primitives::{GeomWithData, Rectangle};
use rstar::{RTree, RTreeObject};
#[cfg(feature = "undoredo")]
use undoredo::{ApplyDelta, Delta, FlushDelta};

use crate::Polygon;
use crate::bool_ops::{Difference, Intersection, Union};
use crate::{
    Add, Inflate, Inflated, Negated, Paralleled, PolygonId, PolygonSet, PolygonUnionFind, Rings,
    Sub, polygon::rectangle_from_polygon,
};
#[cfg(feature = "undoredo")]
use crate::{RecordingNegated, RecordingPolygonSet};

pub type Lamina<K, P = Polygon<K>> =
    Paralleled<Paralleled<Negated<PolygonSet<K, P>, Inflated<PolygonUnionFind<K, P>, K>>>>;
pub type Interlamina<K, P = Polygon<K>> = Paralleled<PolygonSet<K, P>>;

#[cfg(feature = "undoredo")]
pub type RecordingLamina<K, P = Polygon<K>> = Paralleled<Paralleled<RecordingNegated<K, P>>>;

#[cfg(feature = "undoredo")]
pub type RecordingInterlamina<K, P = Polygon<K>> = Paralleled<RecordingPolygonSet<K, P>>;

#[derive(Clone)]
pub struct Laminate<K: RTreeNum, P = Polygon<K>, L = Lamina<K, P>, T = Interlamina<K, P>> {
    laminas: Vec<L>,
    interlaminas: Vec<T>,
    scalar_and_polygon_marker: PhantomData<(K, P)>,
}

impl<K, P> Laminate<K, P>
where
    K: RTreeNum + Default,
    P: Clone + Rings<K> + Union<P> + Difference<P>,
{
    pub fn new<PI>(
        boundary: P,
        num_laminas: usize,
        peripheral_inflations: PI,
        rail_offsets: impl IntoIterator<Item = K>,
    ) -> Self
    where
        PI: IntoIterator<Item = K>,
        PI::IntoIter: core::iter::ExactSizeIterator,
    {
        let peripheral_inflations = peripheral_inflations.into_iter();
        let count_peripheral_inflations = peripheral_inflations.len();
        let rail_offsets: Vec<K> = rail_offsets.into_iter().collect();

        let laminas: Vec<Lamina<K, P>> =
            vec![
                lamina_from_boundary(&boundary, peripheral_inflations, &rail_offsets);
                num_laminas
            ];
        let interlamina = {
            let mut primary_set = PolygonSet::new();
            let _ = primary_set.add(boundary.clone());
            Paralleled::new(
                primary_set,
                core::iter::repeat_n(PolygonSet::new(), count_peripheral_inflations).collect(),
            )
        };
        let interlaminas: Vec<Interlamina<K, P>> = vec![interlamina; num_laminas.saturating_sub(1)];

        Self::with_laminas_interlaminas(laminas, interlaminas)
    }
}

impl<K: RTreeNum, P, L, T> Laminate<K, P, L, T> {
    #[inline]
    pub fn with_laminas_interlaminas(laminas: Vec<L>, interlaminas: Vec<T>) -> Self {
        assert!(
            interlaminas.len() == laminas.len().saturating_sub(1),
            "interlaminas must have exactly laminas.len() - 1 entries"
        );
        Self {
            laminas,
            interlaminas,
            scalar_and_polygon_marker: PhantomData,
        }
    }

    #[inline]
    pub fn laminas(&self) -> &[L] {
        &self.laminas
    }

    #[inline]
    pub fn interlaminas(&self) -> &[T] {
        &self.interlaminas
    }

    #[inline]
    pub fn lamina(&self, index: usize) -> Option<&L> {
        self.laminas.get(&index)
    }

    #[inline]
    pub fn interlamina(&self, index: usize) -> Option<&T> {
        self.interlaminas.get(&index)
    }
}

fn lamina_from_boundary<K, P>(
    boundary: &P,
    peripheral_inflations: impl Iterator<Item = K>,
    rail_offsets: &[K],
) -> Lamina<K, P>
where
    K: RTreeNum + Default,
    P: Clone + Rings<K> + Union<P> + Difference<P>,
{
    let make_negated =
        |offset: K| -> Negated<PolygonSet<K, P>, Inflated<PolygonUnionFind<K, P>, K>> {
            let mut minuend = PolygonSet::new();
            let _ = minuend.add(boundary.clone());
            Negated::new(minuend, Inflated::new(offset))
        };

    let make_paralleled =
        |offset: K| -> Paralleled<Negated<PolygonSet<K, P>, Inflated<PolygonUnionFind<K, P>, K>>> {
            let track = make_negated(offset);
            let rails = rail_offsets
                .iter()
                .map(|&rail_offset| make_negated(offset + rail_offset))
                .collect();
            Paralleled::new(track, rails)
        };

    let core = make_paralleled(K::default());
    let peripherals = peripheral_inflations.map(make_paralleled).collect();
    Paralleled::new(core, peripherals)
}

impl<K, P, M, SI, PS> Laminate<K, P, Paralleled<Paralleled<Negated<M, SI>>>, Paralleled<PS>>
where
    K: RTreeNum + Ord,
    P: Clone + Rings<K> + Inflate<K> + Union<P> + Difference<P> + Intersection<P>,
    M: Get<PolygonId, Value = P>,
    PS: AsRef<RTree<GeomWithData<Rectangle<[K; 2]>, PolygonId>>> + Get<PolygonId, Value = P>,
    Paralleled<Paralleled<Negated<M, SI>>>: Add<
            P,
            Output = (
                ((Vec<PolygonId>, Vec<P>), Vec<(Vec<PolygonId>, Vec<P>)>),
                Vec<((Vec<PolygonId>, Vec<P>), Vec<(Vec<PolygonId>, Vec<P>)>)>,
            ),
        >,
    Paralleled<PS>: Sub<P> + Add<P>,
{
    pub fn add_into_lamina(&mut self, lamina_index: usize, polygon: P) {
        if lamina_index >= self.laminas.len() {
            return;
        }

        let (clipping_polygons, removed_polygons) = {
            let lamina = self.laminas.get_mut(lamina_index).unwrap();
            let (inner_outputs, _outer_parallel_outputs) = lamina.add(polygon);
            let (inner_primary_output, inner_parallel_outputs) = inner_outputs;
            let (parallel_ids, removed_polygons) = inner_parallel_outputs
                .first()
                .cloned()
                .unwrap_or(inner_primary_output);

            // We hard-code the last parallel to be the polygon-set that clips
            // the interlamina.
            let Some(clipping_source) = lamina.primary().parallels().last().map(|s| s.minuend())
            else {
                return;
            };

            (
                Self::polygons_under_ids(&parallel_ids, clipping_source),
                removed_polygons,
            )
        };

        // Interlamina i corresponds to the window (i, i+1). Hence, a write to
        // lamina k affects interlaminas k-1 and k.
        if lamina_index > 0 {
            self.exclude_removed(lamina_index - 1, &removed_polygons);
            self.clip_interpolygon(lamina_index - 1, clipping_polygons.clone());
        }
        if lamina_index < self.interlaminas.len() {
            self.exclude_removed(lamina_index, &removed_polygons);
            self.clip_interpolygon(lamina_index, clipping_polygons);
        }
    }

    fn polygons_under_ids(ids: &[PolygonId], source: &M) -> Vec<P> {
        ids.iter()
            .filter_map(|id| Get::get(source, id).cloned())
            .collect()
    }

    fn exclude_removed(&mut self, interlamina_index: usize, removed_polygons: &[P]) {
        if removed_polygons.is_empty() {
            return;
        }

        let interlamina = &mut self.interlaminas[interlamina_index];

        for removed in removed_polygons {
            let _ = interlamina.sub(removed.clone());
        }
    }

    fn clip_interpolygon(&mut self, interpolygon_index: usize, clipping_polygons: Vec<P>) {
        let interlamina = &mut self.interlaminas[interpolygon_index];

        if clipping_polygons.is_empty() {
            // If there is nothing to clip, the interlamina is unaffected.
            return;
        }

        let mut located_interids = BTreeSet::new();

        for clipping_polygon in &clipping_polygons {
            let clip_bbox = rectangle_from_polygon(clipping_polygon);

            for hit in interlamina
                .primary()
                .as_ref()
                .locate_in_envelope_intersecting(&clip_bbox.envelope())
            {
                located_interids.insert(hit.data);
            }
        }

        let located_interpolygons: Vec<P> = located_interids
            .into_iter()
            .filter_map(|id| Get::get(interlamina.primary(), &id).cloned())
            .collect();

        for interpolygon in located_interpolygons {
            let mut clipped_union = PolygonSet::<K, P>::new();
            let mut has_intersection = false;

            for clipping_polygon in &clipping_polygons {
                for piece in P::intersect(interpolygon.clone(), clipping_polygon.clone()) {
                    has_intersection = true;
                    let _ = clipped_union.add(piece);
                }
            }

            if !has_intersection {
                // If interpolygon is not intersected, skip it. Otherwise, it
                // would get removed, which would be incorrect.
                continue;
            }

            let _ = interlamina.sub(interpolygon);

            for (_idx, piece) in clipped_union.polygons().iter() {
                let _ = interlamina.add(piece.clone());
            }
        }
    }
}

#[cfg(feature = "undoredo")]
/// `Laminate`-equivalent using recording container combinators.
pub type RecordingLaminate<K, P = Polygon<K>> =
    Laminate<K, P, RecordingLamina<K, P>, RecordingInterlamina<K, P>>;

#[cfg(feature = "undoredo")]
/// Half-delta of `Laminate`.
pub type LaminateHalfDelta<K, P, LE, TE> = Laminate<K, P, LE, TE>;

#[cfg(feature = "undoredo")]
/// Delta of `Laminate` for delta-based Undo/Redo.
pub type LaminateDelta<K, P, LE, TE> = Delta<LaminateHalfDelta<K, P, LE, TE>>;

#[cfg(feature = "undoredo")]
impl<K: RTreeNum, P, LE: Clone, L: Clone + ApplyDelta<LE>, TE: Clone, T: Clone + ApplyDelta<TE>>
    ApplyDelta<LaminateHalfDelta<K, P, LE, TE>> for Laminate<K, P, L, T>
{
    fn apply_delta(&mut self, delta: LaminateDelta<K, P, LE, TE>) {
        let (removed, inserted) = delta.dissolve();

        for (lamina, (removed_lamina, inserted_lamina)) in self.laminas.iter_mut().zip(
            removed
                .laminas
                .into_iter()
                .zip(inserted.laminas.into_iter()),
        ) {
            lamina.apply_delta(Delta::with_removed_inserted(
                removed_lamina,
                inserted_lamina,
            ));
        }

        for (interlamina, (removed_interlamina, inserted_interlamina)) in
            self.interlaminas.iter_mut().zip(
                removed
                    .interlaminas
                    .into_iter()
                    .zip(inserted.interlaminas.into_iter()),
            )
        {
            interlamina.apply_delta(Delta::with_removed_inserted(
                removed_interlamina,
                inserted_interlamina,
            ));
        }
    }
}

#[cfg(feature = "undoredo")]
impl<K: RTreeNum, P, LE: Clone, L: FlushDelta<LE>, TE: Clone, T: FlushDelta<TE>>
    FlushDelta<LaminateHalfDelta<K, P, LE, TE>> for Laminate<K, P, L, T>
{
    fn flush_delta(&mut self) -> LaminateDelta<K, P, LE, TE> {
        let mut removed_laminas = Vec::with_capacity(self.laminas.len());
        let mut inserted_laminas = Vec::with_capacity(self.laminas.len());
        let mut removed_interlaminas = Vec::with_capacity(self.interlaminas.len());
        let mut inserted_interlaminas = Vec::with_capacity(self.interlaminas.len());

        for lamina in &mut self.laminas {
            let (removed_lamina, inserted_lamina) = lamina.flush_delta().dissolve();
            removed_laminas.push(removed_lamina);
            inserted_laminas.push(inserted_lamina);
        }

        for interlamina in &mut self.interlaminas {
            let (removed_interlamina, inserted_interlamina) = interlamina.flush_delta().dissolve();
            removed_interlaminas.push(removed_interlamina);
            inserted_interlaminas.push(inserted_interlamina);
        }

        Delta::with_removed_inserted(
            Laminate {
                laminas: removed_laminas,
                interlaminas: removed_interlaminas,
                scalar_and_polygon_marker: PhantomData,
            },
            Laminate {
                laminas: inserted_laminas,
                interlaminas: inserted_interlaminas,
                scalar_and_polygon_marker: PhantomData,
            },
        )
    }
}
