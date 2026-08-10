// End-to-end loads through the public safe API, sized to run under Miri
// (`cargo +nightly miri test --test miri`) while staying cheap enough to also
// run as ordinary tests.
//
// The corpus is chosen for FEATURE coverage, not file size: Miri's cost tracks
// bytes inflated (a 740 KB DEFLATE-heavy scene costs ~30s; every file here is
// under 80 KB and costs well under a second), while the paths worth checking —
// NURBS tessellation, subdivision, skinning, animation evaluation and baking,
// topology and index generation — are reachable from tiny scenes. Loading alone
// is not enough: the post-load APIs allocate and walk their own buffers, so
// each is exercised on a scene that actually has the relevant elements.
//
// The loads go through `load_memory` rather than `load_file`: the default file
// stream calls libc `fopen`, which Miri cannot emulate (`unsupported
// operation: can't call foreign function `fopen``). Reading the bytes with
// `std::fs` first keeps the whole parse/allocate/free path under Miri's
// checker. `load_file` itself is still covered by a `cfg(not(miri))` test.
//
// NOT covered here: geometry caches (`ufbx_load_geometry_cache` and the
// `load_external_files` path resolve sibling files through the same libc
// stream) and the threaded loader. Both need a Miri-compatible file callback;
// they stay on the C-suite and hash oracles until then.
//
// Miri needs `-Zmiri-disable-isolation` for the `std::fs` reads.

// These sums keep the accumulator in f64: the `.x as f64` etc. casts are no-ops
// under the default `Real = f64` but load-bearing f32->f64 widenings under `real-is-f32`.
// (The crate-internal `as_f64!` macro is not reachable from an integration test.)
#![allow(clippy::unnecessary_cast)]

use ufbx::{LoadOpts, Scene};

fn data_path(name: &str) -> std::string::String {
    format!("{}/../../data/{}", env!("CARGO_MANIFEST_DIR"), name)
}

fn read_data(name: &str) -> Vec<u8> {
    std::fs::read(data_path(name)).unwrap_or_else(|e| panic!("failed to read {}: {}", name, e))
}

fn load(name: &str) -> ufbx::SceneRoot {
    let data = read_data(name);
    ufbx::load_memory(&data, LoadOpts::default())
        .unwrap_or_else(|e| panic!("failed to load {}: {:?}", name, e))
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
        // Attribute layers that only some exporters emit.
        for set in &mesh.uv_sets {
            for i in 0..set.vertex_uv.indices.len() {
                let uv = set.vertex_uv[i];
                acc += uv.x as f64 + uv.y as f64;
            }
        }
        for set in &mesh.color_sets {
            for i in 0..set.vertex_color.indices.len() {
                acc += set.vertex_color[i].w as f64;
            }
        }
    }

    for material in &scene.materials {
        acc += material.element.name.as_ref().len() as f64;
    }

    acc
}

fn load_and_walk(name: &str) -> f64 {
    let scene = load(name);
    assert!(!scene.nodes.is_empty(), "{} has no nodes", name);
    walk(&scene)
}

// -- Container formats and FBX versions
//
// The parsers are separate implementations (binary reader, ASCII tokenizer,
// OBJ/MTL reader) and the pre-7000 versions take the legacy `Takes` path, so
// each combination is its own load.

#[test]
fn load_cube_binary() {
    assert!(load_and_walk("maya_cube_7500_binary.fbx").is_finite());
}

#[test]
fn load_cube_ascii() {
    assert!(load_and_walk("maya_cube_7500_ascii.fbx").is_finite());
}

#[test]
fn load_obj() {
    assert!(load_and_walk("blender_279_default.obj").is_finite());
}

/// Pre-7000 binary: legacy `Takes` animation and the 6100-era element layout.
#[test]
fn load_legacy_6100_ascii() {
    assert!(load_and_walk("blender_279_ball_6100_ascii.fbx").is_finite());
}

