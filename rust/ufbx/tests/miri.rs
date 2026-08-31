// End-to-end loads through the public safe API, sized to run under Miri
// (`cargo +nightly miri test --test miri`) while staying cheap enough to also
// run as ordinary tests.
//
// The corpus is chosen for FEATURE coverage, not file size: Miri's cost tracks
// bytes inflated (a 740 KB DEFLATE-heavy scene costs ~30s), while the paths
// worth checking — NURBS tessellation, subdivision, skinning, animation
// evaluation and baking, topology and index generation — are reachable from
// compact scenes. Loading alone is not enough: the post-load APIs allocate and
// walk their own buffers, so each is exercised on a scene that actually has the
// relevant elements.
//
// The loads go through `load_memory` rather than `load_file`: the default file
// stream calls libc `fopen`, which Miri cannot emulate (`unsupported
// operation: can't call foreign function `fopen``). Reading the bytes with
// `std::fs` first keeps the whole parse/allocate/free path under Miri's
// checker. `load_file` itself is still covered by a `cfg(not(miri))` test.
//
// Geometry caches (`ufbx_load_geometry_cache`, the `load_external_files`
// path and cache sampling) open sibling files through `open_file_cb`; the
// tests below install a callback that reads each file with `std::fs` and hands
// ufbx a memory-backed `Stream`, so those paths run under Miri too. The
// threaded loader is driven by a `std::thread` pool implementing the raw
// `ufbx_thread_pool` interface, which puts the ASCII array tasks and the
// binary DEFLATE tasks under Miri's data-race detector.
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
    let original = load("maya_cube_7500_binary.fbx");
    let retained = original.clone();
    drop(original);

    assert!(!retained.nodes.is_empty(), "retained scene has no nodes");
    assert!(walk(&retained).is_finite());
    drop(retained);
}

#[test]
fn load_cube_with_axis_conversion() {
    let data = read_data("maya_cube_7500_binary.fbx");
    let opts = LoadOpts {
        target_axes: ufbx::CoordinateAxes::left_handed_y_up(),
        handedness_conversion_axis: ufbx::MirrorAxis::X,
        ..Default::default()
    };
    let scene = ufbx::load_memory(&data, opts).expect("axis-converted load should succeed");
    assert!(walk(&scene).is_finite());
    assert_eq!(
        scene.metadata.handedness_conversion_axis,
        ufbx::MirrorAxis::X
    );
}

#[test]
fn load_cube_ascii() {
    assert!(load_and_walk("maya_cube_7500_ascii.fbx").is_finite());
}

/// A user-supplied boxed allocator drives every temp/result allocation through
/// the `allocator_imp_*` C-ABI callbacks, whose `user` pointer is the
/// `Box<Box<dyn AllocatorInterface>>` leaked by `Allocator::to_raw_mut`. This
/// covers the by-value leak: matching the variant through `&mut self` instead
/// leaks a one-word `&mut` pointee that the callbacks reinterpret as a
/// two-word fat box (out-of-bounds read, garbage vtable).
#[test]
fn load_with_boxed_allocator() {
    use std::alloc::{GlobalAlloc, Layout, System};

    struct CountingAllocator {
        allocs: usize,
    }
    impl ufbx::AllocatorInterface for CountingAllocator {
        fn alloc(&mut self, layout: Layout) -> *mut u8 {
            self.allocs += 1;
            // SAFETY: ufbx requests non-zero sizes, satisfying `alloc`.
            unsafe { System.alloc(layout) }
        }
        fn realloc(&mut self, ptr: *mut u8, old_layout: Layout, new_layout: Layout) -> *mut u8 {
            self.allocs += 1;
            // SAFETY: `ptr` came from this allocator with `old_layout`.
            unsafe { System.realloc(ptr, old_layout, new_layout.size()) }
        }
        fn free(&mut self, ptr: *mut u8, layout: Layout) {
            // SAFETY: `ptr` came from this allocator with `layout`.
            unsafe { System.dealloc(ptr, layout) }
        }
    }

    let data = read_data("maya_cube_7500_binary.fbx");
    let mut opts = LoadOpts::default();
    opts.temp_allocator.allocator = ufbx::Allocator::Box(Box::new(CountingAllocator { allocs: 0 }));
    opts.result_allocator.allocator =
        ufbx::Allocator::Box(Box::new(CountingAllocator { allocs: 0 }));
    let scene = ufbx::load_memory(&data, opts).expect("boxed-allocator load should succeed");
    assert!(walk(&scene).is_finite());
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
    let scene = load("maya_node_attribute_zoo_6100_binary.fbx");
    assert!(walk(&scene).is_finite());
    let stereo = scene
        .stereo_cameras
        .first()
        .expect("attribute zoo has no stereo camera");
    let left = stereo
        .left
        .as_ref()
        .expect("stereo camera has no left camera");
    let right = stereo
        .right
        .as_ref()
        .expect("stereo camera has no right camera");
    assert_eq!(left.element.type_, ufbx::ElementType::Camera);
    assert_eq!(right.element.type_, ufbx::ElementType::Camera);
    assert_eq!(left.element.name.as_ref(), "StereoLeftShape");
    assert_eq!(right.element.name.as_ref(), "StereoRightShape");
    assert_ne!(left.element.element_id, right.element.element_id);
}

/// Finalized UV and color set lists retain every layer and their indexed data.
#[test]
fn load_uv_and_color_sets_6100_binary() {
    fn check(scene: &Scene, reversed_winding: bool) {
        assert!(walk(scene).is_finite());
        let mesh = scene.meshes.first().expect("missing mesh");
        let uv_names: Vec<&str> = mesh.uv_sets.iter().map(|set| set.name.as_ref()).collect();
        let color_names: Vec<&str> = mesh
            .color_sets
            .iter()
            .map(|set| set.name.as_ref())
            .collect();
        assert_eq!(uv_names, ["UVA", "UVB"]);
        assert_eq!(color_names, ["ColorA", "ColorB"]);
        assert_eq!(mesh.reversed_winding, reversed_winding);
        for edge in &mesh.edges {
            assert!(edge.a == u32::MAX || edge.a < mesh.num_indices as u32);
            assert!(edge.b == u32::MAX || edge.b < mesh.num_indices as u32);
        }
    }

    let scene = load("maya_uv_and_color_sets_6100_binary.fbx");
    check(&scene, false);

    let data = read_data("maya_uv_and_color_sets_6100_binary.fbx");
    let reversed = ufbx::load_memory(
        &data,
        LoadOpts {
            reverse_winding: true,
            ..Default::default()
        },
    )
    .expect("reverse-winding load should succeed");
    check(&reversed, true);
}

/// Legacy constraints use property connections in both directions.
#[test]
fn load_constraint_connections_6100_binary() {
    let scene = load("maya_constraint_zoo_6100_binary.fbx");
    assert!(walk(&scene).is_finite());

    assert!(scene.constraints.iter().any(|constraint| {
        constraint
            .element
            .connections_src
            .iter()
            .any(|conn| conn.src_prop.length > 0 && conn.dst.type_ == ufbx::ElementType::Node)
    }));
    assert!(scene.constraints.iter().any(|constraint| {
        constraint
            .element
            .connections_dst
            .iter()
            .any(|conn| conn.dst_prop.length > 0 && conn.src.type_ == ufbx::ElementType::Node)
    }));

    let parent = scene
        .constraints
        .iter()
        .find(|constraint| constraint.type_ == ufbx::ConstraintType::Parent)
        .expect("missing parent constraint");
    assert_eq!(
        parent
            .node
            .as_ref()
            .expect("parent constraint has no node")
            .element
            .name
            .as_ref(),
        "joint1",
    );
    assert_eq!(parent.targets.len(), 1);
    assert_eq!(parent.targets[0].node.element.name.as_ref(), "Parent");
    assert_eq!(parent.targets[0].weight, 1.0);

    let aim = scene
        .constraints
        .iter()
        .find(|constraint| {
            constraint.type_ == ufbx::ConstraintType::Aim
                && constraint
                    .node
                    .as_ref()
                    .is_some_and(|node| node.element.name.as_ref() == "joint2")
        })
        .expect("missing joint2 aim constraint");
    assert_eq!(aim.targets.len(), 1);
    assert_eq!(aim.targets[0].node.element.name.as_ref(), "Aim");
    assert_eq!(
        aim.aim_up_node
            .as_ref()
            .expect("aim constraint has no up node")
            .element
            .name
            .as_ref(),
        "Up",
    );
}

/// Face-group IDs, parts, and derived mesh metrics are finalized together.
#[test]
fn load_face_group_ids_6100_ascii() {
    let scene = load("synthetic_face_group_id_6100_ascii.fbx");
    assert!(walk(&scene).is_finite());

    let node = scene
        .nodes
        .iter()
        .find(|node| node.element.name.as_ref() == "20 Sided")
        .expect("missing 20 Sided node");
    let mesh = node.mesh.as_ref().expect("20 Sided has no mesh");
    let expected = [
        (i32::MIN, 1usize),
        (-2000, 2),
        (-2, 1),
        (-1, 1),
        (0, 3),
        (1, 2),
        (2, 2),
        (10, 1),
        (20, 1),
        (30, 1),
        (40, 1),
        (50, 1),
        (3000, 2),
        (i32::MAX, 1),
    ];
    assert_eq!(mesh.face_groups.len(), expected.len());
    assert_eq!(mesh.face_group_parts.len(), expected.len());
    for ((group, part), &(id, num_faces)) in mesh
        .face_groups
        .iter()
        .zip(mesh.face_group_parts.iter())
        .zip(expected.iter())
    {
        assert_eq!(group.id, id);
        assert_eq!(part.num_faces, num_faces);
    }
    assert_eq!(mesh.num_faces, 20);
    assert_eq!(mesh.num_triangles, 20);
    assert_eq!(mesh.max_face_triangles, 1);
    assert_eq!(mesh.num_empty_faces, 0);
    assert_eq!(mesh.num_point_faces, 0);
    assert_eq!(mesh.num_line_faces, 0);
}

