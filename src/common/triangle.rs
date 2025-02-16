use bevy_math::prelude::*;

#[derive(Debug)]
pub struct Triangle {
    pub triangle_index: usize,
    pub positions: [Vec3; 3],
    pub normals: Option<[Vec3; 3]>,
}

impl Triangle {
    pub fn new(triangle_index: usize, positions: [Vec3; 3], normals: Option<[Vec3; 3]>) -> Self {
        Self {
            triangle_index,
            positions,
            normals,
        }
    }
}
