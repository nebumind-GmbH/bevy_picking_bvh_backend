use std::time::Instant;

use bevy_picking_more_hitinfo::prelude::*;

use bevy_internal::prelude::*;
use bevy_pbr::PbrPlugin;
use bevy_ecs::component::Component;
use bevy_gltf::GltfPlugin;
use bevy_core_pipeline::CorePipelinePlugin;

use bevy_app::{
  App,
  PluginsState
};
use bevy_log::LogPlugin;
use bevy_math::sampling::UniformMeshSampler;
use bevy_picking_bvh_backend::{
    ray_cast::BvhMeshRayCast, run_if_bvh_cache_ready, BvhBackend, BvhCacheStatus,
    PickingBvhBackend, PickingBvhCache,
};
use bevy_render::{primitives::Aabb, RenderPlugin};
use bevy_scene::ScenePlugin;
use rand::prelude::Distribution;
use rand_chacha::{
    rand_core::{RngCore, SeedableRng},
    ChaCha8Rng,
};

#[test]
fn run_bench() {
    // bench(vec!["models/dragon_high.glb".to_string()]);
    bench(vec!["models/dragon_high.glb".to_string()]);
}

fn bench(meshes: Vec<String>) {
    let mut app = init_app(meshes);

    info!("--- Preparing app for benchmarks");

    // Wait until app is ready
    loop {
        app.update();
        let picking_bvh_cache = app.world().resource::<PickingBvhCache>();
        let test_meshes = app.world().resource::<TestMeshes>();
        if picking_bvh_cache.status == BvhCacheStatus::Ready && test_meshes.loaded {
            break;
        }
    }

    info!("--- App ready for benchmarks");

    // Run 1000 raycasts with None backend
    bench_with_backend(&mut app, BvhBackend::None, 1000);

    #[cfg(feature = "obvhs")]
    {
        // Run 10000 raycasts with ObvhsBvh2 backend
        bench_with_backend(&mut app, BvhBackend::ObvhsBvh2, 10000);
    }

    #[cfg(feature = "bvh")]
    {
        // Run 10000 raycasts with ObvhsBvh2 backend
        bench_with_backend(&mut app, BvhBackend::Bvh, 10000);
    }
}

fn create_test_app() -> App {
    let mut app = App::new();

    // Note the use of `MinimalPlugins` instead of `DefaultPlugins`, as described above.
    app.add_plugins(MinimalPlugins);

    // Inserting a `KeyCode` input resource allows us to inject keyboard inputs, as if the user had
    // pressed them.
    app.insert_resource(ButtonInput::<KeyCode>::default());

    // Spawning a fake window allows testing systems that require a window.
    app.world_mut().spawn(Window::default());

    app
}

/// Init a new app for loading the corresponding meshes
fn init_app(meshes: Vec<String>) -> App {
    // Setup app
    let mut app = create_test_app();

    // Add bevy plugins
    app.add_plugins((
        TransformPlugin::default(),
        HierarchyPlugin::default(),
        WindowPlugin::default(),
        LogPlugin::default(),
        AssetPlugin::default(),
        ScenePlugin::default(),
        RenderPlugin::default(),
        ImagePlugin::default(),
        CorePipelinePlugin::default(),
        PbrPlugin::default(),
        GltfPlugin::default(),
    ));

    // Add our plugin, start with none backend
    app.add_plugins(PickingBvhBackend::with_backend(BvhBackend::None));

    let seeded_rng = ChaCha8Rng::seed_from_u64(19878367467712);
    app.insert_resource(RandomSource(seeded_rng));
    app.insert_resource(Stats::default());
    app.insert_resource(TestMeshes::new(meshes));

    app.add_systems(Startup, setup_scene);
    app.add_systems(PreUpdate, (check_mesh_scenes_loaded, compute_samplers));

    // Raycast only if bvh cache is ready
    app.add_systems(Update, raycast.run_if(run_if_bvh_cache_ready));

    // from `run_once` runner
    while app.plugins_state() == PluginsState::Adding {
        #[cfg(not(target_arch = "wasm32"))]
        bevy_tasks::tick_global_task_pools_on_main_thread();
    }

    app.finish();
    app.cleanup();

    app
}