/// Shader-texture property prefixes are discovered from both explicit compound
/// properties and legacy names, then interned with a trailing separator.
#[test]
fn load_shader_texture_prefixes() {
    // Compile-shape regression: shader-property results borrow the owning
    // shader, while the property-name string is an independent lookup key.
    fn find_for_shader<'a>(
        shader: &'a ufbx::Shader,
        name: &str,
    ) -> (&'a str, &'a str, &'a [ufbx::ShaderPropBinding]) {
        (
            ufbx::find_shader_prop(shader, name),
            shader.find_shader_prop(name),
            ufbx::find_shader_prop_bindings(shader, name),
        )
    }

    fn check_prefix(scene: &ufbx::Scene, name: &str, expected: &str) {
        let mut num_prefixes = 0usize;
        let mut found_expected = false;
        for texture in &scene.textures {
            if let Some(shader) = texture.shader.as_ref() {
                let prefix: &str = shader.prop_prefix.as_ref();
                if !prefix.is_empty() {
                    assert!(prefix.len() > 1);
                    assert!(prefix.ends_with('|'));
                    found_expected |= prefix == expected;
                    num_prefixes += 1;
                }
            }
        }
        assert!(num_prefixes > 0, "{} has no shader-texture prefixes", name);
        assert!(found_expected, "{} is missing prefix {:?}", name, expected);
    }

    let legacy = load("max_texture_mapping_6100_binary.fbx");
    let shader = legacy
        .shaders
        .iter()
        .find(|shader| {
            shader
                .bindings
                .iter()
                .any(|binding| !binding.prop_bindings.is_empty())
        })
        .expect("missing shader with property bindings");
    let binding = shader
        .bindings
        .iter()
        .find_map(|binding| binding.prop_bindings.first())
        .expect("missing shader property binding");
    let (free_prop, member_prop, bindings) = find_for_shader(shader, binding.shader_prop.as_ref());
    assert_eq!(free_prop, binding.material_prop.as_ref());
    assert_eq!(member_prop, free_prop);
    assert!(!bindings.is_empty());
    let (missing_free, missing_member, missing_bindings) =
        find_for_shader(shader, "__missing_shader_property__");
    assert!(missing_free.is_empty());
    assert_eq!(missing_member, missing_free);
    assert!(missing_bindings.is_empty());

    let material = legacy
        .materials
        .iter()
        .find(|material| !material.textures.is_empty())
        .expect("missing material with a property texture");
    let material_texture = &material.textures[0];
    let found = ufbx::find_prop_texture(material, material_texture.material_prop.as_ref())
        .expect("missing material property texture");
    assert!(std::ptr::eq(found, material_texture.texture.as_ref()));
    assert!(ufbx::find_prop_texture(material, "DefinitelyNotAProperty").is_none());

    let base_color = material
        .textures
        .iter()
        .find(|texture| texture.material_prop.as_ref() == "3dsMax|Parameters|base_color_map")
        .expect("missing base-color property texture");
    assert_eq!(base_color.shader_prop.as_ref(), "base_color");
    let mapped_base_color = material
        .pbr
        .base_color
        .texture
        .as_ref()
        .expect("PBR base color has no mapped texture");
    assert!(std::ptr::eq(
        mapped_base_color.as_ref(),
        base_color.texture.as_ref(),
    ));

    check_prefix(
        &legacy,
        "max_texture_mapping_6100_binary.fbx",
        "3dsMax|ai_bump2d Parameters/Connections|",
    );
    let color_tweak = (&legacy.textures)
        .into_iter()
        .find(|texture| {
            texture
                .shader
                .as_ref()
                .is_some_and(|shader| shader.shader_name.as_ref() == "ColorTweak")
        })
        .expect("missing ColorTweak shader texture");
    let shader = color_tweak
        .shader
        .as_ref()
        .expect("ColorTweak has no shader");
    let input = {
        let name = std::string::String::from("Input");
        ufbx::find_shader_texture_input(shader, &name).expect("missing Input")
    };
    let input_texture: &ufbx::Texture = input
        .texture
        .as_ref()
        .expect("Input has no texture")
        .as_ref();
    assert_eq!(input_texture.type_, ufbx::TextureType::File);
    assert_eq!(color_tweak.file_textures.len(), 1);
    assert!(std::ptr::eq(&color_tweak.file_textures[0], input_texture));

    // The OpenPBR ASCII fixture exercises the explicit Compound-property arm.
    // Stacked Borrows rejects its ASCII parser path before scene finalization,
    // so the ordinary integration corpus covers this arm while the legacy
    // binary arm runs under both Miri models.
    #[cfg(not(miri))]
    {
        let openpbr = load("max_openpbr_material_7700_ascii.fbx");
        check_prefix(
            &openpbr,
            "max_openpbr_material_7700_ascii.fbx",
            "3dsMax|parameters|",
        );

        // This ASCII fixture is the exact upstream oracle for multi-hop main
        // texture propagation and output-channel remapping.
        let graph = load("max_shadergraph_7700_ascii.fbx");
        let material = (&graph.materials)
            .into_iter()
            .find(|material| material.element.name.as_ref() == "Material #25")
            .expect("missing Material #25");
        let roughness = material
            .pbr
            .roughness
            .texture
            .as_ref()
            .expect("roughness has no texture");
        assert_eq!(roughness.file_textures.len(), 1);
        assert_eq!(
            roughness.file_textures[0].relative_filename.as_ref(),
            "textures\\checkerboard_ambient.png",
        );
        let shader = roughness.shader.as_ref().expect("roughness has no shader");
        let main = shader
            .main_texture
            .as_ref()
            .expect("roughness shader has no main texture");
        assert_eq!(shader.main_texture_output_index, 6);
        assert_eq!(main.element.name.as_ref(), "Map #30");
        let source = {
            let name = std::string::String::from("sourceMap");
            ufbx::find_shader_texture_input(shader, &name).expect("missing sourceMap input")
        };
        assert_eq!(source.texture_output_index, 6);
        assert_eq!(
            source
                .texture
                .as_ref()
                .expect("sourceMap has no texture")
                .element
                .name
                .as_ref(),
            "Map #30",
        );
    }
}

// -- Scene features exercised on load

/// LOD groups derive their level count from the first instance's children,
/// then combine zero-initialized defaults with threshold and display props.
#[test]
fn load_lod_groups() {
    fn close(actual: ufbx::Real, expected: f32) {
        let expected = expected as ufbx::Real;
        assert!(
            (actual - expected).abs() <= 0.00001f32 as ufbx::Real,
            "{actual:?} != {expected:?}",
        );
    }

    fn get<'a>(scene: &'a ufbx::Scene, name: &str) -> (&'a ufbx::Node, &'a ufbx::LodGroup) {
        let node = scene
            .find_node(name)
            .unwrap_or_else(|| panic!("missing {name}"));
        assert_eq!(node.attrib_type, ufbx::ElementType::LodGroup);
        let attrib = node.attrib.as_ref().expect("LOD node has no attrib");
        let lod = ufbx::as_lod_group(attrib.as_ref()).expect("attrib is not LodGroup");
        assert_eq!(lod.element.type_, ufbx::ElementType::LodGroup);
        (node, lod)
    }

    let scene = load("maya_lod_group_7500_binary.fbx");
    assert_eq!(scene.lod_groups.len(), 2);

    let (node1, lod1) = get(&scene, "LOD_Group_1");
    assert_eq!(node1.children.len(), 3);
    assert_eq!(lod1.element.instances.len(), 1);
    assert_eq!(lod1.element.instances[0].children.len(), 3);
    assert_eq!(lod1.lod_levels.len(), 3);
    assert!(lod1.relative_distances);
    assert!(!lod1.use_distance_limit);
    assert!(!lod1.ignore_parent_transform);
    close(lod1.distance_limit_min, -100.0);
    close(lod1.distance_limit_max, 100.0);
    for (level, distance) in lod1.lod_levels.iter().zip([100.0, 64.0, 32.0]) {
        assert_eq!(level.display, ufbx::LodDisplay::UseLod);
        close(level.distance, distance);
    }

    let (node2, lod2) = get(&scene, "LOD_Group_2");
    assert_eq!(node2.children.len(), 3);
    assert_eq!(lod2.element.instances.len(), 1);
    assert_eq!(lod2.element.instances[0].children.len(), 3);
    assert_eq!(lod2.lod_levels.len(), 3);
    assert!(!lod2.relative_distances);
    assert!(!lod2.use_distance_limit);
    assert!(lod2.ignore_parent_transform);
    close(lod2.distance_limit_min, -100.0);
    close(lod2.distance_limit_max, 100.0);
    let expected = [
        (0.0, ufbx::LodDisplay::UseLod),
        (4.520276, ufbx::LodDisplay::Show),
        (18.081102, ufbx::LodDisplay::Hide),
    ];
    for (level, (distance, display)) in lod2.lod_levels.iter().zip(expected) {
        assert_eq!(level.display, display);
        close(level.distance, distance);
    }
}

/// Selection-node vertex, edge and face lists are validated against their
/// target mesh before the scene publishes them.
#[test]
fn load_selection_sets() {
    let scene = load("max_selection_sets_6100_binary.fbx");
    assert_eq!(scene.selection_sets.len(), 9);
    assert_eq!(scene.selection_nodes.len(), 9);

    let mut seen = 0u16;
    for set in &scene.selection_sets {
        assert_eq!(set.nodes.len(), 1);
        let selection = &set.nodes[0];
        let mesh = selection
            .target_mesh
            .as_ref()
            .expect("selection has no mesh");
        assert!(selection.target_node.is_some());

        let (bit, include_node, vertices, edges, faces): (u16, bool, &[u32], &[u32], &[u32]) =
            match set.element.name.as_ref() {
                "ObjectCube" => (1 << 0, true, &[], &[], &[]),
                "Box001_TopVerts_Vertex" => (1 << 1, false, &[4, 5, 6, 7], &[], &[]),
                "Box001_BottomVerts_Vertex" => (1 << 2, false, &[0, 1, 2, 3], &[], &[]),
                "Box001_TopEdges_Edge" => (1 << 3, false, &[], &[5, 6, 8, 9], &[]),
                "Box001_BottomEdges_Edge" => (1 << 4, false, &[], &[0, 1, 3, 4], &[]),
                "Box001_TopFace_Face" => (1 << 5, false, &[], &[], &[2, 3]),
                "Box001_SideFaces_Face" => (1 << 6, false, &[], &[], &[4, 5, 6, 7, 8, 9, 10, 11]),
                "Box001_BottomFace_Face" => (1 << 7, false, &[], &[], &[0, 1]),
                "Box001_FullCube_Face" => (
                    1 << 8,
                    false,
                    &[],
                    &[],
                    &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
                ),
                name => panic!("unexpected selection set {name:?}"),
            };

        assert_eq!(selection.include_node, include_node);
        assert_eq!(selection.vertices.as_ref(), vertices);
        assert_eq!(selection.edges.as_ref(), edges);
        assert_eq!(selection.faces.as_ref(), faces);
        assert!(vertices.iter().all(|&ix| (ix as usize) < mesh.num_vertices));
        assert!(edges.iter().all(|&ix| (ix as usize) < mesh.num_edges));
        assert!(faces.iter().all(|&ix| (ix as usize) < mesh.num_faces));
        assert_eq!(seen & bit, 0, "duplicate selection set");
        seen |= bit;
    }
    assert_eq!(seen, (1 << 9) - 1);
}

