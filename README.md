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
