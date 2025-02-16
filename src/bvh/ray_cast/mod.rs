use bevy_math::{Dir3, Mat4, Ray3d};
use bevy_picking::mesh_picking::ray_cast::{Backfaces, RayMeshHit};

use crate::{bvh::BvhCache, ray_cast::intersections::triangle_intersection};

/// Casts a ray on a mesh, and returns the intersection, using bvh cache.
pub fn ray_intersection_over_mesh_using_bvh_cache(
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

    let ray = bvh::ray::Ray::new(
        nalgebra::Point3::new(
            mesh_space_ray.origin.x,
            mesh_space_ray.origin.y,
            mesh_space_ray.origin.z,
        ),
        nalgebra::SVector::<f32, 3>::new(
            mesh_space_ray.direction.x,
            mesh_space_ray.direction.y,
            mesh_space_ray.direction.z,
        ),
    );

    let hit_aabbs = bvh_cache.bvh.traverse(&ray, &bvh_cache.triangles);
    // info!("Got {} hit aabbs", hit_aabbs.len());

    // The ray cast can hit the same mesh many times, so we need to track which hit is
    // closest to the camera, and record that.
    let mut closest_hit_distance = f32::MAX;
    let mut closest_hit = None;

    for triangle in hit_aabbs.iter() {
        let tri_vertex_positions = &triangle.0.positions;
        let tri_normals = &triangle.0.normals;

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
            triangle_index: Some(triangle.0.triangle_index),
        });
        closest_hit_distance = hit.distance;
    }

    closest_hit
}