/// Skin deformers: clusters, weights and the bind-pose matrices.
#[test]
fn load_skinned() {
    let scene = load("blender_293_half_skinned_7400_binary.fbx");
    assert!(!scene.skin_deformers.is_empty());
    let mut acc = 0.0f64;
    for skin in &scene.skin_deformers {
        for cluster in &skin.clusters {
            assert!(cluster.bone_node.is_some());
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

/// Legacy geometry transforms can be baked into each independently instanced
/// mesh without helper nodes.
#[test]
fn load_geometry_transform_modify() {
    let name = "max_geometry_transform_instances_6100_binary.fbx";
    let data = read_data(name);
    let scene = ufbx::load_memory(
        &data,
        LoadOpts {
            geometry_transform_handling: ufbx::GeometryTransformHandling::ModifyGeometry,
            ..Default::default()
        },
    )
    .unwrap_or_else(|e| panic!("failed to load {}: {:?}", name, e));

    assert_eq!(scene.meshes.len(), 4);
    assert_eq!(scene.nodes.len(), 5);
    let vertex = scene.meshes[0]
        .vertices
        .first()
        .expect("mesh has no vertices");
    let epsilon = 0.0001f32 as ufbx::Real;
    assert!((vertex.x - 30.0).abs() <= epsilon);
    assert!((vertex.y + 10.0).abs() <= epsilon);
    assert!((vertex.z + 10.0).abs() <= epsilon);
    for node in scene.nodes.iter().skip(1) {
        assert!(!node.has_geometry_transform);
        assert!(node.mesh.is_some());
        assert!(!node.is_geometry_transform_helper);
        assert!(node.geometry_transform_helper.is_none());
    }
}

/// Mesh connection fetches advance only after scanning the current element:
/// materials stop at the first instance hit, while legacy deformers continue
/// through a geometry-transform helper to its parent.
#[test]
fn mesh_connection_fetches_follow_typed_node_walk() {
    let scene = load("max_instanced_material_7700_binary.fbx");
    let red = scene.find_node("Red").expect("missing Red node");
    let green = scene.find_node("Green").expect("missing Green node");
    let blue = scene.find_node("Blue").expect("missing Blue node");
    let mesh = green.mesh.as_ref().expect("Green has no mesh");
    assert!(core::ptr::eq(
        mesh.as_ref(),
        red.mesh.as_ref().expect("Red has no mesh").as_ref()
    ));
    assert!(core::ptr::eq(
        mesh.as_ref(),
        blue.mesh.as_ref().expect("Blue has no mesh").as_ref()
    ));
    for (node, material) in [(red, "RedMat"), (green, "GreenMat"), (blue, "BlueMat")] {
        assert_eq!(node.materials.len(), 1);
        assert_eq!(node.materials[0].element.name.as_ref(), material);
    }
    assert_eq!(mesh.materials.len(), 1);
    assert_eq!(mesh.materials[0].element.name.as_ref(), "GreenMat");

    let name = "max_transformed_skin_6100_binary.fbx";
    let data = read_data(name);
    let scene = ufbx::load_memory(
        &data,
        LoadOpts {
            geometry_transform_handling: ufbx::GeometryTransformHandling::HelperNodes,
            ..Default::default()
        },
    )
    .unwrap_or_else(|e| panic!("failed to load {}: {:?}", name, e));
    let parent = scene.find_node("Box001").expect("missing Box001");
    assert!(parent.mesh.is_none());
    let helper = parent
        .geometry_transform_helper
        .as_ref()
        .expect("missing geometry-transform helper");
    assert!(helper.is_geometry_transform_helper);
    let mesh = helper.mesh.as_ref().expect("helper has no mesh");
    assert!(!mesh.skin_deformers.is_empty());
    assert_eq!(mesh.all_deformers.len(), 1);
    let skin = &mesh.skin_deformers[0];
    assert!(core::ptr::eq(&mesh.all_deformers[0], &skin.element));
}

/// Helper-node inherit handling follows parent, scale-helper and
/// inherit-scale-node links while composing transforms.
#[test]
fn load_scale_helpers() {
    fn close_vec3(actual: ufbx::Vec3, expected: [f32; 3]) {
        let expected = expected.map(|v| v as ufbx::Real);
        let epsilon = 0.00001f32 as ufbx::Real;
        assert!((actual.x - expected[0]).abs() <= epsilon);
        assert!((actual.y - expected[1]).abs() <= epsilon);
        assert!((actual.z - expected[2]).abs() <= epsilon);
    }

    let name = "maya_static_no_inherit_scale_7700_ascii.fbx";
    let data = read_data(name);
    let scene = ufbx::load_memory(
        &data,
        LoadOpts {
            inherit_mode_handling: ufbx::InheritModeHandling::HelperNodes,
            ..Default::default()
        },
    )
    .unwrap_or_else(|e| panic!("failed to load {}: {:?}", name, e));

    let joint1 = scene.find_node("joint1").expect("missing joint1");
    let helper1 = joint1.scale_helper.as_ref().expect("joint1 has no helper");
    close_vec3(helper1.local_transform.scale, [2.0, 2.0, 2.0]);
    close_vec3(
        ufbx::matrix_to_transform(&helper1.node_to_world).translation,
        [0.0, 0.0, 0.0],
    );
    close_vec3(joint1.local_transform.scale, [1.0, 1.0, 1.0]);

    let joint2 = scene.find_node("joint2").expect("missing joint2");
    let helper2 = joint2.scale_helper.as_ref().expect("joint2 has no helper");
    close_vec3(helper2.local_transform.scale, [1.0, 2.0, 2.0]);
    close_vec3(
        ufbx::matrix_to_transform(&helper2.node_to_world).translation,
        [0.0, 8.0, 0.0],
    );
    close_vec3(joint2.local_transform.scale, [1.0, 1.0, 1.0]);

    let joint3 = scene.find_node("joint3").expect("missing joint3");
    assert!(joint3.scale_helper.is_none());
    close_vec3(
        ufbx::matrix_to_transform(&joint3.node_to_world).translation,
        [0.0, 12.0, 0.0],
    );
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

/// Layered textures retain the checked source texture Ref while deriving the
/// blend mode and alpha from that same texture's properties.
#[test]
fn load_texture_layers() {
    let scene = load("maya_texture_layers_7500_binary.fbx");
    let node = scene.find_node("pCube1").expect("missing pCube1");
    let mesh = node.mesh.as_ref().expect("pCube1 has no mesh");
    assert_eq!(mesh.materials.len(), 1);
    let layered = mesh.materials[0]
        .fbx
        .diffuse_color
        .texture
        .as_ref()
        .expect("diffuse color has no layered texture");
    assert_eq!(layered.type_, ufbx::TextureType::Layered);

    let expected = [
        (
            ufbx::BlendMode::Multiply,
            0.75,
            "textures\\checkerboard_weight.png",
        ),
        (
            ufbx::BlendMode::Over,
            0.5,
            "textures\\checkerboard_diffuse.png",
        ),
        (
            ufbx::BlendMode::Additive,
            1.0,
            "textures\\checkerboard_ambient.png",
        ),
    ];
    assert_eq!(layered.layers.len(), expected.len());
    assert_eq!(layered.file_textures.len(), expected.len());
    for (index, (layer, &(mode, alpha, filename))) in
        layered.layers.iter().zip(expected.iter()).enumerate()
    {
        assert_eq!(layer.blend_mode, mode);
        assert_eq!(layer.alpha as f64, alpha);
        assert_eq!(layer.texture.relative_filename.as_ref(), filename);
        assert!(core::ptr::eq(
            layer.texture.as_ref(),
            &layered.file_textures[index]
        ));
    }
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

/// Generated baked lookup wrappers return entries owned by the baked header,
/// while the node/element arguments are lookup keys with independent borrows.
#[test]
fn public_baked_finders_from_owned_header() {
    let scene = load("maya_cube_7500_binary.fbx");
    let source_node: &ufbx::Node = scene
        .nodes
        .first()
        .expect("cube scene has no node")
        .as_ref();

    // SAFETY: `Node`/`Element` are C-compatible arena headers with no destructor
    // or ownership-bearing fields. The copied headers' stored references keep
    // pointing into `scene`, which stays live for the complete test. All-zero is
    // valid for the baked result records: their lists are null/empty pointer-count
    // pairs and every other field is a scalar, boolean, or float.
    let (node_key, element_key, mut baked_node, mut baked_element, mut bake): (
        ufbx::Node,
        ufbx::Element,
        ufbx::BakedNode,
        ufbx::BakedElement,
        ufbx::BakedAnim,
    ) = unsafe {
        (
            std::ptr::read(source_node),
            std::ptr::read(&source_node.element),
            std::mem::MaybeUninit::zeroed().assume_init(),
            std::mem::MaybeUninit::zeroed().assume_init(),
            std::mem::MaybeUninit::zeroed().assume_init(),
        )
    };

    baked_node.typed_id = source_node.element.typed_id;
    baked_node.element_id = source_node.element.element_id;
    baked_element.element_id = source_node.element.element_id;
    bake.nodes.data = &baked_node;
    bake.nodes.count = 1;
    bake.elements.data = &baked_element;
    bake.elements.count = 1;

    let found = ufbx::find_baked_node_by_typed_id(&mut bake, baked_node.typed_id)
        .expect("baked node by typed ID");
    assert!(std::ptr::eq(found, &baked_node));

    let found = {
        let mut key = node_key;
        ufbx::find_baked_node(&mut bake, &mut key).expect("baked node by node")
    };
    assert!(std::ptr::eq(found, &baked_node));

    let found = ufbx::find_baked_element_by_element_id(&mut bake, baked_element.element_id)
        .expect("baked element by element ID");
    assert!(std::ptr::eq(found, &baked_element));

    let found = {
        let mut key = element_key;
        ufbx::find_baked_element(&mut bake, &mut key).expect("baked element by element")
    };
    assert!(std::ptr::eq(found, &baked_element));
}

/// Animated layer weights are evaluated through the layer's own anim-value
/// reference before the layer is blended into each output property.
#[test]
fn bake_animated_layer_weight() {
    let scene = load("maya_anim_layer_anim_7700_ascii.fbx");
    assert!(scene
        .anim_layers
        .iter()
        .any(|layer| layer.weight_is_animated && layer.blended));

    let baked = ufbx::bake_anim(
        &scene,
        &scene.anim,
        ufbx::BakeOpts {
            resample_rate: 24.0,
            ..Default::default()
        },
    )
    .expect("bake_anim failed");
    assert_eq!(baked.nodes.len(), 1);
    assert_eq!(baked.nodes[0].translation_keys.len(), 25);
    assert_eq!(baked.nodes[0].rotation_keys.len(), 25);
    assert_eq!(baked.nodes[0].scale_keys.len(), 25);
}

/// Baking helper-node inherit modes follows parent, scale-helper and
/// inherit-scale-node references while scheduling dependent sibling nodes.
#[test]
fn bake_scale_helpers() {
    fn close(actual: ufbx::Real, expected: f32) {
        let expected = expected as ufbx::Real;
        assert!((actual - expected).abs() <= 0.00001f32 as ufbx::Real);
    }

    let name = "maya_scale_no_inherit_step_7700_ascii.fbx";
    let data = read_data(name);
    let scene = ufbx::load_memory(
        &data,
        LoadOpts {
            inherit_mode_handling: ufbx::InheritModeHandling::HelperNodes,
            ..Default::default()
        },
    )
    .unwrap_or_else(|e| panic!("failed to load {}: {:?}", name, e));

    let joint1 = scene.find_node("joint1").expect("missing joint1");
    let helper = joint1.scale_helper.as_ref().expect("joint1 has no helper");
    let joint2 = scene.find_node("joint2").expect("missing joint2");
    let baked = ufbx::bake_anim(&scene, &scene.anim, Default::default()).expect("bake_anim failed");

    let helper_keys = baked
        .nodes
        .iter()
        .find(|node| node.typed_id == helper.element.typed_id)
        .expect("helper was not baked")
        .scale_keys
        .as_ref();
    let joint2_keys = baked
        .nodes
        .iter()
        .find(|node| node.typed_id == joint2.element.typed_id)
        .expect("joint2 was not baked")
        .translation_keys
        .as_ref();

    assert_eq!(helper_keys.len(), 6);
    close(helper_keys[0].value.x, 2.0);
    close(helper_keys[5].value.x, 5.0);
    assert_eq!(joint2_keys.len(), 6);
    close(joint2_keys[0].value.x, 10.0);
    close(joint2_keys[5].value.x, 25.0);
}

/// NURBS: basis evaluation and curve tessellation into a line curve.
#[test]
fn tessellate_nurbs_curve() {
    fn check_line(curve: &ufbx::NurbsCurve, line: &ufbx::LineCurve, num_sub: usize) {
        let num_spans = curve.basis.spans.len();
        let num_indices = num_spans + (num_spans - 1) * (num_sub - 1);
        let is_open = curve.basis.topology == ufbx::NurbsTopology::Open;
        let num_vertices = num_indices - usize::from(!is_open);

        assert_eq!(line.control_points.len(), num_vertices);
        assert_eq!(line.point_indices.len(), num_indices);
        assert_eq!(line.segments.len(), 1);
        assert_eq!(line.segments[0].index_begin, 0);
        assert_eq!(line.segments[0].num_indices as usize, num_indices);
        for index in 0..num_vertices {
            assert_eq!(line.point_indices[index] as usize, index);
        }
        if !is_open {
            assert_eq!(line.point_indices[num_vertices], 0);
        }
    }

    let scene = load("maya_nurbs_curve_form_7700_binary.fbx");
    assert!(!scene.nurbs_curves.is_empty());
    let mut acc = 0.0f64;
    let mut topology_mask = 0u8;
    for curve in &scene.nurbs_curves {
        let order = curve.basis.order as usize;
        assert!(order > 1);
        let degree = order - 1;
        let knots: &[ufbx::Real] = curve.basis.knot_vector.as_ref();
        assert!(knots.len() >= degree.wrapping_mul(2).wrapping_add(1));

        let expected_wrap = match curve.basis.topology {
            ufbx::NurbsTopology::Closed => {
                topology_mask |= 1 << 2;
                1
            }
            ufbx::NurbsTopology::Periodic => {
                topology_mask |= 1 << 1;
                curve.basis.order.wrapping_sub(1) as usize
            }
            ufbx::NurbsTopology::Open => {
                topology_mask |= 1 << 0;
                0
            }
        };
        assert_eq!(curve.basis.num_wrap_control_points, expected_wrap);
        assert_eq!(curve.basis.t_min.to_bits(), knots[degree].to_bits());
        assert_eq!(
            curve.basis.t_max.to_bits(),
            knots[knots.len() - degree - 1].to_bits()
        );

        let mut expected_spans = Vec::new();
        let mut prev = -(f64::INFINITY as ufbx::Real);
        for &knot in &knots[degree..knots.len() - degree] {
            if knot != prev {
                expected_spans.push(knot);
                prev = knot;
            }
        }
        assert_eq!(curve.basis.spans.len(), expected_spans.len());
        for (&actual, &expected) in curve.basis.spans.iter().zip(&expected_spans) {
            assert_eq!(actual.to_bits(), expected.to_bits());
        }
        assert_eq!(
            curve.basis.valid,
            !knots.windows(2).any(|pair| pair[0] > pair[1])
        );

        let sample_u = (curve.basis.t_min + curve.basis.t_max) * (0.5f32 as ufbx::Real);
        let sentinel = 1234.5f32 as ufbx::Real;
        let mut weights = vec![sentinel; order + 2];
        let mut derivatives = vec![sentinel; order + 2];
        let basis = curve
            .basis
            .evaluate(sample_u, &mut weights, &mut derivatives);
        assert_ne!(basis, usize::MAX);
        let weight_sum: ufbx::Real = weights[..order].iter().copied().sum();
        assert!((weight_sum - 1.0f32 as ufbx::Real).abs() < 0.0001f32 as ufbx::Real);
        assert!(weights[order..]
            .iter()
            .all(|value| value.to_bits() == sentinel.to_bits()));
        assert!(derivatives[order..]
            .iter()
            .all(|value| value.to_bits() == sentinel.to_bits()));

        let mut weights_without_derivatives = vec![sentinel; order];
        let basis_without_derivatives =
            curve
                .basis
                .evaluate(sample_u, &mut weights_without_derivatives, &mut []);
        assert_eq!(basis_without_derivatives, basis);
        for (with, without) in weights[..order].iter().zip(&weights_without_derivatives) {
            assert_eq!(with.to_bits(), without.to_bits());
        }

        let mut short_weights = vec![sentinel; order - 1];
        let mut untouched_derivatives = vec![sentinel; order];
        let short_basis =
            curve
                .basis
                .evaluate(sample_u, &mut short_weights, &mut untouched_derivatives);
        assert_eq!(short_basis, basis);
        assert!(short_weights
            .iter()
            .all(|value| value.to_bits() == sentinel.to_bits()));
        assert!(untouched_derivatives
            .iter()
            .all(|value| value.to_bits() == sentinel.to_bits()));

        for u in [0.0, 0.5, 1.0] {
            let point = ufbx::evaluate_nurbs_curve(curve, u);
            assert!(point.valid);
            acc += point.position.x as f64;
        }
        let line = ufbx::tessellate_nurbs_curve(curve, Default::default())
            .expect("tessellate_nurbs_curve failed");
        check_line(curve, &line, 4);
        acc += line.control_points.len() as f64;
        for &i in &line.point_indices {
            acc += i as f64;
        }

        let line_sub3 = ufbx::tessellate_nurbs_curve(
            curve,
            ufbx::TessellateCurveOpts {
                span_subdivision: 3,
                ..Default::default()
            },
        )
        .expect("tessellate_nurbs_curve with subdivision 3 failed");
        check_line(curve, &line_sub3, 3);
        acc += line_sub3.control_points.len() as f64;
    }
    assert_eq!(topology_mask, 0b111);
    assert!(acc.is_finite());
}

/// NURBS surfaces tessellate into a full mesh (a different allocator path from
/// the curve case).
#[test]
fn tessellate_nurbs_surface() {
    fn check_mesh(
        surface: &ufbx::NurbsSurface,
        mesh: &ufbx::Mesh,
        sub_u: usize,
        sub_v: usize,
    ) -> (usize, usize) {
        let spans_u = surface.basis_u.spans.len();
        let spans_v = surface.basis_v.spans.len();
        let expected_faces = spans_u
            .checked_sub(1)
            .and_then(|n| n.checked_mul(sub_u))
            .and_then(|n| {
                spans_v
                    .checked_sub(1)
                    .and_then(|m| m.checked_mul(sub_v))
                    .and_then(|m| n.checked_mul(m))
            })
            .expect("surface face count overflow");
        let samples_u = spans_u
            .checked_sub(1)
            .and_then(|n| n.checked_mul(sub_u - 1))
            .and_then(|n| spans_u.checked_add(n))
            .expect("surface U sample count overflow");
        let samples_v = spans_v
            .checked_sub(1)
            .and_then(|n| n.checked_mul(sub_v - 1))
            .and_then(|n| spans_v.checked_add(n))
            .expect("surface V sample count overflow");
        let sample_count = samples_u
            .checked_mul(samples_v)
            .expect("surface sample count overflow");

        assert_eq!(mesh.num_faces, expected_faces);
        assert_eq!(mesh.faces.len(), mesh.num_faces);
        assert_eq!(mesh.vertices.len(), mesh.num_vertices);
        assert_eq!(mesh.vertex_indices.len(), mesh.num_indices);
        assert_eq!(mesh.vertex_position.values.len(), mesh.num_vertices);
        assert_eq!(mesh.vertex_position.indices.len(), mesh.num_indices);
        assert_eq!(mesh.vertex_normal.values.len(), mesh.num_vertices);
        assert_eq!(mesh.vertex_normal.indices.len(), mesh.num_indices);
        // These value-list headers carry the corner count, while their storage
        // is the sampled parameter grid. Validate the live index lists against
        // that grid without materializing the oversized value slices.
        assert_eq!(mesh.vertex_uv.indices.len(), mesh.num_indices);
        assert_eq!(mesh.vertex_tangent.indices.len(), mesh.num_indices);
        assert_eq!(mesh.vertex_bitangent.indices.len(), mesh.num_indices);
        assert_eq!(
            mesh.vertex_position.indices.as_ref(),
            mesh.vertex_indices.as_ref()
        );
        assert_eq!(
            mesh.vertex_normal.indices.as_ref(),
            mesh.vertex_indices.as_ref()
        );
        assert_eq!(
            mesh.vertex_tangent.indices.as_ref(),
            mesh.vertex_uv.indices.as_ref()
        );
        assert_eq!(
            mesh.vertex_bitangent.indices.as_ref(),
            mesh.vertex_uv.indices.as_ref()
        );

        let mut cursor = 0usize;
        let mut triangles = 0usize;
        let mut quads = 0usize;
        for face in &mesh.faces {
            assert_eq!(face.index_begin as usize, cursor);
            let count = face.num_indices as usize;
            assert!(count == 3 || count == 4);
            let end = cursor.checked_add(count).expect("face range overflow");
            assert!(end <= mesh.num_indices);

            for index in cursor..end {
                assert!((mesh.vertex_indices[index] as usize) < mesh.num_vertices);
                assert!((mesh.vertex_uv.indices[index] as usize) < sample_count);
                assert!((mesh.vertex_tangent.indices[index] as usize) < sample_count);
                assert!((mesh.vertex_bitangent.indices[index] as usize) < sample_count);
            }

            triangles += usize::from(count == 3);
            quads += usize::from(count == 4);
            cursor = end;
        }

        assert_eq!(cursor, mesh.num_indices);
        assert_eq!(mesh.num_indices, triangles * 3 + quads * 4);
        assert_eq!(mesh.num_triangles, triangles + quads * 2);
        assert_eq!(mesh.max_face_triangles, 2);
        (triangles, quads)
    }

    let mut acc = 0.0f64;
    let mut triangles = 0usize;
    let mut quads = 0usize;
    let sphere_scene = load("maya_nurbs_low_sphere_7500_ascii.fbx");
    assert!(!sphere_scene.nurbs_surfaces.is_empty());
    for surface in &sphere_scene.nurbs_surfaces {
        let sample_u = (surface.basis_u.t_min + surface.basis_u.t_max) * (0.5f32 as ufbx::Real);
        let sample_v = (surface.basis_v.t_min + surface.basis_v.t_max) * (0.5f32 as ufbx::Real);
        let point = ufbx::evaluate_nurbs_surface(surface, sample_u, sample_v);
        assert!(point.valid);
        acc += point.position.x as f64
            + point.position.y as f64
            + point.position.z as f64
            + point.derivative_u.x as f64
            + point.derivative_v.y as f64;

        let mesh = ufbx::tessellate_nurbs_surface(surface, Default::default())
            .expect("tessellate_nurbs_surface failed");
        let (mesh_triangles, mesh_quads) = check_mesh(surface, &mesh, 4, 4);
        triangles += mesh_triangles;
        quads += mesh_quads;
        acc += walk_mesh(&mesh);
    }

    let mut flipped_data = read_data("maya_nurbs_surface_plane_6100_ascii.fbx");
    let needle = b"FlipNormals: 0";
    let replacement = b"FlipNormals: 1";
    let matches: Vec<usize> = flipped_data
        .windows(needle.len())
        .enumerate()
        .filter_map(|(index, window)| (window == needle).then_some(index))
        .collect();
    assert_eq!(matches.len(), 1);
    flipped_data[matches[0]..matches[0] + needle.len()].copy_from_slice(replacement);
    let flipped_scene = ufbx::load_memory(&flipped_data, LoadOpts::default())
        .expect("failed to load flipped-normal surface");
    let flipped_surface = flipped_scene
        .nurbs_surfaces
        .first()
        .expect("no flipped surface");
    assert!(flipped_surface.flip_normals);
    assert_eq!(
        flipped_surface
            .material
            .as_ref()
            .expect("flipped surface has no material")
            .element
            .name
            .as_ref(),
        "lambert1"
    );
    let flipped_mesh = ufbx::tessellate_nurbs_surface(
        flipped_surface,
        ufbx::TessellateSurfaceOpts {
            span_subdivision_u: 1,
            span_subdivision_v: 1,
            ..Default::default()
        },
    )
    .expect("flipped surface tessellation failed");
    let (plane_triangles, plane_quads) = check_mesh(flipped_surface, &flipped_mesh, 1, 1);
    triangles += plane_triangles;
    quads += plane_quads;

    // Recompute from topology, positions, and the normal mapping into an
    // independent buffer; this does not read the tessellated normal values.
    let mut unflipped_normals = vec![ufbx::Vec3::default(); flipped_mesh.num_vertices];
    ufbx::compute_normals(
        &flipped_mesh,
        &flipped_mesh.vertex_position,
        flipped_mesh.vertex_normal.indices.as_ref(),
        &mut unflipped_normals,
    );
    assert_eq!(
        unflipped_normals.len(),
        flipped_mesh.vertex_normal.values.len()
    );
    for (base, flipped) in unflipped_normals
        .iter()
        .zip(&flipped_mesh.vertex_normal.values)
    {
        assert_eq!(
            flipped.x.to_bits(),
            (base.x * (-1.0f32 as ufbx::Real)).to_bits()
        );
        assert_eq!(
            flipped.y.to_bits(),
            (base.y * (-1.0f32 as ufbx::Real)).to_bits()
        );
        assert_eq!(
            flipped.z.to_bits(),
            (base.z * (-1.0f32 as ufbx::Real)).to_bits()
        );
    }
    acc += walk_mesh(&flipped_mesh);

    assert!(triangles > 0);
    assert!(quads > 0);
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

/// Subdivision weight propagation: source-vertex and skin-cluster weights use
/// the type-erased vertex-weight summer and publish one range per output vertex.
#[test]
fn subdivide_with_weights() {
    let scene = load("blender_293_half_skinned_7400_binary.fbx");
    let mesh = scene
        .meshes
        .iter()
        .find(|mesh| !mesh.skin_deformers.is_empty())
        .expect("no skinned mesh");
    let opts = ufbx::SubdivideOpts {
        evaluate_source_vertices: true,
        evaluate_skin_weights: true,
        ..Default::default()
    };
    let subdivided = ufbx::subdivide_mesh(mesh, 2, opts).expect("subdivide_mesh failed");
    let result = subdivided
        .subdivision_result
        .as_ref()
        .expect("no subdivision result");
    assert_eq!(result.source_vertex_ranges.len(), subdivided.num_vertices);
    assert_eq!(result.skin_cluster_ranges.len(), subdivided.num_vertices);
    assert!(!result.source_vertex_weights.is_empty());
    assert!(!result.skin_cluster_weights.is_empty());
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

    let mut saw_five_gon = false;
    let mut saw_larger_ngon = false;
    for face in &mesh.faces {
        let expected_tris = face.num_indices.saturating_sub(2) as usize;
        let mut indices = vec![0u32; expected_tris * 3];
        let tris = ufbx::triangulate_face(&mut indices, mesh, *face);
        assert_eq!(tris as usize, expected_tris);
        let face_begin = face.index_begin;
        let face_end = face.index_begin + face.num_indices;
        assert!(indices
            .iter()
            .all(|&index| index >= face_begin && index < face_end));
        saw_five_gon |= face.num_indices == 5;
        saw_larger_ngon |= face.num_indices >= 6;
        acc += tris as f64;
    }
    assert!(saw_five_gon);
    assert!(saw_larger_ngon);

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

// Cyclic property connections exhaust the bounded walk and retain each local
// value because the terminal re-probe still finds another connection.
#[test]
#[cfg(not(miri))]
fn recursive_connected_property_cycles_preserve_local_values() {
    let scene = load("synthetic_recursive_connections_7700_ascii.fbx");

    for (node_name, expected) in [("pCube1", [1.0, 0.0, 0.0]), ("pCube3", [0.0, 0.0, 1.0])] {
        let node = scene.find_node(node_name).expect("missing connected node");
        let stored = node
            .element
            .props
            .find_prop("Lcl Translation")
            .expect("missing connected property");
        assert!(stored.flags.has_any(ufbx::PropFlags::CONNECTED));

        let value = ufbx::evaluate_prop(&scene.anim, &node.element, "Lcl Translation", 0.5);
        assert_eq!(
            [
                value.value_vec4.x as f64,
                value.value_vec4.y as f64,
                value.value_vec4.z as f64,
            ],
            expected
        );
    }
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

    // Scene-rooted finders: one hit for the generic/node pair and explicit
    // misses for every typed wrapper keep each safe-reference boundary live.
    let node_name = scene.root_node.element.name.as_ref();
    let found_element = ufbx::find_element(scene, ufbx::ElementType::Node, node_name)
        .expect("root node element lookup");
    let found_node = ufbx::find_node(scene, node_name).expect("root node lookup");
    assert!(core::ptr::eq(found_element, &scene.root_node.element));
    assert!(core::ptr::eq(found_node, scene.root_node.as_ref()));
    assert!(ufbx::find_anim_stack(scene, "__missing_anim_stack__").is_none());
    assert!(ufbx::find_material(scene, "__missing_material__").is_none());

    // Element-rooted paths (api.rs public-boundary roots).
    let prop_node = scene
        .nodes
        .iter()
        .find(|node| !node.element.props.props.is_empty())
        .expect("scene has no node with stored properties");
    let elem: &ufbx::Element = &prop_node.element;
    let props: &ufbx::Props = &elem.props;
    let prop = props.props.first().expect("node has no stored properties");
    let by_name = ufbx::find_prop_element(elem, &prop.name, ufbx::ElementType::Node);
    // Direct prop adapter and name lookup share the same Const header core.
    let direct = ufbx::get_prop_element(elem, prop, ufbx::ElementType::Node);
    assert_eq!(
        direct.map(core::ptr::from_ref),
        by_name.map(core::ptr::from_ref)
    );

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
// takes `&mut Mesh`.

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
    let objects = {
        let name = std::string::String::from("Objects");
        ufbx::dom_find(dom_root, &name).expect("Objects dom node")
    };
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
            let found = {
                let name = std::string::String::from(grandchild.name.as_ref());
                ufbx::dom_find(child, &name)
            };
            if let Some(found) = found {
                acc += found.values.len() as f64;
            }
        }
    }
    let leaf = objects
        .children
        .iter()
        .map(|child| child.as_ref())
        .find(|child| child.children.is_empty())
        .expect("Objects contains a DOM leaf");
    assert!(ufbx::dom_find(leaf, "__missing_dom_child__").is_none());
    assert!(acc.is_finite());
}

/// Animation evaluation from shared refs: per-value and per-prop evaluators,
/// transform evaluation over both a plain and an override-carrying anim, the
/// anim-prop finders, baked-keyframe interpolation, and the matrix/transform
/// value helpers fed from scene data.
#[test]
fn public_anim_eval_from_shared_refs() {
    // Compile-shape regression: results borrow only the owning layer, not the
    // independently borrowed element key, for both free and member forms.
    fn find_for_layer<'a>(
        layer: &'a ufbx::AnimLayer,
        element: &ufbx::Element,
        name: &str,
    ) -> (
        Option<&'a ufbx::AnimProp>,
        &'a [ufbx::AnimProp],
        Option<&'a ufbx::AnimProp>,
        &'a [ufbx::AnimProp],
    ) {
        (
            ufbx::find_anim_prop(layer, element, name),
            ufbx::find_anim_props(layer, element),
            layer.find_anim_prop(element, name),
            layer.find_anim_props(element),
        )
    }

    let root = load("maya_interpolation_modes_7500_binary.fbx");
    let scene: &Scene = &root;
    let anim: &ufbx::Anim = &scene.anim;
    let mut acc = 0.0f64;

    for curve in &scene.anim_curves {
        acc += ufbx::evaluate_curve(curve, 0.4, 0.0) as f64;
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
            let (free_prop, free_props, member_prop, member_props) =
                find_for_layer(layer, elem, prop.prop_name.as_ref());
            acc += free_props.len() as f64 + member_props.len() as f64;
            assert_eq!(
                free_prop.map(core::ptr::from_ref),
                member_prop.map(core::ptr::from_ref)
            );
            if let Some(found) = member_prop {
                acc += found.prop_name.as_ref().len() as f64;
            }
        }
    }

    // Seed non-empty output capabilities through the safe per-property API.
    // Both wrappers derive their raw capacity from a one-entry slice and return
    // a header whose animated prefix remains tied to that storage.
    let animated_prop = scene
        .anim_layers
        .iter()
        .flat_map(|layer| layer.anim_props.as_ref())
        .next()
        .expect("fixture has no animated properties");
    let animated_element: &ufbx::Element = animated_prop.element.as_ref();
    {
        let seed = ufbx::evaluate_prop(
            anim,
            animated_element,
            animated_prop.prop_name.as_ref(),
            0.25,
        );
        let mut buffer = [seed];
        let props = ufbx::evaluate_props(anim, animated_element, 0.25, &mut buffer);
        assert_eq!(props.num_animated, 1);
        assert_eq!(props.props.len(), 1);
        assert!(props.defaults.is_some());
        acc += props.props[0].value_vec4.x as f64;
    }
    {
        let seed = ufbx::evaluate_prop(
            anim,
            animated_element,
            animated_prop.prop_name.as_ref(),
            0.25,
        );
        let mut buffer = [seed];
        let props = ufbx::evaluate_props_flags(anim, animated_element, 0.25, &mut buffer, 0);
        assert_eq!(props.num_animated, 1);
        assert_eq!(props.props.len(), 1);
        assert!(props.defaults.is_some());
        acc += props.props[0].value_vec4.x as f64;
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

    // Override-carrying anim: with `prop_overrides` non-empty the internal prop
    // iterator takes its slow path, which yields the iterator's own scratch
    // `tmp` prop and rewrites those bytes on the following step. The read-only
    // views the selected-prop loop mints over each yielded prop must therefore
    // stay confined to one iteration; only an anim with overrides exercises
    // that, so it is here rather than folded into the plain-anim walk above.
    // The node must be a non-root one: `ufbx_evaluate_transform` returns the
    // root's local transform without consulting props at all.
    let node = scene
        .nodes
        .as_ref()
        .iter()
        .find(|n| !n.is_root)
        .expect("non-root node");
    const EXPLICIT_INCLUDES: u32 = 0x4;
    const INCLUDE_TRANSLATION: u32 = 0x10;
    const INCLUDE_ROTATION: u32 = 0x20;
    const INCLUDE_SCALE: u32 = 0x40;

    let identity = ufbx::evaluate_transform_flags(anim, node, 0.25, EXPLICIT_INCLUDES);
    assert_eq!(identity.translation.x, 0.0);
    assert_eq!(identity.rotation.w, 1.0);
    assert_eq!(identity.scale.x, 1.0);

    let rotation =
        ufbx::evaluate_transform_flags(anim, node, 0.25, EXPLICIT_INCLUDES | INCLUDE_ROTATION);
    assert_eq!(rotation.translation.x, 0.0);
    assert_eq!(rotation.scale.x, 1.0);

    let scale = ufbx::evaluate_transform_flags(anim, node, 0.25, EXPLICIT_INCLUDES | INCLUDE_SCALE);
    assert_eq!(scale.translation.x, 0.0);
    assert_eq!(scale.rotation.w, 1.0);

    let rotation_scale = ufbx::evaluate_transform_flags(
        anim,
        node,
        0.25,
        EXPLICIT_INCLUDES | INCLUDE_ROTATION | INCLUDE_SCALE,
    );
    assert_eq!(rotation_scale.translation.x, 0.0);

    let translation =
        ufbx::evaluate_transform_flags(anim, node, 0.25, EXPLICIT_INCLUDES | INCLUDE_TRANSLATION);
    assert_eq!(translation.rotation.w, 1.0);
    assert_eq!(translation.scale.x, 1.0);

    acc += rotation.rotation.w as f64;
    acc += scale.scale.y as f64;
    acc += rotation_scale.rotation.w as f64;
    acc += translation.translation.x as f64;
    let over = ufbx::create_anim(
        scene,
        ufbx::AnimOpts {
            prop_overrides: vec![
                ufbx::PropOverrideDesc {
                    element_id: node.element.element_id,
                    prop_name: "Lcl Translation".into(),
                    value: ufbx::Vec4 {
                        x: 1.0,
                        y: 2.0,
                        z: 3.0,
                        w: 0.0,
                    },
                    ..Default::default()
                },
                ufbx::PropOverrideDesc {
                    element_id: node.element.element_id,
                    prop_name: "Lcl Scaling".into(),
                    value: ufbx::Vec4 {
                        x: 2.0,
                        y: 2.0,
                        z: 2.0,
                        w: 0.0,
                    },
                    ..Default::default()
                },
            ]
            .into(),
            ..Default::default()
        },
    )
    .expect("create_anim");
    let over_anim: &ufbx::Anim = &over;
    let t = ufbx::evaluate_transform(over_anim, node, 0.25);
    // The overridden values reaching the transform is what pins the slow path as
    // covered: were it skipped, the plain animated values would come back here.
    assert_eq!(t.translation.x as f64, 1.0);
    assert_eq!(t.scale.y as f64, 2.0);
    acc += t.translation.z as f64;
    acc += ufbx::evaluate_transform_flags(over_anim, node, 0.25, 0)
        .scale
        .y as f64;
    acc += ufbx::evaluate_prop(over_anim, &node.element, "Lcl Translation", 0.25)
        .value_vec4
        .x as f64;

    // Baked-keyframe interpolation over slices from a baked result.
    let baked = ufbx::bake_anim(scene, anim, Default::default()).expect("bake_anim");
    for node in &baked.nodes {
        acc += ufbx::evaluate_baked_vec3(node.translation_keys.as_ref(), 0.3).x as f64;
        acc += ufbx::evaluate_baked_quat(node.rotation_keys.as_ref(), 0.3).w as f64;
    }
    assert!(acc.is_finite());
}

#[test]
fn public_matrix_value_helpers_from_shared_refs() {
    let matrix = ufbx::Matrix {
        m00: -2.0,
        m11: 4.0,
        m22: 8.0,
        m03: 10.0,
        m13: 20.0,
        m23: 30.0,
        ..Default::default()
    };
    let vector = ufbx::Vec3 {
        x: 1.0,
        y: 2.0,
        z: 3.0,
    };

    let position = ufbx::transform_position(&matrix, vector);
    assert_eq!(position.x, 8.0);
    assert_eq!(position.y, 28.0);
    assert_eq!(position.z, 54.0);

    let direction = ufbx::transform_direction(&matrix, vector);
    assert_eq!(direction.x, -2.0);
    assert_eq!(direction.y, 8.0);
    assert_eq!(direction.z, 24.0);

    assert_eq!(ufbx::matrix_determinant(&matrix), -64.0);
    let inverse = ufbx::matrix_invert(&matrix);
    assert_eq!(inverse.m00, -0.5);
    assert_eq!(inverse.m11, 0.25);
    assert_eq!(inverse.m22, 0.125);
    assert_eq!(inverse.m03, 5.0);
    assert_eq!(inverse.m13, -5.0);
    assert_eq!(inverse.m23, -3.75);

    let normals = ufbx::matrix_for_normals(&matrix);
    assert_eq!(normals.m00, -32.0);
    assert_eq!(normals.m11, 16.0);
    assert_eq!(normals.m22, 8.0);
    assert_eq!(normals.m03, 0.0);
    assert_eq!(normals.m13, 0.0);
    assert_eq!(normals.m23, 0.0);

    let squared = ufbx::matrix_mul(&matrix, &matrix);
    assert_eq!(squared.m00, 4.0);
    assert_eq!(squared.m11, 16.0);
    assert_eq!(squared.m22, 64.0);
    assert_eq!(squared.m03, -10.0);
    assert_eq!(squared.m13, 100.0);
    assert_eq!(squared.m23, 270.0);

    let translation = ufbx::Matrix {
        m00: 1.0,
        m11: 1.0,
        m22: 1.0,
        m03: 1.0,
        m13: 2.0,
        m23: 3.0,
        ..Default::default()
    };
    let scale = ufbx::Matrix {
        m00: 2.0,
        m11: 3.0,
        m22: 4.0,
        ..Default::default()
    };
    let translate_then_scale = ufbx::matrix_mul(&translation, &scale);
    assert_eq!(translate_then_scale.m03, 1.0);
    assert_eq!(translate_then_scale.m13, 2.0);
    assert_eq!(translate_then_scale.m23, 3.0);
    let scale_then_translate = ufbx::matrix_mul(&scale, &translation);
    assert_eq!(scale_then_translate.m03, 2.0);
    assert_eq!(scale_then_translate.m13, 6.0);
    assert_eq!(scale_then_translate.m23, 12.0);

    let transform = ufbx::Transform {
        translation: ufbx::Vec3 {
            x: 5.0,
            y: -6.0,
            z: 7.0,
        },
        rotation: ufbx::Quat {
            x: 0.0,
            y: 0.0,
            z: 1.0,
            w: 0.0,
        },
        scale: ufbx::Vec3 {
            x: -2.0,
            y: 3.0,
            z: 4.0,
        },
    };
    let transformed = ufbx::transform_to_matrix(&transform);
    let decomposed = ufbx::matrix_to_transform(&transformed);
    let rebuilt = ufbx::transform_to_matrix(&decomposed);
    for (actual, expected) in [
        rebuilt.m00,
        rebuilt.m10,
        rebuilt.m20,
        rebuilt.m01,
        rebuilt.m11,
        rebuilt.m21,
        rebuilt.m02,
        rebuilt.m12,
        rebuilt.m22,
        rebuilt.m03,
        rebuilt.m13,
        rebuilt.m23,
    ]
    .into_iter()
    .zip([
        transformed.m00,
        transformed.m10,
        transformed.m20,
        transformed.m01,
        transformed.m11,
        transformed.m21,
        transformed.m02,
        transformed.m12,
        transformed.m22,
        transformed.m03,
        transformed.m13,
        transformed.m23,
    ]) {
        assert!((actual - expected).abs() < 1.0e-5);
    }

    let singular = ufbx::Matrix {
        m00: 1.0,
        m11: 0.0,
        m22: 1.0,
        ..Default::default()
    };
    let zero = ufbx::matrix_invert(&singular);
    for value in [
        zero.m00, zero.m10, zero.m20, zero.m01, zero.m11, zero.m21, zero.m02, zero.m12, zero.m22,
        zero.m03, zero.m13, zero.m23,
    ] {
        assert_eq!(value, 0.0);
    }
}

/// Mesh geometry helpers from shared refs: indexed vertex-attribute getters,
/// face normals, topology edge navigation, normal mapping/accumulation, and
/// bounds rejection over caller-provided buffers.
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
    if let Some(face) = mesh.faces.first().copied() {
        let mut triangles = vec![0u32; face.num_indices.saturating_sub(2) as usize * 3];
        acc += ufbx::triangulate_face(&mut triangles, mesh, face) as f64;
    }

    // These fixtures guarantee that the less common generated vertex wrappers
    // execute instead of being conditionally skipped by the plain cube.
    let attribute_root = load("maya_uv_and_color_sets_6100_binary.fbx");
    let mut saw_uv = false;
    let mut saw_color = false;
    for attribute_mesh in &attribute_root.meshes {
        if !attribute_mesh.vertex_uv.indices.is_empty() {
            acc += ufbx::get_vertex_vec2(&attribute_mesh.vertex_uv, 0).x as f64;
            saw_uv = true;
        }
        if !attribute_mesh.vertex_color.indices.is_empty() {
            acc += ufbx::get_vertex_vec4(&attribute_mesh.vertex_color, 0).w as f64;
            saw_color = true;
        }
    }
    assert!(saw_uv && saw_color);

    let crease_root = load("maya_vertex_crease_6100_binary.fbx");
    let mut saw_crease = false;
    for crease_mesh in &crease_root.meshes {
        if !crease_mesh.vertex_crease.indices.is_empty() {
            acc += ufbx::get_vertex_real(&crease_mesh.vertex_crease, 0) as f64;
            saw_crease = true;
        }
    }
    assert!(saw_crease);

    let mut topo = vec![ufbx::TopoEdge::default(); mesh.num_indices];
    ufbx::compute_topology(mesh, &mut topo);
    acc += ufbx::topo_next_vertex_edge(&topo, 0) as f64;
    acc += ufbx::topo_prev_vertex_edge(&topo, 0) as f64;
    let mut normal_indices = vec![0u32; mesh.num_indices];
    let num_normals = ufbx::generate_normal_mapping(mesh, &topo, &mut normal_indices, false);
    acc += num_normals as f64;

    let mut short_mapping = vec![0x1234_5678; mesh.num_indices - 1];
    let short_mapping_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        ufbx::generate_normal_mapping(mesh, &topo, &mut short_mapping, false);
    }));
    assert!(short_mapping_result.is_err());
    assert!(short_mapping.iter().all(|&index| index == 0x1234_5678));

    let mut normals = vec![ufbx::Vec3::default(); num_normals];
    ufbx::compute_normals(mesh, &mesh.vertex_position, &normal_indices, &mut normals);
    for normal in &normals {
        acc += normal.x as f64 + normal.y as f64 + normal.z as f64;
    }

    let mut untouched_normals = vec![
        ufbx::Vec3 {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        };
        num_normals
    ];
    let short_normals_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        ufbx::compute_normals(
            mesh,
            &mesh.vertex_position,
            &normal_indices[..mesh.num_indices - 1],
            &mut untouched_normals,
        );
    }));
    assert!(short_normals_result.is_err());
    assert!(untouched_normals
        .iter()
        .all(|normal| normal.x == 1.0 && normal.y == 2.0 && normal.z == 3.0));

    // Every mapped normal index must address the caller-provided output run.
    let invalid_indices = vec![num_normals as u32; mesh.num_indices];
    let invalid_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut output = vec![ufbx::Vec3::default(); num_normals];
        ufbx::compute_normals(mesh, &mesh.vertex_position, &invalid_indices, &mut output);
    }));
    assert!(invalid_result.is_err());

    assert!(acc.is_finite());
}

