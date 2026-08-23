from typing import Optional, List

import ufbx_ir as ir
import argparse
import os
import json
import re

g_argv = None

uses = r"""
use std::ffi::{c_void};
use std::{marker, result, ptr, mem, str};
use std::fmt::{self, Debug};
use std::ops::{Deref, BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, FnMut, Index};
use crate::prelude::{Real, List, Ref, RefList, String, Blob, RawString, RawBlob, RawList, Unsafe, RawEnum, ExternalRef, InlineBuf, VertexStream, Arena, ToRaw, StringOpt, BlobOpt, ListOpt, ThreadPoolContext, OpenFileContext, format_flags};
""".strip()

post_ffi = r"""

pub struct SceneRoot {
    scene: *mut Scene,
    _marker: marker::PhantomData<Scene>,
}

pub struct MeshRoot {
    mesh: *mut Mesh,
    _marker: marker::PhantomData<Mesh>,
}

pub struct LineCurveRoot {
    line_curve: *mut LineCurve,
    _marker: marker::PhantomData<LineCurve>,
}

pub struct GeometryCacheRoot {
    cache: *mut GeometryCache,
    _marker: marker::PhantomData<GeometryCache>,
}

pub struct AnimRoot {
    anim: *mut Anim,
    _marker: marker::PhantomData<Anim>,
}

pub struct BakedAnimRoot {
    anim: *mut BakedAnim,
    _marker: marker::PhantomData<BakedAnim>,
}

impl SceneRoot {
    fn new(scene: *mut Scene) -> SceneRoot {
        SceneRoot {
            scene,
            _marker: marker::PhantomData,
        }
    }
}

impl MeshRoot {
    fn new(mesh: *mut Mesh) -> MeshRoot {
        MeshRoot {
            mesh,
            _marker: marker::PhantomData,
        }
    }
}

impl LineCurveRoot {
    fn new(line_curve: *mut LineCurve) -> LineCurveRoot {
        LineCurveRoot {
            line_curve,
            _marker: marker::PhantomData,
        }
    }
}


impl GeometryCacheRoot {
    fn new(cache: *mut GeometryCache) -> GeometryCacheRoot {
        GeometryCacheRoot {
            cache,
            _marker: marker::PhantomData,
        }
    }
}

impl AnimRoot {
    fn new(anim: *mut Anim) -> AnimRoot {
        AnimRoot {
            anim,
            _marker: marker::PhantomData,
        }
    }
}

impl BakedAnimRoot {
    fn new(anim: *mut BakedAnim) -> BakedAnimRoot {
        BakedAnimRoot {
            anim,
            _marker: marker::PhantomData,
        }
    }
}

impl Drop for SceneRoot {
    fn drop(&mut self) {
        unsafe { crate::native::api::free_scene(self.scene) }
    }
}

impl Drop for MeshRoot {
    fn drop(&mut self) {
        unsafe { crate::native::api::free_mesh(self.mesh) }
    }
}

impl Drop for LineCurveRoot {
    fn drop(&mut self) {
        unsafe { crate::native::api::free_line_curve(self.line_curve) }
    }
}

impl Drop for GeometryCacheRoot {
    fn drop(&mut self) {
        unsafe { crate::native::api::free_geometry_cache(self.cache) }
    }
}

impl Drop for AnimRoot {
    fn drop(&mut self) {
        unsafe { crate::native::api::free_anim(self.anim) }
    }
}

impl Drop for BakedAnimRoot {
    fn drop(&mut self) {
        unsafe { crate::native::api::free_baked_anim(self.anim) }
    }
}

impl Clone for SceneRoot {
    fn clone(&self) -> Self {
        unsafe { crate::native::api::retain_scene(self.scene) }
        SceneRoot::new(self.scene)
    }
}

impl Clone for MeshRoot {
    fn clone(&self) -> Self {
        unsafe { crate::native::api::retain_mesh(self.mesh) }
        MeshRoot::new(self.mesh)
    }
}

impl Clone for LineCurveRoot {
    fn clone(&self) -> Self {
        unsafe { crate::native::api::retain_line_curve(self.line_curve) }
        LineCurveRoot::new(self.line_curve)
    }
}

impl Clone for GeometryCacheRoot {
    fn clone(&self) -> Self {
        unsafe { crate::native::api::retain_geometry_cache(self.cache) }
        GeometryCacheRoot::new(self.cache)
    }
}

impl Clone for AnimRoot {
    fn clone(&self) -> Self {
        unsafe { crate::native::api::retain_anim(self.anim) }
        AnimRoot::new(self.anim)
    }
}

impl Clone for BakedAnimRoot {
    fn clone(&self) -> Self {
        unsafe { crate::native::api::retain_baked_anim(self.anim) }
        BakedAnimRoot::new(self.anim)
    }
}

impl Deref for SceneRoot {
    type Target = Scene;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.scene }
    }
}

impl Deref for MeshRoot {
    type Target = Mesh;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.mesh }
    }
}

impl Deref for LineCurveRoot {
    type Target = LineCurve;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.line_curve }
    }
}

impl Deref for GeometryCacheRoot {
    type Target = GeometryCache;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.cache }
    }
}

impl Deref for AnimRoot {
    type Target = Anim;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.anim }
    }
}

impl Deref for BakedAnimRoot {
    type Target = BakedAnim;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.anim }
    }
}

unsafe impl Send for SceneRoot {}
unsafe impl Sync for SceneRoot {}

unsafe impl Send for MeshRoot {}
unsafe impl Sync for MeshRoot {}

unsafe impl Send for LineCurveRoot {}
unsafe impl Sync for LineCurveRoot {}

unsafe impl Send for GeometryCacheRoot {}
unsafe impl Sync for GeometryCacheRoot {}

unsafe impl Send for AnimRoot {}
unsafe impl Sync for AnimRoot {}

unsafe impl Send for BakedAnimRoot {}
unsafe impl Sync for BakedAnimRoot {}

""".strip()

types = { }
structs = { }
enums = { }
enum_values = { }
functions = { }

# Map ufbx_* C name -> (native_module, native_fn) for capi shims whose body is
# a pure verbatim forward `crate::native::MOD::FN(<params in order>)`. Populated
# from capi.rs at startup; see parse_capi_forwarders.
capi_forward = { }

file: ir.File = None

alloc_types = {
    "scene": "SceneRoot",
    "mesh": "MeshRoot",
    "line": "LineCurveRoot",
    "geometryCache": "GeometryCacheRoot",
    "anim": "AnimRoot",
    "bakedAnim": "BakedAnimRoot",
}

raw_types = {
    "ufbx_vertex_stream",
}

callback_signatures = {
    "ufbx_open_file_cb": "(&str, &OpenFileInfo) -> Option<Stream>",
    "ufbx_close_memory_cb": "(*mut c_void, usize)",
    "ufbx_progress_cb": "(&Progress) -> ProgressResult",
}

primitive_types = {
    "void": "c_void",
    "char": "u8",
    # NOTE(ufbx-rs-native): upstream generate_rust.py mapped these to i32/u32,
    # which breaks #[repr(C)] layout for uint8_t fields (ufbx_material_map.value_components).
    # Caught by tests/layout.rs.
    "int8_t": "i8",
    "uint8_t": "u8",
    "int32_t": "i32",
    "uint32_t": "u32",
    "int64_t": "i64",
    "uint64_t": "u64",
    "float": "f32",
    "double": "f64",
    "size_t": "usize",
    "uintptr_t": "usize",
    "ptrdiff_t": "isize",
    "bool": "bool",
    "ufbx_real": "Real",
    "ufbx_thread_pool_context": "ThreadPoolContext",
    "ufbx_open_file_context": "OpenFileContext",
}

default_derive_types = {
    "ufbx_error_frame",
    "ufbx_error_type",
    "ufbx_error",
    "ufbx_panic",
}

# NOTE(ufbx-rs-native): non-POD structs that must still derive Clone, Copy —
# C struct assignment is memcpy (PORTING.md checklist #15) and the native port
# copies these by value (e.g. `ufbxi_allocator` embeds ufbx_allocator_opts and
# is stack-copied in ufbxi_release_ref, ufbx.c:30277-30283). All fields are
# trivially copyable (fn pointers / raw pointers / usize).
copy_derive_types = {
    "ufbx_allocator_opts",
    # `ufbx_prop` is copied by value all over the reader: the property sort
    # (`ufbxi_macro_stable_sort(ufbx_prop, ...)`, ufbx.c:11881) and
    # `ufbxi_deduplicate_properties` (`ps[dst++] = ps[src++]`, ufbx.c:11894)
    # are plain C struct assignments.
    "ufbx_prop",
    # `ufbx_shader_prop_binding` is sorted by value in `ufbxi_sort_shader_prop_bindings`
    # (`ufbxi_macro_stable_sort(ufbx_shader_prop_binding, ...)`, ufbx.c:14692)
    # which is a plain C struct assignment through the sort scratch.
    "ufbx_shader_prop_binding",
    # `ufbx_connection` is sorted by value in `ufbxi_sort_connections`
    # (`ufbxi_macro_stable_sort(ufbx_connection, ...)`, ufbx.c:18651) and
    # bulk-copied src->dst in `ufbxi_resolve_connections` (ufbx.c:18769).
    # Contains `ufbx_element*` (emitted as `Ref<Element>`, Copy in prelude.rs).
    "ufbx_connection",
    # `ufbx_name_element` is sorted by value in `ufbxi_sort_name_elements`
    # (`ufbxi_macro_stable_sort(ufbx_name_element, ...)`, ufbx.c:18587), a plain
    # C struct assignment through the sort scratch.
    "ufbx_name_element",
    # `ufbx_anim_prop` is sorted by value in `ufbxi_sort_anim_props`
    # (`ufbxi_macro_stable_sort(ufbx_anim_prop, ...)`, ufbx.c:19301), a plain C
    # struct assignment through the sort scratch. Contains `ufbx_element*` /
    # `ufbx_anim_value*` (emitted as `Ref<T>`, Copy in prelude.rs).
    "ufbx_anim_prop",
}

ignore_types = {
    "ufbx_string",
    "ufbx_blob",
}

# NOTE(ufbx-rs-native): structs that get `View<T, M>` field accessors emitted
# into src/generated_views.rs (crate-internal; soundness model in
# src/native/view.rs). The set is the mesh-view campaign surface: `ufbx_mesh`
# plus every aggregate reached from it by field access. A field whose type is
# itself in this set gets an in-place `&View<F, M>` projection accessor; every
# other field gets a by-value read. Order is emission order.
view_accessor_structs = [
    "ufbx_element",
    "ufbx_props",
    "ufbx_prop",
    "ufbx_face",
    "ufbx_edge",
    "ufbx_vertex_attrib",
    "ufbx_vertex_real",
    "ufbx_vertex_vec2",
    "ufbx_vertex_vec3",
    "ufbx_vertex_vec4",
    "ufbx_uv_set",
    "ufbx_color_set",
    "ufbx_mesh_part",
    "ufbx_face_group",
    "ufbx_mesh",
    "ufbx_skin_deformer",
    "ufbx_subdivision_result",
    "ufbx_cache_deformer",
    "ufbx_material_map",
    "ufbx_material_feature_info",
    "ufbx_material_texture",
    "ufbx_shader_prop_binding",
    "ufbx_prop_override",
    "ufbx_dom_node",
    "ufbx_anim_curve",
    "ufbx_anim_value",
    "ufbx_keyframe",
    "ufbx_shader",
    "ufbx_shader_binding",
    "ufbx_shader_texture",
    "ufbx_shader_texture_input",
]