/// Legacy binary with one of nearly every node attribute type, and enough
/// compressed array data to drive the inflate fast path's 16-byte match copies
/// — including the deliberate over-read past the write cursor, which is how the
/// `copy_16_bytes` uninitialized-read bug surfaced. Miri's cost tracks inflated
/// bytes, so this is deliberately a mid-sized file rather than the largest one.
#[test]
fn load_attribute_zoo_6100_binary() {
    assert!(load_and_walk("maya_node_attribute_zoo_6100_binary.fbx").is_finite());
}

// -- Scene features exercised on load

/// Skin deformers: clusters, weights and the bind-pose matrices.
#[test]
fn load_skinned() {
    let scene = load("blender_293_half_skinned_7400_binary.fbx");
    assert!(!scene.skin_deformers.is_empty());
    let mut acc = 0.0f64;
    for skin in &scene.skin_deformers {
        for cluster in &skin.clusters {
            acc += cluster.weights.len() as f64;
            for w in &cluster.weights {
                acc += *w as f64;
            }
        }
        for i in 0..skin.vertices.len() {
            let v = skin.vertices[i];
            acc += v.num_weights as f64;
        }
    }
    assert!(acc.is_finite());
}

/// Blend shapes: channels, keyframes and per-shape offsets.
#[test]
fn load_blend_shapes() {
    let scene = load("blender_279_shape_weights_7400_binary.fbx");
    assert!(!scene.blend_deformers.is_empty());
    let mut acc = 0.0f64;
    for deformer in &scene.blend_deformers {
        for channel in &deformer.channels {
            acc += channel.weight as f64;
            for shape in &channel.keyframes {
                acc += shape.target_weight as f64;
                acc += shape.shape.num_offsets as f64;
            }
        }
    }
    assert!(acc.is_finite());
}

/// Instancing: one attribute shared by several nodes (element/node back-refs).
#[test]
fn load_instancing() {
    let scene = load("blender_293_instancing_7400_binary.fbx");
    let mut acc = 0.0f64;
    for mesh in &scene.meshes {
        acc += mesh.element.instances.len() as f64;
    }
    assert!(acc > 0.0);
}

/// Embedded textures: the base64/binary blob path and video elements.
#[test]
fn load_embedded_textures() {
    let scene = load("blender_279_internal_textures_7400_binary.fbx");
    let mut acc = 0.0f64;
    for texture in &scene.textures {
        acc += texture.content.len() as f64;
        acc += texture.filename.as_ref().len() as f64;
    }
    for video in &scene.videos {
        acc += video.content.len() as f64;
    }
    assert!(acc.is_finite());
}

// -- Post-load APIs
//
// Each allocates its own result buffers through the same allocator machinery as
// a load, so they need to be visited separately from any scene walk.

/// Curve evaluation, whole-scene evaluation and baking, plus the interpolation
/// modes that make the curve evaluator take every branch.
#[test]
fn evaluate_and_bake_animation() {
    let scene = load("maya_interpolation_modes_7500_binary.fbx");
    assert!(!scene.anim_stacks.is_empty());

    let mut acc = 0.0f64;
    for curve in &scene.anim_curves {
        for t in [0.0, 0.25, 0.5, 1.0, 100.0] {
            acc += ufbx::evaluate_curve(curve, t, 0.0) as f64;
        }
    }

    // Whole-scene evaluation: allocates and populates a second scene.
    let anim = &scene.anim;
    let evaluated =
        ufbx::evaluate_scene(&scene, anim, 0.5, Default::default()).expect("evaluate_scene failed");
    acc += walk(&evaluated);

    // Baking: resamples every animated property into keyframe lists.
    let baked = ufbx::bake_anim(&scene, anim, Default::default()).expect("bake_anim failed");
    for node in &baked.nodes {
        for key in &node.translation_keys {
            acc += key.value.x as f64;
        }
        for key in &node.rotation_keys {
            acc += key.value.w as f64;
        }
    }
    assert!(acc.is_finite());
}