/// Deformer helpers from shared refs: skin vertex matrices, blend-shape
/// offset lookups, blend-weight evaluation and bind-pose queries.
#[test]
fn public_deform_helpers_from_shared_refs() {
    // Compile-shape regression: the returned entry belongs to `pose`; `node`
    // is only an independently borrowed search key.
    fn find_for_pose<'a>(pose: &'a ufbx::Pose, node: &ufbx::Node) -> Option<&'a ufbx::BonePose> {
        ufbx::get_bone_pose(pose, node)
    }

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
            if let Some(found) = find_for_pose(pose, node) {
                acc += found.bone_to_world.m00 as f64;
            }
        }
    }

    let pose_root = load("maya_poses_7700_ascii.fbx");
    let pose_scene: &Scene = &pose_root;
    let pose = pose_scene.poses.first().expect("missing pose");
    let bone_pose = pose.bone_poses.first().expect("missing bone pose");
    let found =
        find_for_pose(pose, bone_pose.bone_node.as_ref()).expect("missing indexed bone pose");
    assert!(std::ptr::eq(found, bone_pose));

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

// -- Geometry caches (memory-backed file callback)

/// `open_file_cb` that serves every requested file from memory: ufbx's default
/// stream calls libc `fopen`, which Miri cannot emulate.
fn open_from_memory(name: &str, _info: &ufbx::OpenFileInfo) -> Option<ufbx::Stream> {
    let data = std::fs::read(name).ok()?;
    Some(ufbx::Stream::Read(Box::new(std::io::Cursor::new(data))))
}