# `(struct, field)` pairs whose READ accessor is hand-written instead of
# generated (the Mut setter / raw-pointer accessors are still emitted):
#   * `ufbx_props.defaults`: the sound read is the mode-preserving
#     `Option<&View<Props, M>>` nav accessor in native/parse.rs (niche-packed
#     bare-pointer read; see its SAFETY comment) — a plain by-value read of
#     `Option<Ref<Props>>` would push call sites back through `Ref::as_ref`,
#     whose `&Props` formation is the Stacked Borrows trap the view avoids.
#   * `ufbx_anim_value.curves`: the sound read is the indexed
#     `Option<&View<AnimCurve, M>>` nav accessor `curve_view` in native/api.rs
#     (niche-packed bare-pointer read of one slot) — a by-value read of the
#     whole `[Option<Ref<AnimCurve>>; 3]` array would push call sites back
#     through `Ref::as_ref`, the same Stacked Borrows trap as above.
view_accessor_skip_read = {
    ("ufbx_props", "defaults"),
    ("ufbx_anim_value", "curves"),
}

ignore_non_raw = {
    "ufbx_open_file",
    "ufbx_open_memory",
    "ufbx_open_file_ctx",
    "ufbx_open_memory_ctx",
    "ufbx_default_open_file",
}

force_mut_args = {
    ("ufbx_generate_indices", 0),
}

# NOTE(ufbx-rs-native): `(struct, field)` pairs where ufbx.h omits the
# `ufbx_nullable` annotation on a pointer field that the C implementation
# nevertheless leaves or sets NULL. Without the override the field is emitted as
# `Ref<T>` (`#[repr(transparent)]` over `NonNull<T>`) and the port would store an
# invalid (null) `NonNull` into a live scene field.
#   * `ufbx_shader_texture.main_texture` (ufbx.h:2843): NULL for every shader
#     whose type is not `UFBX_SHADER_TEXTURE_SELECT_OUTPUT` (only ufbx.c:20531
#     ever assigns it), and explicitly re-nulled by the cyclic-main-texture pass
#     (`shader->main_texture = NULL;` ufbx.c:20723). C null-tests it at
#     ufbx.c:20705/20708/20719/20735/20747; the doc comment above the field
#     ("Only specified if ...") describes an optional field.
#   * `ufbx_shader_texture_input.prop` (ufbx.h:2802): zeroed by
#     `memset(input, 0, sizeof(ufbx_shader_texture_input))` (ufbx.c:20651) and
#     null-tested by `ufbxi_update_shader_texture` (ufbx.c:20499) — while the
#     sibling `texture_prop`/`texture_enabled_prop` two lines down (ufbx.h:2805,
#     2808) *are* annotated, so the omission is an upstream inconsistency.
# Remove an entry once upstream ufbx.h annotates the field. See COMPAT.md §2.
nullable_field_overrides = {
    ("ufbx_shader_texture", "main_texture"),
    ("ufbx_shader_texture_input", "prop"),
}

override_functions = { }
override_member_functions = { }

override_functions["ufbx_find_real_len"] = """
// TODO: Property find functions
"""

override_functions["ufbx_find_int_len"] = """
// TODO: Property find functions
"""

override_functions["ufbx_find_bool_len"] = """
// TODO: Property find functions
"""

override_functions["ufbx_find_vec3_len"] = """
// TODO: Property find functions
"""

override_functions["ufbx_find_string_len"] = """
// TODO: Property find functions
"""

override_functions["ufbx_find_prop_concat"] = """
// TODO: ufbx_find_prop_concat()
"""

override_functions["ufbx_find_shader_prop_len"] = """
pub fn find_shader_prop<'a>(shader: &'a Shader, name: &'a str) -> &'a str {
    let result = unsafe { crate::native::api::find_shader_prop_len(shader as *const Shader, name.as_bytes()) };
    unsafe { result.as_static_ref() }
}
"""


# The dom_* family: native fns take mode-generic views; the safe wrappers mint
# read-only `Const` views from the caller's `&DomNode` (the mint every readable
# provenance supports) and call native directly. Signatures stay verbatim
# upstream-parity (including their historical free lifetimes).
override_functions["ufbx_dom_find_len"] = """
pub fn dom_find<'a>(parent: &DomNode, name: &str) -> Option<&'a DomNode> {
    let result = unsafe {
        crate::native::api::dom_find_len(
            crate::native::view::View::<DomNode, crate::native::view::Const>::from_ptr(parent as *const DomNode),
            name.as_bytes(),
        )
    };
    result.map(|node| unsafe { &*node.as_ptr() })
}
"""

# The native shader finders take views (`Const` is legal over these
# `&`-derived pointers), so the safe wrappers mint the view instead of casting
# to a raw pointer.
override_functions["ufbx_find_shader_prop_len"] = """
pub fn find_shader_prop<'a>(shader: &'a Shader, name: &'a str) -> &'a str {
    let result = crate::native::api::find_shader_prop_len(
        Some(unsafe {
            crate::native::view::View::<Shader, crate::native::view::Const>::from_ptr(shader as *const Shader)
        }),
        name.as_bytes(),
    );
    unsafe { result.as_static_ref() }
}
"""

override_functions["ufbx_find_shader_prop_bindings_len"] = """
#[allow(clippy::needless_lifetimes)]
pub fn find_shader_prop_bindings<'a>(shader: &'a Shader, name: &str) -> &'a [ShaderPropBinding] {
    let result = crate::native::api::find_shader_prop_bindings_len(
        Some(unsafe {
            crate::native::view::View::<Shader, crate::native::view::Const>::from_ptr(shader as *const Shader)
        }),
        name.as_bytes(),
    );
    unsafe { result.as_static_ref() }
}
"""

override_functions["ufbx_find_shader_texture_input_len"] = """
pub fn find_shader_texture_input<'a>(
    shader: &ShaderTexture,
    name: &str,
) -> Option<&'a ShaderTextureInput> {
    let result = crate::native::api::find_shader_texture_input_len(
        unsafe {
            crate::native::view::View::<ShaderTexture, crate::native::view::Const>::from_ptr(shader as *const ShaderTexture)
        },
        name.as_bytes(),
    );
    result.map(|input| unsafe { &*input.as_ptr() })
}
"""

# The native evaluate fns take `Option<&View<_, Const>>` (safe fns; a `Const`
# view is legal over these `&`-derived pointers), so the safe wrappers mint the
# view instead of casting to a raw pointer.
override_functions["ufbx_evaluate_curve"] = """
pub fn evaluate_curve(curve: &AnimCurve, time: f64, default_value: Real) -> Real {
    crate::native::api::evaluate_curve(
        Some(unsafe {
            crate::native::view::View::<AnimCurve, crate::native::view::Const>::from_ptr(curve as *const AnimCurve)
        }),
        time,
        default_value,
    )
}
"""

override_functions["ufbx_evaluate_curve_flags"] = """
pub fn evaluate_curve_flags(curve: &AnimCurve, time: f64, default_value: Real, flags: u32) -> Real {
    crate::native::api::evaluate_curve_flags(
        Some(unsafe {
            crate::native::view::View::<AnimCurve, crate::native::view::Const>::from_ptr(curve as *const AnimCurve)
        }),
        time,
        default_value,
        flags,
    )
}
"""

override_functions["ufbx_evaluate_anim_value_real"] = """
pub fn evaluate_anim_value_real(anim_value: &AnimValue, time: f64) -> Real {
    crate::native::api::evaluate_anim_value_real(
        Some(unsafe {
            crate::native::view::View::<AnimValue, crate::native::view::Const>::from_ptr(anim_value as *const AnimValue)
        }),
        time,
    )
}
"""

override_functions["ufbx_evaluate_anim_value_vec3"] = """
pub fn evaluate_anim_value_vec3(anim_value: &AnimValue, time: f64) -> Vec3 {
    crate::native::api::evaluate_anim_value_vec3(
        Some(unsafe {
            crate::native::view::View::<AnimValue, crate::native::view::Const>::from_ptr(anim_value as *const AnimValue)
        }),
        time,
    )
}
"""

override_functions["ufbx_evaluate_anim_value_real_flags"] = """
pub fn evaluate_anim_value_real_flags(anim_value: &AnimValue, time: f64, flags: u32) -> Real {
    crate::native::api::evaluate_anim_value_real_flags(
        Some(unsafe {
            crate::native::view::View::<AnimValue, crate::native::view::Const>::from_ptr(anim_value as *const AnimValue)
        }),
        time,
        flags,
    )
}
"""

override_functions["ufbx_evaluate_anim_value_vec3_flags"] = """
pub fn evaluate_anim_value_vec3_flags(anim_value: &AnimValue, time: f64, flags: u32) -> Vec3 {
    crate::native::api::evaluate_anim_value_vec3_flags(
        Some(unsafe {
            crate::native::view::View::<AnimValue, crate::native::view::Const>::from_ptr(anim_value as *const AnimValue)
        }),
        time,
        flags,
    )
}
"""

override_functions["ufbx_dom_is_array"] = """
pub fn dom_is_array(node: &DomNode) -> bool {
    crate::native::api::dom_is_array(Some(unsafe {
        crate::native::view::View::<DomNode, crate::native::view::Const>::from_ptr(node as *const DomNode)
    }))
}
"""

override_functions["ufbx_dom_array_size"] = """
pub fn dom_array_size(node: &DomNode) -> usize {
    crate::native::api::dom_array_size(Some(unsafe {
        crate::native::view::View::<DomNode, crate::native::view::Const>::from_ptr(node as *const DomNode)
    }))
}
"""

override_functions["ufbx_dom_as_int32_list"] = """
pub fn dom_as_int32_list<'a>(node: &DomNode) -> &'a [i32] {
    let result = crate::native::api::dom_as_int32_list(Some(unsafe {
        crate::native::view::View::<DomNode, crate::native::view::Const>::from_ptr(node as *const DomNode)
    }));
    unsafe { result.as_static_ref() }
}
"""

override_functions["ufbx_dom_as_int64_list"] = """
pub fn dom_as_int64_list<'a>(node: &DomNode) -> &'a [i64] {
    let result = crate::native::api::dom_as_int64_list(Some(unsafe {
        crate::native::view::View::<DomNode, crate::native::view::Const>::from_ptr(node as *const DomNode)
    }));
    unsafe { result.as_static_ref() }
}
"""

override_functions["ufbx_dom_as_float_list"] = """
pub fn dom_as_float_list<'a>(node: &DomNode) -> &'a [f32] {
    let result = crate::native::api::dom_as_float_list(Some(unsafe {
        crate::native::view::View::<DomNode, crate::native::view::Const>::from_ptr(node as *const DomNode)
    }));
    unsafe { result.as_static_ref() }
}
"""

override_functions["ufbx_dom_as_double_list"] = """
pub fn dom_as_double_list<'a>(node: &DomNode) -> &'a [f64] {
    let result = crate::native::api::dom_as_double_list(Some(unsafe {
        crate::native::view::View::<DomNode, crate::native::view::Const>::from_ptr(node as *const DomNode)
    }));
    unsafe { result.as_static_ref() }
}
"""

override_functions["ufbx_dom_as_real_list"] = """
pub fn dom_as_real_list<'a>(node: &DomNode) -> &'a [Real] {
    let result = crate::native::api::dom_as_real_list(Some(unsafe {
        crate::native::view::View::<DomNode, crate::native::view::Const>::from_ptr(node as *const DomNode)
    }));
    unsafe { result.as_static_ref() }
}
"""

override_functions["ufbx_dom_as_blob_list"] = """
pub fn dom_as_blob_list<'a>(node: &DomNode) -> &'a [Blob] {
    let result = crate::native::api::dom_as_blob_list(Some(unsafe {
        crate::native::view::View::<DomNode, crate::native::view::Const>::from_ptr(node as *const DomNode)
    }));
    unsafe { result.as_static_ref() }
}
"""


