use bevy_app::prelude::*;
use bevy_asset::AssetEvent;
use bevy_ecs::{prelude::*, world::CommandQueue};
use bevy_reflect::prelude::*;
use bevy_render::prelude::*;
use bevy_tasks::{prelude::*, Task};
#[cfg(feature = "bvh")]
use bvh::{compute_bvh_cache_assets, BvhCache};
use futures_lite::future;

#[cfg(feature = "obvhs")]
use obvhs::{compute_obvhs_bvh2_cache_assets, ObvhsBvh2Cache};
#[cfg(any(feature = "obvhs", feature = "bvh"))]
use storage::AssetsBvhCaches;

pub mod mesh_picking;

pub mod storage;

#[cfg(feature = "bvh")]
pub mod bvh;

#[cfg(feature = "obvhs")]
pub mod obvhs;

pub mod common;
pub mod ray_cast;

#[derive(Clone, Debug, Reflect, Default, PartialEq, Eq)]
pub enum BvhCacheStatus {
    Building,
    #[default]
    Ready,
}

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
    pub backend: BvhBackend,
}

impl PickingBvhBackend {
    pub fn with_backend(backend: BvhBackend) -> Self {
        Self { backend }
    }
}

#[derive(Clone, Debug, Default, Resource, Reflect)]
#[reflect(Resource, Default, Debug)]
pub struct PickingBvhCache {
    pub status: BvhCacheStatus,
}

impl Plugin for PickingBvhBackend {
    fn build(&self, app: &mut App) {
        app.init_resource::<PickingBvhCache>();

        #[cfg(any(feature = "bvh", feature = "obvhs"))]
        {
            app.add_systems(PreUpdate, detect_meshes);
            app.add_systems(PreUpdate, handle_tasks.after(detect_meshes));
        }

        #[cfg(feature = "bvh")]
        {
            app.add_systems(
                PreUpdate,
                compute_bvh_cache_assets
                    .before(handle_tasks)
                    .after(detect_meshes),
            );
            app.insert_resource(AssetsBvhCaches::<Mesh, BvhCache>::default());
        }

        #[cfg(feature = "obvhs")]
        {
            app.add_systems(
                PreUpdate,
                compute_obvhs_bvh2_cache_assets
                    .before(handle_tasks)
                    .after(detect_meshes),
            );
            app.insert_resource(AssetsBvhCaches::<Mesh, ObvhsBvh2Cache>::default());
        }

        app.insert_resource(self.clone());
    }
}

#[derive(Component)]
struct ComputeBvhCache(Task<CommandQueue>);

fn detect_meshes(
    mut asset_events: EventReader<AssetEvent<Mesh>>,
    mut bvh_cache: ResMut<PickingBvhCache>,
) {
    'iter: for ev in asset_events.read() {
        match ev {
            AssetEvent::Added { id: _ } => {
                bvh_cache.status = BvhCacheStatus::Building;
                break 'iter;
            }
            _ => {}
        }
    }
}

/// This system queries for entities that have our Task<Transform> component. It polls the
/// tasks to see if they're complete. If the task is complete it takes the result, adds a
/// new [`Mesh3d`] and [`MeshMaterial3d`] to the entity using the result from the task's work, and
/// removes the task component from the entity.
fn handle_tasks(
    mut commands: Commands,
    mut transform_tasks: Query<(Entity, &mut ComputeBvhCache)>,
    mut bvh_cache: ResMut<PickingBvhCache>,
) {
    let mut remaining_tasks: usize = 0;
    for (task_entity, mut task) in &mut transform_tasks {
        if let Some(mut commands_queue) = block_on(future::poll_once(&mut task.0)) {
            // append the returned command queue to have it execute later
            commands.append(&mut commands_queue);
            // remove the task entity to prevent polling it again
            commands.entity(task_entity).despawn();
        } else {
            remaining_tasks += 1;
        }
    }
    if remaining_tasks > 0 {
        bvh_cache.status = BvhCacheStatus::Building;
    } else {
        bvh_cache.status = BvhCacheStatus::Ready;
    }
}

pub fn run_if_bvh_cache_ready(bvh_cache: Res<PickingBvhCache>) -> bool {
    bvh_cache.status == BvhCacheStatus::Ready
}