unsafe extern "C" fn mark_broken_stream_closed(user: *mut std::ffi::c_void) {
    // SAFETY: `broken_open_file` installs the pointer to its caller-owned
    // `AtomicBool`, which remains live for the synchronous cache load.
    unsafe { &*(user as *const std::sync::atomic::AtomicBool) }
        .store(true, std::sync::atomic::Ordering::Relaxed);
}

unsafe extern "C" fn broken_open_file(
    user: *mut std::ffi::c_void,
    stream: *mut ufbx::RawStream,
    _filename: *const u8,
    _filename_len: usize,
    _info: *const ufbx::OpenFileInfo,
) -> bool {
    // SAFETY: ufbx supplies a live out-parameter for one `RawStream`.
    unsafe {
        (*stream).close_fn = Some(mark_broken_stream_closed);
        (*stream).user = user;
    }
    true
}

#[test]
fn geometry_cache_rejects_stream_without_read_callback() {
    let closed = std::sync::atomic::AtomicBool::new(false);
    let raw = ufbx::RawOpenFileCb {
        fn_: Some(broken_open_file),
        user: (&closed as *const std::sync::atomic::AtomicBool)
            .cast_mut()
            .cast(),
    };
    let result = ufbx::load_geometry_cache(
        "broken.xml",
        ufbx::GeometryCacheOpts {
            // SAFETY: the callback writes only the live `RawStream` out-parameter;
            // its user pointer remains valid for this synchronous call.
            open_file_cb: ufbx::OpenFileCb::Raw(unsafe { ufbx::Unsafe::new(raw) }),
            ..Default::default()
        },
    );
    let error = match result {
        Ok(_) => panic!("a stream without read_fn must fail"),
        Err(error) => error,
    };
    assert_eq!(error.type_, ufbx::ErrorType::Io);
    assert!(closed.load(std::sync::atomic::Ordering::Relaxed));
}