# The baked-anim finders: native fns are safe over mode-generic views with
# `Option` params for C's null checks. The upstream `&mut` signatures are
# parity-locked (C non-const artifacts); wrappers keep them verbatim and mint
# read-only `Const` views.
override_functions["ufbx_find_baked_node_by_typed_id"] = """
pub fn find_baked_node_by_typed_id<'a>(
    bake: &mut BakedAnim,
    typed_id: u32,
) -> Option<&'a BakedNode> {
    let result = crate::native::api::find_baked_node_by_typed_id(
        unsafe { crate::native::view::View::<BakedAnim, crate::native::view::Const>::from_ptr(bake as *const BakedAnim) },
        typed_id,
    );
    result.map(|node| unsafe { &*node.as_ptr() })
}
"""

override_functions["ufbx_find_baked_node"] = """
pub fn find_baked_node<'a>(bake: &mut BakedAnim, node: &'a mut Node) -> Option<&'a BakedNode> {
    let result = crate::native::api::find_baked_node(
        Some(unsafe { crate::native::view::View::<BakedAnim, crate::native::view::Const>::from_ptr(bake as *const BakedAnim) }),
        Some(unsafe { crate::native::view::View::<Node, crate::native::view::Const>::from_ptr(node as *const Node) }),
    );
    result.map(|node| unsafe { &*node.as_ptr() })
}
"""

override_functions["ufbx_find_baked_element_by_element_id"] = """
pub fn find_baked_element_by_element_id<'a>(
    bake: &mut BakedAnim,
    element_id: u32,
) -> Option<&'a BakedElement> {
    let result = crate::native::api::find_baked_element_by_element_id(
        unsafe { crate::native::view::View::<BakedAnim, crate::native::view::Const>::from_ptr(bake as *const BakedAnim) },
        element_id,
    );
    result.map(|elem| unsafe { &*elem.as_ptr() })
}
"""

override_functions["ufbx_find_baked_element"] = """
pub fn find_baked_element<'a>(
    bake: &mut BakedAnim,
    element: &'a mut Element,
) -> Option<&'a BakedElement> {
    let result = crate::native::api::find_baked_element(
        Some(unsafe { crate::native::view::View::<BakedAnim, crate::native::view::Const>::from_ptr(bake as *const BakedAnim) }),
        Some(unsafe { crate::native::view::View::<Element, crate::native::view::Const>::from_ptr(element as *const Element) }),
    );
    result.map(|elem| unsafe { &*elem.as_ptr() })
}
"""

# The find-prop adapters call the native finder directly with a read-only
# `Const` view minted from the caller's `&Props` — the mint every readable
# provenance (including a shared reference) supports, unlike the
# interior-mutable `Mut` view (Miri Stacked Borrows; see native/view.rs).
# The capi shims' null checks are C-ABI-only concerns a `&Props` cannot need.
override_functions["ufbx_find_prop_len"] = """
#[allow(clippy::needless_lifetimes)]
pub fn find_prop<'a>(props: &'a Props, name: &str) -> Option<&'a Prop> {
    let result = unsafe {
        crate::native::api::find_prop_len(
            crate::native::view::View::<Props, crate::native::view::Const>::from_ptr(props as *const Props),
            name.as_bytes(),
        )
    };
    result.map(|prop| unsafe { &*prop.as_ptr() })
}
"""

override_functions["ufbx_find_blob_len"] = """
pub fn find_blob(props: &Props, name: &str, def: Blob) -> Blob {
    unsafe {
        crate::native::api::find_blob_len(
            crate::native::view::View::<Props, crate::native::view::Const>::from_ptr(props as *const Props),
            name.as_bytes(),
            def,
        )
    }
}
"""

override_functions["ufbx_thread_pool_set_user_ptr"] = """
pub unsafe fn thread_pool_set_user_ptr(ctx: ThreadPoolContext, user_ptr: *mut c_void) {
    ufbx_thread_pool_set_user_ptr(ctx, user_ptr)
}
"""

override_functions["ufbx_thread_pool_get_user_ptr"] = """
pub unsafe fn thread_pool_get_user_ptr(ctx: ThreadPoolContext) -> *mut c_void {
    ufbx_thread_pool_get_user_ptr(ctx)
}
"""

override_functions["ufbx_evaluate_prop_len"] = """
pub fn evaluate_prop<'a, 'b>(anim: &'a Anim, element: &'a Element, name: &'b str, time: f64) -> ExternalRef<'b, Prop>
    where 'a: 'b
{
    let result = unsafe { ufbx_evaluate_prop_len(anim as *const Anim, element as *const Element, name.as_ptr(), name.len(), time) };
    unsafe { ExternalRef::new(result) }
}
"""

override_functions["ufbx_prepare_prop_overrides"] = """
// TODO: ufbx_prepare_prop_overrides()
"""

override_functions["ufbx_evaluate_props"] = """
pub fn evaluate_props<'a, 'b>(anim: &'a Anim, element: &'a Element, time: f64, buffer: &'b mut [ExternalRef<'b, Prop>]) -> ExternalRef<'b, Props>
    where 'a: 'b
{
    let result = unsafe { ufbx_evaluate_props(anim as *const Anim, element as *const Element, time, buffer.as_ptr() as *mut Prop, buffer.len()) };
    unsafe { ExternalRef::new(result) }
}
"""

override_member_functions["ufbx_find_real_len"] = """
// TODO: find_real()
"""

override_member_functions["ufbx_find_vec3_len"] = """
// TODO: find_vec3()
"""

override_member_functions["ufbx_find_int_len"] = """
// TODO: find_int()
"""

override_member_functions["ufbx_find_bool_len"] = """
// TODO: find_bool()
"""

override_member_functions["ufbx_find_string_len"] = """
// TODO: find_string()
"""

override_member_functions["ufbx_find_shader_prop_len"] = """
pub fn find_shader_prop<'a>(&'a self, name: &'a str) -> &'a str {
    find_shader_prop(self, name)
}
"""

def get_struct_name(st: ir.Struct):
    name = ir.to_pascal(st.short_name)
    if st.is_input or st.is_callback or st.is_interface or st.name in raw_types:
        name = "Raw" + name
    return name

def get_struct_rust_name(st: ir.Struct):
    name = ir.to_pascal(st.short_name)
    return name

def get_enum_name(en: ir.Enum):
    return ir.to_pascal(en.short_name)

def get_field_name(fd: ir.Field):
    name = fd.name
    if name in ("type", "fn"):
        name = name + "_"
    return name

def get_arg_name(arg: ir.Argument):
    name = arg.name
    if name in ("type", "fn"):
        name = name + "_"
    return name

def get_func_name(fn: ir.Function):
    name = fn.short_name
    if fn.is_catch:
        name = name.replace("catch_", "")
    if fn.is_len:
        name = name[:-4]
    return name

def get_global_name(fn: ir.Global):
    name = fn.short_name
    return name

def get_member_func_name(fn: ir.Function, name: str):
    if fn.is_catch:
        name = name.replace("catch_", "")
    if fn.is_len:
        name = name[:-4]
    return name

class RustType:
    def __init__(self, irt: Optional[ir.Type], inner: Optional["RustType"]):
        self.ir = irt
        self.needs_lifetime = False
        self.rust_needs_lifetime = False
        self.is_list = False
        self.is_ref_list = False
        self.is_result = False
        self.is_void = False
        self.is_synthetic = False
        self.is_function = False
        self.is_raw = False
        self.is_string = False
        self.kind = ""
        self.inner = inner
        if irt:
            self.is_function = irt.is_function
            self.kind = irt.kind
            if irt.kind == "struct":
                st = file.structs[irt.base_name]
                self.name = get_struct_name(st)
                self.rust_name = get_struct_rust_name(st)
                if st.is_list:
                    self.is_list = True
                    data_type = file.types[file.types[st.fields[0].type].inner]
                    if data_type.kind == "pointer":
                        self.is_ref_list = True
                        data_type = file.types[data_type.inner]
                    self.inner = init_type(data_type)
                if st.name == "ufbx_string":
                    self.is_string = True
            elif irt.kind == "enum":
                en = file.enums[irt.base_name]
                self.name = get_enum_name(en)
                self.rust_name = self.name
            elif irt.key in primitive_types:
                self.name = primitive_types[irt.key]
                self.rust_name = self.name
                if irt.key == "void":
                    self.is_void = True
            else:
                self.name = "???"
                self.rust_name = self.name

    def get_leaf(self):
        typ = self
        while typ.inner:
            typ = typ.inner
        return typ

    def fmt_raw(self):
        if self.is_result:
            return f"Result<{self.inner.fmt_raw()}>"
        elif self.is_function:
            typ = self.ir
            if typ.kind == "pointer":
                typ = file.types[typ.inner]
            if typ.kind == "typedef":
                typ = file.types[typ.inner]

            ret_type = types[typ.inner]
            arg_types = [types[arg.type] for arg in typ.func_args]
            arg_str = ", ".join(arg.fmt_raw() for arg in arg_types)
            if ret_type.ir and ret_type.ir.key == "void":
                return f"Option<unsafe extern \"C\" fn ({arg_str})>"
            else:
                ret_str = ret_type.fmt_raw()
                # NOTE(ufbx-rs-native): enum returns in raw C callback signatures are
                # emitted as RawEnum<Enum> (#[repr(transparent)] u32) — C only
                # guarantees an integer comes back (and casts it, e.g. progress_cb at
                # ufbx.c:2152-2153); materializing a Rust enum from an arbitrary C
                # return value would be UB for out-of-range values.
                if ret_type.ir and ret_type.ir.kind == "enum":
                    ret_str = f"RawEnum<{ret_str}>"
                return f"Option<unsafe extern \"C\" fn ({arg_str}) -> {ret_str}>"
        elif self.kind == "pointer":
            if self.ir.is_const:
                return f"*const {self.inner.fmt_raw()}"
            else:
                return f"*mut {self.inner.fmt_raw()}"
        elif self.kind == "unsafe":
            return self.inner.fmt_raw()
        else:
            return self.name

    def fmt_member(self, lifetime=""):
        if self.is_result:
            return f"Result<{self.inner.fmt_member(lifetime)}>"
        elif self.is_function:
            return self.fmt_raw()
        elif self.is_synthetic and self.name == "RawList":
            return f"RawList<{self.inner.fmt_member(lifetime)}>"
        elif self.is_list:
            list_type = "RefList" if self.is_ref_list else "List"
            lt = f"'{lifetime}, " if lifetime else ""
            return f"{list_type}<{self.inner.fmt_member(lifetime)}>"
        elif self.kind == "array":
            num = self.ir.array_length
            return f"[{self.inner.fmt_member(lifetime)}; {num}]"
        elif self.kind == "unsafe":
            return self.inner.fmt_member(lifetime)
        elif self.kind == "pointer":
            lt = f"'{lifetime}, " if lifetime else ""
            if self.ir.inner == "void":
                return self.fmt_raw()
            if self.ir.is_nullable:
                return f"Option<Ref<{self.inner.fmt_member(lifetime)}>>"
            else:
                return f"Ref<{self.inner.fmt_member(lifetime)}>"
        else:
            if self.needs_lifetime:
                lt = f"<'{lifetime}>" if lifetime else ""
                return f"{self.name}{lt}"
            else:
                return self.name

    def fmt_arg(self, lifetime="", force_const=False, non_raw=False):
        if self.is_result:
            return f"Result<{self.inner.fmt_arg(lifetime)}>"
        elif self.is_function:
            return self.fmt_raw()
        elif self.is_list:
            lt = f"'{lifetime} " if lifetime else ""
            return f"&{lt}[{self.inner.fmt_arg(lifetime)}]"
        elif self.kind == "array":
            num = self.ir.array_length
            return f"[{self.inner.fmt_member(lifetime)}; {num}]"
        elif self.kind == "pointer":
            if self.ir.inner == "void":
                return self.fmt_raw()
            lt = f"'{lifetime} " if lifetime else ""
            mut = "" if self.ir.is_const or force_const else "mut "
            if self.ir.is_nullable:
                return f"Option<&{lt}{mut}{self.inner.fmt_arg(lifetime)}>"
            else:
                return f"&{lt}{mut}{self.inner.fmt_arg(lifetime)}"
        elif self.is_string:
            lt = f"'{lifetime} " if lifetime else ""
            return f"&{lt}str"
        else:
            if self.needs_lifetime:
                lt = f"<'{lifetime}>" if lifetime else ""
                return f"{self.name}{lt}"
            elif self.is_raw and non_raw:
                return self.rust_name
            else:
                return self.name

    def fmt_input(self, lifetime=""):
        if self.is_result:
            return f"Result<{self.inner.fmt_input(lifetime)}>"
        elif self.is_function:
            return self.fmt_raw()
        elif self.is_list:
            return f"Vec<{self.inner.fmt_input(lifetime)}>"
        elif self.is_synthetic and self.name == "RawString":
            assert lifetime
            return f"StringOpt<'{lifetime}>"
        elif self.is_synthetic and self.name == "RawBlob":
            assert lifetime
            return f"BlobOpt<'{lifetime}>"
        elif self.is_synthetic and self.name == "RawList":
            assert lifetime
            return f"ListOpt<'{lifetime}, {self.inner.fmt_input(lifetime)}>"
        elif self.kind == "array":
            num = self.ir.array_length
            return f"[{self.inner.fmt_input(lifetime)}; {num}]"
        elif self.kind == "pointer":
            lt = f"'{lifetime}, " if lifetime else ""
            if self.ir.inner == "void":
                return self.fmt_raw()
            if self.ir.is_nullable:
                return f"OptionRef<{self.inner.fmt_input(lifetime)}>"
            else:
                return f"Ref<{self.inner.fmt_input(lifetime)}>"
        elif self.kind == "unsafe":
            return f"Unsafe<{self.inner.fmt_input(lifetime)}>"
        elif self.kind == "struct":
            rs = structs[self.ir.key]
            if rs.ir.is_input or rs.ir.is_interface:
                lt = f"'{lifetime}" if lifetime and self.rust_needs_lifetime else ""
                return f"{rs.rust_name}<{lt}>"
            elif rs.ir.is_callback:
                assert lifetime
                lt = f"'{lifetime}"
                return f"{rs.rust_name}<{lt}>"
            elif rs.ir.name == "ufbx_string":
                lt = f"'{lifetime} " if lifetime else ""
                return f"&{lt}str"
            elif self.needs_lifetime:
                lt = f"<'{lifetime}>" if lifetime else ""
                return f"{self.name}{lt}"
            else:
                return self.name
        else:
            if self.needs_lifetime:
                lt = f"<'{lifetime}>" if lifetime else ""
                return f"{self.name}{lt}"
            else:
                return self.name

    def fmt_raw_default(self):
        if self.is_function:
            return "None"
        elif self.kind == "pointer":
            if self.ir.inner == "void":
                return f"ptr::null::<c_void>() as *mut c_void"
        raise RuntimeError(f"No default for {self.name}")