/// NURBS: basis evaluation and curve tessellation into a line curve.
#[test]
fn tessellate_nurbs_curve() {
    let scene = load("maya_nurbs_curve_form_7700_binary.fbx");
    assert!(!scene.nurbs_curves.is_empty());
    let mut acc = 0.0f64;
    for curve in &scene.nurbs_curves {
        for u in [0.0, 0.5, 1.0] {
            acc += ufbx::evaluate_nurbs_curve(curve, u).position.x as f64;
        }
        let line = ufbx::tessellate_nurbs_curve(curve, Default::default())
            .expect("tessellate_nurbs_curve failed");
        acc += line.control_points.len() as f64;
        for &i in &line.point_indices {
            acc += i as f64;
        }
    }
    assert!(acc.is_finite());
}

/// NURBS surfaces tessellate into a full mesh (a different allocator path from
/// the curve case).
#[test]
fn tessellate_nurbs_surface() {
    let scene = load("maya_nurbs_surface_plane_6100_ascii.fbx");
    assert!(!scene.nurbs_surfaces.is_empty());
    let mut acc = 0.0f64;
    for surface in &scene.nurbs_surfaces {
        let mesh = ufbx::tessellate_nurbs_surface(surface, Default::default())
            .expect("tessellate_nurbs_surface failed");
        acc += walk_mesh(&mesh);
    }
    assert!(acc.is_finite());
}

/// Catmull-Clark subdivision: the heaviest derived-geometry path. Subdividing
/// a boundary-edge mesh two levels covers the interior, boundary and corner
/// rules on a mesh small enough for the interpreter.
#[test]
fn subdivide() {
    let scene = load("blender_293x_subsurf_boundary_7400_binary.fbx");
    let mesh = scene.meshes.first().expect("no mesh");
    let subdivided =
        ufbx::subdivide_mesh(mesh, 2, Default::default()).expect("subdivide_mesh failed");
    assert!(subdivided.num_faces > mesh.num_faces);
    assert!(walk_mesh(&subdivided).is_finite());
}

/// Topology, triangulation and index generation: the three helper APIs that
/// write into caller-provided buffers.
#[test]
fn topology_triangulate_and_index_generation() {
    let scene = load("blender_293_ngon_subsurf_7400_binary.fbx");
    let mesh = scene.meshes.first().expect("no mesh");

    let mut topo = vec![ufbx::TopoEdge::default(); mesh.num_indices];
    ufbx::compute_topology(mesh, &mut topo);
    let mut acc = 0.0f64;
    for edge in &topo {
        acc += edge.index as f64 + edge.next as f64;
    }

    let mut indices = vec![0u32; mesh.max_face_triangles * 3];
    for face in &mesh.faces {
        let tris = ufbx::triangulate_face(&mut indices, mesh, *face);
        acc += tris as f64;
    }

    // Deduplicate a position stream back into an indexed mesh.
    let mut positions: Vec<ufbx::Vec3> = (0..mesh.num_indices)
        .map(|i| mesh.vertex_position[i])
        .collect();
    let mut out_indices = vec![0u32; mesh.num_indices];
    let mut streams = [ufbx::VertexStream::new(&mut positions)];
    let count = ufbx::generate_indices(&mut streams, &mut out_indices, Default::default())
        .expect("generate_indices failed");
    assert!(count > 0);
    assert!(acc.is_finite());
}

fn walk_mesh(mesh: &ufbx::Mesh) -> f64 {
    let mut acc = mesh.num_faces as f64;
    for i in 0..mesh.num_indices {
        let p = mesh.vertex_position[i];
        acc += p.x as f64 + p.y as f64 + p.z as f64;
    }
    acc
}

// `load_file` goes through the libc stdio stream, which Miri cannot execute.
#[test]
#[cfg(not(miri))]
fn load_cube_binary_from_file() {
    let scene = ufbx::load_file(&data_path("maya_cube_7500_binary.fbx"), LoadOpts::default())
        .expect("failed to load maya_cube_7500_binary.fbx");
    assert!(!scene.nodes.is_empty());
    assert!(walk(&scene).is_finite());
}
