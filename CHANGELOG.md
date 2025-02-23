# Release Notes

## Unreleased

### Changes

- split of `PickingBvhBackend` plugin (for just generating BVH Cache) and move mesh picking logic in `MeshPickingBvhPlugin` plugin (as for `MeshPickingPlugin` in Bevy 0.15.2)
- add a resource to monitor the computing state of BVH cache
- remove logging and use spans instead
- rename `MeshRayCast` to `BvhMeshRayCast` to prevent name conflicts with Bevy's implementation
- make backend public in `PickingBvhBackend` resource to allow user to change it if needed
- add a benchmark test

### Thanks

- @Driffin and @derekt for their suggestions for developing the benchmark test

## 0.1.0

### Features

- initial implementation of `PickingBvhBackend` to act as a drop-in replacement of `MeshPickingPlugin` plugin.
- `None` backend implementation (use same algorithm as `MeshPickingPlugin` in Bevy 0.15.2)
- `ObvhsBvh2` backend implementation: building of BVH cache asynchronously and use it if available, fallback to `None` if cache is not ready

  Uses https://github.com/dgriffin91/obvhs crate and `BVH2` algorithm.
- `Bvh` backend implementation: building of BVH cache asynchronously and use it if available, fallback to `None` if cache is not ready

  Uses https://github.com/svenstaro/bvh crate.

### Thanks

- @DGriffin91 (github) for the powerful https://github.com/dgriffin91/obvhs lib and his help
- @svenstaro (github) for the powerful https://github.com/svenstaro/bvh lib
- @bestRanar and @derekt (Discord) for the suggestion of using Obvhs lib and their help