class RustField:
    def __init__(self, irf: ir.Field, rt: RustType):
        self.ir = irf
        self.name = get_field_name(irf)
        self.type = rt

class RustStruct:
    fields: List[RustField]
    def __init__(self, st: ir.Struct):
        self.ir = st
        self.fields = []
        self.name = get_struct_name(st)
        self.rust_name = get_struct_rust_name(st)
        self.is_raw = False
        self.has_inline_bufs = False

class RustEnumValue:
    def __init__(self, ev: ir.EnumValue):
        self.ir = ev
        if ev.flag:
            self.name = ev.short_name
        else:
            self.name = ir.to_pascal(ev.short_name)
        self.value = ev.value

class RustEnum:
    values: List[RustEnumValue]
    def __init__(self, en: ir.Enum):
        self.ir = en
        self.name = get_enum_name(en)
        self.values = []

class RustArgument:
    def __init__(self, arg: ir.Argument, kind: str, original_index: int):
        self.ir = arg
        self.num_ir = None
        self.name = get_arg_name(arg)
        self.type = init_type(file.types[arg.type])
        self.is_const = self.type.ir.is_const
        self.kind = kind
        leaf = self.type.get_leaf()
        self.is_raw = leaf.is_raw
        self.original_index = original_index

    def fmt_arg(self, lifetime: str, non_raw: bool = False, force_mut: bool = False) -> str:
        if not self.ir.return_ref:
            lifetime = ""
        if self.kind == "string":
            return f"{self.name}: &str"
        elif self.kind == "blob":
            mut = "" if self.is_const else "mut "
            return f"{self.name}: &{mut}[u8]"
        elif self.kind == "slice":
            mut = "" if self.is_const and not force_mut else "mut "
            return f"{self.name}: &{mut}[{self.type.fmt_arg(lifetime, non_raw=non_raw)}]"
        elif non_raw and self.is_raw:
            leaf = self.type.get_leaf()
            return f"{self.name}: {leaf.rust_name}"
        else:
            return f"{self.name}: {self.type.fmt_arg(lifetime)}"

class RustFunction:
    args: List[RustArgument]
    def __init__(self, fn: ir.Function):
        self.ir = fn
        self.name = get_func_name(fn)
        self.args = []
        self.is_raw = False
        self.emitted = False

        if fn.alloc_type:
            name = alloc_types[fn.alloc_type]
            self.return_type = RustType(None, None)
            self.return_type.name = name
            self.return_type.is_synthetic = True
        else:
            self.return_type = init_type(file.types[fn.return_type])

        if fn.has_error:
            self.return_type = RustType(None, self.return_type)
            self.return_type.is_result = True

lifetime_types = set()

def init_type(typ: ir.Type) -> RustType:
    if typ.key not in types:
        inner = None
        inner_lifetime = False
        rust_inner_lifetime = False
        if typ.inner:
            inner = init_type(file.types[typ.inner])
            if inner.needs_lifetime:
                inner_lifetime = True
            if inner.rust_needs_lifetime:
                rust_inner_lifetime = True
        rt = RustType(typ, inner)
        types[typ.key] = rt

        if inner_lifetime:
            rt.needs_lifetime = True
        if rust_inner_lifetime:
            rt.rust_needs_lifetime = True
        if typ.key in lifetime_types:
            rt.needs_lifetime = True

    return types[typ.key]

def propagate_lifetimes():
    updated = True
    while updated:
        updated = False
        for rt in types.values():
            if rt.kind != "struct": continue
            if not rt.needs_lifetime:
                rs = structs[rt.ir.key]
                for field in rs.fields:
                    if field.type.needs_lifetime:
                        rt.needs_lifetime = True
                        updated = True
            if not rt.rust_needs_lifetime:
                rs = structs[rt.ir.key]
                for field in rs.fields:
                    if field.type.rust_needs_lifetime:
                        rt.rust_needs_lifetime = True
                        updated = True

def init_fields(rs: RustStruct, field: ir.Field):
    if field.name == "":
        ist = file.structs[field.type]
        if ist.is_union:
            for ifield in ist.fields:
                if not ifield.union_sized: continue
                if not ifield.union_preferred: continue
                init_fields(rs, ifield)
                break
            else:
                for ifield in ist.fields:
                    if not ifield.union_sized: continue
                    init_fields(rs, ifield)
                    break
                else:
                    raise RuntimeError(f"Could not choose union alternative for {ist.name}")
        else:
            for ifield in ist.fields:
                init_fields(rs, ifield)
        return

    rt = init_type(file.types[field.type])

    # NOTE(ufbx-rs-native): upstream-annotation gap fixup, see
    # `nullable_field_overrides` above.
    if (rs.ir.name, field.name) in nullable_field_overrides:
        assert rt.ir.kind == "pointer" and not rt.ir.is_nullable, \
            f"{rs.ir.name}.{field.name} is no longer a non-nullable pointer, drop the override"
        nullable_key = f"{rt.ir.base_name}?*"
        assert nullable_key in file.types, \
            f"no nullable type {nullable_key} in the IR for {rs.ir.name}.{field.name}"
        rt = init_type(file.types[nullable_key])

    if rs.ir.is_input and rt.ir.key == "ufbx_string":
        rt = RustType(None, None)
        rt.name = "RawString"
        rt.is_synthetic = True
        rt.rust_needs_lifetime = True
    elif rs.ir.is_input and rt.ir.key == "ufbx_blob":
        rt = RustType(None, None)
        rt.name = "RawBlob"
        rt.is_synthetic = True
        rt.rust_needs_lifetime = True
    elif rs.ir.is_input and rt.ir.kind == "struct" and file.structs[rt.ir.key].is_list:
        rt = RustType(None, rt.inner)
        rt.name = "RawList"
        rt.is_synthetic = True
        rt.rust_needs_lifetime = True
    elif field.kind == "inlineBuf":
        rt = RustType(None, rt)
        rt.name = f"InlineBuf<[{rt.inner.inner.name}; {rt.inner.ir.array_length}]>"
        rt.is_synthetic = True
        field.name = f"{field.name}_buf"
        rs.has_inline_bufs = True

    rs.fields.append(RustField(field, rt))

def init_struct(st: ir.Struct):
    rs = RustStruct(st)
    for field in st.fields:
        init_fields(rs, field)
    structs[st.name] = rs

    if rs.ir.is_callback or rs.ir.is_input or rs.ir.is_interface or rs.ir.name in raw_types:
        rs.is_raw = True
        types[rs.ir.name].is_raw = True

    return rs

def init_enum(en: ir.Enum):
    re = RustEnum(en)
    enums[en.name] = re

    #HACK
    seen_values = set()

    for val in en.values:
        ev = file.enum_values[val]
        if ev.value in seen_values: continue
        seen_values.add(ev.value)
        rv = RustEnumValue(ev)
        re.values.append(rv)
        enum_values[val] = rv
    return en

def init_function(fn: ir.Function):
    rf = RustFunction(fn)
    functions[fn.name] = rf
    for arg_ix, arg in enumerate(fn.arguments):
        if arg.kind == "stringPointer":
            rf.args.append(RustArgument(arg, "string", arg_ix))
        elif arg.kind == "stringLength":
            pass
        elif arg.kind == "arrayPointer":
            ra = RustArgument(arg, "slice", arg_ix)
            ra.type = ra.type.inner
            rf.args.append(ra)
        elif arg.kind == "arrayLength":
            pass
        elif arg.kind == "blobPointer":
            rf.args.append(RustArgument(arg, "blob", arg_ix))
        elif arg.kind == "blobSize":
            pass
        elif arg.kind == "error":
            pass
        elif arg.kind == "panic":
            pass
        else:
            arg_type = types[arg.type]
            while arg_type.inner:
                arg_type = arg_type.inner
            if arg_type.is_raw:
                rf.is_raw = True
            rf.args.append(RustArgument(arg, arg.kind, arg_ix))

def init_file():
    for name in file.types:
        init_type(file.types[name])
    for name in file.structs:
        init_struct(file.structs[name])

    propagate_lifetimes()

    for name in file.enums:
        init_enum(file.enums[name])
    for name in file.functions:
        init_function(file.functions[name])

g_outfile = None
g_indent = 0

def emit(line=""):
    global g_indent
    global g_outfile
    if line:
        print("    " * g_indent + line, file=g_outfile)
    else:
        print("", file=g_outfile)

def indent(delta=1):
    global g_indent
    g_indent += delta

def unindent(delta=1):
    global g_indent
    g_indent -= delta

def emit_lines(extra: str):
    for line in extra.splitlines():
        emit(line)

