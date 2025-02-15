use bevy_math::{Dir3, Mat4, Ray3d, Vec3};
use bevy_picking::mesh_picking::ray_cast::{ray_mesh_intersection, RayMeshHit};
use bevy_render::mesh::{Indices, Mesh, PrimitiveTopology};

use crate::bvh::BvhCache;

use super::Backfaces;

/// Hit data for an intersection between a ray and a triangle.
#[derive(Default, Debug)]
pub struct RayTriangleHit {
    pub distance: f32,
    pub barycentric_coords: (f32, f32),
}

/// Casts a ray on a mesh, and returns the intersection.
pub(super) fn ray_intersection_over_mesh(
    mesh: &Mesh,
    transform: &Mat4,
    ray: Ray3d,
    culling: Backfaces,
) -> Option<RayMeshHit> {
    if mesh.primitive_topology() != PrimitiveTopology::TriangleList {
        return None; // ray_mesh_intersection assumes vertices are laid out in a triangle list
    }
    // Vertex positions are required
    let positions = mesh.attribute(Mesh::ATTRIBUTE_POSITION)?.as_float3()?;

    // Normals are optional
    let normals = mesh
        .attribute(Mesh::ATTRIBUTE_NORMAL)
        .and_then(|normal_values| normal_values.as_float3());

    match mesh.indices() {
        Some(Indices::U16(indices)) => {
            ray_mesh_intersection(ray, transform, positions, normals, Some(indices), culling)
        }
        Some(Indices::U32(indices)) => {
            ray_mesh_intersection(ray, transform, positions, normals, Some(indices), culling)
        }
        None => ray_mesh_intersection::<usize>(ray, transform, positions, normals, None, culling),
    }
}

/// Casts a ray on a mesh, and returns the intersection, using bvh cache.
pub(super) fn ray_intersection_over_mesh_using_bvh_cache(
    transform: &Mat4,
    ray: Ray3d,
    culling: Backfaces,
    bvh_cache: &BvhCache,
) -> Option<RayMeshHit> {
    let world_to_mesh = transform.inverse();

    let mesh_space_ray = Ray3d::new(
        world_to_mesh.transform_point3(ray.origin),
        Dir3::new(world_to_mesh.transform_vector3(*ray.direction)).ok()?,
    );

    let origin = world_to_mesh.transform_point3(ray.origin);
    let direction = Dir3::new(world_to_mesh.transform_vector3(*ray.direction)).ok()?;

    let ray = bvh::ray::Ray::new(
        nalgebra::Point3::new(origin.x, origin.y, origin.z),
        nalgebra::SVector::<f32, 3>::new(direction.x, direction.y, direction.z),
    );

    let hit_aabbs = bvh_cache.bvh.traverse(&ray, &bvh_cache.triangles);
    // info!("Got {} hit aabbs", hit_aabbs.len());

    // The ray cast can hit the same mesh many times, so we need to track which hit is
    // closest to the camera, and record that.
    let mut closest_hit_distance = f32::MAX;
    let mut closest_hit = None;

    for (triangle_index, triangle) in hit_aabbs.iter().enumerate() {
        let tri_vertex_positions = &triangle.positions;
        let tri_normals = &triangle.normals;

        let Some(hit) = triangle_intersection(
            tri_vertex_positions,
            tri_normals,
            closest_hit_distance,
            &mesh_space_ray,
            culling,
        ) else {
            continue;
        };

        closest_hit = Some(RayMeshHit {
            point: transform.transform_point3(hit.point),
            normal: transform.transform_vector3(hit.normal),
            barycentric_coords: hit.barycentric_coords,
            distance: transform
                .transform_vector3(mesh_space_ray.direction * hit.distance)
                .length(),
            triangle: hit.triangle.map(|tri| {
                [
                    transform.transform_point3(tri[0]),
                    transform.transform_point3(tri[1]),
                    transform.transform_point3(tri[2]),
                ]
            }),
            triangle_index: Some(triangle_index),
        });
        closest_hit_distance = hit.distance;
    }

    closest_hit
}

