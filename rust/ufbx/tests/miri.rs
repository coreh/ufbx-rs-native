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

// Public-boundary provenance regression: every one of these entry points takes
// a `&`-derived pointer (SharedReadOnly provenance) into memory the internal
// view machinery navigates. They must route through read-only `Const` views —
// an interior-mutable `Mut` view mint here retags SharedReadOnly ->
// SharedReadWrite and fails Stacked Borrows even though nothing is written
// (caught 2026-08-18; the corpus previously never called the safe find
// wrappers, so the class was invisible to CI).
#[test]
fn public_find_wrappers_from_shared_refs() {
    let root = load("maya_interpolation_modes_7500_binary.fbx");
    let scene: &ufbx::Scene = &root;
    let node: &ufbx::Node = &scene.root_node;
    let props: &ufbx::Props = &node.element.props;

    // Free find_prop: hit, miss, and the defaults-chain walk.
    let hit = ufbx::find_prop(props, "Lcl Translation");
    let miss = ufbx::find_prop(props, "DoesNotExist");
    assert!(miss.is_none());
    let mut acc = hit.map_or(0.0, |p| p.value_vec4.x as f64);

    // Inherent method form and the value adapter.
    if let Some(p) = props.find_prop("Lcl Scaling") {
        acc += p.value_vec4.y as f64;
    }
    if let Some(p) = hit {
        // Exercises the value-adapter shim path (`ufbx_find_blob_len`).
        let blob = ufbx::find_blob(props, "Lcl Rotation", p.value_blob);
        acc += blob.size as f64;
    }

    // Element-rooted paths (api.rs public-boundary roots).
    let elem: &ufbx::Element = &node.element;
    let _ = ufbx::find_prop_element(elem, "Lcl Translation", ufbx::ElementType::Node);

    // Anim-eval path: evaluate_prop_flags reads props through the same roots,
    // and evaluate_props builds a stack `Props` whose defaults chain crosses
    // back into the scene arena — find_prop must walk it read-only.
    if let Some(stack) = scene.anim_stacks.first() {
        let anim: &ufbx::Anim = &stack.anim;
        let prop = ufbx::evaluate_prop_flags(anim, elem, "Lcl Translation", 0.1, 0);
        acc += prop.value_vec4.x as f64;
    }

    for material in &scene.materials {
        let _ = ufbx::find_prop_texture(material, "DiffuseColor");
    }
    assert!(acc.is_finite());
}

// Public-boundary provenance regression #2 — downcast widening: `as_mesh(&e)`
// takes a reference whose retag covers only the `Element` header and returns a
// reference to the FULL containing struct. The native `as_*` family must
// reconstitute a wide pointer via the arena allocation's exposed provenance
// (allocator.rs `expose_provenance`); the naive `element as *mut Mesh` cast
// keeps the narrowed range and reading any tail field is Stacked Borrows UB
// (caught 2026-08-18; Tree Borrows accepts it, so SB is the load-bearing gate).
#[test]
fn public_downcasts_from_narrowed_element_refs() {
    let root = load("maya_interpolation_modes_7500_binary.fbx");
    let scene: &ufbx::Scene = &root;

    let mut acc = 0.0f64;
    for elem in &scene.elements {
        let e: &ufbx::Element = elem;
        if let Some(mesh) = ufbx::as_mesh(e) {
            acc += mesh.num_vertices as f64; // tail read, outside &Element's range
        }
        if let Some(node) = ufbx::as_node(e) {
            acc += node.local_transform.translation.x as f64;
        }
        if let Some(layer) = ufbx::as_anim_layer(e) {
            acc += layer.anim_values.len() as f64;
        }
        if let Some(stack) = ufbx::as_anim_stack(e) {
            acc += stack.time_end;
        }
    }
    assert!(acc.is_finite());
}

// -- Public-API provenance sweep
//
// Calls every remaining `&T`-taking public wrapper family from shared
// references rooted in the frozen scene, so the Miri SB leg checks the
// boundary provenance of each cluster instead of us reasoning about it
// (the find-family and as_* downcast regressions above were both found
// empirically, not by inspection). Values are folded into an accumulator so
// nothing is optimized away.
//
// Not coverable from safe code (upstream signature quirks, noted here so the
// gap is deliberate): `find_baked_node`/`find_baked_element`(+`_by_*_id`)
// take `&mut BakedAnim` (no `DerefMut` on `BakedAnimRoot`), `find_face_index`
// takes `&mut Mesh`, and `evaluate_props` needs a `&mut [ExternalRef<Prop>]`
// buffer that safe code cannot construct (`ExternalRef::new` is unsafe and
// `Prop` has no public constructor).

