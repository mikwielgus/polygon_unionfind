// SPDX-FileCopyrightText: 2026 polygon_unionfind contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use alloc::vec::Vec;
#[cfg(feature = "undoredo")]
use maplike::containers::Container;
use maplike::ops::{Clear, Get, Push, Set};
#[cfg(feature = "undoredo")]
use undoredo::{ApplyDelta, Delta, FlushDelta};

/// Disjoint-set union data structure, widely also known as *union-find*.
#[derive(Clone, Debug, Default)]
pub struct UnionFind<PC = Vec<usize>, RC = PC> {
    /// `parents[i]` is the parent of node `i`.
    parents: PC,
    /// `ranks[i]` is the upper bound of tree height for node `i`.
    ranks: RC,
}

impl<
    PC: Get<usize, Value = usize> + FromIterator<usize> + Push<usize> + Set<usize>,
    RC: Get<usize, Value = usize> + FromIterator<usize> + Push<usize> + Set<usize>,
> UnionFind<PC, RC>
{
    /// Create a new `UnionFind` with `len` singleton sets (`0..len`).
    #[inline]
    pub fn with_len(len: usize) -> Self {
        Self::from_parents_ranks(
            PC::from_iter(0..len),
            RC::from_iter(core::iter::repeat_n(0, len)),
        )
    }
}

impl<PC: Default, RC: Default> UnionFind<PC, RC> {
    /// Create an empty `UnionFind`.
    #[inline]
    pub fn new() -> Self {
        Self::from_parents_ranks(Default::default(), Default::default())
    }
}

impl<PC, RC> UnionFind<PC, RC> {
    /// Create a new `UnionFind` from given parent and rank collections.
    #[inline]
    pub fn from_parents_ranks(parents: PC, ranks: RC) -> Self {
        Self { parents, ranks }
    }

    /// Dissolve this structure and return the parent and rank collections.
    #[inline]
    pub fn dissolve(self) -> (PC, RC) {
        (self.parents, self.ranks)
    }
}

impl<
    PC: Get<usize, Value = usize> + Push<usize> + Set<usize>,
    RC: Get<usize, Value = usize> + Push<usize> + Set<usize>,
> UnionFind<PC, RC>
{
    /// Add one singleton set and return its node.
    pub fn new_set(&mut self) -> usize {
        let new_set_index = self.ranks.push(0);
        self.parents.push(new_set_index);

        new_set_index
    }

    /// Find the representative of the given node.
    ///
    /// If you want path compression to be performed, use [`find_compress()`]
    /// instead.
    pub fn find(&self, node: usize) -> usize {
        let parent = *self.parents.get(&node).unwrap();
        if parent != node {
            return self.find(parent);
        }
        parent
    }

    /// Find the representative of element under the given node, performing path
    /// compression along the way.
    ///
    /// [https://cp-algorithms.com/data_structures/disjoint_set_union.html#path-compression-optimization](Path compression)
    /// is an optimization that speeds up finding an element by flattening the
    /// tree formed by all the connected nodes.
    pub fn find_compress(&mut self, node: usize) -> usize {
        let mut parent = *self.parents.get(&node).unwrap();
        if parent != node {
            // Perform the path compression.
            parent = self.find_compress(parent);
            self.parents.set(node, parent);
        }
        parent
    }

    /// Unionize the sets containing nodes `x` and `y`, minimizing the rank.
    ///
    /// Returns true if merged, false if already in the same set.
    pub fn union(&mut self, x: usize, y: usize) -> bool {
        let mut x_representative = self.find_compress(x);
        let mut y_representative = self.find_compress(y);

        if x_representative == y_representative {
            return false; // Already connected.
        }

        // Perform union by rank.

        if self.ranks.get(&x_representative).unwrap() < self.ranks.get(&y_representative).unwrap() {
            core::mem::swap(&mut x_representative, &mut y_representative);
        }

        self.parents.set(y_representative, x_representative);

        if self.ranks.get(&x_representative).unwrap() == self.ranks.get(&y_representative).unwrap()
        {
            let rank = *self.ranks.get(&x_representative).unwrap();
            self.ranks.set(x_representative, rank + 1);
        }

        true
    }

    /// Check if `x` and `y` are in the same set.
    pub fn connected(&mut self, x: usize, y: usize) -> bool {
        self.find_compress(x) == self.find_compress(y)
    }
}

