use bevy_math::{Mat4, Ray3d, Vec3};
use bevy_picking_more_hitinfo::mesh_picking::ray_cast::{ray_mesh_intersection, Backfaces, RayMeshHit};
use bevy_render::mesh::{Indices, Mesh, PrimitiveTopology};

/// Hit data for an intersection between a ray and a triangle.
#[derive(Default, Debug)]
pub struct RayTriangleHit {
    pub distance: f32,
    pub barycentric_coords: (f32, f32),
}

/// Casts a ray on a mesh, and returns the intersection.
pub fn ray_intersection_over_mesh(
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

pub fn triangle_intersection(
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
pub fn ray_triangle_intersection(
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
    use bevy_math::{Dir3, Vec3};

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
