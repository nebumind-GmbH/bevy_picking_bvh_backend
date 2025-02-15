use bevy_ecs::prelude::*;

use bevy_log::prelude::*;
use bevy_math::prelude::*;

use bevy_render::{
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
};

use bvh::bvh::Bvh;
use shape::Triangle;

pub mod ray_cast;
mod shape;

#[derive(Component)]
pub struct BvhCache {
    pub bvh: Bvh<f32, 3>,
    pub triangles: Vec<shape::Triangle>,
}

pub fn build_bvh_cache(mesh: &Mesh) -> Option<BvhCache> {
    if mesh.primitive_topology() != PrimitiveTopology::TriangleList {
        warn!("No triangle list topology");
        return None; // ray_mesh_intersection assumes vertices are laid out in a triangle list
    }

    // Vertex positions are required
    let positions = mesh.attribute(Mesh::ATTRIBUTE_POSITION)?.as_float3()?;

    // Normals are optional
    let normals = mesh
        .attribute(Mesh::ATTRIBUTE_NORMAL)
        .and_then(|normal_values| normal_values.as_float3());

    let mut triangles = if let Some(indices) = mesh.indices() {
        match indices {
            Indices::U16(items) => get_triangles(positions, normals, Some(items)),
            Indices::U32(items) => get_triangles(positions, normals, Some(items)),
        }
    } else {
        get_triangles::<u16>(positions, normals, None)
    };

    info!("Triangle count: {}", triangles.len());

    let bvh = Bvh::build(&mut triangles);

    Some(BvhCache { bvh, triangles })
}

fn get_triangles<I: TryInto<usize> + Clone + Copy>(
    positions: &[[f32; 3]],
    vertex_normals: Option<&[[f32; 3]]>,
    indices: Option<&[I]>,
) -> Vec<Triangle> {
    if let Some(indices) = indices {
        indices
            .chunks_exact(3)
            .flat_map(|triangle| -> Option<Triangle> {
                let [a, b, c] = [
                    triangle[0].try_into().ok()?,
                    triangle[1].try_into().ok()?,
                    triangle[2].try_into().ok()?,
                ];

                let triangle_index = a;
                let tri_vertex_positions = &[
                    Vec3::from(positions[a]),
                    Vec3::from(positions[b]),
                    Vec3::from(positions[c]),
                ];
                let tri_normals = vertex_normals.map(|normals| {
                    [
                        Vec3::from(normals[a]),
                        Vec3::from(normals[b]),
                        Vec3::from(normals[c]),
                    ]
                });

                Some(Triangle::new(
                    triangle_index,
                    tri_vertex_positions.clone(),
                    tri_normals,
                ))
            })
            .collect()
    } else {
        positions
            .chunks_exact(3)
            .enumerate()
            .flat_map(|(i, triangle)| -> Option<Triangle> {
                let &[a, b, c] = triangle else {
                    return None;
                };
                let triangle_index = i;
                let tri_vertex_positions = &[Vec3::from(a), Vec3::from(b), Vec3::from(c)];
                let tri_normals = vertex_normals.map(|normals| {
                    [
                        Vec3::from(normals[i]),
                        Vec3::from(normals[i + 1]),
                        Vec3::from(normals[i + 2]),
                    ]
                });

                Some(Triangle::new(
                    triangle_index,
                    tri_vertex_positions.clone(),
                    tri_normals,
                ))
            })
            .collect()
    }
}
