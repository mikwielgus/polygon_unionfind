// SPDX-FileCopyrightText: 2026 polygon_unionfind contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use i_overlay::mesh::int::{outline::offset::IntOutlineOffset, style::IntOutlineStyle};

use crate::{
    Polygon, PolygonWithData, Scalar,
    polygon::{from_i_overlay_ring, to_i_overlay_shape},
};

pub trait Inflate<K> {
    fn inflate(self, offset: K) -> Self;
}

impl<K: Scalar> Inflate<K> for Polygon<K> {
    fn inflate(self, offset: K) -> Self {
        let Some(shape) = to_i_overlay_shape(&self)
            .outline(&IntOutlineStyle::new(offset.cast_into()))
            .ok()
            .and_then(|buffered| buffered.into_iter().next())
            .filter(|shape| !shape.is_empty())
        else {
            return self;
        };

        Polygon {
            exterior: from_i_overlay_ring(&shape[0]),
            interiors: shape[1..]
                .iter()
                .map(|ring| from_i_overlay_ring(ring))
                .collect(),
        }
    }
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