/// Mirrors `ufbxt_test_sine_cache` (test_cache.h): the `pCubeShape1` channel
/// of each sine cache, sampled at 1/240s steps, follows a known sine.
fn check_sine_cache(path: &str, begin: f64, end: f64, err_threshold: f64) {
    let cache = ufbx::load_geometry_cache(
        &data_path(path),
        ufbx::GeometryCacheOpts {
            open_file_cb: ufbx::OpenFileCb::Ref(&open_from_memory),
            ..Default::default()
        },
    )
    .unwrap_or_else(|e| panic!("failed to load cache {}: {:?}", path, e));
    assert_eq!(cache.channels.len(), 2);

    let mut found_cube1 = false;
    for channel in &cache.channels {
        assert_eq!(
            channel.interpretation,
            ufbx::CacheInterpretation::VertexPosition
        );
        if channel.name.as_ref() != "pCubeShape1" {
            continue;
        }
        found_cube1 = true;

        let mut pos = [ufbx::Vec3::default(); 64];
        let mut time = begin;
        while time <= end + 0.0001 {
            let num_verts = ufbx::sample_geometry_cache_vec3(
                channel,
                time,
                &mut pos,
                ufbx::GeometryCacheDataOpts {
                    open_file_cb: ufbx::OpenFileCb::Ref(&open_from_memory),
                    ..Default::default()
                },
            );
            assert_eq!(num_verts, 36);

            let t = (time - 1.0 / 24.0) / (29.0 / 24.0) * 4.0;
            let pi2 = std::f64::consts::PI * 2.0;
            for v in &pos[..num_verts] {
                let sx = ((v.y as f64 + t * 0.5) * pi2).sin() * 0.25;
                let mut vx = v.x as f64;
                vx += if vx > 0.0 { -0.5 } else { 0.5 };
                assert!(
                    (vx - sx).abs() <= err_threshold,
                    "{}: t={} vx={} sx={}",
                    path,
                    time,
                    vx,
                    sx
                );
            }
            time += 0.1 / 24.0;
        }
    }
    assert!(found_cube1);
}