/// DOM retention: `dom_find` navigation and the typed `dom_as_*` array reads,
/// all through `&DomNode`s reached from the scene's retained DOM root.
#[test]
fn public_dom_walkers_from_shared_refs() {
    let data = read_data("maya_cube_7500_ascii.fbx");
    let opts = LoadOpts {
        retain_dom: true,
        ..Default::default()
    };
    let root = ufbx::load_memory(&data, opts).expect("load with retain_dom");
    let scene: &Scene = &root;
    let dom_root: &ufbx::DomNode = scene.dom_root.as_ref().expect("dom_root retained").as_ref();

    let mut acc = 0.0f64;
    let objects = ufbx::dom_find(dom_root, "Objects").expect("Objects dom node");
    for child in &objects.children {
        acc += child.name.as_ref().len() as f64;
        if ufbx::dom_is_array(child) {
            acc += ufbx::dom_array_size(child) as f64;
        }
        // The typed list readers return empty lists on type mismatch, so
        // calling each on every node is safe and cheap.
        acc += ufbx::dom_as_int32_list(child).len() as f64;
        acc += ufbx::dom_as_int64_list(child).len() as f64;
        acc += ufbx::dom_as_float_list(child).len() as f64;
        acc += ufbx::dom_as_double_list(child).len() as f64;
        acc += ufbx::dom_as_real_list(child).len() as f64;
        acc += ufbx::dom_as_blob_list(child)
            .iter()
            .map(|b| b.size)
            .sum::<usize>() as f64;
        // One per node is enough for provenance coverage.
        if let Some(grandchild) = child.children.as_ref().first() {
            if let Some(found) = ufbx::dom_find(child, grandchild.name.as_ref()) {
                acc += found.values.len() as f64;
            }
        }
    }
    assert!(acc.is_finite());
}

/// Animation evaluation from shared refs: per-value and per-prop evaluators,
/// transform evaluation, the anim-prop finders, baked-keyframe interpolation,
/// and the matrix/transform value helpers fed from scene data.
#[test]
fn public_anim_eval_from_shared_refs() {
    let root = load("maya_interpolation_modes_7500_binary.fbx");
    let scene: &Scene = &root;
    let anim: &ufbx::Anim = &scene.anim;
    let mut acc = 0.0f64;

    for curve in &scene.anim_curves {
        acc += ufbx::evaluate_curve_flags(curve, 0.4, 0.0, 0) as f64;
    }
    for layer in &scene.anim_layers {
        for value in &layer.anim_values {
            acc += ufbx::evaluate_anim_value_real(value, 0.2) as f64;
            acc += ufbx::evaluate_anim_value_vec3(value, 0.2).x as f64;
            acc += ufbx::evaluate_anim_value_real_flags(value, 0.3, 0) as f64;
            acc += ufbx::evaluate_anim_value_vec3_flags(value, 0.3, 0).y as f64;
        }
        // Anim-prop finders navigate the layer's sorted prop table.
        for prop in layer.anim_props.as_ref().iter().take(2) {
            let elem: &ufbx::Element = prop.element.as_ref();
            acc += ufbx::find_anim_props(layer, elem).len() as f64;
            if let Some(found) = layer.find_anim_prop(elem, prop.prop_name.as_ref()) {
                acc += found.prop_name.as_ref().len() as f64;
            }
        }
    }
    if let Some(stack) = ufbx::find_anim_stack(scene, "Take 001") {
        acc += stack.time_end;
    }

    for node in scene.nodes.as_ref().iter().take(3) {
        // evaluate_transform(_flags) from &Node: the stack `Props` these seed
        // carries the caller's read-only provenance in its `defaults` chain, so
        // the internal find family must walk it through `Const` views (the SB
        // UB this once was is the regression this exercises).
        let t = ufbx::evaluate_transform(anim, node, 0.25);
        acc += t.translation.x as f64;
        acc += ufbx::evaluate_transform_flags(anim, node, 0.25, 0).scale.y as f64;
        let t = node.local_transform;

        // Value helpers on data read from the scene.
        let m = ufbx::transform_to_matrix(&t);
        let back = ufbx::matrix_to_transform(&m);
        acc += back.rotation.w as f64;
        let prod = ufbx::matrix_mul(&m, &node.node_to_world);
        acc += ufbx::matrix_determinant(&prod) as f64;
        let inv = ufbx::matrix_invert(&m);
        acc += inv.m03 as f64;
        acc += ufbx::get_compatible_matrix_for_normals(node).m00 as f64;

        // Per-prop evaluation through &Anim + &Element.
        let elem: &ufbx::Element = &node.element;
        let prop = ufbx::evaluate_prop(anim, elem, "Lcl Translation", 0.25);
        acc += prop.value_vec4.x as f64;
    }

    // Baked-keyframe interpolation over slices from a baked result.
    let baked = ufbx::bake_anim(scene, anim, Default::default()).expect("bake_anim");
    for node in &baked.nodes {
        acc += ufbx::evaluate_baked_vec3(node.translation_keys.as_ref(), 0.3).x as f64;
        acc += ufbx::evaluate_baked_quat(node.rotation_keys.as_ref(), 0.3).w as f64;
    }
    assert!(acc.is_finite());
}