def emit_struct(rs: RustStruct):
    if rs.ir.is_list: return

    lifetime = ""
    typ = types[rs.ir.name]
    if typ.needs_lifetime:
        lifetime = "<'a>"

    emit()
    emit(f"#[repr(C)]")
    # NOTE(ufbx-rs-native): callback/interface structs ({fn, user} pairs) and
    # `copy_derive_types` derive Clone, Copy so C-style by-value struct
    # assignment stays memcpy-like in the native port (PORTING.md #15).
    if rs.ir.is_pod or rs.ir.is_callback or rs.ir.is_interface or rs.ir.name in copy_derive_types:
        emit(f"#[derive(Clone, Copy)]")
    if rs.ir.name in default_derive_types or rs.ir.is_pod or rs.ir.is_input:
        emit(f"#[derive(Default)]")
    if rs.ir.is_pod:
        emit(f"#[derive(Debug)]")
    emit(f"pub struct {rs.name}{lifetime} {{")
    indent()

    for field in rs.fields:
        # NOTE(ufbx-rs-native): private/inline-buf fields are pub(crate) rather
        # than fully private — the native port (crate::native) writes them
        # directly (e.g. ufbx_error.info_buf/info_length, ufbx_panic.message_buf),
        # exactly as ufbx.c does. External API visibility is unchanged.
        prefix = "pub(crate) "
        if (not field.ir.private and field.ir.kind not in ("inlineBuf", "inlineBufLength")) or rs.ir.is_input:
            prefix = "pub "
        lifetime = "a"

        emit(f"{prefix}{field.name}: {field.type.fmt_member(lifetime)},")

    unindent()
    emit("}")

    if rs.ir.is_callback or rs.ir.is_interface:
        emit()
        # some field-default structs are equivalent to `#[derive(Default)]`
        emit("#[allow(clippy::derivable_impls)]")
        emit(f"impl Default for {rs.name} {{")
        indent()
        emit("fn default() -> Self {")
        indent()
        emit(f"{rs.name} {{")
        indent()
        for field in rs.fields:
            emit(f"{field.name}: {field.type.fmt_raw_default()},")
        unindent()
        emit("}")
        unindent()
        emit("}")
        unindent()
        emit("}")

    if rs.ir.vertex_attrib_type:
        attrib_type = types[rs.ir.vertex_attrib_type]
        emit()
        emit(f"impl Index<usize> for {rs.name} {{")
        indent()
        emit(f"type Output = {attrib_type.name};")
        emit(f"fn index(&self, index: usize) -> &{attrib_type.name} {{")
        indent()
        emit(f"&self.values[self.indices[index] as usize]")
        unindent()
        emit("}")
        unindent()
        emit("}")

    if rs.has_inline_bufs:
        emit()
        emit(f"impl {rs.name} {{")
        indent()
        for field in rs.fields:
            if field.ir.kind != "inlineBuf": continue
            assert field.name.endswith("_buf")
            irt = field.type.inner.inner
            n = field.type.inner.ir.array_length
            base_name = field.name[:-4]
            len_name = ""
            for len_field in rs.fields:
                if len_field.ir.kind == "inlineBufLength" and len_field.name.startswith(base_name):
                    len_name = len_field.name
                    break
            # the two `mem::transmute`s below are emitted without turbofish types
            emit("#[allow(clippy::missing_transmute_annotations)]")
            emit(f"pub fn {base_name}(&self) -> &str {{")
            indent()
            emit("unsafe {")
            indent()
            emit(f"let buf: &[mem::MaybeUninit<{irt.name}>; {n}] = mem::transmute(&self.{base_name}_buf);")
            emit(f"str::from_utf8(mem::transmute(&buf[..self.{len_name}])).unwrap()")
            unindent()
            emit("}")
            unindent()
            emit("}")
        unindent()
        emit("}")

def emit_input_callback(rs: RustStruct):
    sig = callback_signatures[rs.ir.name]

    emit()
    emit(f"pub enum {rs.rust_name}<'a> {{")
    indent()
    emit("Unset,")
    emit(f"Mut(&'a mut dyn FnMut{sig}),")
    emit(f"Ref(&'a dyn Fn{sig}),")
    emit(f"Raw(Unsafe<{rs.name}>),")
    unindent()
    emit("}")

    emit()
    emit("#[allow(clippy::derivable_impls)]")
    emit(f"impl<'a> Default for {rs.rust_name}<'a> {{")
    indent()
    emit("fn default() -> Self { Self::Unset }")
    unindent()
    emit("}")

    emit()
    emit(f"impl {rs.name} {{")
    indent()

    emit(f"fn from_func<F: FnMut{sig}>(arg: &mut F) -> Self {{")
    indent()
    emit(f"{rs.name} {{")
    indent()
    emit(f"fn_: Some(call_{rs.ir.short_name}::<F>),")
    emit(f"user: arg as *mut F as *mut c_void,")
    unindent()
    emit("}")
    unindent()
    emit("}")

    unindent()
    emit("}")

    emit()
    emit(f"impl {rs.rust_name}<'_> {{")
    indent()

    emit()
    # `to_raw` mirrors the crate-internal ToRaw convention (Rust value -> raw)
    emit(f"fn to_raw(&self) -> {rs.name} {{")
    indent()
    emit("match self {")
    indent()
    emit(f"{rs.rust_name}::Unset => Default::default(),")
    emit(f"_ => panic!(\"required mutable\"),")
    unindent()
    emit("}")
    unindent()
    emit("}")

    emit()
    emit(f"fn to_raw_mut(&mut self) -> {rs.name} {{")
    indent()
    emit("match self {")
    indent()
    emit(f"{rs.rust_name}::Unset => Default::default(),")
    emit(f"{rs.rust_name}::Ref(f) => {rs.name}::from_func(f),")
    emit(f"{rs.rust_name}::Mut(f) => {rs.name}::from_func(f),")
    emit(f"{rs.rust_name}::Raw(raw) => raw.take(),")
    unindent()
    emit("}")
    unindent()
    emit("}")

    unindent()
    emit("}")

def emit_input_struct(rs: RustStruct):
    if rs.ir.is_callback:
        emit_input_callback(rs)
    if not rs.ir.is_input: return

    typ = types[rs.ir.name]
    needs_lifetime = typ.needs_lifetime

    for field in rs.fields:
        if field.ir.private: continue
        if field.type.kind == "struct":
            frs = structs[field.type.ir.key]
            if frs.ir.is_callback or frs.ir.name in ("ufbx_string", "ufbx_blob") or frs.ir.is_list:
                needs_lifetime = True
        elif field.type.rust_needs_lifetime:
            needs_lifetime = True

    lifetime = ""
    lifetime_a = ""
    lt_a = ""
    typ = types[rs.ir.name]
    if needs_lifetime:
        lifetime = "a"
        lifetime_a = "<'a>"
        lt_a = "'a "

    emit()
    emit(f"#[derive(Default)]")
    emit(f"pub struct {rs.rust_name}{lifetime_a} {{")
    indent()

    for field in rs.fields:
        if field.ir.private: continue
        prefix = "pub "
        emit(f"{prefix}{field.name}: {field.type.fmt_input(lifetime)},")

    unindent()
    emit("}")

    emit()
    emit(f"impl{lifetime_a} ToRaw for {rs.rust_name}{lifetime_a} {{")
    indent()

    emit(f"type Result = {rs.name};")

    for mut in ("", "mut "):
        mut_us = "_mut" if mut  else ""
        emit(f"#[allow(unused, unused_variables, dead_code)]")
        emit(f"fn to_raw{mut_us}(&{mut}self, arena: &mut Arena) -> Self::Result {{")
        indent()
        emit(f"{rs.name} {{")
        indent()

        for field in rs.fields:
            if field.ir.private:
                emit(f"{field.name}: 0,")
                continue

            has_from = False
            has_arena = False
            if field.type.kind == "struct":
                frs = structs[field.type.ir.key]
                if frs.ir.is_callback or frs.ir.is_input or frs.ir.is_interface or frs.ir.is_list:
                    has_from = True
                    if frs.ir.is_input or frs.ir.is_list:
                        has_arena = True
            elif field.type.name in ("RawString", "RawBlob", "RawList"):
                has_from = True
                has_arena = True

            if has_from:
                if has_arena:
                    emit(f"{field.name}: self.{field.name}.to_raw{mut_us}(arena),")
                else:
                    emit(f"{field.name}: self.{field.name}.to_raw{mut_us}(),")
            elif field.type.kind == "unsafe":
                if mut:
                    emit(f"{field.name}: self.{field.name}.take(),")
                else:
                    emit(f"{field.name}: panic!(\"required mutable\"),")
            else:
                emit(f"{field.name}: self.{field.name},")

        unindent()
        emit("}")
        unindent()
        emit("}")

    unindent()
    emit("}")

def emit_flag(re: RustEnum):
    emit()
    emit(f"#[repr(transparent)]")
    emit(f"#[derive(Clone, Copy)]")
    emit(f"pub struct {re.name}(u32);")
    emit(f"impl {re.name} {{")
    indent()
    emit(f"pub const NONE: {re.name} = {re.name}(0);")
    num_values = 0
    for value in re.values:
        if value.ir.auxiliary: continue
        emit(f"pub const {value.name}: {re.name} = {re.name}(0x{value.value:x});")
        num_values += 1
    unindent()
    emit("}")

    emit()
    names_name = f"{re.name.upper()}_NAMES"
    emit(f"const {names_name}: [(&str, u32); {num_values}] = [")
    indent()
    for value in re.values:
        if value.ir.auxiliary: continue
        emit(f"(\"{value.name}\", 0x{value.value:x}),")
    unindent()
    emit("];")

    emit()
    emit(f"impl {re.name} {{")
    indent()
    emit("pub fn any(self) -> bool { self.0 != 0 }")
    emit("pub fn has_any(self, bits: Self) -> bool { (self.0 & bits.0) != 0 }")
    emit("pub fn has_all(self, bits: Self) -> bool { (self.0 & bits.0) == bits.0 }")
    # NOTE(ufbx-rs-native): crate-internal raw accessors for the native port of
    # ufbx.c. C accumulates flag sets as plain `uint32_t` and casts once at the
    # end, including shifted sub-fields no named constant covers (e.g.
    # `flags |= ((uint32_t)(next - '0') & 0xf) << 4;` ufbx.c:11818, then
    # `prop->flags = (ufbx_prop_flags)flags;` ufbx.c:11866). The port needs the
    # same u32 arithmetic; `pub(crate)` keeps the public surface unchanged.
    # Registered in COMPAT.md §1.
    emit("#[allow(dead_code)] pub(crate) const fn from_raw(bits: u32) -> Self { Self(bits) }")
    emit("#[allow(dead_code)] pub(crate) const fn raw(self) -> u32 { self.0 }")
    unindent()
    emit("}")

    emit("#[allow(clippy::derivable_impls)]")
    emit(f"impl Default for {re.name} {{")
    indent()
    emit(f"fn default() -> Self {{ Self(0) }}")
    unindent()
    emit("}")

    emit(f"impl Debug for {re.name} {{")
    indent()
    emit("fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {")
    indent()
    emit(f"format_flags(f, &{names_name}, self.0)")
    unindent()
    emit("}")
    unindent()
    emit("}")

    ops = [
        ("BitAnd", ["type Output = Self;", "fn bitand(self, rhs: Self) -> Self::Output { Self(self.0 & rhs.0) }"]),
        ("BitAndAssign", ["fn bitand_assign(&mut self, rhs: Self) { *self = Self(self.0 & rhs.0) }"]),
        ("BitOr", ["type Output = Self;", "fn bitor(self, rhs: Self) -> Self::Output { Self(self.0 | rhs.0) }"]),
        ("BitOrAssign", ["fn bitor_assign(&mut self, rhs: Self) { *self = Self(self.0 | rhs.0) }"]),
        ("BitXor", ["type Output = Self;", "fn bitxor(self, rhs: Self) -> Self::Output { Self(self.0 ^ rhs.0) }"]),
        ("BitXorAssign", ["fn bitxor_assign(&mut self, rhs: Self) { *self = Self(self.0 ^ rhs.0) }"]),
    ]

    for op, impl in ops:
        emit(f"impl {op} for {re.name} {{")
        indent()
        for line in impl:
            emit(line)
        unindent()
        emit("}")