#[test]
fn geometry_cache_sine_regular() {
    check_sine_cache(
        "caches/sine_mxsf_regular/cache.xml",
        1.0 / 24.0,
        29.0 / 24.0,
        0.008,
    );
}

#[test]
fn geometry_cache_sine_undersample() {
    check_sine_cache(
        "caches/sine_mcmf_undersample/cache.xml",
        1.0 / 24.0,
        29.0 / 24.0,
        0.04,
    );
}

/// The scene-side path: `load_external_files` resolves the cache next to the
/// FBX through the callback, `evaluate_caches` applies it to the skinned
/// vertices, and `evaluate_scene` re-samples it at another time.
#[test]
fn geometry_cache_through_scene() {
    let name = "maya_cache_sine_6100_binary.fbx";
    let path = data_path(name);
    let data = read_data(name);
    let root = ufbx::load_memory(
        &data,
        LoadOpts {
            filename: ufbx::StringOpt::Ref(&path),
            load_external_files: true,
            evaluate_caches: true,
            open_file_cb: ufbx::OpenFileCb::Ref(&open_from_memory),
            ..Default::default()
        },
    )
    .unwrap_or_else(|e| panic!("failed to load {}: {:?}", name, e));
    let scene: &Scene = &root;

    assert!(!scene.cache_deformers.is_empty());
    for deformer in &scene.cache_deformers {
        let file = deformer
            .file
            .as_ref()
            .expect("cache deformer resolves its file");
        assert!(
            file.external_cache.is_some(),
            "external cache loaded through the callback"
        );
        assert!(deformer.external_cache.is_some());
    }

    let mut acc = 0.0f64;
    for mesh in &scene.meshes {
        if mesh.cache_deformers.is_empty() {
            continue;
        }
        for i in 0..mesh.num_indices {
            let p = mesh.skinned_position[i];
            acc += p.x as f64 + p.y as f64 + p.z as f64;
        }
    }
    assert!(acc.is_finite());

    let evaluated = ufbx::evaluate_scene(
        scene,
        &scene.anim,
        0.5,
        ufbx::EvaluateOpts {
            evaluate_caches: true,
            load_external_files: true,
            open_file_cb: ufbx::OpenFileCb::Ref(&open_from_memory),
            ..Default::default()
        },
    )
    .expect("evaluate_scene with caches");
    let mut acc2 = 0.0f64;
    for mesh in &evaluated.meshes {
        for i in 0..mesh.num_indices {
            let p = mesh.skinned_position[i];
            acc2 += p.x as f64 + p.y as f64 + p.z as f64;
        }
    }
    assert!(acc2.is_finite());
}