impl<PC: Clear, RC: Clear> UnionFind<PC, RC> {
    /// Remove all sets.
    pub fn clear(&mut self) {
        self.parents.clear();
        self.ranks.clear();
    }
}

#[cfg(feature = "undoredo")]
impl<
    PCE: Clone + Container,
    PC: Clone + ApplyDelta<PCE>,
    RCE: Clone + Container,
    RC: Clone + ApplyDelta<RCE>,
> ApplyDelta<UnionFind<PCE, RCE>> for UnionFind<PC, RC>
{
    fn apply_delta(&mut self, delta: Delta<UnionFind<PCE, RCE>>) {
        let (removed, inserted) = delta.dissolve();

        let parents_delta = Delta::with_removed_inserted(removed.parents, inserted.parents);
        self.parents.apply_delta(parents_delta);

        let ranks_delta = Delta::with_removed_inserted(removed.ranks, inserted.ranks);
        self.ranks.apply_delta(ranks_delta);
    }
}

#[cfg(feature = "undoredo")]
impl<PCE: Container, PC: FlushDelta<PCE>, RCE: Container, RC: FlushDelta<RCE>>
    FlushDelta<UnionFind<PCE, RCE>> for UnionFind<PC, RC>
{
    fn flush_delta(&mut self) -> Delta<UnionFind<PCE, RCE>> {
        let (removed_parents, inserted_parents) = self.parents.flush_delta().dissolve();
        let (removed_ranks, inserted_ranks) = self.ranks.flush_delta().dissolve();

        Delta::with_removed_inserted(
            UnionFind {
                parents: removed_parents,
                ranks: removed_ranks,
            },
            UnionFind {
                parents: inserted_parents,
                ranks: inserted_ranks,
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let unionfind: UnionFind<Vec<usize>> = UnionFind::with_len(5);
        for i in 0..5 {
            assert_eq!(unionfind.parents[i], i);
        }
    }

    #[test]
    fn test_new_set() {
        let mut unionfind: UnionFind<Vec<usize>> = UnionFind::new();

        // First set.
        let s0 = unionfind.new_set();
        assert_eq!(s0, 0);
        assert_eq!(*unionfind.parents.get(&s0).unwrap(), s0);
        assert_eq!(*unionfind.ranks.get(&s0).unwrap(), 0);

        // Second set.
        let s1 = unionfind.new_set();
        assert_eq!(s1, 1);
        assert_eq!(*unionfind.parents.get(&s1).unwrap(), s1);
        assert_eq!(*unionfind.ranks.get(&s1).unwrap(), 0);

        // Make sure the two sets are not connected.
        assert!(!unionfind.connected(s0, s1));
    }

    #[test]
    fn test_union_idempotence() {
        let mut unionfind: UnionFind<Vec<usize>> = UnionFind::with_len(3);

        unionfind.union(0, 1);
        let representative_before = unionfind.find_compress(0);

        unionfind.union(0, 1); // Perform union on the same pair again.
        let representative_after = unionfind.find_compress(1);

        assert_eq!(representative_before, representative_after);
    }

    #[test]
    fn test_union_and_find() {
        let mut unionfind: UnionFind<Vec<usize>> = UnionFind::with_len(5);
        unionfind.union(0, 1);
        unionfind.union(1, 2);

        let representative0 = unionfind.find_compress(0);
        let representative1 = unionfind.find_compress(1);
        let representative2 = unionfind.find_compress(2);

        // The first three elements should have the same representative.
        assert_eq!(representative0, representative1);
        assert_eq!(representative1, representative2);

        // 3 and 4 are however still separate.
        assert_ne!(unionfind.find_compress(3), representative0);
        assert_ne!(unionfind.find_compress(4), representative0);
    }

    #[test]
    fn test_connected() {
        let mut unionfind: UnionFind<Vec<usize>> = UnionFind::with_len(4);
        unionfind.union(0, 1);
        unionfind.union(2, 3);

        assert!(unionfind.connected(0, 1));
        assert!(unionfind.connected(2, 3));
        assert!(!unionfind.connected(0, 2));

        unionfind.union(1, 2);
        assert!(unionfind.connected(0, 3));
    }
}
