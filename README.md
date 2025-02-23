# Bevy Picking Bvh Backend

This plugins is a drop-in replacement of MeshPickingPlugin, reusing most of its components, to provider better performance when raycasting through a (very) large mesh.

To try the example, just run :

```bash
cargo run --example simple --release
```

For large meshes, it is recommended to use release mode, because building of the BVH tree can take some time

## Large mesh example

Download the following mesh https://github.com/pettett/dragon_high using the following commands:

```bash
mkdir -p assets/models
wget https://github.com/pettett/dragon_high/raw/refs/heads/main/dragon_high.glb -O assets/models/dragon_high.glb
```

Then run the `dragon_high` sample:

```bash
cargo run --example dragon_high --release
```

## Running the benchmark

After downloaded "dragon_high.glb" (see above), run the following command:

```
cargo test --test bench --all-features
```

The output will look-likes this:

```
running 1 test
2025-02-23T17:31:19.780600Z  INFO bevy_render::renderer: AdapterInfo { name: "NVIDIA GeForce RTX 2070 SUPER", vendor: 4318, device: 7812, device_type: DiscreteGpu, driver: "NVIDIA", driver_info: "560.35.05", backend: Vulkan }
2025-02-23T17:31:20.028540Z  INFO bench: --- Preparing app for benchmarks
2025-02-23T17:31:21.662793Z  INFO bench: --- App ready for benchmarks
2025-02-23T17:31:21.662810Z  INFO bench: --- Benchmark with backend None and 1000 raycasts
2025-02-23T17:31:56.441425Z  INFO bench: Spawned 1000 rays with 942 hits
2025-02-23T17:31:56.441446Z  INFO bench: Average raycast time: 33476898ns
2025-02-23T17:31:56.441451Z  INFO bench: --- Benchmark with backend ObvhsBvh2 and 10000 raycasts
2025-02-23T17:32:05.209912Z  INFO bench: Spawned 10000 rays with 9261 hits
2025-02-23T17:32:05.209930Z  INFO bench: Average raycast time: 31237ns
2025-02-23T17:32:05.209934Z  INFO bench: --- Benchmark with backend Bvh and 10000 raycasts
2025-02-23T17:32:13.603344Z  INFO bench: Spawned 10000 rays with 9266 hits
2025-02-23T17:32:13.603360Z  INFO bench: Average raycast time: 35731ns
test run_bench ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 54.22s
```

But you can easily change the mesh in the `tests/bench.rs` file.

You can even add multiple meshes.

Performance boost: **1000x**.

On meshes with more triangles, the performance boost is increased again.

This is normal because the initial implementation of Bevy Mesh Picking does not use any optimized structure, and it is stated in the release notes that a BVH structure would be used.

See https://bevyengine.org/news/bevy-0-15/#entity-picking-selection

### How the benchmark works ?

- First an app is initialized with the desired meshes, then the process waits for the app to be ready (meshes loaded and bvh caches generated).
- Then random rays are spawned from a random position on the aabb boundary of a randomly picked mesh to a random position on this mesh using the `UniformMeshSampler` from Bevy
- When the desired number of raycasts is reached, the loop exits and basic statistics are printed.
- Then the backend is changed in the same running app, and another bench is run.
- Each backends available on the selected features is benchmarked

1000 rays are spawned for the `None` backend (default mesh picking in Bevy 0.15), then 10000 rays for each other backends.

The result is a **1000x performance boost** for the `dragon_high.glb` mesh.
