use bevy_app::prelude::*;
use bevy_asset::prelude::*;
use bevy_ecs::{prelude::*, world::CommandQueue};
use bevy_log::prelude::*;
use bevy_picking::{
    backend::{ray::RayMap, HitData, PointerHits},
    mesh_picking::{
        ray_cast::{RayCastSettings, SimplifiedMesh},
        MeshPickingSettings, RayCastPickable,
    },
    PickSet, PickingBehavior,
};
use bevy_reflect::prelude::*;
use bevy_render::{prelude::*, primitives::Aabb, view::RenderLayers};
use bevy_tasks::{prelude::*, Task};
use bevy_transform::components::GlobalTransform;
use bvh::{build_bvh_cache, ray_cast::BvhMeshRayCast, BvhCache};
use futures_lite::future;

pub mod bvh;

#[derive(Copy, Clone, Debug, Resource, Reflect)]
#[reflect(Resource, Default, Debug)]
pub struct PickingBvhBackend;

impl Default for PickingBvhBackend {
    fn default() -> Self {
        Self {}
    }
}

impl Plugin for PickingBvhBackend {
    fn build(&self, app: &mut App) {
        app
            // register bevy_picking dependencies
            .init_resource::<MeshPickingSettings>()
            .register_type::<(RayCastPickable, MeshPickingSettings, SimplifiedMesh)>()
            // register our systems
            .add_systems(PreUpdate, compute_bvh_cache)
            .add_systems(PreUpdate, update_hits.in_set(PickSet::Backend))
            .add_systems(Update, handle_tasks);
    }
}

#[derive(Component)]
struct BuildingBvhCache;

/// Detect new assets and generate BVH tree
fn compute_bvh_cache(
    mut commands: Commands,
    mesh_3ds_to_load: Query<Entity, (With<Mesh3d>, Without<BuildingBvhCache>, Without<BvhCache>)>,
    mesh_3ds: Query<(&Mesh3d, &Aabb, &GlobalTransform)>,
    meshes: Res<Assets<Mesh>>,
) {
    // Iterate over mesh 3ds
    for entity in mesh_3ds_to_load.iter() {
        if let Ok((mesh_3d, _aabb, _global_transform)) = mesh_3ds.get(entity) {
            info!("Entity {} mesh {}", entity.index(), mesh_3d.id());

            let Some(mesh) = meshes.get(mesh_3d) else {
                warn!("Missing mesh for mesh_3d {}", mesh_3d.id());
                continue;
            };

            let thread_pool = AsyncComputeTaskPool::get();

            let task_entity = commands.spawn_empty().id();
            let task = thread_pool.spawn({
                // We need to clone the mesh to be able to process it asynchronously
                let mesh = mesh.clone();
                async move {
                    let mut command_queue = CommandQueue::default();

                    info!("Building BVH cache...");
                    let bvh_cache = build_bvh_cache(&mesh);
                    info!("BVH cache built.");

                    if let Some(bvh_cache) = bvh_cache {
                        command_queue.push(move |world: &mut World| {
                            world
                                .entity_mut(entity)
                                // add the bvh cache to the mesh
                                .insert(bvh_cache)
                                // remove the marker BuildingBvhCache
                                .remove::<BuildingBvhCache>();
                        })
                    }

                    command_queue
                }
            });
            commands.entity(entity).insert(BuildingBvhCache);

            // Spawn new entity and add our new task as a component
            commands.entity(task_entity).insert(ComputeBvhCache(task));
        }
    }
}

#[derive(Component)]
struct ComputeBvhCache(Task<CommandQueue>);

/// This system queries for entities that have our Task<Transform> component. It polls the
/// tasks to see if they're complete. If the task is complete it takes the result, adds a
/// new [`Mesh3d`] and [`MeshMaterial3d`] to the entity using the result from the task's work, and
/// removes the task component from the entity.
fn handle_tasks(
    mut commands: Commands,
    mut transform_tasks: Query<(Entity, &mut ComputeBvhCache)>,
) {
    for (task_entity, mut task) in &mut transform_tasks {
        if let Some(mut commands_queue) = block_on(future::poll_once(&mut task.0)) {
            // append the returned command queue to have it execute later
            commands.append(&mut commands_queue);
            // remove the task entity to prevent polling it again
            commands.entity(task_entity).despawn();
        }
    }
}

/// Casts rays into the scene using [`MeshPickingSettings`] and sends [`PointerHits`] events.
#[allow(clippy::too_many_arguments)]
pub fn update_hits(
    backend_settings: Res<MeshPickingSettings>,
    ray_map: Res<RayMap>,
    picking_cameras: Query<(&Camera, Option<&RayCastPickable>, Option<&RenderLayers>)>,
    pickables: Query<&PickingBehavior>,
    marked_targets: Query<&RayCastPickable>,
    layers: Query<&RenderLayers>,
    mut ray_cast: BvhMeshRayCast,
    mut output: EventWriter<PointerHits>,
) {
    for (&ray_id, &ray) in ray_map.map().iter() {
        let Ok((camera, cam_pickable, cam_layers)) = picking_cameras.get(ray_id.camera) else {
            continue;
        };
        if backend_settings.require_markers && cam_pickable.is_none() {
            continue;
        }

        let cam_layers = cam_layers.to_owned().unwrap_or_default();

        let settings = RayCastSettings {
            visibility: backend_settings.ray_cast_visibility,
            filter: &|entity| {
                let marker_requirement =
                    !backend_settings.require_markers || marked_targets.get(entity).is_ok();

                // Other entities missing render layers are on the default layer 0
                let entity_layers = layers.get(entity).cloned().unwrap_or_default();
                let render_layers_match = cam_layers.intersects(&entity_layers);

                let is_pickable = pickables
                    .get(entity)
                    .map(|p| p.is_hoverable)
                    .unwrap_or(true);

                marker_requirement && render_layers_match && is_pickable
            },
            early_exit_test: &|entity_hit| {
                pickables
                    .get(entity_hit)
                    .is_ok_and(|pickable| pickable.should_block_lower)
            },
        };
        let picks = ray_cast
            .cast_ray(ray, &settings)
            .iter()
            .map(|(entity, hit)| {
                let hit_data = HitData::new(
                    ray_id.camera,
                    hit.distance,
                    Some(hit.point),
                    Some(hit.normal),
                );
                (*entity, hit_data)
            })
            .collect::<Vec<_>>();
        let order = camera.order as f32;
        if !picks.is_empty() {
            output.send(PointerHits::new(ray_id.pointer, picks, order));
        }
    }
}
