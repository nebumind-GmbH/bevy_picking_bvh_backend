use bevy_math::prelude::*;
use bvh::{
    aabb::{Aabb, Bounded},
    bounding_hierarchy::BHShape,
};
use nalgebra::Point;

#[derive(Debug)]
pub struct Triangle {
    pub triangle_index: usize,
    pub positions: [Vec3; 3],
    pub normals: Option<[Vec3; 3]>,
    node_index: usize,
}

impl Triangle {
    pub fn new(triangle_index: usize, positions: [Vec3; 3], normals: Option<[Vec3; 3]>) -> Self {
        Self {
            triangle_index,
            positions,
            normals,
            node_index: 0,
        }
    }
}

impl Bounded<f32, 3> for Triangle {
    fn aabb(&self) -> Aabb<f32, 3> {
        let x_min = self
            .positions
            .iter()
            .map(|p| p[0])
            .fold(f32::INFINITY, |a, b| a.min(b));
        let y_min = self
            .positions
            .iter()
            .map(|p| p[1])
            .fold(f32::INFINITY, |a, b| a.min(b));
        let z_min = self
            .positions
            .iter()
            .map(|p| p[2])
            .fold(f32::INFINITY, |a, b| a.min(b));

        let x_max = self
            .positions
            .iter()
            .map(|p| p[0])
            .fold(f32::NEG_INFINITY, |a, b| a.max(b));
        let y_max = self
            .positions
            .iter()
            .map(|p| p[1])
            .fold(f32::NEG_INFINITY, |a, b| a.max(b));
        let z_max = self
            .positions
            .iter()
            .map(|p| p[2])
            .fold(f32::NEG_INFINITY, |a, b| a.max(b));

        Aabb::with_bounds(
            Point::<f32, 3>::new(x_min, y_min, z_min),
            Point::<f32, 3>::new(x_max, y_max, z_max),
        )
    }
}

impl BHShape<f32, 3> for Triangle {
    fn set_bh_node_index(&mut self, index: usize) {
        self.node_index = index;
    }

    fn bh_node_index(&self) -> usize {
        self.node_index
    }
}
