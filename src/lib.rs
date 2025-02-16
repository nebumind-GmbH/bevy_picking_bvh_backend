use bevy_app::prelude::*;
use bevy_ecs::{prelude::*, world::CommandQueue};
use bevy_picking::{
    backend::{ray::RayMap, HitData, PointerHits},
    mesh_picking::{
        ray_cast::{RayCastSettings, SimplifiedMesh},
        MeshPickingSettings, RayCastPickable,
    },
    PickSet, PickingBehavior,
};
use bevy_reflect::prelude::*;
use bevy_render::{prelude::*, view::RenderLayers};
use bevy_tasks::{prelude::*, Task};
#[cfg(feature = "bvh")]
use bvh::{compute_bvh_cache_assets, BvhCache};
use futures_lite::future;

#[cfg(feature = "obvhs")]
use obvhs::{compute_obvhs_bvh2_cache_assets, ObvhsBvh2Cache};
use ray_cast::MeshRayCast;
#[cfg(any(feature = "obvhs", feature = "bvh"))]
use storage::AssetsBvhCaches;

pub mod storage;

#[cfg(feature = "bvh")]
pub mod bvh;

#[cfg(feature = "obvhs")]
pub mod obvhs;

pub mod common;
pub mod ray_cast;

#[derive(Clone, Debug, Reflect)]
pub enum BvhBackend {
    None,
    #[cfg(feature = "bvh")]
    Bvh,
    #[cfg(feature = "obvhs")]
    ObvhsBvh2,
}

impl Default for BvhBackend {
    #[cfg(all(not(feature = "obvhs"), not(feature = "bvh")))]
    fn default() -> Self {
        Self::None
    }
    #[cfg(all(not(feature = "obvhs"), feature = "bvh"))]
    fn default() -> Self {
        Self::Bvh
    }
    #[cfg(feature = "obvhs")]
    fn default() -> Self {
        Self::ObvhsBvh2
    }
}

#[derive(Clone, Debug, Default, Resource, Reflect)]
#[reflect(Resource, Default, Debug)]
pub struct PickingBvhBackend {
    backend: BvhBackend,
}

impl PickingBvhBackend {
    pub fn with_backend(backend: BvhBackend) -> Self {
        Self { backend }
    }
}

impl Plugin for PickingBvhBackend {
    fn build(&self, app: &mut App) {
        app
            // register bevy_picking dependencies
            .init_resource::<MeshPickingSettings>()
            .register_type::<(RayCastPickable, MeshPickingSettings, SimplifiedMesh)>()
            // register our systems
            .add_systems(PreUpdate, update_hits.in_set(PickSet::Backend))
            .add_systems(Update, handle_tasks);

        #[cfg(feature = "bvh")]
        {
            app.add_systems(PreUpdate, compute_bvh_cache_assets);
            app.insert_resource(AssetsBvhCaches::<Mesh, BvhCache>::default());
        }

        #[cfg(feature = "obvhs")]
        {
            app.add_systems(PreUpdate, compute_obvhs_bvh2_cache_assets);
            app.insert_resource(AssetsBvhCaches::<Mesh, ObvhsBvh2Cache>::default());
        }

        app.insert_resource(self.clone());
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
    mut ray_cast: MeshRayCast,
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
