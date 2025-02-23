//! Ray casting for meshes.
//!
//! See the [`MeshRayCast`] system parameter for more information.

pub mod intersections;

use bevy_math::{bounding::Aabb3d, Ray3d};
use bevy_picking::mesh_picking::ray_cast::{
    ray_aabb_intersection_3d, Backfaces, RayCastBackfaces, RayCastSettings, RayCastVisibility,
    RayMeshHit, SimplifiedMesh,
};
use bevy_render::mesh::Mesh;

use bevy_asset::Assets;
use bevy_ecs::{prelude::*, system::lifetimeless::Read, system::SystemParam};
use bevy_math::FloatOrd;
use bevy_render::{prelude::*, primitives::Aabb};
use bevy_transform::components::GlobalTransform;
use bevy_utils::tracing::*;

#[cfg(feature = "bvh")]
use crate::bvh::{ray_cast::ray_intersection_over_mesh_using_bvh_cache, BvhCache};

#[cfg(feature = "obvhs")]
use crate::obvhs::{ray_cast::ray_intersection_over_mesh_using_obvhs_bvh2_cache, ObvhsBvh2Cache};

#[cfg(any(feature = "obvhs", feature = "bvh"))]
use crate::storage::AssetsBvhCaches;

use crate::{ray_cast::intersections::ray_intersection_over_mesh, PickingBvhBackend};

type MeshFilter = Or<(With<Mesh3d>, With<Mesh2d>, With<SimplifiedMesh>)>;

/// Add this ray casting [`SystemParam`] to your system to cast rays into the world with an
/// immediate-mode API. Call `cast_ray` to immediately perform a ray cast and get a result.
///
/// Under the hood, this is a collection of regular bevy queries, resources, and local parameters
/// that are added to your system.
///
/// ## Usage
///
/// The following system casts a ray into the world with the ray positioned at the origin, pointing in
/// the X-direction, and returns a list of intersections:
///
/// ```
/// # use bevy_math::prelude::*;
/// # use bevy_picking::prelude::*;
/// fn ray_cast_system(mut ray_cast: MeshRayCast) {
///     let ray = Ray3d::new(Vec3::ZERO, Dir3::X);
///     let hits = ray_cast.cast_ray(ray, &RayCastSettings::default());
/// }
/// ```
///
/// ## Configuration
///
/// You can specify the behavior of the ray cast using [`RayCastSettings`]. This allows you to filter out
/// entities, configure early-out behavior, and set whether the [`Visibility`] of an entity should be
/// considered.
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # use bevy_math::prelude::*;
/// # use bevy_picking::prelude::*;
/// # #[derive(Component)]
/// # struct Foo;
/// fn ray_cast_system(mut ray_cast: MeshRayCast, foo_query: Query<(), With<Foo>>) {
///     let ray = Ray3d::new(Vec3::ZERO, Dir3::X);
///
///     // Only ray cast against entities with the `Foo` component.
///     let filter = |entity| foo_query.contains(entity);
///
///     // Never early-exit. Note that you can change behavior per-entity.
///     let early_exit_test = |_entity| false;
///
///     // Ignore the visibility of entities. This allows ray casting hidden entities.
///     let visibility = RayCastVisibility::Any;
///
///     let settings = RayCastSettings::default()
///         .with_filter(&filter)
///         .with_early_exit_test(&early_exit_test)
///         .with_visibility(visibility);
///
///     // Cast the ray with the settings, returning a list of intersections.
///     let hits = ray_cast.cast_ray(ray, &settings);
/// }
/// ```
#[derive(SystemParam)]
pub struct BvhMeshRayCast<'w, 's> {
    #[doc(hidden)]
    pub meshes: Res<'w, Assets<Mesh>>,
    #[cfg(feature = "bvh")]
    #[doc(hidden)]
    pub bvh_caches: Res<'w, AssetsBvhCaches<Mesh, BvhCache>>,
    #[cfg(feature = "obvhs")]
    #[doc(hidden)]
    pub obvhs_bvh2_caches: Res<'w, AssetsBvhCaches<Mesh, ObvhsBvh2Cache>>,
    #[doc(hidden)]
    pub picking_bvh_backend: Res<'w, PickingBvhBackend>,
    #[doc(hidden)]
    pub hits: Local<'s, Vec<(FloatOrd, (Entity, RayMeshHit))>>,
    #[doc(hidden)]
    pub output: Local<'s, Vec<(Entity, RayMeshHit)>>,
    #[doc(hidden)]
    pub culled_list: Local<'s, Vec<(FloatOrd, Entity)>>,
    #[doc(hidden)]
    pub culling_query: Query<
        'w,
        's,
        (
            Read<InheritedVisibility>,
            Read<ViewVisibility>,
            Read<Aabb>,
            Read<GlobalTransform>,
            Entity,
        ),
        MeshFilter,
    >,
    #[doc(hidden)]
    pub mesh_query: Query<
        'w,
        's,
        (
            Option<Read<Mesh2d>>,
            Option<Read<Mesh3d>>,
            Option<Read<SimplifiedMesh>>,
            Has<RayCastBackfaces>,
            Read<GlobalTransform>,
        ),
        MeshFilter,
    >,
}