def emit_enum(re: RustEnum):
    if re.ir.flag:
        emit_flag(re)
        return

    emit()
    emit(f"#[repr(u32)]")
    emit(f"#[derive(Clone, Copy, PartialEq, Eq, Debug)]")
    emit(f"pub enum {re.name} {{")
    indent()

    for value in re.values:
        if value.ir.auxiliary: continue
        emit(f"{value.name} = {value.value},")

    unindent()
    emit("}")

    emit()
    emit("#[allow(clippy::derivable_impls)]")
    emit(f"impl Default for {re.name} {{")
    indent()
    emit(f"fn default() -> Self {{ Self::{re.values[0].name} }}")
    unindent()
    emit("}")

def fmt_ffi_type(typ: ir.Type, lifetime: str):
    if typ.kind == "pointer":
        inner = file.types[typ.inner]
        mut = "const " if typ.is_const else "mut "
        return f"*{mut}{fmt_ffi_type(inner, lifetime)}"
    elif typ.key == "ufbx_string":
        return f"String"
    elif types[typ.key].is_list:
        rtyp = types[typ.key]
        list_type = "RefList" if rtyp.is_ref_list else "List"
        return f"{list_type}<{rtyp.inner.fmt_member(lifetime)}>"
    elif typ.key in primitive_types:
        return primitive_types[typ.key]
    else:
        rt = types[typ.key]
        return rt.fmt_arg(lifetime)

def fmt_ffi_arg(arg: ir.Argument, lifetime: str):
    typ = file.types[arg.type]
    name = arg.name
    if name == "type":
        name = "type_"
    if not arg.return_ref:
        lifetime = ""
    return f"{name}: {fmt_ffi_type(typ, lifetime)}"

def emit_ffi_function(fn: ir.Function):
    if fn.is_inline: return

    needs_ref = False

    lt = "<'a>" if needs_ref else ""
    lifetime = "a" if needs_ref else ""

    args = ", ".join(fmt_ffi_arg(arg, lifetime) for arg in fn.arguments)
    if fn.return_type == "void":
        emit(f"pub fn {fn.name}{lt}({args});")
    else:
        ret = fmt_ffi_type(file.types[fn.return_type], lifetime)
        emit(f"pub fn {fn.name}{lt}({args}) -> {ret};")

def emit_ffi_global(gl: ir.Global):
    typ = file.types[gl.type]
    if typ.kind != "const":
        return
    typ = file.types[typ.inner]
    gt = fmt_ffi_type(typ, "")
    emit(f"pub static {gl.name}: {gt};")


# Wrappers whose native fns take mode-generic views for these `&T` reference
# args: the raw `name as *const T` pass is replaced with a read-only `Const`
# view mint — the mint every readable provenance (including a safe caller's
# `&T`) supports. Keys are C names; values map arg name -> generated type.
const_view_args = {
    "ufbx_catch_get_vertex_real": {"v": "VertexReal"},
    "ufbx_catch_get_vertex_vec2": {"v": "VertexVec2"},
    "ufbx_catch_get_vertex_vec3": {"v": "VertexVec3"},
    "ufbx_catch_get_vertex_vec4": {"v": "VertexVec4"},
    "ufbx_catch_get_vertex_w_vec3": {"v": "VertexVec3"},
    "ufbx_catch_get_skin_vertex_matrix": {"skin": "SkinDeformer"},
    "ufbx_catch_triangulate_face": {"mesh": "Mesh"},
    "ufbx_catch_compute_topology": {"mesh": "Mesh"},
    "ufbx_catch_generate_normal_mapping": {"mesh": "Mesh"},
    "ufbx_catch_compute_normals": {"mesh": "Mesh", "positions": "VertexVec3"},
    "ufbx_catch_get_weighted_face_normal": {"positions": "VertexVec3"},
}

def apply_const_view_args(cname, arg_pass):
    for aname, aty in const_view_args.get(cname, {}).items():
        raw = f"{aname} as *const {aty}"
        mint = (f"crate::native::view::View::<{aty}, crate::native::view::Const>"
                f"::from_ptr({raw})")
        arg_pass[:] = [mint if a == raw else a for a in arg_pass]

def parse_capi_forwarders(capi_path):
    # The safe wrappers in generated.rs historically call the `unsafe extern
    # "C"` capi shims, which themselves just forward to the native port. When a
    # shim is a *pure verbatim* forward — its whole body is one
    # `crate::native::MOD::FN(<its own params, in order>)` expression — the
    # wrapper can call that native fn directly, dropping one `unsafe fn` hop
    # (and, when the native fn is safe, the `unsafe` block entirely). Shims that
    # do anything else (null checks, `?`, `match`, view bridging like
    # `PropsView::from_ptr(props ...)`, casts, `.into()`) are NOT verbatim and
    # keep routing through capi. Result: capi_forward[cname] = (mod, fn).
    #
    # Parsed from capi.rs text rather than duplicated as a hand-list so the set
    # tracks capi.rs automatically; any misclassification fails the build (a
    # bad path or an arg-type mismatch against the native signature).
    try:
        src = open(capi_path, "rt", encoding="utf-8").read()
    except FileNotFoundError:
        return {}
    out = {}
    fn_re = re.compile(r'pub (?:unsafe )?extern "C" fn (ufbx_[A-Za-z0-9_]+)\s*\(')
    for m in fn_re.finditer(src):
        cname = m.group(1)
        # Walk the parameter list to its closing paren, collecting top-level
        # param names (the identifier before each top-level `:`).
        j = m.end()
        depth = 1
        params = []
        cur = ""
        while depth:
            ch = src[j]
            if ch in "([{": depth += 1
            elif ch in ")]}": depth -= 1
            if depth == 0:
                break
            if depth == 1 and ch == ",":
                params.append(cur); cur = ""
            else:
                cur += ch
            j += 1
        if cur.strip():
            params.append(cur)
        pnames = []
        ok = True
        for p in params:
            p = p.strip()
            if not p:
                continue
            pm = re.match(r'([A-Za-z_][A-Za-z0-9_]*)\s*:', p)
            if not pm:
                ok = False; break
            pnames.append(pm.group(1))
        if not ok:
            continue
        # Body: from the first '{' after the params to its matching '}'.
        b = src.index("{", j)
        depth = 1; k = b + 1
        while depth:
            if src[k] == "{": depth += 1
            elif src[k] == "}": depth -= 1
            k += 1
        body = src[b + 1:k - 1]
        # Strip line comments and collapse whitespace to a single statement.
        lines = [ln.split("//", 1)[0].rstrip() for ln in body.splitlines()]
        stmt = " ".join(ln.strip() for ln in lines if ln.strip())
        # `unsafe { ... }` blocks (unsafe_op_in_unsafe_fn discipline — whole-body
        # or per-argument) are transparent to forwarding: strip them,
        # innermost-first so nested wrappers unwrap outward.
        while True:
            stripped = re.sub(r'unsafe \{ ([^{}]*) \}', r'\1', stmt)
            if stripped == stmt:
                break
            stmt = stripped
        stmt = re.sub(r'\s+', ' ', stmt).replace(" ,", ",").replace("( ", "(").replace(" )", ")")
        # A read-only Const-view mint of a param counts as verbatim (the
        # wrapper re-mints from its own `&T`, see apply_const_view_args);
        # strip it BEFORE arg-splitting — the mint's generic args contain
        # commas the paren-depth splitter cannot see past.
        stmt = re.sub(
            r"crate::native::view::View::<[^>]*>::from_ptr\(\s*"
            r"([A-Za-z_][A-Za-z0-9_]*)\s*,?\s*\)",
            r"\1", stmt)
        # A `slice_from_ptr(name, name_len)` mint of a (ptr, len) param pair
        # also counts as verbatim: the native fn takes the pair as one `&[u8]`
        # (see native/api.rs `find_prop_len`). Record the minted param so the
        # direct-native call emits `name.as_bytes()` instead of ptr+len.
        slice_args = set()
        def _mint(m):
            slice_args.add(m.group(1))
            return m.group(1) + ", " + m.group(2)
        stmt = re.sub(
            r"crate::prelude::slice_from_ptr\(\s*"
            r"([A-Za-z_][A-Za-z0-9_]*)\s*,\s*([A-Za-z_][A-Za-z0-9_]*)\s*\)",
            _mint, stmt)
        cm = re.fullmatch(
            r'crate::native::([a-z_]+)::([A-Za-z0-9_]+)\s*\((.*)\)', stmt)
        if not cm:
            continue
        mod, fn, arg_str = cm.group(1), cm.group(2), cm.group(3)
        # Split call args at top-level commas and require them to equal the
        # param names verbatim (this is what makes generated.rs's own arg_pass,
        # which mirrors the same C-ABI signature, a valid call to the native fn).
        call_args = []
        depth = 0; cur = ""
        for ch in arg_str:
            if ch in "([{": depth += 1
            elif ch in ")]}": depth -= 1
            if depth == 0 and ch == ",":
                call_args.append(cur.strip()); cur = ""
            else:
                cur += ch
        if cur.strip():
            call_args.append(cur.strip())
        # A `panic.as_mut()` bridge (C-ABI nullable `*mut Panic` -> native
        # `Option<&mut Panic>`) still counts as a verbatim forward: the safe
        # wrapper passes `Some(&mut panic)`, the same value by construction.
        call_args = [a[:-len(".as_mut()")] if a.endswith(".as_mut()") else a
                     for a in call_args]
        if call_args == pnames:
            out[cname] = (mod, fn, slice_args)
    return out

def emit_arg_pass(args: List[str], ra: RustArgument):
    if ra.kind == "string":
        if ra.is_const:
            args.append(f"{ra.name}.as_ptr()")
        else:
            args.append(f"{ra.name}.as_mut_ptr()")
        args.append(f"{ra.name}.len()")
    elif ra.kind == "slice":
        if ra.is_const:
            args.append(f"{ra.name}.as_ptr()")
        else:
            args.append(f"{ra.name}.as_mut_ptr()")
        args.append(f"{ra.name}.len()")
    elif ra.kind == "blob":
        if ra.is_const:
            args.append(f"{ra.name}.as_ptr() as *const c_void")
        else:
            args.append(f"{ra.name}.as_mut_ptr() as *mut c_void")
        args.append(f"{ra.name}.len()")
    elif ra.type.ir.kind == "pointer":
        raw = ra.type.fmt_raw()
        # a `void*` arg is already spelled `*mut/*const c_void` in the safe
        # signature, so `x as *mut c_void` at the call boundary is a no-op cast.
        if raw in ("*mut c_void", "*const c_void"):
            args.append(ra.name)
        else:
            args.append(f"{ra.name} as {raw}")
    elif ra.type.is_list:
        args.append(f"List::from_slice({ra.name})")
    else:
        args.append(ra.name)

