use bevy_asset::prelude::*;
use bevy_ecs::{prelude::*, world::CommandQueue};
use bevy_log::prelude::*;
use bevy_tasks::prelude::*;

use bevy_render::{
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
};

use bvh::bvh::Bvh;
use triangle::BVHTriangle;

use crate::{
    common::get_triangles,
    storage::{AssetBvhCache, AssetsBvhCaches},
    ComputeBvhCache,
};

pub mod ray_cast;
mod triangle;

pub struct BvhCache {
    pub bvh: Bvh<f32, 3>,
    pub triangles: Vec<triangle::BVHTriangle>,
}

impl AssetBvhCache for BvhCache {}

/// Detect new assets and generate BVH tree
pub fn compute_bvh_cache_assets(
    mut commands: Commands,
    mut asset_events: EventReader<AssetEvent<Mesh>>,
    meshes: Res<Assets<Mesh>>,
) {
    let thread_pool = AsyncComputeTaskPool::get();

    for ev in asset_events.read() {
        match ev {
            AssetEvent::Added { id } => {
                let Some(mesh) = meshes.get(*id) else {
                    warn!("Missing mesh for mesh {}", id);
                    continue;
                };

                let task_entity = commands.spawn_empty().id();
                let task = thread_pool.spawn({
                    // We need to clone the mesh to be able to process it asynchronously
                    let mesh = mesh.clone();
                    let asset_id = id.clone();
                    async move {
                        let mut command_queue = CommandQueue::default();

                        let build_bvh_cache_span = info_span!("build_bvh_cache");
                        let build_bvh_cache_guard = build_bvh_cache_span.enter();
                        let bvh_cache = build_bvh_cache(&mesh);
                        drop(build_bvh_cache_guard);

                        if let Some(bvh_cache) = bvh_cache {
                            command_queue.push(move |world: &mut World| {
                                let mut bvh_caches =
                                    world.resource_mut::<AssetsBvhCaches<Mesh, BvhCache>>();
                                bvh_caches.insert(asset_id, bvh_cache);
                            })
                        }

                        command_queue
                    }
                });
                // Spawn new entity and add our new task as a component
                commands.entity(task_entity).insert(ComputeBvhCache(task));
            }
            _ => {}
        }
    }
}

fn build_bvh_cache(mesh: &Mesh) -> Option<BvhCache> {
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

    let triangles = if let Some(indices) = mesh.indices() {
        match indices {
            Indices::U16(items) => get_triangles(positions, normals, Some(items)),
            Indices::U32(items) => get_triangles(positions, normals, Some(items)),
        }
    } else {
        get_triangles::<u16>(positions, normals, None)
    };

    // Convert triangles to the correct type
    let mut triangles = triangles
        .into_iter()
        .map(BVHTriangle::from_triangle)
        .collect::<Vec<_>>();

    let bvh = Bvh::build(&mut triangles);

    Some(BvhCache { bvh, triangles })
}
