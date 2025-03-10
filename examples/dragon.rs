use bevy_color::palettes::tailwind::*;
use bevy_asset::*;
use bevy_app::*;
use bevy_sprite::*;
use bevy_ecs::prelude::Commands;
use bevy_ecs::prelude::Res;
use bevy_pbr::PointLight;
use bevy_utils::default;
use bevy_core_pipeline::prelude::Camera3d;
use bevy_math::Vec3;
use bevy_internal::prelude::*;
use bevy_internal::DefaultPlugins;


use bevy_picking_bvh_backend::{mesh_picking::MeshPickingBvhPlugin, PickingBvhBackend};
use bevy_picking_more_hitinfo::{
    *,
    pointer::PointerInteraction,
};

fn main() {
    App::new()
        .init_resource::<backend::ray::RayMap>()
        .init_resource::<Assets<TextureAtlasLayout>>()
        .add_plugins((
            DefaultPlugins,
            bevy_picking_more_hitinfo::DefaultPickingPlugins,
            PickingBvhBackend::default(),
            MeshPickingBvhPlugin,
        ))
        .add_systems(Startup, setup_scene)
        .add_systems(Update, draw_mesh_intersections)
        .run();
}

fn setup_scene(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Light
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            intensity: 10_000_000.,
            range: 100.0,
            shadow_depth_bias: 0.2,
            ..default()
        },
        Transform::from_xyz(8.0, 16.0, 8.0),
    ));

    // Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 7., 14.0).looking_at(Vec3::new(0., 1., 0.), Vec3::Y),
    ));

    commands
        .spawn((
            SceneRoot(
                asset_server.load(GltfAssetLabel::Scene(0).from_asset("models/dragon_high.glb")),
            ),
            Transform::from_scale(Vec3::splat(3.)),
        ))
        .observe(rotate_on_drag);
}

/// A system that draws hit indicators for every pointer.
fn draw_mesh_intersections(pointers: Query<&PointerInteraction>, mut gizmos: Gizmos) {
    for ((point, normal), tri_index) in pointers
        .iter()
        .filter_map(|interaction| interaction.get_nearest_hit())
        .filter_map(|(_entity, hit)| ( hit.position.zip(hit.normal).zip(hit.triangle_index) ) )
    {
        gizmos.sphere(point, 0.05, RED_500);
        gizmos.arrow(point, point + normal.normalize() * 0.5, PINK_100);
    }
}

/// An observer to rotate an entity when it is dragged
fn rotate_on_drag(drag: Trigger<Pointer<Drag>>, mut transforms: Query<&mut Transform>) {
    let mut transform = transforms.get_mut(drag.entity()).unwrap();
    transform.rotate_y(drag.delta.x * 0.02);
    transform.rotate_x(drag.delta.y * 0.02);
}