def emit_function(rf: RustFunction, non_raw: bool = False):
    if rf.ir.is_inline: return
    if rf.ir.is_ffi: return
    if rf.ir.catch_name: return
    if rf.ir.len_name: return
    if rf.ir.kind in { "retain", "free" }: return
    rf.emitted = True

    if rf.ir.name in override_functions:
        emit()
        emit_lines(override_functions[rf.ir.name].strip())
        return

    is_raw = rf.is_raw and not non_raw

    needs_ref = False
    if rf.return_type.kind == "pointer":
        needs_ref = True
    elif rf.return_type.is_string or rf.return_type.is_list:
        needs_ref = True

    lt = "<'a>" if needs_ref else ""
    lifetime = "a" if needs_ref else ""

    arg_str = ", ".join(
        arg.fmt_arg(lifetime, non_raw,
            force_mut = non_raw and (rf.ir.name, ix) in force_mut_args
        ) for ix, arg in enumerate(rf.args))

    ret = ""
    if not rf.return_type.is_void:
        rt = rf.return_type.fmt_arg(lifetime, force_const=True)
        if rf.ir.nullable_return:
            rt = f"Option<{rt}>"
        ret = f" -> {rt}"

    # Bypass the `unsafe extern "C"` capi ABI shim for wrappers whose shim is a
    # pure verbatim forward to the native port (see parse_capi_forwarders): call
    # the native fn directly, dropping one `unsafe fn` hop. The `pub use
    # crate::capi::{...}` re-export above still exposes the raw ufbx_* symbol for
    # C-ABI and raw callers; this only shortcuts the safe Rust path, which
    # already routed through capi straight into the same native impl. The safe
    # wrapper's arg_pass mirrors the same C-ABI signature the native fn was
    # forwarded, so it is a valid direct call; a mismatch fails the build.
    fwd = capi_forward.get(rf.ir.name)
    # `direct_safe` additionally drops the `unsafe` block: a pure by-value shim
    # (no pointer/slice/string/blob/list arg, plain-value return, no
    # error/panic/alloc post-processing) forwards to a *safe* native fn.
    def _arg_by_value(a):
        return (a.kind not in ("string", "slice", "blob")
                and a.type.ir.kind != "pointer"
                and not a.type.is_list)
    direct_safe = (
        fwd is not None
        and not is_raw
        and not rf.ir.is_unsafe
        and not rf.ir.has_error
        and not rf.ir.has_panic
        and not rf.ir.alloc_type
        and not rf.return_type.is_void
        and not rf.return_type.is_list
        and not rf.return_type.is_string
        and rf.return_type.kind != "pointer"
        and all(_arg_by_value(a) for a in rf.args)
    )
    # The callee for this wrapper body: the native fn when the shim is a pure
    # forward, else the capi shim itself (adapters that bridge/adapt args).
    call_head = f"crate::native::{fwd[0]}::{fwd[1]}" if fwd else rf.ir.name

    arg_pass = []
    for arg in rf.args:
        # A param the capi shim slice-mints reaches the native fn as one
        # `&[u8]` — the safe wrapper's `&str` maps to it via `as_bytes()`.
        if fwd and arg.kind == "string" and arg.name in fwd[2]:
            arg_pass.append(f"{arg.name}.as_bytes()")
        else:
            emit_arg_pass(arg_pass, arg)

    is_unsafe = False

    emit()
    # Per-item clippy allows for idioms this wrapper template emits uniformly.
    _allows = []
    if lt:
        _allows.append("clippy::needless_lifetimes")
    if not non_raw:
        # non-void passthrough with no error/panic/wrapping emits `let r = ..; r`
        if (not rf.return_type.is_void and not rf.ir.has_error and not rf.ir.has_panic
                and not rf.ir.alloc_type and not rf.return_type.is_list
                and rf.return_type.kind != "pointer"):
            _allows.append("clippy::let_and_return")
    if _allows:
        emit(f"#[allow({', '.join(_allows)})]")
    if is_raw:
        emit(f"pub unsafe fn {rf.name}_raw{lt}({arg_str}){ret} {{")
        is_unsafe = True
    else:
        unsafe_fn = ""
        if rf.ir.is_unsafe:
            unsafe_fn = "unsafe "
            is_unsafe = True
        emit(f"pub {unsafe_fn}fn {rf.name}{lt}({arg_str}){ret} {{")
    indent()

    unsafe = "" if is_unsafe else "unsafe "

    if non_raw:
        has_arena = False
        for arg in rf.args:
            if arg.is_raw:
                use_arena = True
                use_mut = True
                leaf = arg.type.get_leaf()
                if leaf and leaf.ir and leaf.ir.kind == "struct":
                    rs = file.structs[leaf.ir.key]
                    if rs.is_interface:
                        use_arena = False
                if arg.kind == "slice":
                    use_mut = False
                if use_arena:
                    if not has_arena:
                        has_arena = True
                        emit(f"let mut arena = Arena::new();")
                    if use_mut:
                        emit(f"let mut {arg.name}_mut = {arg.name};")
                        emit(f"let {arg.name}_raw = {arg.name}_mut.to_raw_mut(&mut arena);")
                    else:
                        emit(f"let {arg.name}_raw = {arg.name}.to_raw_mut(&mut arena);")
                else:
                    if use_mut:
                        emit(f"let mut {arg.name}_mut = {arg.name};")
                        emit(f"let {arg.name}_raw = {arg.name}_mut.to_raw_mut();")
                    else:
                        emit(f"let {arg.name}_raw = {arg.name}.to_raw_mut();")
        params = []
        for arg in rf.args:
            if arg.is_raw:
                mut = "" if arg.is_const else "mut "
                params.append(f"&{mut}{arg.name}_raw")
            else:
                params.append(arg.name)
        params_str = ", ".join(params)
        emit(f"{unsafe}{{ {rf.name}_raw({params_str}) }}")
    else:
        if rf.ir.has_error:
            emit(f"let mut error: Error = Error::default();")
            arg_pass.append("&mut error")
        if rf.ir.has_panic:
            emit(f"let mut panic: Panic = Default::default();")
            # Native fns take `Option<&mut Panic>`; the capi shims keep the
            # C-ABI `*mut Panic`.
            arg_pass.insert(0, "Some(&mut panic)" if fwd else "&mut panic")

        apply_const_view_args(rf.ir.name, arg_pass)
        arg_pass_str = ", ".join(arg_pass)
        if direct_safe:
            emit(f"let result = {call_head}({arg_pass_str});")
        elif not rf.return_type.is_void:
            emit(f"let result = {unsafe}{{ {call_head}({arg_pass_str}) }};")
        elif unsafe:
            emit(f"{unsafe}{{ {call_head}({arg_pass_str}) }};")
        else:
            # already inside an `unsafe fn`: no wrapping block needed, and a bare
            # `{ ... };` statement would trip clippy::unnecessary_operation.
            emit(f"{call_head}({arg_pass_str});")

        if rf.ir.has_panic:
            emit(f"if panic.did_panic {{")
            indent()
            emit(f"panic!(\"ufbx::{rf.name}() {{}}\", panic.message());")
            unindent()
            emit("}")
        if rf.ir.has_error:
            emit(f"if error.type_ != ErrorType::None {{")
            indent()
            emit(f"return Err(error)")
            unindent()
            emit("}")

        if not rf.return_type.is_void:
            res = "result"
            if rf.ir.alloc_type:
                alloc_type = alloc_types[rf.ir.alloc_type]
                res = f"{alloc_type}::new({res})"
            elif rf.return_type.is_list:
                res = f"{unsafe}{{ {res}.as_static_ref() }}"
            elif rf.return_type.kind == "pointer":
                if rf.ir.nullable_return:
                    res = f"if result.is_null() {{ None }} else {{ {unsafe}{{ Some(&*{res}) }} }}"
                else:
                    res = f"{unsafe}{{ &*{res} }}"
            if rf.ir.has_error:
                res = f"Ok({res})"

            emit(res)

    unindent()
    emit("}")

    if rf.is_raw and not non_raw:
        if rf.ir.name not in ignore_non_raw:
            emit_function(rf, non_raw=True)

def emit_global(gl: ir.Global):
    typ = file.types[gl.type]
    if typ.kind != "const": return
    typ = file.types[typ.inner]
    if typ.base_name in ("ufbx_string", "ufbx_blob"): return

    gt = fmt_ffi_type(typ, "")
    name = get_global_name(gl)
    # NOTE(ufbx-rs-native): the globals are plain Rust statics (capi aliases of
    # native::api), so the read is safe — no unsafe block (extern statics needed one).
    emit(f"pub fn {name}() -> {gt} {{ {gl.name} }}")

def emit_struct_impl(rs: RustStruct):
    if not rs.ir: return
    if not rs.ir.member_functions and not rs.ir.member_globals: return

    members = []
    member_globals = []
    for name in rs.ir.member_functions:
        rf = functions[name]
        if rf.emitted:
            members.append((file.member_functions[name], rf))

    for name in rs.ir.member_globals:
        mg = file.member_globals[name]
        gl = file.globals[name]
        typ = file.types[gl.type]
        if typ.kind != "const": continue
        typ = file.types[typ.inner]
        if typ.base_name in ("ufbx_string", "ufbx_blob"): continue
        member_globals.append((mg, gl, typ))

    if not members and not member_globals: return

    emit()
    emit(f"impl {rs.name} {{")
    indent()

    for mg, gl, typ in member_globals:
        gt = fmt_ffi_type(typ, "")
        name = mg.member_name
        emit(f"pub fn {name}() -> {gt} {{ {gl.name} }}")

    for mf, rf in members:
        if mf.func in override_member_functions:
            emit()
            emit_lines(override_member_functions[mf.func].strip())
            continue

        func = file.functions[mf.func]
        name = get_member_func_name(func, mf.member_name)

        non_raw = rf.is_raw

        needs_ref = False
        if rf.return_type.kind == "pointer":
            needs_ref = True
        elif rf.return_type.is_string or rf.return_type.is_list:
            needs_ref = True

        lt = "<'a>" if needs_ref else ""
        lifetime = "a" if needs_ref else ""
        self_lt = "'a " if needs_ref else ""

        args = [arg for arg in rf.args if arg.original_index != mf.self_index]
        arg_fmt = [arg.fmt_arg(lifetime, non_raw) for arg in args]
        arg_fmt.insert(0, f"&{self_lt}self")
        arg_str = ", ".join(arg_fmt)

        ret = ""
        if not rf.return_type.is_void:
            rt = rf.return_type.fmt_arg(lifetime, force_const=True)
            if rf.ir.nullable_return:
                rt = f"Option<{rt}>"
            ret = f" -> {rt}"

        emit()
        if lt:
            # explicit `<'a>` is emitted uniformly for reference-returning wrappers
            emit("#[allow(clippy::needless_lifetimes)]")
        emit(f"pub fn {name}{lt}({arg_str}){ret} {{")
        indent()

        pass_args = []
        for arg in rf.args:
            if arg.original_index == mf.self_index:
                # `self` is already `&Self` here; passing `&self` would be a
                # needless double borrow (clippy::needless_borrow).
                pass_args.append("self")
            else:
                pass_args.append(arg.name)
        pass_str = ", ".join(pass_args)
        emit(f"{rf.name}({pass_str})")

        unindent()
        emit("}")

    unindent()
    emit("}")

def emit_element_data():
    emit()
    emit("pub enum ElementData<'a> {")
    indent()

    for name in file.element_types:
        typ = types[name]
        emit(f"{typ.name}(&'a {typ.name}),")

    unindent()
    emit("}")

    emit()
    emit("impl Element {")
    indent()
    emit("pub fn as_data(&self) -> ElementData<'_> {")
    indent()
    emit("unsafe {")
    indent()
    emit("match self.type_ {")
    indent()

    for name in file.element_types:
        typ = types[name]
        emit(f"ElementType::{typ.name} => ElementData::{typ.name}(&*(self as *const _ as *const {typ.name})),")

    unindent()
    emit("}")
    unindent()
    emit("}")
    unindent()
    emit("}")
    unindent()
    emit("}")

def is_view_projection_field(field: RustField) -> bool:
    # A field whose type is itself a view-accessor struct projects to a nested
    # `&View<F, M>` instead of copying the aggregate out by value.
    ft = field.type
    return (
        ft.kind == "struct"
        and not ft.is_list
        and not ft.is_string
        and ft.ir is not None
        and ft.ir.base_name in view_accessor_structs
    )