impl<'w, 's> BvhMeshRayCast<'w, 's> {
    /// Casts the `ray` into the world and returns a sorted list of intersections, nearest first.
    pub fn cast_ray(&mut self, ray: Ray3d, settings: &RayCastSettings) -> &[(Entity, RayMeshHit)] {
        let ray_cull = info_span!("ray culling");
        let ray_cull_guard = ray_cull.enter();

        self.hits.clear();
        self.culled_list.clear();
        self.output.clear();

        // TODO: create a BVH cache for meshes also, useful if there is many meshes, but would need updating if they move/rotate

        // Check all entities to see if the ray intersects the AABB. Use this to build a short list
        // of entities that are in the path of the ray.
        let (aabb_hits_tx, aabb_hits_rx) = crossbeam_channel::unbounded::<(FloatOrd, Entity)>();
        let visibility_setting = settings.visibility;
        self.culling_query.par_iter().for_each(
            |(inherited_visibility, view_visibility, aabb, transform, entity)| {
                let should_ray_cast = match visibility_setting {
                    RayCastVisibility::Any => true,
                    RayCastVisibility::Visible => inherited_visibility.get(),
                    RayCastVisibility::VisibleInView => view_visibility.get(),
                };
                if should_ray_cast {
                    if let Some(distance) = ray_aabb_intersection_3d(
                        ray,
                        &Aabb3d::new(aabb.center, aabb.half_extents),
                        &transform.compute_matrix(),
                    ) {
                        aabb_hits_tx.send((FloatOrd(distance), entity)).ok();
                    }
                }
            },
        );
        *self.culled_list = aabb_hits_rx.try_iter().collect();

        // Sort by the distance along the ray.
        self.culled_list.sort_by_key(|(aabb_near, _)| *aabb_near);

        drop(ray_cull_guard);

        // Perform ray casts against the culled entities.
        let mut nearest_blocking_hit = FloatOrd(f32::INFINITY);
        let ray_cast_guard = debug_span!("ray_cast");
        self.culled_list
            .iter()
            .filter(|(_, entity)| (settings.filter)(*entity))
            .for_each(|(aabb_near, entity)| {
                // Get the mesh components and transform.
                let Ok((mesh2d, mesh3d, simplified_mesh, has_backfaces, transform)) =
                    self.mesh_query.get(*entity)
                else {
                    return;
                };

                // Get the underlying mesh handle. One of these will always be `Some` because of the query filters.
                let Some(mesh_handle) = simplified_mesh
                    .map(|m| &m.0)
                    .or(mesh3d.map(|m| &m.0).or(mesh2d.map(|m| &m.0)))
                else {
                    return;
                };

                // Is it even possible the mesh could be closer than the current best?
                if *aabb_near > nearest_blocking_hit {
                    return;
                }

                // Does the mesh handle resolve?
                let Some(mesh) = self.meshes.get(mesh_handle) else {
                    return;
                };

                // Backfaces of 2d meshes are never culled, unlike 3d mehses.
                let backfaces = match (has_backfaces, mesh2d.is_some()) {
                    (false, false) => Backfaces::Cull,
                    _ => Backfaces::Include,
                };

                // Perform the actual ray cast.
                let _ray_cast_guard = ray_cast_guard.enter();
                let transform = transform.compute_matrix();

                let intersection = match self.picking_bvh_backend.backend {
                    crate::BvhBackend::None => {
                        ray_intersection_over_mesh(mesh, &transform, ray, backfaces)
                    }
                    #[cfg(feature = "bvh")]
                    crate::BvhBackend::Bvh => {
                        let bvh_cache = self.bvh_caches.get(mesh_handle);
                        if let Some(bvh_cache) = bvh_cache {
                            ray_intersection_over_mesh_using_bvh_cache(
                                &transform, ray, backfaces, bvh_cache,
                            )
                        } else {
                            ray_intersection_over_mesh(mesh, &transform, ray, backfaces)
                        }
                    }
                    #[cfg(feature = "obvhs")]
                    crate::BvhBackend::ObvhsBvh2 => {
                        let obvhs_bvh2_cache = self.obvhs_bvh2_caches.get(mesh_handle);
                        if let Some(obvhs_bvh2_cache) = obvhs_bvh2_cache {
                            ray_intersection_over_mesh_using_obvhs_bvh2_cache(
                                &transform,
                                ray,
                                backfaces,
                                obvhs_bvh2_cache,
                            )
                        } else {
                            ray_intersection_over_mesh(mesh, &transform, ray, backfaces)
                        }
                    }
                };

                if let Some(intersection) = intersection {
                    let distance = FloatOrd(intersection.distance);
                    if (settings.early_exit_test)(*entity) && distance < nearest_blocking_hit {
                        // The reason we don't just return here is because right now we are
                        // going through the AABBs in order, but that doesn't mean that an
                        // AABB that starts further away can't end up with a closer hit than
                        // an AABB that starts closer. We need to keep checking AABBs that
                        // could possibly contain a nearer hit.
                        nearest_blocking_hit = distance.min(nearest_blocking_hit);
                    }
                    self.hits.push((distance, (*entity, intersection)));
                };
            });

        self.hits.retain(|(dist, _)| *dist <= nearest_blocking_hit);
        self.hits.sort_by_key(|(k, _)| *k);
        let hits = self.hits.iter().map(|(_, (e, i))| (*e, i.to_owned()));
        self.output.extend(hits);
        self.output.as_ref()
    }
}
