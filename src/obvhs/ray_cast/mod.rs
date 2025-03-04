use bevy_math::{Dir3, Mat4, Ray3d, Vec3A};
use bevy_picking_more_hitinfo::mesh_picking::ray_cast::{Backfaces, RayMeshHit};
use obvhs::ray::RayHit;
use std::f32;

use crate::ray_cast::intersections::triangle_intersection;

use super::ObvhsBvh2Cache;

/// Casts a ray on a mesh, and returns the intersection, using bvh cache.
pub fn ray_intersection_over_mesh_using_obvhs_bvh2_cache(
    transform: &Mat4,
    ray: Ray3d,
    culling: Backfaces,
    cache: &ObvhsBvh2Cache,
) -> Option<RayMeshHit> {
    let world_to_mesh = transform.inverse();

    let mesh_space_ray = Ray3d::new(
        world_to_mesh.transform_point3(ray.origin),
        Dir3::new(world_to_mesh.transform_vector3(*ray.direction)).ok()?,
    );

    let ray = obvhs::ray::Ray::new_inf(
        mesh_space_ray.origin.into(),
        Vec3A::from_array(mesh_space_ray.direction.to_array()),
    );

    let mut closest_hit_distance = f32::MAX;
    let mut closest_hit = None;

    let mut ray_hit = RayHit::none();

    let mut ray_traversal = cache.bvh.new_ray_traversal(ray);
    while cache
        .bvh
        .ray_traverse_dynamic(&mut ray_traversal, &mut ray_hit, |_ray, id| {
            let Some(triangle) = cache
                .triangles
                .get(cache.bvh.primitive_indices[id] as usize)
            else {
                return f32::INFINITY;
            };

            let Some(hit) = triangle_intersection(
                &triangle.positions,
                &triangle.normals,
                closest_hit_distance,
                &mesh_space_ray,
                culling,
            ) else {
                return f32::INFINITY;
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
                triangle_index: Some(triangle.triangle_index),
            });
            closest_hit_distance = hit.distance;

            hit.distance
        })
    {}

    closest_hit
}