fn triangle_intersection(
    tri_vertices: &[Vec3; 3],
    tri_normals: &Option<[Vec3; 3]>,
    max_distance: f32,
    ray: &Ray3d,
    backface_culling: Backfaces,
) -> Option<RayMeshHit> {
    let hit = ray_triangle_intersection(ray, tri_vertices, backface_culling)?;

    if hit.distance < 0.0 || hit.distance > max_distance {
        return None;
    };

    let point = ray.get_point(hit.distance);
    let u = hit.barycentric_coords.0;
    let v = hit.barycentric_coords.1;
    let w = 1.0 - u - v;
    let barycentric = Vec3::new(u, v, w);

    let normal = if let Some(normals) = tri_normals {
        normals[1] * u + normals[2] * v + normals[0] * w
    } else {
        (tri_vertices[1] - tri_vertices[0])
            .cross(tri_vertices[2] - tri_vertices[0])
            .normalize()
    };

    Some(RayMeshHit {
        point,
        normal,
        barycentric_coords: barycentric,
        distance: hit.distance,
        triangle: Some(*tri_vertices),
        triangle_index: None,
    })
}

/// Takes a ray and triangle and computes the intersection.
fn ray_triangle_intersection(
    ray: &Ray3d,
    triangle: &[Vec3; 3],
    backface_culling: Backfaces,
) -> Option<RayTriangleHit> {
    // Source: https://www.scratchapixel.com/lessons/3d-basic-rendering/ray-tracing-rendering-a-triangle/moller-trumbore-ray-triangle-intersection
    let vector_v0_to_v1: Vec3 = triangle[1] - triangle[0];
    let vector_v0_to_v2: Vec3 = triangle[2] - triangle[0];
    let p_vec: Vec3 = ray.direction.cross(vector_v0_to_v2);
    let determinant: f32 = vector_v0_to_v1.dot(p_vec);

    match backface_culling {
        Backfaces::Cull => {
            // if the determinant is negative the triangle is back facing
            // if the determinant is close to 0, the ray misses the triangle
            // This test checks both cases
            if determinant < f32::EPSILON {
                return None;
            }
        }
        Backfaces::Include => {
            // ray and triangle are parallel if det is close to 0
            if determinant.abs() < f32::EPSILON {
                return None;
            }
        }
    }

    let determinant_inverse = 1.0 / determinant;

    let t_vec = ray.origin - triangle[0];
    let u = t_vec.dot(p_vec) * determinant_inverse;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }

    let q_vec = t_vec.cross(vector_v0_to_v1);
    let v = (*ray.direction).dot(q_vec) * determinant_inverse;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }

    // The distance between ray origin and intersection is t.
    let t: f32 = vector_v0_to_v2.dot(q_vec) * determinant_inverse;

    Some(RayTriangleHit {
        distance: t,
        barycentric_coords: (u, v),
    })
}

#[cfg(test)]
mod tests {
    use bevy_math::Vec3;

    use super::*;

    // Triangle vertices to be used in a left-hand coordinate system
    const V0: [f32; 3] = [1.0, -1.0, 2.0];
    const V1: [f32; 3] = [1.0, 2.0, -1.0];
    const V2: [f32; 3] = [1.0, -1.0, -1.0];

    #[test]
    fn ray_cast_triangle_mt() {
        let triangle = [V0.into(), V1.into(), V2.into()];
        let ray = Ray3d::new(Vec3::ZERO, Dir3::X);
        let result = ray_triangle_intersection(&ray, &triangle, Backfaces::Include);
        assert!(result.unwrap().distance - 1.0 <= f32::EPSILON);
    }

    #[test]
    fn ray_cast_triangle_mt_culling() {
        let triangle = [V2.into(), V1.into(), V0.into()];
        let ray = Ray3d::new(Vec3::ZERO, Dir3::X);
        let result = ray_triangle_intersection(&ray, &triangle, Backfaces::Cull);
        assert!(result.is_none());
    }
}
