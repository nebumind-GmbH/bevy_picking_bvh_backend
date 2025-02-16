use bevy_asset::prelude::*;
use bevy_ecs::{prelude::*, world::CommandQueue};
use bevy_log::prelude::*;
use bevy_render::{
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
};
use bevy_tasks::prelude::*;
use obvhs::{
    bvh2::{builder::build_bvh2_from_tris, Bvh2},
    triangle::Triangle as ObvhTriangle,
    BvhBuildParams,
};
use std::time::Duration;

use crate::{
    common::{get_triangles, triangle::Triangle},
    storage::{AssetBvhCache, AssetsBvhCaches},
    ComputeBvhCache,
};

pub mod ray_cast;

pub struct ObvhsBvh2Cache {
    pub bvh: Bvh2,
    pub triangles: Vec<Triangle>,
}

impl AssetBvhCache for ObvhsBvh2Cache {}

/// Detect new assets and generate BVH tree
pub fn compute_obvhs_bvh2_cache_assets(
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

                        info!("Building Obvhs Bvh2 cache...");
                        let bvh_cache = build_bvh2_cache(&mesh);
                        info!("Obvhs Bvh2 cache built.");

                        if let Some(bvh_cache) = bvh_cache {
                            command_queue.push(move |world: &mut World| {
                                let mut bvh_caches =
                                    world.resource_mut::<AssetsBvhCaches<Mesh, ObvhsBvh2Cache>>();
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

fn build_bvh2_cache(mesh: &Mesh) -> Option<ObvhsBvh2Cache> {
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

    // Skip building this cache if not enough triangles
    if triangles.len() < 64 {
        info!("Skip building obvhs ovh2 cache, not enough triangles.");
        return None;
    }

    let obvhs_triangles = triangles
        .iter()
        .map(|t| ObvhTriangle {
            v0: t.positions[0].into(),
            v1: t.positions[1].into(),
            v2: t.positions[2].into(),
        })
        .collect::<Vec<_>>();

    info!("Triangle count: {}", triangles.len());

    // TODO: make build params configurable at plugin level
    let bvh = build_bvh2_from_tris(
        &obvhs_triangles,
        BvhBuildParams::medium_build(),
        &mut Duration::default(),
    );

    Some(ObvhsBvh2Cache { bvh, triangles })
}
