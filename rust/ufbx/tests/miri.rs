// Small end-to-end loads through the public safe API, sized to run under Miri
// (`cargo +nightly miri test --test miri`) while staying cheap enough to also
// run as ordinary tests. Each test loads one small file and walks the scene so
// the pool allocations, string interning and result buffers are actually read
// back, not just allocated.
//
// The loads go through `load_memory` rather than `load_file`: the default file
// stream calls libc `fopen`, which Miri cannot emulate (`unsupported
// operation: can't call foreign function `fopen``). Reading the bytes with
// `std::fs` first keeps the whole parse/allocate/free path under Miri's
// checker. `load_file` itself is still covered by a `cfg(not(miri))` test.
//
// Miri needs `-Zmiri-disable-isolation` for the `std::fs` reads.

use ufbx::{LoadOpts, Scene};

fn data_path(name: &str) -> std::string::String {
    format!("{}/../../data/{}", env!("CARGO_MANIFEST_DIR"), name)
}

fn read_data(name: &str) -> Vec<u8> {
    std::fs::read(data_path(name)).unwrap_or_else(|e| panic!("failed to read {}: {}", name, e))
}

/// Walk the scene: names, node transforms and mesh vertex/index data. Returns a
/// checksum-ish accumulator so nothing gets optimized away.
fn walk(scene: &Scene) -> f64 {
    let mut acc = 0.0f64;

    for node in &scene.nodes {
        acc += node.element.name.as_ref().len() as f64;
        acc += node.local_transform.translation.x as f64;
        acc += node.node_to_world.m00 as f64;
        acc += node.children.len() as f64;
    }

    for mesh in &scene.meshes {
        acc += mesh.num_vertices as f64;
        for v in &mesh.vertices {
            acc += v.x as f64 + v.y as f64 + v.z as f64;
        }
        for &i in &mesh.vertex_indices {
            acc += i as f64;
        }
        for face in &mesh.faces {
            acc += face.index_begin as f64 + face.num_indices as f64;
        }
        // Indexed vertex attributes (the `ufbx_vertex_*` accessor pattern).
        for i in 0..mesh.num_indices {
            let p = mesh.vertex_position[i];
            acc += p.x as f64 + p.y as f64 + p.z as f64;
        }
    }

    for material in &scene.materials {
        acc += material.element.name.as_ref().len() as f64;
    }

    acc
}

fn load_and_walk(name: &str) -> f64 {
    let data = read_data(name);
    let scene = ufbx::load_memory(&data, LoadOpts::default())
        .unwrap_or_else(|e| panic!("failed to load {}: {:?}", name, e));
    assert!(scene.nodes.len() > 0, "{} has no nodes", name);
    walk(&scene)
}

#[test]
fn load_cube_binary() {
    let acc = load_and_walk("maya_cube_7500_binary.fbx");
    assert!(acc.is_finite());
}

#[test]
fn load_cube_ascii() {
    let acc = load_and_walk("maya_cube_7500_ascii.fbx");
    assert!(acc.is_finite());
}

#[test]
fn load_obj() {
    let acc = load_and_walk("blender_279_default.obj");
    assert!(acc.is_finite());
}

// `load_file` goes through the libc stdio stream, which Miri cannot execute.
#[test]
#[cfg(not(miri))]
fn load_cube_binary_from_file() {
    let scene = ufbx::load_file(&data_path("maya_cube_7500_binary.fbx"), LoadOpts::default())
        .expect("failed to load maya_cube_7500_binary.fbx");
    assert!(scene.nodes.len() > 0);
    assert!(walk(&scene).is_finite());
}
