use bevy_math::Vec3;
use triangle::Triangle;

pub mod triangle;

pub fn get_triangles<I: TryInto<usize> + Clone + Copy>(
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