/// Mesh geometry helpers from shared refs: indexed vertex-attribute getters,
/// face normals, and topology edge navigation plus normal-mapping generation
/// over a caller-provided topology buffer.
#[test]
fn public_mesh_helpers_from_shared_refs() {
    let root = load("maya_cube_7500_binary.fbx");
    let scene: &Scene = &root;
    let mesh = scene.meshes.first().expect("cube mesh");
    let mut acc = 0.0f64;

    for i in 0..mesh.num_indices.min(8) {
        acc += ufbx::get_vertex_vec3(&mesh.vertex_position, i).x as f64;
        acc += ufbx::get_vertex_w_vec3(&mesh.vertex_position, i) as f64;
        if !mesh.vertex_uv.indices.is_empty() {
            acc += ufbx::get_vertex_vec2(&mesh.vertex_uv, i).x as f64;
        }
        if !mesh.vertex_crease.indices.is_empty() {
            acc += ufbx::get_vertex_real(&mesh.vertex_crease, i) as f64;
        }
        if !mesh.vertex_color.indices.is_empty() {
            acc += ufbx::get_vertex_vec4(&mesh.vertex_color, i).w as f64;
        }
    }
    for face in &mesh.faces {
        acc += ufbx::get_weighted_face_normal(&mesh.vertex_position, *face).y as f64;
    }

    let mut topo = vec![ufbx::TopoEdge::default(); mesh.num_indices];
    ufbx::compute_topology(mesh, &mut topo);
    acc += ufbx::topo_next_vertex_edge(&topo, 0) as f64;
    acc += ufbx::topo_prev_vertex_edge(&topo, 0) as f64;
    let mut normal_indices = vec![0u32; mesh.num_indices];
    acc += ufbx::generate_normal_mapping(mesh, &topo, &mut normal_indices, false) as f64;

    assert!(acc.is_finite());
}

/// Deformer helpers from shared refs: skin vertex matrices, blend-shape
/// offset lookups, blend-weight evaluation and bind-pose queries.
#[test]
fn public_deform_helpers_from_shared_refs() {
    let mut acc = 0.0f64;

    let root = load("blender_293_half_skinned_7400_binary.fbx");
    let scene: &Scene = &root;
    let identity = ufbx::Matrix::default();
    for skin in &scene.skin_deformers {
        for vertex in 0..skin.vertices.len().min(4) {
            acc += ufbx::get_skin_vertex_matrix(skin, vertex, &identity).m00 as f64;
        }
    }
    for pose in &scene.poses {
        for bone_pose in pose.bone_poses.as_ref().iter().take(2) {
            let node: &ufbx::Node = bone_pose.bone_node.as_ref();
            if let Some(found) = ufbx::get_bone_pose(pose, node) {
                acc += found.bone_to_world.m00 as f64;
            }
        }
    }

    let root = load("blender_279_shape_weights_7400_binary.fbx");
    let scene: &Scene = &root;
    let anim: &ufbx::Anim = &scene.anim;
    for deformer in &scene.blend_deformers {
        for vertex in 0..4 {
            acc += ufbx::get_blend_vertex_offset(deformer, vertex).x as f64;
        }
        for channel in &deformer.channels {
            acc += ufbx::evaluate_blend_weight(anim, channel, 0.5) as f64;
            acc += ufbx::evaluate_blend_weight_flags(anim, channel, 0.5, 0) as f64;
            for shape in &channel.keyframes {
                let s: &ufbx::BlendShape = shape.shape.as_ref();
                acc += ufbx::get_blend_shape_offset_index(s, 0) as f64;
                acc += ufbx::get_blend_shape_vertex_offset(s, 0).x as f64;
            }
        }
    }
    assert!(acc.is_finite());
}