def emit_view_impls(rs: RustStruct):
    typ = types[rs.ir.name]
    assert not typ.needs_lifetime and not typ.rust_needs_lifetime, \
        f"view accessors assume a lifetime-free struct, got {rs.ir.name}"
    name = rs.name

    fields = [f for f in rs.fields if f.ir.kind not in ("inlineBuf", "inlineBufLength")]

    # Mode-generic read surface: by-value leaf reads + in-place aggregate
    # projections. Serves both `Mut` (arena/context provenance) and `Const`
    # (frozen `&`-derived provenance) views.
    emit()
    emit("#[allow(dead_code)]")
    emit(f"impl<M: Mode> View<{name}, M> {{")
    indent()
    for field in fields:
        base = field.name.strip("_")
        if (rs.ir.name, field.ir.name) not in view_accessor_skip_read:
            # A field accessor named after a `from_*` field is not a conversion
            # constructor; keep the field's name.
            if field.name.startswith("from_"):
                emit("#[allow(clippy::wrong_self_convention)]")
            emit("#[inline(always)]")
            if is_view_projection_field(field):
                emit(f"pub(crate) fn {field.name}(&self) -> &View<{field.type.name}, M> {{")
                indent()
                emit(f"// SAFETY: in-place projection of the `{field.name}` field; liveness and")
                emit("// `M`-adequate provenance carry over from this view's own mint.")
                emit(f"unsafe {{ View::mint((&raw const (*self.as_ptr()).{field.name}).cast_mut()) }}")
                unindent()
                emit("}")
            else:
                fty = field.type.fmt_member("")
                emit(f"pub(crate) fn {field.name}(&self) -> {fty} {{")
                indent()
                emit(f"// SAFETY: by-value read of the `{field.name}` field; the viewed allocation is")
                emit("// live and unmoved per this view's mint vouch, and this field's bytes are")
                emit("// initialized per the caller's per-leaf discipline (the `mint`/`Const`")
                emit("// contracts do not claim whole-struct validity).")
                emit(f"unsafe {{ ptr::read(&raw const (*self.as_ptr()).{field.name}) }}")
                unindent()
                emit("}")
        if field.type.is_string:
            # In-place string projection: the anchored carrier for the safe
            # `bytes()` read (prelude.rs `View<String, M>`); the by-value read
            # above serves whole-String copies.
            emit("#[inline(always)]")
            emit(f"pub(crate) fn {base}_view(&self) -> &View<String, M> {{")
            indent()
            emit(f"// SAFETY: in-place projection of the `{field.name}` field; liveness and")
            emit("// `M`-adequate provenance carry over from this view's own mint.")
            emit(f"unsafe {{ View::mint((&raw const (*self.as_ptr()).{field.name}).cast_mut()) }}")
            unindent()
            emit("}")
        if field.type.is_list:
            # In-place list projection for sub-field access (`.data()`,
            # `.count()`, and Mut-side sub-field writes via the ListView
            # surface); the by-value read above serves whole-List copies.
            list_type = "RefList" if field.type.is_ref_list else "List"
            inner = field.type.inner.fmt_member("")
            emit("#[inline(always)]")
            emit(f"pub(crate) fn {base}_view(&self) -> &View<{list_type}<{inner}>, M> {{")
            indent()
            emit(f"// SAFETY: in-place projection of the `{field.name}` field; liveness and")
            emit("// `M`-adequate provenance carry over from this view's own mint.")
            emit(f"unsafe {{ View::mint((&raw const (*self.as_ptr()).{field.name}).cast_mut()) }}")
            unindent()
            emit("}")
        # Read-address projection: legal in both modes (a `*const` inherits the
        # view's provenance; deref stays on the caller's obligation).
        fty = field.type.fmt_member("")
        if base.startswith("from_"):
            emit("#[allow(clippy::wrong_self_convention)]")
        emit("#[inline(always)]")
        emit(f"pub(crate) fn {base}_ptr(&self) -> *const {fty} {{")
        indent()
        emit(f"// SAFETY: in-bounds projection of the `{field.name}` field; the returned")
        emit("// read pointer inherits the view's provenance.")
        emit(f"unsafe {{ &raw const (*self.as_ptr()).{field.name} }}")
        unindent()
        emit("}")
    unindent()
    emit("}")

    # `Mut`-only write surface: whole-field setters + raw field pointers (for
    # in-place mutation, `&raw`-taking call sites, and by-value aggregate reads
    # via `ptr::read` during construction).
    emit()
    emit("#[allow(dead_code)]")
    emit(f"impl View<{name}, Mut> {{")
    indent()
    for field in fields:
        fty = field.type.fmt_member("")
        # Compose from the underscore-stripped base so keyword-renamed (`type_`)
        # and private (`_internal_key`) fields yield snake_case method names.
        base = field.name.strip("_")
        emit("#[inline(always)]")
        emit(f"pub(crate) fn set_{base}(&self, value: {fty}) {{")
        indent()
        emit(f"// SAFETY: field write through the `Mut` view's write-capable viewed")
        emit("// memory (mint vouch); no reference to the viewed bytes outside the")
        emit("// `UnsafeCell` view — no plain `&T`/`&mut T`, no `Const` view — is live")
        emit("// across the write.")
        emit(f"unsafe {{ (*self.get()).{field.name} = value }}")
        unindent()
        emit("}")
        if base.startswith("from_"):
            emit("#[allow(clippy::wrong_self_convention)]")
        emit("#[inline(always)]")
        emit(f"pub(crate) fn {base}_raw(&self) -> *mut {fty} {{")
        indent()
        emit("// SAFETY: in-bounds field projection; the returned raw pointer")
        emit("// inherits the view's write-capable provenance.")
        emit(f"unsafe {{ &raw mut (*self.get()).{field.name} }}")
        unindent()
        emit("}")
    unindent()
    emit("}")

def emit_views_file():
    emit("// GENERATED FILE — do not edit by hand. Produced by rust/regen.sh from")
    emit("// ufbx.h via bindgen/ufbx_ir.py + rust/ufbx/bindgen/generate_rust.py.")
    emit("// Fixes belong in the GENERATOR (see PORTING.md); hand edits are")
    emit("// silently overwritten on the next regeneration and CI diffs this file.")
    emit("//")
    emit("// Crate-internal `View<T, M>` field accessors over the generated public")
    emit("// structs (`view_accessor_structs` in generate_rust.py): a by-value read")
    emit("// per leaf field, an in-place `&View` projection per aggregate and list")
    emit("// (`*_view`) field, a `*_ptr` read-address projection per field, and")
    emit("// `Mut`-only setters / raw field pointers. Soundness model (mint vouch,")
    emit("// `Mut`/`Const` provenance): src/native/view.rs.")
    emit()
    emit("use crate::generated::*;")
    emit("use crate::native::view::{Mode, Mut, View};")
    emit("use crate::prelude::*;")
    emit("use std::ptr;")

    for cname in view_accessor_structs:
        emit_view_impls(structs[cname])

    emit()

def emit_file():
    emit("// GENERATED FILE — do not edit by hand. Produced by rust/regen.sh from")
    emit("// ufbx.h via bindgen/ufbx_ir.py + rust/ufbx/bindgen/generate_rust.py.")
    emit("// Fixes belong in the GENERATOR (see PORTING.md); hand edits are")
    emit("// silently overwritten on the next regeneration and CI diffs this file.")
    emit("")
    # Clippy: rather than a single file-wide `#![allow]`, the emitters that
    # produce a lint-triggering idiom attach a targeted `#[allow(...)]` to that
    # specific item (see emit_function / emit_struct_impl / emit_struct /
    # emit_input_callback), and the trivially-fixable idioms (redundant field
    # names, `-> ()`, `&'static str`, needless `&self` borrows) are emitted in
    # their clean form. This keeps each suppression scoped to the construct that
    # needs it instead of blanket-disabling lints for the whole generated file.
    emit_lines(uses)

    rust_uses = []
    for rs in structs.values():
        if rs.ir.is_interface:
            rust_uses.append(rs.rust_name)
        if rs.ir.is_callback:
            rust_uses.append(f"call_{rs.ir.short_name}")
    rust_uses_str = ", ".join(rust_uses)
    emit(f"use crate::prelude::{{{rust_uses_str}}};")

    for decl in file.declarations:
        if decl.name in ignore_types: continue
        if decl.kind == "struct":
            emit_struct(structs[decl.name])
        elif decl.kind == "enum":
            emit_enum(enums[decl.name])

    for decl in file.declarations:
        if decl.name in ignore_types: continue
        if decl.kind == "struct":
            emit_input_struct(structs[decl.name])

    emit()
    emit("pub type Result<T> = result::Result<T, Error>;")

    emit()
    # NOTE(ufbx-rs-native): ufbx-rust declares the ufbx_* surface as an
    # `extern "C"` block resolved by the linker against the C library. Here the
    # implementations are the crate's own `capi` module, so the safe wrappers
    # below bind to them with a plain re-export — direct Rust calls, no FFI.
    # The C ABI declarations stay verbatim in capi.rs; `c-abi` only controls
    # whether the symbols are additionally exported with C linkage.
    # ufbx-rust's extern block items are public API (callers may use the raw
    # ufbx_* surface directly), so the function re-exports are `pub`. The
    # string/blob globals are the exception: their statics carry Sync wrapper
    # types (see COMPAT.md §2), so they stay crate-internal.
    emit("#[allow(unused_imports)]")
    emit("pub use crate::capi::{")
    indent()
    for decl in file.declarations:
        if decl.kind == "function":
            if file.functions[decl.name].is_inline:
                continue
            emit(f"{decl.name},")
        elif decl.kind == "global":
            gl = file.globals[decl.name]
            typ = file.types[gl.type]
            if typ.kind != "const":
                continue
            if file.types[typ.inner].base_name in ("ufbx_string", "ufbx_blob"):
                continue
            emit(f"{decl.name},")
    unindent()
    emit("};")
    emit("#[allow(unused_imports)]")
    emit("pub(crate) use crate::capi::{")
    indent()
    for decl in file.declarations:
        if decl.kind == "global":
            gl = file.globals[decl.name]
            typ = file.types[gl.type]
            if typ.kind != "const":
                continue
            if file.types[typ.inner].base_name in ("ufbx_string", "ufbx_blob"):
                emit(f"{decl.name},")
    unindent()
    emit("};")

    emit_lines(post_ffi)

    for decl in file.declarations:
        if decl.kind == "function":
            emit_function(functions[decl.name])

    for decl in file.declarations:
        if decl.kind == "global":
            emit_global(file.globals[decl.name])

    for decl in file.declarations:
        if decl.kind == "struct":
            emit_struct_impl(structs[decl.name])

    emit_element_data()

    emit()

if __name__ == "__main__":

    parser = argparse.ArgumentParser("gen_rust.py")
    parser.add_argument("-i", help="Input ufbx_typed.json file")
    parser.add_argument("-o", help="Output path")
    argv = parser.parse_args()
    g_argv = argv

    src_path = os.path.dirname(os.path.realpath(__file__))

    input_file = argv.i
    if not input_file:
        input_file = os.path.join(src_path, "build", "ufbx_typed.json")

    output_path = argv.o
    if not output_path:
        output_path = os.path.join(src_path, "..", "src")

    with open(input_file, "rt") as f:
        file = ir.from_json(ir.File, json.load(f))

    capi_forward = parse_capi_forwarders(os.path.join(output_path, "capi.rs"))

    if not os.path.exists(output_path):
        os.makedirs(output_path, exist_ok=True)

    with open(os.path.join(output_path, "generated.rs"), "wt", encoding="utf-8") as f:
        g_outfile = f
        init_file()
        emit_file()

    with open(os.path.join(output_path, "generated_views.rs"), "wt", encoding="utf-8") as f:
        g_outfile = f
        g_indent = 0
        emit_views_file()
