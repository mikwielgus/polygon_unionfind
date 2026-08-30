// SPDX-FileCopyrightText: 2026 polygon_unionfind contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use num_traits::{Num, NumCast, Zero};

use crate::{Polygon, PolygonWithData};

pub trait Centroid<K> {
    fn centroid(&self) -> [K; 2];
}

fn polygon_centroid<K>(vertices: impl IntoIterator<Item = [K; 2]>) -> [K; 2]
where
    K: Copy + Num + NumCast + Zero,
{
    let mut sum = [K::zero(), K::zero()];
    let mut len = 0usize;

    for [x, y] in vertices {
        sum[0] = sum[0] + x;
        sum[1] = sum[1] + y;
        len += 1;
    }

    if len == 0 {
        return [K::zero(), K::zero()];
    }

    let n = NumCast::from(len).expect("vertex count fits in coordinate type");
    [sum[0] / n, sum[1] / n]
}

impl<K> Centroid<K> for Polygon<K>
where
    K: Copy + Num + NumCast + Zero,
{
    fn centroid(&self) -> [K; 2] {
        polygon_centroid(self.exterior.iter().copied())
    }
}

impl<K, W> Centroid<K> for PolygonWithData<K, W>
where
    K: Copy + Num + NumCast + Zero,
{
    fn centroid(&self) -> [K; 2] {
        polygon_centroid(self.exterior.iter().copied())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::{vec, vec::Vec};

    #[test]
    fn polygon_centroid_of_triangle() {
        let polygon = Polygon {
            exterior: vec![[0, 0], [3, 0], [0, 3]],
            interiors: Vec::new(),
        };
        assert_eq!(Centroid::centroid(&polygon), [1, 1]);
    }

    #[test]
    fn polygon_centroid_of_empty_polygon() {
        let polygon = Polygon::<i64> {
            exterior: Vec::new(),
            interiors: Vec::new(),
        };
        assert_eq!(Centroid::centroid(&polygon), [0, 0]);
    }

    #[test]
    fn polygon_with_data_centroid() {
        let polygon = PolygonWithData {
            exterior: vec![[0, 0], [4, 0], [4, 2], [0, 2]],
            interiors: Vec::new(),
            data: (),
        };
        assert_eq!(Centroid::centroid(&polygon), [2, 1]);
    }
}
