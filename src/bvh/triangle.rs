use std::f32;

use bevy_math::prelude::*;
use bvh::{
    aabb::{Aabb, Bounded},
    bounding_hierarchy::BHShape,
};
use nalgebra::Point;

use crate::common::triangle::Triangle;

#[derive(Debug)]
pub struct BVHTriangle(pub Triangle, usize);

impl BVHTriangle {
    pub fn new(triangle_index: usize, positions: [Vec3; 3], normals: Option<[Vec3; 3]>) -> Self {
        Self(Triangle::new(triangle_index, positions, normals), 0)
    }

    pub fn from_triangle(triangle: Triangle) -> Self {
        Self(triangle, 0)
    }
}

impl Bounded<f32, 3> for BVHTriangle {
    fn aabb(&self) -> Aabb<f32, 3> {
        let min = self
            .0
            .positions
            .into_iter()
            .fold(Vec3::splat(f32::INFINITY), |a, b| a.min(b));
        let max = self
            .0
            .positions
            .into_iter()
            .fold(Vec3::splat(f32::NEG_INFINITY), |a, b| a.max(b));

        Aabb::with_bounds(
            Point::<f32, 3>::new(min.x, min.y, min.z),
            Point::<f32, 3>::new(max.x, max.y, max.z),
        )
    }
}

impl BHShape<f32, 3> for BVHTriangle {
    fn set_bh_node_index(&mut self, index: usize) {
        self.1 = index;
    }

    fn bh_node_index(&self) -> usize {
        self.1
    }
}