#[test]
fn geometry_cache_retries_absolute_filename() {
    let name = "maya_cache_sine_6100_binary.fbx";
    let path = data_path(name);
    let data = read_data(name);
    let mut requests = Vec::new();
    let mut open_missing = |filename: &str, _info: &ufbx::OpenFileInfo| {
        requests.push(filename.to_owned());
        None
    };
    let result = ufbx::load_memory(
        &data,
        LoadOpts {
            filename: ufbx::StringOpt::Ref(&path),
            load_external_files: true,
            open_file_cb: ufbx::OpenFileCb::Mut(&mut open_missing),
            ..Default::default()
        },
    );
    let error = match result {
        Ok(_) => panic!("missing geometry cache must fail"),
        Err(error) => error,
    };

    assert_eq!(error.type_, ufbx::ErrorType::ExternalFileNotFound);
    assert_eq!(requests.len(), 2);
    assert!(requests[0].ends_with(".xml"));
    assert_ne!(requests[0], requests[1]);
    assert!(requests[1].starts_with(r"D:\Dev\clean\ufbx\data\"));
    assert_eq!(error.info(), requests[1]);
}

// -- Threaded loader (std::thread pool over the raw `ufbx_thread_pool` interface)

/// One `std::thread` per `run_fn` batch, joined by `wait_fn`. The task indices
/// of a batch run sequentially on that thread; what Miri checks is that the
/// per-task buffers ufbx hands the pool are disjoint from the loader thread's.
#[derive(Default)]
struct TestThreadPool {
    groups: Vec<Vec<std::thread::JoinHandle<()>>>,
    tasks_run: u32,
}

unsafe extern "C" fn test_pool_run(
    user: *mut std::ffi::c_void,
    ctx: ufbx::ThreadPoolContext,
    group: u32,
    start_index: u32,
    count: u32,
) {
    // SAFETY: `user` is the `TestThreadPool` the test owns for the whole load,
    // and ufbx calls the pool from the loader thread only.
    let pool = unsafe { &mut *(user as *mut TestThreadPool) };
    pool.tasks_run += count;
    let group = group as usize;
    if pool.groups.len() <= group {
        pool.groups.resize_with(group + 1, Vec::new);
    }
    pool.groups[group].push(std::thread::spawn(move || {
        for index in start_index..start_index + count {
            // SAFETY: `ctx` is the live pool context ufbx passed to `run_fn`, and
            // `index` is inside the batch it asked us to run.
            unsafe { ufbx::thread_pool_run_task(ctx, index) };
        }
    }));
}

unsafe extern "C" fn test_pool_wait(
    user: *mut std::ffi::c_void,
    _ctx: ufbx::ThreadPoolContext,
    group: u32,
    _max_index: u32,
) {
    // SAFETY: as in `test_pool_run`.
    let pool = unsafe { &mut *(user as *mut TestThreadPool) };
    if let Some(handles) = pool.groups.get_mut(group as usize) {
        for handle in handles.drain(..) {
            handle.join().expect("ufbx task thread panicked");
        }
    }
}

unsafe extern "C" fn test_pool_free(user: *mut std::ffi::c_void, _ctx: ufbx::ThreadPoolContext) {
    // SAFETY: as in `test_pool_run`.
    let pool = unsafe { &mut *(user as *mut TestThreadPool) };
    for handles in &mut pool.groups {
        for handle in handles.drain(..) {
            handle.join().expect("ufbx task thread panicked");
        }
    }
}

fn load_threaded(name: &str) -> f64 {
    let data = read_data(name);
    let mut pool = TestThreadPool::default();
    let raw_pool = ufbx::RawThreadPool {
        init_fn: None,
        run_fn: Some(test_pool_run),
        wait_fn: Some(test_pool_wait),
        free_fn: Some(test_pool_free),
        user: &raw mut pool as *mut std::ffi::c_void,
    };
    let root = ufbx::load_memory(
        &data,
        LoadOpts {
            thread_opts: ufbx::ThreadOpts {
                // SAFETY: the callbacks above implement the `ufbx_thread_pool`
                // contract (every index in a batch runs exactly once, `wait_fn`
                // blocks until the batch is done) and `pool` outlives the load.
                pool: ufbx::ThreadPool::Raw(unsafe { ufbx::Unsafe::new(raw_pool) }),
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .unwrap_or_else(|e| panic!("failed to load {} threaded: {:?}", name, e));
    assert!(
        pool.groups.iter().all(|g| g.is_empty()),
        "every batch was waited for"
    );
    assert!(
        pool.tasks_run > 0,
        "{}: the loader never handed the pool a task",
        name
    );
    walk(&root)
}

/// The ASCII parser hands array parsing to the pool.
#[test]
fn threaded_load_ascii() {
    let threaded = load_threaded("maya_cube_7500_ascii.fbx");
    let plain = walk(&load("maya_cube_7500_ascii.fbx"));
    assert_eq!(threaded.to_bits(), plain.to_bits());
}

/// The binary parser hands DEFLATE-compressed arrays (256+ encoded bytes,
/// `UFBXI_MIN_THREADED_DEFLATE_BYTES`) to the pool; this file has six such.
#[test]
fn threaded_load_binary() {
    let threaded = load_threaded("blender_293_instancing_7400_binary.fbx");
    let plain = walk(&load("blender_293_instancing_7400_binary.fbx"));
    assert_eq!(threaded.to_bits(), plain.to_bits());
}