/// Run a benchmark with the selected backend
/// Returns the average raycast time in nano seconds
fn bench_with_backend(app: &mut App, backend: BvhBackend, num_raycasts: usize) -> u128 {
    info!(
        "--- Benchmark with backend {:?} and {} raycasts",
        backend, num_raycasts
    );
    // Set the backend
    let mut picking_bvh_backend = app.world_mut().resource_mut::<PickingBvhBackend>();
    picking_bvh_backend.backend = backend;

    let mut stats = app.world_mut().resource_mut::<Stats>();
    stats.reset();

    // update app's state until target number of raycasts is reached
    loop {
        app.update();
        let stats = app.world().resource::<Stats>();
        // exit when target number of raycasts is reached
        if stats.raycasts >= num_raycasts {
            break;
        }
    }

    let stats = app.world().resource::<Stats>();
    info!("Spawned {} rays with {} hits", stats.raycasts, stats.hits);
    if stats.raycasts > 0 {
        info!(
            "Average raycast time: {}ns",
            stats.total / stats.raycasts as u128
        );
    }

    stats.total / stats.raycasts as u128
}

#[derive(Resource)]
struct TestMeshes {
    pub meshes: Vec<String>,
    pub loaded: bool,
    pub scene_handles: Vec<Handle<Scene>>,
}

impl TestMeshes {
    pub fn new(meshes: Vec<String>) -> Self {
        TestMeshes {
            meshes,
            loaded: false,
            scene_handles: Vec::new(),
        }
    }
}

#[derive(Component)]
struct MeshSampler {
    pub mesh_sampler: UniformMeshSampler,
    pub aabb_sampler: Cuboid,
}

#[derive(Resource, Default)]
struct Stats {
    durations: Vec<f64>,
    total: u128,
    raycasts: usize,
    hits: usize,
}

impl Stats {
    pub fn reset(&mut self) {
        self.durations = Vec::new();
        self.total = 0;
        self.raycasts = 0;
        self.hits = 0;
    }
}

fn setup_scene(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut test_meshes: ResMut<TestMeshes>,
) {
    for mesh in test_meshes.meshes.clone() {
        let scene: Handle<Scene> = asset_server.load(GltfAssetLabel::Scene(0).from_asset(mesh));
        commands.spawn(SceneRoot(scene.clone()));
        test_meshes.scene_handles.push(scene);
    }
}

fn check_mesh_scenes_loaded(scenes: Res<Assets<Scene>>, mut test_meshes: ResMut<TestMeshes>) {
    // if all scenes not yet loaded
    if !test_meshes.loaded {
        // check if all scenes are loaded yet
        test_meshes.loaded = test_meshes
            .scene_handles
            .iter()
            .all(|scene| scenes.get(scene).is_some());
    }
}

fn compute_samplers(
    mut commands: Commands,
    meshe_3ds: Query<(Entity, &Mesh3d, &Aabb), Without<MeshSampler>>,
    meshes: Res<Assets<Mesh>>,
) {
    for (entity, mesh_3d, aabb) in meshe_3ds.iter() {
        // get the mesh asset
        let Some(mesh) = meshes.get(&mesh_3d.0) else {
            continue;
        };

        let mesh_sampler = UniformMeshSampler::try_new(mesh.triangles().unwrap()).unwrap();
        let aabb_sampler = Cuboid::from_corners(aabb.min().into(), aabb.max().into());

        commands.entity(entity).insert(MeshSampler {
            mesh_sampler,
            aabb_sampler,
        });
    }
}

fn raycast(
    mut ray_cast: BvhMeshRayCast,
    mut random_source: ResMut<RandomSource>,
    mut stats: ResMut<Stats>,
    samplers: Query<&MeshSampler>,
) {
    let settings = RayCastSettings {
        visibility: RayCastVisibility::Any,
        filter: &|_| {
            return true;
        },
        early_exit_test: &|_| {
            return false;
        },
    };

    // Pick a random mesh
    let samplers = samplers.iter().collect::<Vec<_>>();
    if samplers.len() > 0 {
        let i = (random_source.0.next_u32() as usize) % samplers.len();
        let sampler = samplers[i];

        let origin = sampler.aabb_sampler.sample_boundary(&mut random_source.0);
        let target = sampler.mesh_sampler.sample(&mut random_source.0);
        let Ok(dir) = (target - origin).try_into() else {
            info!("Invalid dir generated");
            return;
        };

        let ray = Ray3d::new(origin, dir);

        let now = Instant::now();
        let hits = ray_cast.cast_ray(ray, &settings);
        let elapsed = now.elapsed().as_nanos();
        stats.durations.push(elapsed as f64);
        stats.raycasts += 1;
        stats.hits += hits.len();
        stats.total += elapsed;
    };
}

#[derive(Resource)]
struct RandomSource(ChaCha8Rng);
