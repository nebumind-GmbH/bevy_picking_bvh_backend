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

By default, the benchmark uses the "dragon_high.glb" mesh upper. But you can easily change it in the `tests/bench.rs` file.

You can even add multiple meshes.

How it works ?

- First an app is initialized with the desired meshes, then the process waits for the app to be ready (meshes loaded and bvh caches generated).
- Then random rays are spawned from a random position on the aabb boundary of a randomly picked mesh to a random position on this mesh using the `UniformMeshSampler` from Bevy
- When the desired number of raycasts is reached, the loop exits and basic statistics are printed.
- Then the backend is changed in the same running app, and another bench is run.
- Each backends available on the selected features is benchmarked

1000 rays are spawned for the `None` backend (default mesh picking in Bevy 0.15), then 10000 rays for each other backends.

The result is a **1000x performance boost** for the `dragon_high.glb` mesh.
