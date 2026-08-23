//! The ufbx.h ABI surface as Rust functions with exact C signatures; the
//! generated safe wrappers call these directly (no FFI). Under `c-abi` each
//! is additionally exported with C linkage so the upstream C test suite can
//! link the crate as a drop-in ufbx.c replacement.
#![cfg_attr(not(feature = "c-abi"), allow(dead_code))] // without exports, shims outside the safe API's call set are intentionally unreferenced
#![allow(non_upper_case_globals)]
// statics carry their C names verbatim

// ufbx.c:878 `ufbx_abi_data_def const uint32_t ufbx_source_version = UFBX_SOURCE_VERSION;`
#[cfg_attr(feature = "c-abi", no_mangle)]
pub static ufbx_source_version: u32 = crate::native::platform::SOURCE_VERSION;

// C-named aliases of the ufbx.h ABI globals (defined in native::api with
// their exported linkage names); generated.rs binds its safe accessors to
// these. `ufbx_empty_string`/`ufbx_empty_blob` carry their `Sync` wrapper
// types — the generator emits no accessor for string/blob globals, so the
// aliases exist only to satisfy the declaration surface.
#[allow(unused_imports)]
pub use crate::native::api::{
    default_open_file as ufbx_default_open_file,
    AXES_LEFT_HANDED_Y_UP as ufbx_axes_left_handed_y_up,
    AXES_LEFT_HANDED_Z_UP as ufbx_axes_left_handed_z_up,
    AXES_RIGHT_HANDED_Y_UP as ufbx_axes_right_handed_y_up,
    AXES_RIGHT_HANDED_Z_UP as ufbx_axes_right_handed_z_up, EMPTY_BLOB as ufbx_empty_blob,
    EMPTY_STRING as ufbx_empty_string, IDENTITY_MATRIX as ufbx_identity_matrix,
    IDENTITY_QUAT as ufbx_identity_quat, IDENTITY_TRANSFORM as ufbx_identity_transform,
    ZERO_VEC2 as ufbx_zero_vec2, ZERO_VEC3 as ufbx_zero_vec3, ZERO_VEC4 as ufbx_zero_vec4,
};

// ufbx.c:3131-3276 `ufbx_inflate` (impl: native/deflate.rs `inflate`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_inflate(
    dst: *mut core::ffi::c_void,
    dst_size: usize,
    input: *const crate::generated::InflateInput,
    retain: *mut crate::generated::InflateRetain,
) -> isize {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::deflate::inflate(dst, dst_size, input, retain) }
}

// ufbx.c:30339-30404 `ufbx_abi_data_def` globals (`ufbx_empty_string`,
// `ufbx_empty_blob`, `ufbx_identity_matrix`, `ufbx_identity_transform`,
// `ufbx_zero_vec2/3/4`, `ufbx_identity_quat`, the four `ufbx_axes_*` and
// `ufbx_element_type_size`): NO definitions here. C has exactly ONE object per
// global — the internal reads (`ufbx_element_type_size[src->type]`
// ufbx.c:26149, `ufbx_zero_vec2` ufbx.c:27997/33003) hit the same object the
// header exports — so the exports live directly on the impls in
// `native::api` via `#[export_name]`, same as `ufbx_default_open_file` below.

// ufbx.c:30406-30410 `ufbx_default_open_file`: NO shim here. C compares this
// callback BY ADDRESS (`uc->opts.open_file_cb.fn == &ufbx_default_open_file`,
// ufbx.c:25224, stored at 24645/25532/32712), so there must be exactly one
// function address; the export is `#[cfg_attr(feature = "c-abi", export_name = "ufbx_default_open_file")]`
// directly on the impl `native::api::default_open_file`.

// ufbx.c:30412-30415 `ufbx_open_file` (impl: native/api.rs `open_file`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_open_file(
    stream: *mut crate::generated::RawStream,
    path: *const u8,
    path_len: usize,
    opts: *const crate::generated::RawOpenFileOpts,
    error: *mut crate::generated::Error,
) -> bool {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::open_file(stream, path, path_len, opts, error) }
}

// ufbx.c:30417-30435 `ufbx_open_file_ctx` (impl: native/api.rs `open_file_ctx`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_open_file_ctx(
    stream: *mut crate::generated::RawStream,
    ctx: crate::prelude::OpenFileContext,
    path: *const u8,
    path_len: usize,
    opts: *const crate::generated::RawOpenFileOpts,
    error: *mut crate::generated::Error,
) -> bool {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::open_file_ctx(stream, ctx, path, path_len, opts, error) }
}

// ufbx.c:30437-30440 `ufbx_open_memory` (impl: native/api.rs `open_memory`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_open_memory(
    stream: *mut crate::generated::RawStream,
    data: *const core::ffi::c_void,
    data_size: usize,
    opts: *const crate::generated::RawOpenMemoryOpts,
    error: *mut crate::generated::Error,
) -> bool {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::open_memory(stream, data, data_size, opts, error) }
}

// ufbx.c:30442-30495 `ufbx_open_memory_ctx` (impl: native/api.rs `open_memory_ctx`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_open_memory_ctx(
    stream: *mut crate::generated::RawStream,
    ctx: crate::prelude::OpenFileContext,
    data: *const core::ffi::c_void,
    data_size: usize,
    opts: *const crate::generated::RawOpenMemoryOpts,
    error: *mut crate::generated::Error,
) -> bool {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::open_memory_ctx(stream, ctx, data, data_size, opts, error) }
}

// ufbx.c:30497-30500 `ufbx_is_thread_safe` (impl: native/api.rs `is_thread_safe`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_is_thread_safe() -> bool {
    crate::native::api::is_thread_safe()
}

// ufbx.c:30502-30511 `ufbx_load_memory` (impl: native/api.rs `load_memory`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_load_memory(
    data: *const core::ffi::c_void,
    size: usize,
    opts: *const crate::generated::RawLoadOpts,
    error: *mut crate::generated::Error,
) -> *mut crate::generated::Scene {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::load_memory(data, size, opts, error) }
}

// ufbx.c:30513-30516 `ufbx_load_file` (impl: native/api.rs `load_file`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_load_file(
    filename: *const u8,
    opts: *const crate::generated::RawLoadOpts,
    error: *mut crate::generated::Error,
) -> *mut crate::generated::Scene {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::load_file(filename, opts, error) }
}

// ufbx.c:30518-30527 `ufbx_load_file_len` (impl: native/api.rs `load_file_len`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_load_file_len(
    filename: *const u8,
    filename_len: usize,
    opts: *const crate::generated::RawLoadOpts,
    error: *mut crate::generated::Error,
) -> *mut crate::generated::Scene {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::load_file_len(filename, filename_len, opts, error) }
}

// ufbx.c:30529-30532 `ufbx_load_stdio` (impl: native/api.rs `load_stdio`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_load_stdio(
    file_void: *mut core::ffi::c_void,
    opts: *const crate::generated::RawLoadOpts,
    error: *mut crate::generated::Error,
) -> *mut crate::generated::Scene {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::load_stdio(file_void, opts, error) }
}

// ufbx.c:30534-30554 `ufbx_load_stdio_prefix` (impl: native/api.rs `load_stdio_prefix`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_load_stdio_prefix(
    file_void: *mut core::ffi::c_void,
    prefix: *const core::ffi::c_void,
    prefix_size: usize,
    opts: *const crate::generated::RawLoadOpts,
    error: *mut crate::generated::Error,
) -> *mut crate::generated::Scene {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::load_stdio_prefix(file_void, prefix, prefix_size, opts, error) }
}

// ufbx.c:30556-30559 `ufbx_load_stream` (impl: native/api.rs `load_stream`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_load_stream(
    stream: *const crate::generated::RawStream,
    opts: *const crate::generated::RawLoadOpts,
    error: *mut crate::generated::Error,
) -> *mut crate::generated::Scene {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::load_stream(stream, opts, error) }
}

// ufbx.c:30561-30576 `ufbx_load_stream_prefix` (impl: native/api.rs `load_stream_prefix`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_load_stream_prefix(
    stream: *const crate::generated::RawStream,
    prefix: *const core::ffi::c_void,
    prefix_size: usize,
    opts: *const crate::generated::RawLoadOpts,
    error: *mut crate::generated::Error,
) -> *mut crate::generated::Scene {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::load_stream_prefix(stream, prefix, prefix_size, opts, error) }
}

// ufbx.c:30578-30586 `ufbx_free_scene` (impl: native/api.rs `free_scene`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_free_scene(scene: *mut crate::generated::Scene) {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::free_scene(scene) }
}

// ufbx.c:30588-30596 `ufbx_retain_scene` (impl: native/api.rs `retain_scene`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_retain_scene(scene: *mut crate::generated::Scene) {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::retain_scene(scene) }
}

// ufbx.c:30598-30633 `ufbx_format_error` (impl: native/api.rs `format_error`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_format_error(
    dst: *mut u8,
    dst_size: usize,
    error: *const crate::generated::Error,
) -> usize {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::format_error(dst, dst_size, error) }
}

// ufbx.c:30635-30650 `ufbx_find_prop_len` (impl: native/api.rs `find_prop_len`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_find_prop_len(
    props: *const crate::generated::Props,
    name: *const u8,
    name_len: usize,
) -> *mut crate::generated::Prop {
    // C-ABI root: `props` is a raw C pointer with no lifetime; bridge it to a
    // read-only `&View<Props, Const>` (legal for ANY readable provenance,
    // including a Rust caller's `&Props`) and map the correlated view back to
    // raw. Null `props` yields null (the internal `while !is_null` behavior).
    if props.is_null() {
        return core::ptr::null_mut();
    }
    // SAFETY: an ABI shim; the source pointer is bridged to a read-only
    // `View<_, Const>` (sound for any readable provenance) and the caller's
    // `name`/`name_len` key-buffer contract becomes the slice mint
    // (`slice_from_ptr` maps the null/0 case to the empty slice).
    match unsafe {
        crate::native::api::find_prop_len(
        crate::native::view::View::<crate::generated::Props, crate::native::view::Const>::from_ptr(
            props,
        ),
        crate::prelude::slice_from_ptr(name, name_len),
    )
    } {
        Some(prop) => prop.as_ptr() as *mut crate::generated::Prop,
        None => core::ptr::null_mut(),
    }
}

// ufbx.c:30652-30660 `ufbx_find_real_len` (impl: native/api.rs `find_real_len`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_find_real_len(
    props: *const crate::generated::Props,
    name: *const u8,
    name_len: usize,
    def: crate::prelude::Real,
) -> crate::prelude::Real {
    if props.is_null() {
        return def;
    }
    // SAFETY: an ABI shim; the source pointer is bridged to a read-only
    // `View<_, Const>` (sound for any readable provenance) and the caller's
    // `name`/`name_len` key-buffer contract becomes the slice mint
    // (`slice_from_ptr` maps the null/0 case to the empty slice).
    unsafe {
        crate::native::api::find_real_len(
        crate::native::view::View::<crate::generated::Props, crate::native::view::Const>::from_ptr(
            props,
        ),
        crate::prelude::slice_from_ptr(name, name_len),
        def,
    )
    }
}

// ufbx.c:30662-30670 `ufbx_find_vec3_len` (impl: native/api.rs `find_vec3_len`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_find_vec3_len(
    props: *const crate::generated::Props,
    name: *const u8,
    name_len: usize,
    def: crate::generated::Vec3,
) -> crate::generated::Vec3 {
    if props.is_null() {
        return def;
    }
    // SAFETY: an ABI shim; the source pointer is bridged to a read-only
    // `View<_, Const>` (sound for any readable provenance) and the caller's
    // `name`/`name_len` key-buffer contract becomes the slice mint
    // (`slice_from_ptr` maps the null/0 case to the empty slice).
    unsafe {
        crate::native::api::find_vec3_len(
        crate::native::view::View::<crate::generated::Props, crate::native::view::Const>::from_ptr(
            props,
        ),
        crate::prelude::slice_from_ptr(name, name_len),
        def,
    )
    }
}

// ufbx.c:30672-30680 `ufbx_find_int_len` (impl: native/api.rs `find_int_len`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_find_int_len(
    props: *const crate::generated::Props,
    name: *const u8,
    name_len: usize,
    def: i64,
) -> i64 {
    if props.is_null() {
        return def;
    }
    // SAFETY: an ABI shim; the source pointer is bridged to a read-only
    // `View<_, Const>` (sound for any readable provenance) and the caller's
    // `name`/`name_len` key-buffer contract becomes the slice mint
    // (`slice_from_ptr` maps the null/0 case to the empty slice).
    unsafe {
        crate::native::api::find_int_len(
        crate::native::view::View::<crate::generated::Props, crate::native::view::Const>::from_ptr(
            props,
        ),
        crate::prelude::slice_from_ptr(name, name_len),
        def,
    )
    }
}

// ufbx.c:30682-30690 `ufbx_find_bool_len` (impl: native/api.rs `find_bool_len`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_find_bool_len(
    props: *const crate::generated::Props,
    name: *const u8,
    name_len: usize,
    def: bool,
) -> bool {
    if props.is_null() {
        return def;
    }
    // SAFETY: an ABI shim; the source pointer is bridged to a read-only
    // `View<_, Const>` (sound for any readable provenance) and the caller's
    // `name`/`name_len` key-buffer contract becomes the slice mint
    // (`slice_from_ptr` maps the null/0 case to the empty slice).
    unsafe {
        crate::native::api::find_bool_len(
        crate::native::view::View::<crate::generated::Props, crate::native::view::Const>::from_ptr(
            props,
        ),
        crate::prelude::slice_from_ptr(name, name_len),
        def,
    )
    }
}

// ufbx.c:30692-30700 `ufbx_find_string_len` (impl: native/api.rs `find_string_len`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_find_string_len(
    props: *const crate::generated::Props,
    name: *const u8,
    name_len: usize,
    def: crate::prelude::String,
) -> crate::prelude::String {
    if props.is_null() {
        return def;
    }
    // SAFETY: an ABI shim; the source pointer is bridged to a read-only
    // `View<_, Const>` (sound for any readable provenance) and the caller's
    // `name`/`name_len` key-buffer contract becomes the slice mint
    // (`slice_from_ptr` maps the null/0 case to the empty slice).
    unsafe {
        crate::native::api::find_string_len(
        crate::native::view::View::<crate::generated::Props, crate::native::view::Const>::from_ptr(
            props,
        ),
        crate::prelude::slice_from_ptr(name, name_len),
        def,
    )
    }
}

// ufbx.c:30702-30710 `ufbx_find_blob_len` (impl: native/api.rs `find_blob_len`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_find_blob_len(
    props: *const crate::generated::Props,
    name: *const u8,
    name_len: usize,
    def: crate::prelude::Blob,
) -> crate::prelude::Blob {
    if props.is_null() {
        return def;
    }
    // SAFETY: an ABI shim; the source pointer is bridged to a read-only
    // `View<_, Const>` (sound for any readable provenance) and the caller's
    // `name`/`name_len` key-buffer contract becomes the slice mint
    // (`slice_from_ptr` maps the null/0 case to the empty slice).
    unsafe {
        crate::native::api::find_blob_len(
        crate::native::view::View::<crate::generated::Props, crate::native::view::Const>::from_ptr(
            props,
        ),
        crate::prelude::slice_from_ptr(name, name_len),
        def,
    )
    }
}

// ufbx.c:30712-30728 `ufbx_find_prop_concat` (impl: native/api.rs `find_prop_concat`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_find_prop_concat(
    props: *const crate::generated::Props,
    parts: *const crate::prelude::String,
    num_parts: usize,
) -> *mut crate::generated::Prop {
    if props.is_null() {
        return core::ptr::null_mut();
    }
    // SAFETY: an ABI shim; the source pointer is bridged to a read-only
    // `View<_, Const>` (sound for any readable provenance) and the caller's
    // `parts`/`num_parts` key-array contract becomes the slice mint.
    match unsafe {
        crate::native::api::find_prop_concat(
        crate::native::view::View::<crate::generated::Props, crate::native::view::Const>::from_ptr(
            props,
        ),
        crate::prelude::slice_from_ptr(parts, num_parts),
    )
    } {
        Some(prop) => prop.as_ptr() as *mut crate::generated::Prop,
        None => core::ptr::null_mut(),
    }
}

// ufbx.c:30730-30741 `ufbx_find_element_len` (impl: native/api.rs `find_element_len`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_find_element_len(
    scene: *const crate::generated::Scene,
    type_: crate::generated::ElementType,
    name: *const u8,
    name_len: usize,
) -> *mut crate::generated::Element {
    // SAFETY: an ABI shim; the raw struct pointer carries this `unsafe fn`'s
    // own raw-pointer contract, and the caller's name/len key-buffer contract
    // becomes the slice mint (`slice_from_ptr` maps the null/0 case to the
    // empty slice).
    unsafe {
        crate::native::api::find_element_len(
            scene,
            type_,
            crate::prelude::slice_from_ptr(name, name_len),
        )
    }
}

// ufbx.c:30743-30748 `ufbx_get_prop_element` (impl: native/api.rs `get_prop_element`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_get_prop_element(
    element: *const crate::generated::Element,
    prop: *const crate::generated::Prop,
    type_: crate::generated::ElementType,
) -> *mut crate::generated::Element {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::get_prop_element(element, prop, type_) }
}

// ufbx.c:30750-30757 `ufbx_find_prop_element_len` (impl: native/api.rs `find_prop_element_len`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_find_prop_element_len(
    element: *const crate::generated::Element,
    name: *const u8,
    name_len: usize,
    type_: crate::generated::ElementType,
) -> *mut crate::generated::Element {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::find_prop_element_len(element, name, name_len, type_) }
}

// ufbx.c:30760-30763 `ufbx_find_node_len` (impl: native/api.rs `find_node_len`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_find_node_len(
    scene: *const crate::generated::Scene,
    name: *const u8,
    name_len: usize,
) -> *mut crate::generated::Node {
    // SAFETY: an ABI shim; the raw struct pointer carries this `unsafe fn`'s
    // own raw-pointer contract, and the caller's name/len key-buffer contract
    // becomes the slice mint (`slice_from_ptr` maps the null/0 case to the
    // empty slice).
    unsafe {
        crate::native::api::find_node_len(scene, crate::prelude::slice_from_ptr(name, name_len))
    }
}

// ufbx.c:30765-30768 `ufbx_find_anim_stack_len` (impl: native/api.rs `find_anim_stack_len`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_find_anim_stack_len(
    scene: *const crate::generated::Scene,
    name: *const u8,
    name_len: usize,
) -> *mut crate::generated::AnimStack {
    // SAFETY: an ABI shim; the raw struct pointer carries this `unsafe fn`'s
    // own raw-pointer contract, and the caller's name/len key-buffer contract
    // becomes the slice mint (`slice_from_ptr` maps the null/0 case to the
    // empty slice).
    unsafe {
        crate::native::api::find_anim_stack_len(
            scene,
            crate::prelude::slice_from_ptr(name, name_len),
        )
    }
}

// ufbx.c:30770-30773 `ufbx_find_material_len` (impl: native/api.rs `find_material_len`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_find_material_len(
    scene: *const crate::generated::Scene,
    name: *const u8,
    name_len: usize,
) -> *mut crate::generated::Material {
    // SAFETY: an ABI shim; the raw struct pointer carries this `unsafe fn`'s
    // own raw-pointer contract, and the caller's name/len key-buffer contract
    // becomes the slice mint (`slice_from_ptr` maps the null/0 case to the
    // empty slice).
    unsafe {
        crate::native::api::find_material_len(scene, crate::prelude::slice_from_ptr(name, name_len))
    }
}

// ufbx.c:30775-30790 `ufbx_find_anim_prop_len` (impl: native/api.rs `find_anim_prop_len`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_find_anim_prop_len(
    layer: *const crate::generated::AnimLayer,
    element: *const crate::generated::Element,
    prop: *const u8,
    prop_len: usize,
) -> *mut crate::generated::AnimProp {
    // SAFETY: an ABI shim; the caller's null-or-live layer contract becomes
    // the read-only `View<_, Const>` mint (legal for any readable provenance),
    // `element` is address-only, and the name/len key-buffer contract becomes
    // the slice mint (`slice_from_ptr` maps the null/0 case to the empty
    // slice).
    match unsafe {
        crate::native::api::find_anim_prop_len(
            if layer.is_null() {
                None
            } else {
                Some(crate::native::view::View::<
                    crate::generated::AnimLayer,
                    crate::native::view::Const,
                >::from_ptr(layer))
            },
            element,
            crate::prelude::slice_from_ptr(prop, prop_len),
        )
    } {
        Some(found) => found.as_ptr() as *mut crate::generated::AnimProp,
        None => core::ptr::null_mut(),
    }
}

// ufbx.c:30792-30812 `ufbx_find_anim_props` (impl: native/api.rs `find_anim_props`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_find_anim_props(
    layer: *const crate::generated::AnimLayer,
    element: *const crate::generated::Element,
) -> crate::prelude::List<crate::generated::AnimProp> {
    // SAFETY: an ABI shim; the caller's null-or-live layer contract becomes
    // the read-only `View<_, Const>` mint (legal for any readable provenance);
    // `element` is address-only.
    crate::native::api::find_anim_props(
        if layer.is_null() {
            None
        } else {
            Some(unsafe {
                crate::native::view::View::<crate::generated::AnimLayer, crate::native::view::Const>::from_ptr(layer)
            })
        },
        element,
    )
}

// ufbx.c:30814-30825 `ufbx_get_compatible_matrix_for_normals`
// (impl: native/api.rs `get_compatible_matrix_for_normals`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_get_compatible_matrix_for_normals(
    node: *const crate::generated::Node,
) -> crate::generated::Matrix {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::get_compatible_matrix_for_normals(node) }
}

// ufbx.c:30827-30830 `ufbx_evaluate_curve` (impl: native/api.rs `evaluate_curve`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_evaluate_curve(
    curve: *const crate::generated::AnimCurve,
    time: f64,
    default_value: crate::prelude::Real,
) -> crate::prelude::Real {
    // SAFETY: an ABI shim; the caller's null-or-live pointer contract
    // becomes the read-only `View<_, Const>` mint (legal for any readable
    // provenance).
    let curve = if curve.is_null() {
        None
    } else {
        Some(unsafe {
            crate::native::view::View::<crate::generated::AnimCurve, crate::native::view::Const>::from_ptr(curve)
        })
    };
    crate::native::api::evaluate_curve(curve, time, default_value)
}

// ufbx.c:30832-30914 `ufbx_evaluate_curve_flags` (impl: native/api.rs
// `evaluate_curve_flags`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_evaluate_curve_flags(
    curve: *const crate::generated::AnimCurve,
    time: f64,
    default_value: crate::prelude::Real,
    flags: u32,
) -> crate::prelude::Real {
    // SAFETY: an ABI shim; the caller's null-or-live pointer contract
    // becomes the read-only `View<_, Const>` mint (legal for any readable
    // provenance).
    let curve = if curve.is_null() {
        None
    } else {
        Some(unsafe {
            crate::native::view::View::<crate::generated::AnimCurve, crate::native::view::Const>::from_ptr(curve)
        })
    };
    crate::native::api::evaluate_curve_flags(curve, time, default_value, flags)
}

// ufbx.c:30916-30919 `ufbx_evaluate_anim_value_real` (impl: native/api.rs
// `evaluate_anim_value_real`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_evaluate_anim_value_real(
    anim_value: *const crate::generated::AnimValue,
    time: f64,
) -> crate::prelude::Real {
    // SAFETY: an ABI shim; the caller's null-or-live pointer contract
    // becomes the read-only `View<_, Const>` mint (legal for any readable
    // provenance).
    let anim_value = if anim_value.is_null() {
        None
    } else {
        Some(unsafe {
            crate::native::view::View::<crate::generated::AnimValue, crate::native::view::Const>::from_ptr(anim_value)
        })
    };
    crate::native::api::evaluate_anim_value_real(anim_value, time)
}

// ufbx.c:30921-30924 `ufbx_evaluate_anim_value_vec3` (impl: native/api.rs
// `evaluate_anim_value_vec3`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_evaluate_anim_value_vec3(
    anim_value: *const crate::generated::AnimValue,
    time: f64,
) -> crate::generated::Vec3 {
    // SAFETY: an ABI shim; the caller's null-or-live pointer contract
    // becomes the read-only `View<_, Const>` mint (legal for any readable
    // provenance).
    let anim_value = if anim_value.is_null() {
        None
    } else {
        Some(unsafe {
            crate::native::view::View::<crate::generated::AnimValue, crate::native::view::Const>::from_ptr(anim_value)
        })
    };
    crate::native::api::evaluate_anim_value_vec3(anim_value, time)
}

// ufbx.c:30926-30935 `ufbx_evaluate_anim_value_real_flags` (impl: native/api.rs
// `evaluate_anim_value_real_flags`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_evaluate_anim_value_real_flags(
    anim_value: *const crate::generated::AnimValue,
    time: f64,
    flags: u32,
) -> crate::prelude::Real {
    // SAFETY: an ABI shim; the caller's null-or-live pointer contract
    // becomes the read-only `View<_, Const>` mint (legal for any readable
    // provenance).
    let anim_value = if anim_value.is_null() {
        None
    } else {
        Some(unsafe {
            crate::native::view::View::<crate::generated::AnimValue, crate::native::view::Const>::from_ptr(anim_value)
        })
    };
    crate::native::api::evaluate_anim_value_real_flags(anim_value, time, flags)
}

// ufbx.c:30937-30949 `ufbx_evaluate_anim_value_vec3_flags` (impl: native/api.rs
// `evaluate_anim_value_vec3_flags`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_evaluate_anim_value_vec3_flags(
    anim_value: *const crate::generated::AnimValue,
    time: f64,
    flags: u32,
) -> crate::generated::Vec3 {
    // SAFETY: an ABI shim; the caller's null-or-live pointer contract
    // becomes the read-only `View<_, Const>` mint (legal for any readable
    // provenance).
    let anim_value = if anim_value.is_null() {
        None
    } else {
        Some(unsafe {
            crate::native::view::View::<crate::generated::AnimValue, crate::native::view::Const>::from_ptr(anim_value)
        })
    };
    crate::native::api::evaluate_anim_value_vec3_flags(anim_value, time, flags)
}

// ufbx.c:30951-30954 `ufbx_evaluate_prop_len` (impl: native/api.rs
// `evaluate_prop_len`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_evaluate_prop_len(
    anim: *const crate::generated::Anim,
    element: *const crate::generated::Element,
    name: *const u8,
    name_len: usize,
    time: f64,
) -> crate::generated::Prop {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::evaluate_prop_len(anim, element, name, name_len, time) }
}

// ufbx.c:30956-30989 `ufbx_evaluate_prop_flags_len` (impl: native/api.rs
// `evaluate_prop_flags_len`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_evaluate_prop_flags_len(
    anim: *const crate::generated::Anim,
    element: *const crate::generated::Element,
    name: *const u8,
    name_len: usize,
    time: f64,
    flags: u32,
) -> crate::generated::Prop {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe {
        crate::native::api::evaluate_prop_flags_len(anim, element, name, name_len, time, flags)
    }
}

// ufbx.c:30991-30994 `ufbx_evaluate_props` (impl: native/api.rs `evaluate_props`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_evaluate_props(
    anim: *const crate::generated::Anim,
    element: *const crate::generated::Element,
    time: f64,
    buffer: *mut crate::generated::Prop,
    buffer_size: usize,
) -> crate::generated::Props {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::evaluate_props(anim, element, time, buffer, buffer_size) }
}

// ufbx.c:30996-31023 `ufbx_evaluate_props_flags` (impl: native/api.rs
// `evaluate_props_flags`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_evaluate_props_flags(
    anim: *const crate::generated::Anim,
    element: *const crate::generated::Element,
    time: f64,
    buffer: *mut crate::generated::Prop,
    buffer_size: usize,
    flags: u32,
) -> crate::generated::Props {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe {
        crate::native::api::evaluate_props_flags(anim, element, time, buffer, buffer_size, flags)
    }
}

// ufbx.c:31025-31028 `ufbx_evaluate_transform` (impl: native/api.rs
// `evaluate_transform`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_evaluate_transform(
    anim: *const crate::generated::Anim,
    node: *const crate::generated::Node,
    time: f64,
) -> crate::generated::Transform {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::evaluate_transform(anim, node, time) }
}

// ufbx.c:31062-31160 `ufbx_evaluate_transform_flags` (impl: native/api.rs
// `evaluate_transform_flags`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_evaluate_transform_flags(
    anim: *const crate::generated::Anim,
    node: *const crate::generated::Node,
    time: f64,
    flags: u32,
) -> crate::generated::Transform {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::evaluate_transform_flags(anim, node, time, flags) }
}

// ufbx.c:31162-31165 `ufbx_evaluate_blend_weight` (impl: native/api.rs
// `evaluate_blend_weight`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_evaluate_blend_weight(
    anim: *const crate::generated::Anim,
    channel: *const crate::generated::BlendChannel,
    time: f64,
) -> crate::prelude::Real {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::evaluate_blend_weight(anim, channel, time) }
}

// ufbx.c:31167-31176 `ufbx_evaluate_blend_weight_flags` (impl: native/api.rs
// `evaluate_blend_weight_flags`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_evaluate_blend_weight_flags(
    anim: *const crate::generated::Anim,
    channel: *const crate::generated::BlendChannel,
    time: f64,
    flags: u32,
) -> crate::prelude::Real {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::evaluate_blend_weight_flags(anim, channel, time, flags) }
}

// ufbx.c:31178-31192 `ufbx_evaluate_scene` (impl: native/api.rs
// `evaluate_scene` — a cfg'd fn per C arm, so the shim itself is
// unconditional).
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_evaluate_scene(
    scene: *const crate::generated::Scene,
    anim: *const crate::generated::Anim,
    time: f64,
    opts: *const crate::generated::RawEvaluateOpts,
    error: *mut crate::generated::Error,
) -> *mut crate::generated::Scene {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract. The native impl is `Result`-shaped; this shim
    // owns the C slot writes — cleared on success, the fixed error on failure
    // (the entry's C write pattern, byte-exact).
    match unsafe { crate::native::api::evaluate_scene(scene, anim, time, opts) } {
        Ok(result) => {
            if !error.is_null() {
                // SAFETY: `error` is non-null (checked) and the caller's live
                // slot per this shim's contract.
                unsafe { crate::native::error::clear_error(error) };
            }
            result
        }
        Err(e) => {
            if !error.is_null() {
                // SAFETY: as above; the write covers exactly one `Error`.
                unsafe { core::ptr::write(error, e) };
            }
            core::ptr::null_mut()
        }
    }
}

// ufbx.c:31194-31218 `ufbx_create_anim` (impl: native/api.rs `create_anim`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_create_anim(
    scene: *const crate::generated::Scene,
    opts: *const crate::generated::RawAnimOpts,
    error: *mut crate::generated::Error,
) -> *mut crate::generated::Anim {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::create_anim(scene, opts, error) }
}

// ufbx.c:31220-31229 `ufbx_free_anim` (impl: native/api.rs `free_anim`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_free_anim(anim: *mut crate::generated::Anim) {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::free_anim(anim) }
}

// ufbx.c:31231-31240 `ufbx_retain_anim` (impl: native/api.rs `retain_anim`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_retain_anim(anim: *mut crate::generated::Anim) {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::retain_anim(anim) }
}

// ufbx.c:31242-31289 `ufbx_bake_anim` (impl: native/api.rs `bake_anim` — a
// cfg'd fn per C arm, so the shim itself is unconditional).
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_bake_anim(
    scene: *const crate::generated::Scene,
    anim: *const crate::generated::Anim,
    opts: *const crate::generated::RawBakeOpts,
    error: *mut crate::generated::Error,
) -> *mut crate::generated::BakedAnim {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::bake_anim(scene, anim, opts, error) }
}

// ufbx.c:31291-31299 `ufbx_retain_baked_anim` (impl: native/api.rs
// `retain_baked_anim`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_retain_baked_anim(bake: *mut crate::generated::BakedAnim) {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::retain_baked_anim(bake) }
}

// ufbx.c:31301-31309 `ufbx_free_baked_anim` (impl: native/api.rs
// `free_baked_anim`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_free_baked_anim(bake: *mut crate::generated::BakedAnim) {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::free_baked_anim(bake) }
}

// ufbx.c:31312-31318 `ufbx_find_baked_node_by_typed_id`
// (impl: native/api.rs `find_baked_node_by_typed_id`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_find_baked_node_by_typed_id(
    bake: *mut crate::generated::BakedAnim,
    typed_id: u32,
) -> *mut crate::generated::BakedNode {
    match crate::native::api::find_baked_node_by_typed_id(
        // C-parity: no null check on `bake` (mirrors the unchecked C deref).
        // SAFETY: C-ABI root; `from_ptr` reinterprets the caller's pointer as a
        // read-only `View<_, Const>`, sound for any readable provenance, over a
        // pointee the caller owns per this `unsafe fn`'s contract.
        unsafe {
            crate::native::view::View::<crate::generated::BakedAnim, crate::native::view::Const>::from_ptr(bake)
        },
        typed_id,
    ) {
        Some(node) => node.as_ptr() as *mut crate::generated::BakedNode,
        None => core::ptr::null_mut(),
    }
}

// ufbx.c:31320-31324 `ufbx_find_baked_node` (impl: native/api.rs `find_baked_node`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_find_baked_node(
    bake: *mut crate::generated::BakedAnim,
    node: *mut crate::generated::Node,
) -> *mut crate::generated::BakedNode {
    match crate::native::api::find_baked_node(
        if bake.is_null() {
            None
        } else {
            // SAFETY: C-ABI root; `from_ptr` reinterprets the caller's pointer as a
            // read-only `View<_, Const>`, sound for any readable provenance, over a
            // pointee the caller owns per this `unsafe fn`'s contract.
            Some(unsafe {
                crate::native::view::View::<
                crate::generated::BakedAnim,
                crate::native::view::Const,
            >::from_ptr(bake)
            })
        },
        if node.is_null() {
            None
        } else {
            // SAFETY: C-ABI root; `from_ptr` reinterprets the caller's pointer as a
            // read-only `View<_, Const>`, sound for any readable provenance, over a
            // pointee the caller owns per this `unsafe fn`'s contract.
            Some(unsafe {
                crate::native::view::View::<
                crate::generated::Node,
                crate::native::view::Const,
            >::from_ptr(node)
            })
        },
    ) {
        Some(baked) => baked.as_ptr() as *mut crate::generated::BakedNode,
        None => core::ptr::null_mut(),
    }
}

// ufbx.c:31326-31332 `ufbx_find_baked_element_by_element_id`
// (impl: native/api.rs `find_baked_element_by_element_id`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_find_baked_element_by_element_id(
    bake: *mut crate::generated::BakedAnim,
    element_id: u32,
) -> *mut crate::generated::BakedElement {
    match crate::native::api::find_baked_element_by_element_id(
        // C-parity: no null check on `bake` (mirrors the unchecked C deref).
        // SAFETY: C-ABI root; `from_ptr` reinterprets the caller's pointer as a
        // read-only `View<_, Const>`, sound for any readable provenance, over a
        // pointee the caller owns per this `unsafe fn`'s contract.
        unsafe {
            crate::native::view::View::<crate::generated::BakedAnim, crate::native::view::Const>::from_ptr(bake)
        },
        element_id,
    ) {
        Some(elem) => elem.as_ptr() as *mut crate::generated::BakedElement,
        None => core::ptr::null_mut(),
    }
}

// ufbx.c:31334-31338 `ufbx_find_baked_element` (impl: native/api.rs `find_baked_element`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_find_baked_element(
    bake: *mut crate::generated::BakedAnim,
    element: *mut crate::generated::Element,
) -> *mut crate::generated::BakedElement {
    match crate::native::api::find_baked_element(
        if bake.is_null() {
            None
        } else {
            // SAFETY: C-ABI root; `from_ptr` reinterprets the caller's pointer as a
            // read-only `View<_, Const>`, sound for any readable provenance, over a
            // pointee the caller owns per this `unsafe fn`'s contract.
            Some(unsafe {
                crate::native::view::View::<
                crate::generated::BakedAnim,
                crate::native::view::Const,
            >::from_ptr(bake)
            })
        },
        if element.is_null() {
            None
        } else {
            // SAFETY: C-ABI root; `from_ptr` reinterprets the caller's pointer as a
            // read-only `View<_, Const>`, sound for any readable provenance, over a
            // pointee the caller owns per this `unsafe fn`'s contract.
            Some(unsafe {
                crate::native::view::View::<
                crate::generated::Element,
                crate::native::view::Const,
            >::from_ptr(element)
            })
        },
    ) {
        Some(elem) => elem.as_ptr() as *mut crate::generated::BakedElement,
        None => core::ptr::null_mut(),
    }
}

// ufbx.c:31340-31370 `ufbx_evaluate_baked_vec3` (impl: native/api.rs `evaluate_baked_vec3`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_evaluate_baked_vec3(
    keyframes: crate::prelude::List<crate::generated::BakedVec3>,
    time: f64,
) -> crate::generated::Vec3 {
    // SAFETY: an ABI shim; `keyframes.data`/`count` describe the caller's live
    // keyframe run per this `unsafe fn`'s contract, forwarded unchanged to the
    // native impl whose contract is identical.
    unsafe { crate::native::api::evaluate_baked_vec3(keyframes, time) }
}

// ufbx.c:31372-31403 `ufbx_evaluate_baked_quat` (impl: native/api.rs `evaluate_baked_quat`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_evaluate_baked_quat(
    keyframes: crate::prelude::List<crate::generated::BakedQuat>,
    time: f64,
) -> crate::generated::Quat {
    // SAFETY: an ABI shim; `keyframes.data`/`count` describe the caller's live
    // keyframe run per this `unsafe fn`'s contract, forwarded unchanged to the
    // native impl whose contract is identical.
    unsafe { crate::native::api::evaluate_baked_quat(keyframes, time) }
}

// ufbx.c:31405-31412 `ufbx_get_bone_pose` (impl: native/api.rs `get_bone_pose`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_get_bone_pose(
    pose: *const crate::generated::Pose,
    node: *const crate::generated::Node,
) -> *mut crate::generated::BonePose {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::get_bone_pose(pose, node) }
}

// ufbx.c:31414-31423 `ufbx_find_prop_texture_len` (impl: native/api.rs `find_prop_texture_len`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_find_prop_texture_len(
    material: *const crate::generated::Material,
    name: *const u8,
    name_len: usize,
) -> *mut crate::generated::Texture {
    // SAFETY: an ABI shim; the raw struct pointer carries this `unsafe fn`'s
    // own raw-pointer contract, and the caller's name/len key-buffer contract
    // becomes the slice mint (`slice_from_ptr` maps the null/0 case to the
    // empty slice).
    unsafe {
        crate::native::api::find_prop_texture_len(
            material,
            crate::prelude::slice_from_ptr(name, name_len),
        )
    }
}

// ufbx.c:31425-31432 `ufbx_find_shader_prop_len` (impl: native/api.rs `find_shader_prop_len`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_find_shader_prop_len(
    shader: *const crate::generated::Shader,
    name: *const u8,
    name_len: usize,
) -> crate::prelude::String {
    // SAFETY: an ABI shim; the caller's null-or-live shader contract becomes
    // the read-only `View<_, Const>` mint (legal for any readable provenance),
    // and the name/len key-buffer contract becomes the slice mint
    // (`slice_from_ptr` maps the null/0 case to the empty slice).
    unsafe {
        crate::native::api::find_shader_prop_len(
            if shader.is_null() {
                None
            } else {
                Some(crate::native::view::View::<
                    crate::generated::Shader,
                    crate::native::view::Const,
                >::from_ptr(shader))
            },
            crate::prelude::slice_from_ptr(name, name_len),
        )
    }
}

// ufbx.c:31434-31461 `ufbx_find_shader_prop_bindings_len`
// (impl: native/api.rs `find_shader_prop_bindings_len`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_find_shader_prop_bindings_len(
    shader: *const crate::generated::Shader,
    name: *const u8,
    name_len: usize,
) -> crate::prelude::List<crate::generated::ShaderPropBinding> {
    // SAFETY: an ABI shim; the caller's null-or-live shader contract becomes
    // the read-only `View<_, Const>` mint (legal for any readable provenance),
    // and the name/len key-buffer contract becomes the slice mint
    // (`slice_from_ptr` maps the null/0 case to the empty slice).
    unsafe {
        crate::native::api::find_shader_prop_bindings_len(
            if shader.is_null() {
                None
            } else {
                Some(crate::native::view::View::<
                    crate::generated::Shader,
                    crate::native::view::Const,
                >::from_ptr(shader))
            },
            crate::prelude::slice_from_ptr(name, name_len),
        )
    }
}

// ufbx.c:31463-31476 `ufbx_find_shader_texture_input_len`
// (impl: native/api.rs `find_shader_texture_input_len`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_find_shader_texture_input_len(
    shader: *const crate::generated::ShaderTexture,
    name: *const u8,
    name_len: usize,
) -> *mut crate::generated::ShaderTextureInput {
    // SAFETY: an ABI shim; the caller's live shader contract becomes the
    // read-only `View<_, Const>` mint (legal for any readable provenance), and
    // the name/len key-buffer contract becomes the slice mint (`slice_from_ptr`
    // maps the null/0 case to the empty slice).
    match unsafe {
        crate::native::api::find_shader_texture_input_len(
            crate::native::view::View::<crate::generated::ShaderTexture, crate::native::view::Const>::from_ptr(shader),
            crate::prelude::slice_from_ptr(name, name_len),
        )
    } {
        Some(input) => input.as_ptr() as *mut crate::generated::ShaderTextureInput,
        None => core::ptr::null_mut(),
    }
}

// ufbx.c:31478-31490 `ufbx_coordinate_axes_valid` (impl: native/api.rs `coordinate_axes_valid`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_coordinate_axes_valid(
    axes: crate::generated::CoordinateAxes,
) -> bool {
    crate::native::api::coordinate_axes_valid(axes)
}

// ufbx.c:31492-31495 `ufbx_quat_mul` (impl: native/api.rs `quat_mul`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_quat_mul(
    a: crate::generated::Quat,
    b: crate::generated::Quat,
) -> crate::generated::Quat {
    crate::native::api::quat_mul(a, b)
}

// ufbx.c:31497-31500 `ufbx_vec3_normalize` (impl: native/api.rs `vec3_normalize`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_vec3_normalize(v: crate::generated::Vec3) -> crate::generated::Vec3 {
    crate::native::api::vec3_normalize(v)
}

// ufbx.c:31502-31505 `ufbx_quat_dot` (impl: native/api.rs `quat_dot`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_quat_dot(
    a: crate::generated::Quat,
    b: crate::generated::Quat,
) -> crate::prelude::Real {
    crate::native::api::quat_dot(a, b)
}

// ufbx.c:31507-31517 `ufbx_quat_normalize` (impl: native/api.rs `quat_normalize`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_quat_normalize(q: crate::generated::Quat) -> crate::generated::Quat {
    crate::native::api::quat_normalize(q)
}

// ufbx.c:31519-31525 `ufbx_quat_fix_antipodal` (impl: native/api.rs `quat_fix_antipodal`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_quat_fix_antipodal(
    q: crate::generated::Quat,
    reference: crate::generated::Quat,
) -> crate::generated::Quat {
    crate::native::api::quat_fix_antipodal(q, reference)
}

// ufbx.c:31527-31552 `ufbx_quat_slerp` (impl: native/api.rs `quat_slerp`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_quat_slerp(
    a: crate::generated::Quat,
    b: crate::generated::Quat,
    t: crate::prelude::Real,
) -> crate::generated::Quat {
    crate::native::api::quat_slerp(a, b, t)
}

// ufbx.c:31554-31564 `ufbx_quat_rotate_vec3` (impl: native/api.rs `quat_rotate_vec3`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_quat_rotate_vec3(
    q: crate::generated::Quat,
    v: crate::generated::Vec3,
) -> crate::generated::Vec3 {
    crate::native::api::quat_rotate_vec3(q, v)
}

// ufbx.c:31566-31620 `ufbx_euler_to_quat` (impl: native/api.rs `euler_to_quat`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_euler_to_quat(
    v: crate::generated::Vec3,
    order: crate::generated::RotationOrder,
) -> crate::generated::Quat {
    crate::native::api::euler_to_quat(v, order)
}

// ufbx.c:31622-31721 `ufbx_quat_to_euler` (impl: native/api.rs `quat_to_euler`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_quat_to_euler(
    q: crate::generated::Quat,
    order: crate::generated::RotationOrder,
) -> crate::generated::Vec3 {
    crate::native::api::quat_to_euler(q, order)
}

// ufbx.c:31723-31747 `ufbx_matrix_mul` (impl: native/api.rs `matrix_mul`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_matrix_mul(
    a: *const crate::generated::Matrix,
    b: *const crate::generated::Matrix,
) -> crate::generated::Matrix {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::matrix_mul(a, b) }
}

// ufbx.c:31749-31754 `ufbx_matrix_determinant` (impl: native/api.rs `matrix_determinant`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_matrix_determinant(
    m: *const crate::generated::Matrix,
) -> crate::prelude::Real {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::matrix_determinant(m) }
}

// ufbx.c:31756-31782 `ufbx_matrix_invert` (impl: native/api.rs `matrix_invert`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_matrix_invert(
    m: *const crate::generated::Matrix,
) -> crate::generated::Matrix {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::matrix_invert(m) }
}

// ufbx.c:31784-31802 `ufbx_matrix_for_normals` (impl: native/api.rs `matrix_for_normals`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_matrix_for_normals(
    m: *const crate::generated::Matrix,
) -> crate::generated::Matrix {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::matrix_for_normals(m) }
}

// ufbx.c:31804-31814 `ufbx_transform_position` (impl: native/api.rs `transform_position`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_transform_position(
    m: *const crate::generated::Matrix,
    v: crate::generated::Vec3,
) -> crate::generated::Vec3 {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::transform_position(m, v) }
}

// ufbx.c:31816-31826 `ufbx_transform_direction` (impl: native/api.rs `transform_direction`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_transform_direction(
    m: *const crate::generated::Matrix,
    v: crate::generated::Vec3,
) -> crate::generated::Vec3 {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::transform_direction(m, v) }
}

// ufbx.c:31828-31852 `ufbx_transform_to_matrix` (impl: native/api.rs `transform_to_matrix`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_transform_to_matrix(
    t: *const crate::generated::Transform,
) -> crate::generated::Matrix {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::transform_to_matrix(t) }
}

// ufbx.c:31854-31926 `ufbx_matrix_to_transform` (impl: native/api.rs `matrix_to_transform`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_matrix_to_transform(
    m: *const crate::generated::Matrix,
) -> crate::generated::Transform {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::matrix_to_transform(m) }
}

// ufbx.c:31928-32018 `ufbx_catch_get_skin_vertex_matrix`
// (impl: native/api.rs `catch_get_skin_vertex_matrix`). `ufbx_get_skin_vertex_matrix`
// is `ufbx_inline` in ufbx.h (5601-5603) and needs no shim.
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_catch_get_skin_vertex_matrix(
    panic: *mut crate::generated::Panic,
    skin: *const crate::generated::SkinDeformer,
    vertex: usize,
    fallback: *const crate::generated::Matrix,
) -> crate::generated::Matrix {
    // SAFETY: an ABI shim; `skin` is bridged to a read-only `View<_, Const>`
    // (sound for any readable provenance), `panic` is null or caller-owned with
    // exclusive access for this call so `as_mut` is sound, and the remaining raw
    // arguments carry this `unsafe fn`'s contract, forwarded to the native impl.
    unsafe {
        crate::native::api::catch_get_skin_vertex_matrix(
        panic.as_mut(),
        crate::native::view::View::<crate::generated::SkinDeformer, crate::native::view::Const>::from_ptr(skin),
        vertex,
        fallback,
    )
    }
}

// ufbx.c:32020-32033 `ufbx_get_blend_shape_offset_index`
// (impl: native/api.rs `get_blend_shape_offset_index`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_get_blend_shape_offset_index(
    shape: *const crate::generated::BlendShape,
    vertex: usize,
) -> u32 {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::get_blend_shape_offset_index(shape, vertex) }
}

// ufbx.c:32035-32040 `ufbx_get_blend_shape_vertex_offset`
// (impl: native/api.rs `get_blend_shape_vertex_offset`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_get_blend_shape_vertex_offset(
    shape: *const crate::generated::BlendShape,
    vertex: usize,
) -> crate::generated::Vec3 {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::get_blend_shape_vertex_offset(shape, vertex) }
}

// ufbx.c:32042-32060 `ufbx_get_blend_vertex_offset`
// (impl: native/api.rs `get_blend_vertex_offset`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_get_blend_vertex_offset(
    blend: *const crate::generated::BlendDeformer,
    vertex: usize,
) -> crate::generated::Vec3 {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::get_blend_vertex_offset(blend, vertex) }
}

// ufbx.c:32062-32081 `ufbx_add_blend_shape_vertex_offsets`
// (impl: native/api.rs `add_blend_shape_vertex_offsets`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_add_blend_shape_vertex_offsets(
    shape: *const crate::generated::BlendShape,
    vertices: *mut crate::generated::Vec3,
    num_vertices: usize,
    weight: crate::prelude::Real,
) {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe {
        crate::native::api::add_blend_shape_vertex_offsets(shape, vertices, num_vertices, weight)
    }
}

// ufbx.c:32083-32095 `ufbx_add_blend_vertex_offsets`
// (impl: native/api.rs `add_blend_vertex_offsets`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_add_blend_vertex_offsets(
    blend: *const crate::generated::BlendDeformer,
    vertices: *mut crate::generated::Vec3,
    num_vertices: usize,
    weight: crate::prelude::Real,
) {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::add_blend_vertex_offsets(blend, vertices, num_vertices, weight) }
}

// ufbx.c:32097-32166 `ufbx_evaluate_nurbs_basis` (impl: native/api.rs
// `evaluate_nurbs_basis`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_evaluate_nurbs_basis(
    basis: *const crate::generated::NurbsBasis,
    u: crate::prelude::Real,
    weights: *mut crate::prelude::Real,
    num_weights: usize,
    derivatives: *mut crate::prelude::Real,
    num_derivatives: usize,
) -> usize {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe {
        crate::native::api::evaluate_nurbs_basis(
            basis,
            u,
            weights,
            num_weights,
            derivatives,
            num_derivatives,
        )
    }
}

// ufbx.c:32168-32212 `ufbx_evaluate_nurbs_curve` (impl: native/api.rs
// `evaluate_nurbs_curve`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_evaluate_nurbs_curve(
    curve: *const crate::generated::NurbsCurve,
    u: crate::prelude::Real,
) -> crate::generated::CurvePoint {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::evaluate_nurbs_curve(curve, u) }
}

// ufbx.c:32214-32280 `ufbx_evaluate_nurbs_surface` (impl: native/api.rs
// `evaluate_nurbs_surface`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_evaluate_nurbs_surface(
    surface: *const crate::generated::NurbsSurface,
    u: crate::prelude::Real,
    v: crate::prelude::Real,
) -> crate::generated::SurfacePoint {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::evaluate_nurbs_surface(surface, u, v) }
}

// ufbx.c:32282-32318 `ufbx_tessellate_nurbs_curve` (impl: native/api.rs
// `tessellate_nurbs_curve` — a cfg'd fn per C arm, so the shim itself is
// unconditional).
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_tessellate_nurbs_curve(
    curve: *const crate::generated::NurbsCurve,
    opts: *const crate::generated::RawTessellateCurveOpts,
    error: *mut crate::generated::Error,
) -> *mut crate::generated::LineCurve {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::tessellate_nurbs_curve(curve, opts, error) }
}

// ufbx.c:32320-32357 `ufbx_tessellate_nurbs_surface` (impl: native/api.rs
// `tessellate_nurbs_surface`). Same cfg rationale.
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_tessellate_nurbs_surface(
    surface: *const crate::generated::NurbsSurface,
    opts: *const crate::generated::RawTessellateSurfaceOpts,
    error: *mut crate::generated::Error,
) -> *mut crate::generated::Mesh {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::tessellate_nurbs_surface(surface, opts, error) }
}

// ufbx.c:32359-32368 `ufbx_free_line_curve` (impl: native/api.rs
// `free_line_curve`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_free_line_curve(line_curve: *mut crate::generated::LineCurve) {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::free_line_curve(line_curve) }
}

// ufbx.c:32370-32379 `ufbx_retain_line_curve` (impl: native/api.rs
// `retain_line_curve`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_retain_line_curve(line_curve: *mut crate::generated::LineCurve) {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::retain_line_curve(line_curve) }
}

// ufbx.c:32381-32390 `ufbx_find_face_index` (impl: native/api.rs
// `find_face_index`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_find_face_index(
    mesh: *mut crate::generated::Mesh,
    index: usize,
) -> u32 {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::find_face_index(mesh, index) }
}

// ufbx.c:32392-32475 `ufbx_catch_triangulate_face` (impl: native/api.rs
// `catch_triangulate_face`). Both `#if UFBXI_FEATURE_TRIANGULATION` arms are
// ported, so the shim is unconditional.
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_catch_triangulate_face(
    panic: *mut crate::generated::Panic,
    indices: *mut u32,
    num_indices: usize,
    mesh: *const crate::generated::Mesh,
    face: crate::generated::Face,
) -> u32 {
    // SAFETY: an ABI shim; `mesh` is bridged to a read-only `View<_, Const>`,
    // `panic` is null or caller-owned with exclusive access for this call so
    // `as_mut` is sound, and the remaining raw arguments carry this `unsafe fn`'s
    // contract, forwarded to the native impl.
    unsafe {
        crate::native::api::catch_triangulate_face(
        panic.as_mut(),
        indices,
        num_indices,
        crate::native::view::View::<crate::generated::Mesh, crate::native::view::Const>::from_ptr(
            mesh,
        ),
        face,
    )
    }
}

// ufbx.c:32477-32482 `ufbx_catch_compute_topology` (impl: native/api.rs
// `catch_compute_topology`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_catch_compute_topology(
    panic: *mut crate::generated::Panic,
    mesh: *const crate::generated::Mesh,
    indices: *mut crate::generated::TopoEdge,
    num_indices: usize,
) {
    // SAFETY: an ABI shim; `mesh` is bridged to a read-only `View<_, Const>`,
    // `panic` is null or caller-owned with exclusive access for this call so
    // `as_mut` is sound, and the remaining raw arguments carry this `unsafe fn`'s
    // contract, forwarded to the native impl.
    unsafe {
        crate::native::api::catch_compute_topology(
        panic.as_mut(),
        crate::native::view::View::<crate::generated::Mesh, crate::native::view::Const>::from_ptr(
            mesh,
        ),
        indices,
        num_indices,
    )
    }
}

// ufbx.c:32484-32492 `ufbx_catch_topo_next_vertex_edge` (impl: native/api.rs
// `catch_topo_next_vertex_edge`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_catch_topo_next_vertex_edge(
    panic: *mut crate::generated::Panic,
    topo: *const crate::generated::TopoEdge,
    num_topo: usize,
    index: u32,
) -> u32 {
    // SAFETY: an ABI shim; `panic` is null or caller-owned with exclusive access
    // for this call so `as_mut` is sound; the remaining raw arguments carry this
    // `unsafe fn`'s contract, forwarded unchanged to the native impl.
    unsafe {
        crate::native::api::catch_topo_next_vertex_edge(panic.as_mut(), topo, num_topo, index)
    }
}

// ufbx.c:32494-32499 `ufbx_catch_topo_prev_vertex_edge` (impl: native/api.rs
// `catch_topo_prev_vertex_edge`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_catch_topo_prev_vertex_edge(
    panic: *mut crate::generated::Panic,
    topo: *const crate::generated::TopoEdge,
    num_topo: usize,
    index: u32,
) -> u32 {
    // SAFETY: an ABI shim; `panic` is null or caller-owned with exclusive access
    // for this call so `as_mut` is sound; the remaining raw arguments carry this
    // `unsafe fn`'s contract, forwarded unchanged to the native impl.
    unsafe {
        crate::native::api::catch_topo_prev_vertex_edge(panic.as_mut(), topo, num_topo, index)
    }
}

// ufbx.c:32501-32532 `ufbx_catch_get_weighted_face_normal` (impl: native/api.rs
// `catch_get_weighted_face_normal`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_catch_get_weighted_face_normal(
    panic: *mut crate::generated::Panic,
    positions: *const crate::generated::VertexVec3,
    face: crate::generated::Face,
) -> crate::generated::Vec3 {
    crate::native::api::catch_get_weighted_face_normal(
        // SAFETY: C-ABI root; per the public contract `panic` is null or points
        // to a caller-owned `ufbx_panic` we may access exclusively for this call,
        // so `as_mut` yields a sound `Option<&mut Panic>`.
        unsafe { panic.as_mut() },
        // SAFETY: C-ABI root; `from_ptr` reinterprets the caller's pointer as a
        // read-only `View<_, Const>`, sound for any readable provenance, over a
        // pointee the caller owns per this `unsafe fn`'s contract.
        unsafe {
            crate::native::view::View::<crate::generated::VertexVec3, crate::native::view::Const>::from_ptr(positions)
        },
        face,
    )
}

// ufbx.c:32534-32578 `ufbx_catch_generate_normal_mapping` (impl: native/api.rs
// `catch_generate_normal_mapping`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_catch_generate_normal_mapping(
    panic: *mut crate::generated::Panic,
    mesh: *const crate::generated::Mesh,
    topo: *const crate::generated::TopoEdge,
    num_topo: usize,
    normal_indices: *mut u32,
    num_normal_indices: usize,
    assume_smooth: bool,
) -> usize {
    // SAFETY: an ABI shim; `mesh` is bridged to a read-only `View<_, Const>`,
    // `panic` is null or caller-owned with exclusive access for this call so
    // `as_mut` is sound, and the remaining raw arguments carry this `unsafe fn`'s
    // contract, forwarded to the native impl.
    unsafe {
        crate::native::api::catch_generate_normal_mapping(
        panic.as_mut(),
        crate::native::view::View::<crate::generated::Mesh, crate::native::view::Const>::from_ptr(
            mesh,
        ),
        topo,
        num_topo,
        normal_indices,
        num_normal_indices,
        assume_smooth,
    )
    }
}

// ufbx.c:32580-32583 `ufbx_generate_normal_mapping` (impl: native/api.rs
// `generate_normal_mapping`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_generate_normal_mapping(
    mesh: *const crate::generated::Mesh,
    topo: *const crate::generated::TopoEdge,
    num_topo: usize,
    normal_indices: *mut u32,
    num_normal_indices: usize,
    assume_smooth: bool,
) -> usize {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe {
        crate::native::api::generate_normal_mapping(
            mesh,
            topo,
            num_topo,
            normal_indices,
            num_normal_indices,
            assume_smooth,
        )
    }
}

// ufbx.c:32585-32612 `ufbx_catch_compute_normals` (impl: native/api.rs
// `catch_compute_normals`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_catch_compute_normals(
    panic: *mut crate::generated::Panic,
    mesh: *const crate::generated::Mesh,
    positions: *const crate::generated::VertexVec3,
    normal_indices: *const u32,
    num_normal_indices: usize,
    normals: *mut crate::generated::Vec3,
    num_normals: usize,
) {
    // SAFETY: an ABI shim; `mesh` and `positions` are bridged to read-only
    // `View<_, Const>`s, `panic` is null or caller-owned with exclusive access
    // for this call so `as_mut` is sound, and the remaining raw arguments carry
    // this `unsafe fn`'s contract, forwarded to the native impl.
    unsafe {
        crate::native::api::catch_compute_normals(
        panic.as_mut(),
        crate::native::view::View::<crate::generated::Mesh, crate::native::view::Const>::from_ptr(mesh),
        crate::native::view::View::<crate::generated::VertexVec3, crate::native::view::Const>::from_ptr(positions),
        normal_indices,
        num_normal_indices,
        normals,
        num_normals,
    )
    }
}

// ufbx.c:32614-32617 `ufbx_compute_normals` (impl: native/api.rs
// `compute_normals`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_compute_normals(
    mesh: *const crate::generated::Mesh,
    positions: *const crate::generated::VertexVec3,
    normal_indices: *const u32,
    num_normal_indices: usize,
    normals: *mut crate::generated::Vec3,
    num_normals: usize,
) {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe {
        crate::native::api::compute_normals(
            mesh,
            positions,
            normal_indices,
            num_normal_indices,
            normals,
            num_normals,
        )
    }
}

// ufbx.c:32619-32625 `ufbx_subdivide_mesh` (impl: native/api.rs
// `subdivide_mesh`). Unconditional: the `UFBXI_FEATURE_SUBDIVISION` split lives
// in `ufbxi_subdivide_mesh` (native/subdivision.rs), which keeps both arms.
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_subdivide_mesh(
    mesh: *const crate::generated::Mesh,
    level: usize,
    opts: *const crate::generated::RawSubdivideOpts,
    error: *mut crate::generated::Error,
) -> *mut crate::generated::Mesh {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::subdivide_mesh(mesh, level, opts, error) }
}

// ufbx.c:32627-32636 `ufbx_free_mesh` (impl: native/api.rs `free_mesh`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_free_mesh(mesh: *mut crate::generated::Mesh) {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::free_mesh(mesh) }
}

// ufbx.c:32638-32647 `ufbx_retain_mesh` (impl: native/api.rs `retain_mesh`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_retain_mesh(mesh: *mut crate::generated::Mesh) {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::retain_mesh(mesh) }
}

// ufbx.c:32649-32655 `ufbx_load_geometry_cache` (impl: native/api.rs
// `load_geometry_cache`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_load_geometry_cache(
    filename: *const u8,
    opts: *const crate::generated::RawGeometryCacheOpts,
    error: *mut crate::generated::Error,
) -> *mut crate::generated::GeometryCache {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::load_geometry_cache(filename, opts, error) }
}

// ufbx.c:32657-32664 `ufbx_load_geometry_cache_len` (impl: native/api.rs
// `load_geometry_cache_len`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_load_geometry_cache_len(
    filename: *const u8,
    filename_len: usize,
    opts: *const crate::generated::RawGeometryCacheOpts,
    error: *mut crate::generated::Error,
) -> *mut crate::generated::GeometryCache {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::load_geometry_cache_len(filename, filename_len, opts, error) }
}

// ufbx.c:32666-32675 `ufbx_free_geometry_cache` (impl: native/api.rs
// `free_geometry_cache`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_free_geometry_cache(cache: *mut crate::generated::GeometryCache) {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::free_geometry_cache(cache) }
}

// ufbx.c:32677-32686 `ufbx_retain_geometry_cache` (impl: native/api.rs
// `retain_geometry_cache`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_retain_geometry_cache(cache: *mut crate::generated::GeometryCache) {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::retain_geometry_cache(cache) }
}

// ufbx.c:32696-32859 `ufbx_read_geometry_cache_real` (impl: native/api.rs
// `read_geometry_cache_real`; `#[cfg]` internally returns 0 when
// `feature = "geometry-cache"` is off, matching C's `#else return 0`).
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_read_geometry_cache_real(
    frame: *const crate::generated::CacheFrame,
    data: *mut crate::prelude::Real,
    num_data: usize,
    opts: *const crate::generated::RawGeometryCacheDataOpts,
) -> usize {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::read_geometry_cache_real(frame, data, num_data, opts) }
}

// ufbx.c:32861-32931 `ufbx_sample_geometry_cache_real` (impl: native/api.rs
// `sample_geometry_cache_real`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_sample_geometry_cache_real(
    channel: *const crate::generated::CacheChannel,
    time: f64,
    data: *mut crate::prelude::Real,
    num_data: usize,
    opts: *const crate::generated::RawGeometryCacheDataOpts,
) -> usize {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::sample_geometry_cache_real(channel, time, data, num_data, opts) }
}

// ufbx.c:32933-32943 `ufbx_read_geometry_cache_vec3` (impl: native/api.rs
// `read_geometry_cache_vec3`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_read_geometry_cache_vec3(
    frame: *const crate::generated::CacheFrame,
    data: *mut crate::generated::Vec3,
    num_data: usize,
    opts: *const crate::generated::RawGeometryCacheDataOpts,
) -> usize {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::read_geometry_cache_vec3(frame, data, num_data, opts) }
}

// ufbx.c:32945-32955 `ufbx_sample_geometry_cache_vec3` (impl: native/api.rs
// `sample_geometry_cache_vec3`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_sample_geometry_cache_vec3(
    channel: *const crate::generated::CacheChannel,
    time: f64,
    data: *mut crate::generated::Vec3,
    num_data: usize,
    opts: *const crate::generated::RawGeometryCacheDataOpts,
) -> usize {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::sample_geometry_cache_vec3(channel, time, data, num_data, opts) }
}

// ufbx.c:32957-32964 `ufbx_dom_find_len` (impl: native/api.rs `dom_find_len`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_dom_find_len(
    parent: *const crate::generated::DomNode,
    name: *const u8,
    name_len: usize,
) -> *mut crate::generated::DomNode {
    // C-ABI root: mirror C's unchecked `parent` deref — mint a read-only view
    // (legal for any readable provenance) and map the correlated result to raw.
    // SAFETY: an ABI shim; the source pointer is bridged to a read-only
    // `View<_, Const>` (sound for any readable provenance) and the caller's
    // `name`/`name_len` key-buffer contract becomes the slice mint.
    match unsafe {
        crate::native::api::dom_find_len(crate::native::view::View::<crate::generated::DomNode, crate::native::view::Const>::from_ptr(parent), crate::prelude::slice_from_ptr(name, name_len))
    } {
        Some(node) => node.as_ptr() as *mut crate::generated::DomNode,
        None => core::ptr::null_mut(),
    }
}

// ufbx.c:32966-32974 `ufbx_generate_indices` (impl: native/api.rs
// `generate_indices`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_generate_indices(
    streams: *const crate::generated::RawVertexStream,
    num_streams: usize,
    indices: *mut u32,
    num_indices: usize,
    allocator: *const crate::generated::RawAllocatorOpts,
    error: *mut crate::generated::Error,
) -> usize {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe {
        crate::native::api::generate_indices(
            streams,
            num_streams,
            indices,
            num_indices,
            allocator,
            error,
        )
    }
}

// ufbx.c:32976-32979 `ufbx_thread_pool_run_task` (impl: native/api.rs
// `thread_pool_run_task`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_thread_pool_run_task(
    ctx: crate::prelude::ThreadPoolContext,
    index: u32,
) {
    // SAFETY: an ABI shim; `ctx` is this `unsafe fn`'s opaque handle over a live
    // `ThreadPool` per the public contract, forwarded unchanged to the native
    // impl whose contract is identical.
    unsafe { crate::native::api::thread_pool_run_task(ctx, index) }
}

// ufbx.c:32981-32985 `ufbx_thread_pool_set_user_ptr` (impl: native/api.rs
// `thread_pool_set_user_ptr`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_thread_pool_set_user_ptr(
    ctx: crate::prelude::ThreadPoolContext,
    user_ptr: *mut core::ffi::c_void,
) {
    // SAFETY: an ABI shim; `ctx` is this `unsafe fn`'s opaque handle over a live
    // `ThreadPool` per the public contract, and both arguments are forwarded
    // unchanged to the native impl whose contract is identical.
    unsafe { crate::native::api::thread_pool_set_user_ptr(ctx, user_ptr) }
}

// ufbx.c:32987-32991 `ufbx_thread_pool_get_user_ptr` (impl: native/api.rs
// `thread_pool_get_user_ptr`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_thread_pool_get_user_ptr(
    ctx: crate::prelude::ThreadPoolContext,
) -> *mut core::ffi::c_void {
    // SAFETY: an ABI shim; `ctx` is this `unsafe fn`'s opaque handle over a live
    // `ThreadPool` per the public contract, forwarded unchanged to the native
    // impl whose contract is identical.
    unsafe { crate::native::api::thread_pool_get_user_ptr(ctx) }
}

// ufbx.c:32993-32999 `ufbx_catch_get_vertex_real` (impl: native/api.rs
// `catch_get_vertex_real`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_catch_get_vertex_real(
    panic: *mut crate::generated::Panic,
    v: *const crate::generated::VertexReal,
    index: usize,
) -> crate::prelude::Real {
    // SAFETY: C-ABI root; per the public contract `panic` is null or points to
    // a caller-owned `ufbx_panic` we may access exclusively for this call, so
    // `as_mut` yields a sound `Option<&mut Panic>`.
    // SAFETY: C-ABI root; `from_ptr` reinterprets the caller's pointer as a
    // read-only `View<_, Const>`, sound for any readable provenance, over a
    // pointee the caller owns per this `unsafe fn`'s contract.
    crate::native::api::catch_get_vertex_real(
        unsafe { panic.as_mut() },
        unsafe {
            crate::native::view::View::<crate::generated::VertexReal, crate::native::view::Const>::from_ptr(v)
        },
        index,
    )
}

// ufbx.c:33001-33007 `ufbx_catch_get_vertex_vec2` (impl: native/api.rs
// `catch_get_vertex_vec2`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_catch_get_vertex_vec2(
    panic: *mut crate::generated::Panic,
    v: *const crate::generated::VertexVec2,
    index: usize,
) -> crate::generated::Vec2 {
    // SAFETY: C-ABI root; per the public contract `panic` is null or points to
    // a caller-owned `ufbx_panic` we may access exclusively for this call, so
    // `as_mut` yields a sound `Option<&mut Panic>`.
    // SAFETY: C-ABI root; `from_ptr` reinterprets the caller's pointer as a
    // read-only `View<_, Const>`, sound for any readable provenance, over a
    // pointee the caller owns per this `unsafe fn`'s contract.
    crate::native::api::catch_get_vertex_vec2(
        unsafe { panic.as_mut() },
        unsafe {
            crate::native::view::View::<crate::generated::VertexVec2, crate::native::view::Const>::from_ptr(v)
        },
        index,
    )
}

// ufbx.c:33009-33015 `ufbx_catch_get_vertex_vec3` (impl: native/api.rs
// `catch_get_vertex_vec3`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_catch_get_vertex_vec3(
    panic: *mut crate::generated::Panic,
    v: *const crate::generated::VertexVec3,
    index: usize,
) -> crate::generated::Vec3 {
    // SAFETY: C-ABI root; `from_ptr` reinterprets the caller's pointer as a
    // read-only `View<_, Const>`, sound for any readable provenance, over a
    // pointee the caller owns per this `unsafe fn`'s contract.
    // SAFETY: C-ABI root; per the public contract `panic` is null or points to
    // a caller-owned `ufbx_panic` we may access exclusively for this call, so
    // `as_mut` yields a sound `Option<&mut Panic>`.
    crate::native::api::catch_get_vertex_vec3(
        unsafe { panic.as_mut() },
        unsafe {
            crate::native::view::View::<crate::generated::VertexVec3, crate::native::view::Const>::from_ptr(v)
        },
        index,
    )
}

// ufbx.c:33017-33023 `ufbx_catch_get_vertex_vec4` (impl: native/api.rs
// `catch_get_vertex_vec4`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_catch_get_vertex_vec4(
    panic: *mut crate::generated::Panic,
    v: *const crate::generated::VertexVec4,
    index: usize,
) -> crate::generated::Vec4 {
    // SAFETY: C-ABI root; per the public contract `panic` is null or points to
    // a caller-owned `ufbx_panic` we may access exclusively for this call, so
    // `as_mut` yields a sound `Option<&mut Panic>`.
    // SAFETY: C-ABI root; `from_ptr` reinterprets the caller's pointer as a
    // read-only `View<_, Const>`, sound for any readable provenance, over a
    // pointee the caller owns per this `unsafe fn`'s contract.
    crate::native::api::catch_get_vertex_vec4(
        unsafe { panic.as_mut() },
        unsafe {
            crate::native::view::View::<crate::generated::VertexVec4, crate::native::view::Const>::from_ptr(v)
        },
        index,
    )
}

// ufbx.c:33025-33032 `ufbx_catch_get_vertex_w_vec3` (impl: native/api.rs
// `catch_get_vertex_w_vec3`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_catch_get_vertex_w_vec3(
    panic: *mut crate::generated::Panic,
    v: *const crate::generated::VertexVec3,
    index: usize,
) -> crate::prelude::Real {
    // SAFETY: C-ABI root; per the public contract `panic` is null or points to
    // a caller-owned `ufbx_panic` we may access exclusively for this call, so
    // `as_mut` yields a sound `Option<&mut Panic>`.
    // SAFETY: C-ABI root; `from_ptr` reinterprets the caller's pointer as a
    // read-only `View<_, Const>`, sound for any readable provenance, over a
    // pointee the caller owns per this `unsafe fn`'s contract.
    crate::native::api::catch_get_vertex_w_vec3(
        unsafe { panic.as_mut() },
        unsafe {
            crate::native::view::View::<crate::generated::VertexVec3, crate::native::view::Const>::from_ptr(v)
        },
        index,
    )
}

// ufbx.c:33034-33075 `ufbx_as_*` (impls: native/api.rs `as_*`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_as_unknown(
    element: *const crate::generated::Element,
) -> *mut crate::generated::Unknown {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::as_unknown(element) }
}
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_as_node(
    element: *const crate::generated::Element,
) -> *mut crate::generated::Node {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::as_node(element) }
}
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_as_mesh(
    element: *const crate::generated::Element,
) -> *mut crate::generated::Mesh {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::as_mesh(element) }
}
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_as_light(
    element: *const crate::generated::Element,
) -> *mut crate::generated::Light {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::as_light(element) }
}
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_as_camera(
    element: *const crate::generated::Element,
) -> *mut crate::generated::Camera {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::as_camera(element) }
}
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_as_bone(
    element: *const crate::generated::Element,
) -> *mut crate::generated::Bone {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::as_bone(element) }
}
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_as_empty(
    element: *const crate::generated::Element,
) -> *mut crate::generated::Empty {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::as_empty(element) }
}
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_as_line_curve(
    element: *const crate::generated::Element,
) -> *mut crate::generated::LineCurve {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::as_line_curve(element) }
}
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_as_nurbs_curve(
    element: *const crate::generated::Element,
) -> *mut crate::generated::NurbsCurve {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::as_nurbs_curve(element) }
}
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_as_nurbs_surface(
    element: *const crate::generated::Element,
) -> *mut crate::generated::NurbsSurface {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::as_nurbs_surface(element) }
}
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_as_nurbs_trim_surface(
    element: *const crate::generated::Element,
) -> *mut crate::generated::NurbsTrimSurface {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::as_nurbs_trim_surface(element) }
}
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_as_nurbs_trim_boundary(
    element: *const crate::generated::Element,
) -> *mut crate::generated::NurbsTrimBoundary {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::as_nurbs_trim_boundary(element) }
}
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_as_procedural_geometry(
    element: *const crate::generated::Element,
) -> *mut crate::generated::ProceduralGeometry {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::as_procedural_geometry(element) }
}
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_as_stereo_camera(
    element: *const crate::generated::Element,
) -> *mut crate::generated::StereoCamera {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::as_stereo_camera(element) }
}
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_as_camera_switcher(
    element: *const crate::generated::Element,
) -> *mut crate::generated::CameraSwitcher {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::as_camera_switcher(element) }
}
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_as_marker(
    element: *const crate::generated::Element,
) -> *mut crate::generated::Marker {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::as_marker(element) }
}
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_as_lod_group(
    element: *const crate::generated::Element,
) -> *mut crate::generated::LodGroup {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::as_lod_group(element) }
}
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_as_skin_deformer(
    element: *const crate::generated::Element,
) -> *mut crate::generated::SkinDeformer {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::as_skin_deformer(element) }
}
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_as_skin_cluster(
    element: *const crate::generated::Element,
) -> *mut crate::generated::SkinCluster {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::as_skin_cluster(element) }
}
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_as_blend_deformer(
    element: *const crate::generated::Element,
) -> *mut crate::generated::BlendDeformer {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::as_blend_deformer(element) }
}
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_as_blend_channel(
    element: *const crate::generated::Element,
) -> *mut crate::generated::BlendChannel {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::as_blend_channel(element) }
}
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_as_blend_shape(
    element: *const crate::generated::Element,
) -> *mut crate::generated::BlendShape {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::as_blend_shape(element) }
}
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_as_cache_deformer(
    element: *const crate::generated::Element,
) -> *mut crate::generated::CacheDeformer {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::as_cache_deformer(element) }
}
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_as_cache_file(
    element: *const crate::generated::Element,
) -> *mut crate::generated::CacheFile {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::as_cache_file(element) }
}
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_as_material(
    element: *const crate::generated::Element,
) -> *mut crate::generated::Material {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::as_material(element) }
}
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_as_texture(
    element: *const crate::generated::Element,
) -> *mut crate::generated::Texture {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::as_texture(element) }
}
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_as_video(
    element: *const crate::generated::Element,
) -> *mut crate::generated::Video {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::as_video(element) }
}
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_as_shader(
    element: *const crate::generated::Element,
) -> *mut crate::generated::Shader {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::as_shader(element) }
}
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_as_shader_binding(
    element: *const crate::generated::Element,
) -> *mut crate::generated::ShaderBinding {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::as_shader_binding(element) }
}
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_as_anim_stack(
    element: *const crate::generated::Element,
) -> *mut crate::generated::AnimStack {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::as_anim_stack(element) }
}
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_as_anim_layer(
    element: *const crate::generated::Element,
) -> *mut crate::generated::AnimLayer {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::as_anim_layer(element) }
}
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_as_anim_value(
    element: *const crate::generated::Element,
) -> *mut crate::generated::AnimValue {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::as_anim_value(element) }
}
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_as_anim_curve(
    element: *const crate::generated::Element,
) -> *mut crate::generated::AnimCurve {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::as_anim_curve(element) }
}
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_as_display_layer(
    element: *const crate::generated::Element,
) -> *mut crate::generated::DisplayLayer {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::as_display_layer(element) }
}
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_as_selection_set(
    element: *const crate::generated::Element,
) -> *mut crate::generated::SelectionSet {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::as_selection_set(element) }
}
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_as_selection_node(
    element: *const crate::generated::Element,
) -> *mut crate::generated::SelectionNode {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::as_selection_node(element) }
}
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_as_character(
    element: *const crate::generated::Element,
) -> *mut crate::generated::Character {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::as_character(element) }
}
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_as_constraint(
    element: *const crate::generated::Element,
) -> *mut crate::generated::Constraint {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::as_constraint(element) }
}
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_as_audio_layer(
    element: *const crate::generated::Element,
) -> *mut crate::generated::AudioLayer {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::as_audio_layer(element) }
}
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_as_audio_clip(
    element: *const crate::generated::Element,
) -> *mut crate::generated::AudioClip {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::as_audio_clip(element) }
}
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_as_pose(
    element: *const crate::generated::Element,
) -> *mut crate::generated::Pose {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::as_pose(element) }
}
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_as_metadata_object(
    element: *const crate::generated::Element,
) -> *mut crate::generated::MetadataObject {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::as_metadata_object(element) }
}

// ufbx.c:33077-33081 `ufbx_dom_is_array` (impl: native/api.rs `dom_is_array`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_dom_is_array(node: *const crate::generated::DomNode) -> bool {
    crate::native::api::dom_is_array(if node.is_null() {
        None
    } else {
        // SAFETY: C-ABI root; `from_ptr` reinterprets the caller's pointer as a
        // read-only `View<_, Const>`, sound for any readable provenance, over a
        // pointee the caller owns per this `unsafe fn`'s contract.
        Some(unsafe {
            crate::native::view::View::<
            crate::generated::DomNode,
            crate::native::view::Const,
        >::from_ptr(node)
        })
    })
}
// ufbx.c:33082-33084 `ufbx_dom_array_size` (impl: native/api.rs `dom_array_size`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_dom_array_size(node: *const crate::generated::DomNode) -> usize {
    crate::native::api::dom_array_size(if node.is_null() {
        None
    } else {
        // SAFETY: C-ABI root; `from_ptr` reinterprets the caller's pointer as a
        // read-only `View<_, Const>`, sound for any readable provenance, over a
        // pointee the caller owns per this `unsafe fn`'s contract.
        Some(unsafe {
            crate::native::view::View::<
            crate::generated::DomNode,
            crate::native::view::Const,
        >::from_ptr(node)
        })
    })
}
// ufbx.c:33085-33093 `ufbx_dom_as_int32_list` (impl: native/api.rs `dom_as_int32_list`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_dom_as_int32_list(
    node: *const crate::generated::DomNode,
) -> crate::prelude::List<i32> {
    crate::native::api::dom_as_int32_list(if node.is_null() {
        None
    } else {
        // SAFETY: C-ABI root; `from_ptr` reinterprets the caller's pointer as a
        // read-only `View<_, Const>`, sound for any readable provenance, over a
        // pointee the caller owns per this `unsafe fn`'s contract.
        Some(unsafe {
            crate::native::view::View::<
            crate::generated::DomNode,
            crate::native::view::Const,
        >::from_ptr(node)
        })
    })
}
// ufbx.c:33094-33102 `ufbx_dom_as_int64_list` (impl: native/api.rs `dom_as_int64_list`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_dom_as_int64_list(
    node: *const crate::generated::DomNode,
) -> crate::prelude::List<i64> {
    crate::native::api::dom_as_int64_list(if node.is_null() {
        None
    } else {
        // SAFETY: C-ABI root; `from_ptr` reinterprets the caller's pointer as a
        // read-only `View<_, Const>`, sound for any readable provenance, over a
        // pointee the caller owns per this `unsafe fn`'s contract.
        Some(unsafe {
            crate::native::view::View::<
            crate::generated::DomNode,
            crate::native::view::Const,
        >::from_ptr(node)
        })
    })
}
// ufbx.c:33103-33111 `ufbx_dom_as_float_list` (impl: native/api.rs `dom_as_float_list`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_dom_as_float_list(
    node: *const crate::generated::DomNode,
) -> crate::prelude::List<f32> {
    crate::native::api::dom_as_float_list(if node.is_null() {
        None
    } else {
        // SAFETY: C-ABI root; `from_ptr` reinterprets the caller's pointer as a
        // read-only `View<_, Const>`, sound for any readable provenance, over a
        // pointee the caller owns per this `unsafe fn`'s contract.
        Some(unsafe {
            crate::native::view::View::<
            crate::generated::DomNode,
            crate::native::view::Const,
        >::from_ptr(node)
        })
    })
}
// ufbx.c:33112-33120 `ufbx_dom_as_double_list` (impl: native/api.rs `dom_as_double_list`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_dom_as_double_list(
    node: *const crate::generated::DomNode,
) -> crate::prelude::List<f64> {
    crate::native::api::dom_as_double_list(if node.is_null() {
        None
    } else {
        // SAFETY: C-ABI root; `from_ptr` reinterprets the caller's pointer as a
        // read-only `View<_, Const>`, sound for any readable provenance, over a
        // pointee the caller owns per this `unsafe fn`'s contract.
        Some(unsafe {
            crate::native::view::View::<
            crate::generated::DomNode,
            crate::native::view::Const,
        >::from_ptr(node)
        })
    })
}
// ufbx.c:33121-33129 `ufbx_dom_as_real_list` (impl: native/api.rs `dom_as_real_list`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_dom_as_real_list(
    node: *const crate::generated::DomNode,
) -> crate::prelude::List<crate::prelude::Real> {
    crate::native::api::dom_as_real_list(if node.is_null() {
        None
    } else {
        // SAFETY: C-ABI root; `from_ptr` reinterprets the caller's pointer as a
        // read-only `View<_, Const>`, sound for any readable provenance, over a
        // pointee the caller owns per this `unsafe fn`'s contract.
        Some(unsafe {
            crate::native::view::View::<
            crate::generated::DomNode,
            crate::native::view::Const,
        >::from_ptr(node)
        })
    })
}
// ufbx.c:33130-33138 `ufbx_dom_as_blob_list` (impl: native/api.rs `dom_as_blob_list`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_dom_as_blob_list(
    node: *const crate::generated::DomNode,
) -> crate::prelude::List<crate::prelude::Blob> {
    crate::native::api::dom_as_blob_list(if node.is_null() {
        None
    } else {
        // SAFETY: C-ABI root; `from_ptr` reinterprets the caller's pointer as a
        // read-only `View<_, Const>`, sound for any readable provenance, over a
        // pointee the caller owns per this `unsafe fn`'s contract.
        Some(unsafe {
            crate::native::view::View::<
            crate::generated::DomNode,
            crate::native::view::Const,
        >::from_ptr(node)
        })
    })
}

// -- String API (ufbx.c:33140+): the `strlen` wrappers over the `_len` entry
// points above. Only the wrappers whose `_len` impl exists are defined here.

// ufbx.c:33142 `ufbx_find_prop` (impl: native/api.rs `find_prop`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_find_prop(
    props: *const crate::generated::Props,
    name: *const u8,
) -> *mut crate::generated::Prop {
    if props.is_null() {
        return core::ptr::null_mut();
    }
    // SAFETY: an ABI shim; the source pointer is bridged to a read-only
    // `View<_, Const>` (sound for any readable provenance) and the remaining
    // raw arguments carry this `unsafe fn`'s contract, forwarded to the native
    // impl unchanged.
    match unsafe {
        crate::native::api::find_prop(
        crate::native::view::View::<crate::generated::Props, crate::native::view::Const>::from_ptr(
            props,
        ),
        name,
    )
    } {
        Some(prop) => prop.as_ptr() as *mut crate::generated::Prop,
        None => core::ptr::null_mut(),
    }
}

// ufbx.c:33143 `ufbx_find_real` (impl: native/api.rs `find_real`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_find_real(
    props: *const crate::generated::Props,
    name: *const u8,
    def: crate::prelude::Real,
) -> crate::prelude::Real {
    if props.is_null() {
        return def;
    }
    // SAFETY: an ABI shim; the source pointer is bridged to a read-only
    // `View<_, Const>` (sound for any readable provenance) and the remaining
    // raw arguments carry this `unsafe fn`'s contract, forwarded to the native
    // impl unchanged.
    unsafe {
        crate::native::api::find_real(
        crate::native::view::View::<crate::generated::Props, crate::native::view::Const>::from_ptr(
            props,
        ),
        name,
        def,
    )
    }
}

// ufbx.c:33144 `ufbx_find_vec3` (impl: native/api.rs `find_vec3`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_find_vec3(
    props: *const crate::generated::Props,
    name: *const u8,
    def: crate::generated::Vec3,
) -> crate::generated::Vec3 {
    if props.is_null() {
        return def;
    }
    // SAFETY: an ABI shim; the source pointer is bridged to a read-only
    // `View<_, Const>` (sound for any readable provenance) and the remaining
    // raw arguments carry this `unsafe fn`'s contract, forwarded to the native
    // impl unchanged.
    unsafe {
        crate::native::api::find_vec3(
        crate::native::view::View::<crate::generated::Props, crate::native::view::Const>::from_ptr(
            props,
        ),
        name,
        def,
    )
    }
}

// ufbx.c:33145 `ufbx_find_int` (impl: native/api.rs `find_int`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_find_int(
    props: *const crate::generated::Props,
    name: *const u8,
    def: i64,
) -> i64 {
    if props.is_null() {
        return def;
    }
    // SAFETY: an ABI shim; the source pointer is bridged to a read-only
    // `View<_, Const>` (sound for any readable provenance) and the remaining
    // raw arguments carry this `unsafe fn`'s contract, forwarded to the native
    // impl unchanged.
    unsafe {
        crate::native::api::find_int(
        crate::native::view::View::<crate::generated::Props, crate::native::view::Const>::from_ptr(
            props,
        ),
        name,
        def,
    )
    }
}

// ufbx.c:33146 `ufbx_find_bool` (impl: native/api.rs `find_bool`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_find_bool(
    props: *const crate::generated::Props,
    name: *const u8,
    def: bool,
) -> bool {
    if props.is_null() {
        return def;
    }
    // SAFETY: an ABI shim; the source pointer is bridged to a read-only
    // `View<_, Const>` (sound for any readable provenance) and the remaining
    // raw arguments carry this `unsafe fn`'s contract, forwarded to the native
    // impl unchanged.
    unsafe {
        crate::native::api::find_bool(
        crate::native::view::View::<crate::generated::Props, crate::native::view::Const>::from_ptr(
            props,
        ),
        name,
        def,
    )
    }
}

// ufbx.c:33147 `ufbx_find_string` (impl: native/api.rs `find_string`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_find_string(
    props: *const crate::generated::Props,
    name: *const u8,
    def: crate::prelude::String,
) -> crate::prelude::String {
    if props.is_null() {
        return def;
    }
    // SAFETY: an ABI shim; the source pointer is bridged to a read-only
    // `View<_, Const>` (sound for any readable provenance) and the remaining
    // raw arguments carry this `unsafe fn`'s contract, forwarded to the native
    // impl unchanged.
    unsafe {
        crate::native::api::find_string(
        crate::native::view::View::<crate::generated::Props, crate::native::view::Const>::from_ptr(
            props,
        ),
        name,
        def,
    )
    }
}

// ufbx.c:33148 `ufbx_find_blob` (impl: native/api.rs `find_blob`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_find_blob(
    props: *const crate::generated::Props,
    name: *const u8,
    def: crate::prelude::Blob,
) -> crate::prelude::Blob {
    if props.is_null() {
        return def;
    }
    // SAFETY: an ABI shim; the source pointer is bridged to a read-only
    // `View<_, Const>` (sound for any readable provenance) and the remaining
    // raw arguments carry this `unsafe fn`'s contract, forwarded to the native
    // impl unchanged.
    unsafe {
        crate::native::api::find_blob(
        crate::native::view::View::<crate::generated::Props, crate::native::view::Const>::from_ptr(
            props,
        ),
        name,
        def,
    )
    }
}

// ufbx.c:33149 `ufbx_find_prop_element` (impl: native/api.rs `find_prop_element`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_find_prop_element(
    element: *const crate::generated::Element,
    name: *const u8,
    type_: crate::generated::ElementType,
) -> *mut crate::generated::Element {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::find_prop_element(element, name, type_) }
}

// ufbx.c:33150 `ufbx_find_element` (impl: native/api.rs `find_element`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_find_element(
    scene: *const crate::generated::Scene,
    type_: crate::generated::ElementType,
    name: *const u8,
) -> *mut crate::generated::Element {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::find_element(scene, type_, name) }
}

// ufbx.c:33151 `ufbx_find_node` (impl: native/api.rs `find_node`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_find_node(
    scene: *const crate::generated::Scene,
    name: *const u8,
) -> *mut crate::generated::Node {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::find_node(scene, name) }
}

// ufbx.c:33152 `ufbx_find_anim_stack` (impl: native/api.rs `find_anim_stack`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_find_anim_stack(
    scene: *const crate::generated::Scene,
    name: *const u8,
) -> *mut crate::generated::AnimStack {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::find_anim_stack(scene, name) }
}

// ufbx.c:33153 `ufbx_find_material` (impl: native/api.rs `find_material`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_find_material(
    scene: *const crate::generated::Scene,
    name: *const u8,
) -> *mut crate::generated::Material {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::find_material(scene, name) }
}

// ufbx.c:33154 `ufbx_find_anim_prop` (impl: native/api.rs `find_anim_prop`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_find_anim_prop(
    layer: *const crate::generated::AnimLayer,
    element: *const crate::generated::Element,
    prop: *const u8,
) -> *mut crate::generated::AnimProp {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::find_anim_prop(layer, element, prop) }
}

// ufbx.c:33155 `ufbx_evaluate_prop` (impl: native/api.rs `evaluate_prop`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_evaluate_prop(
    anim: *const crate::generated::Anim,
    element: *const crate::generated::Element,
    name: *const u8,
    time: f64,
) -> crate::generated::Prop {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::evaluate_prop(anim, element, name, time) }
}

// ufbx.c:33156 `ufbx_evaluate_prop_flags` (impl: native/api.rs
// `evaluate_prop_flags`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_evaluate_prop_flags(
    anim: *const crate::generated::Anim,
    element: *const crate::generated::Element,
    name: *const u8,
    time: f64,
    flags: u32,
) -> crate::generated::Prop {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::evaluate_prop_flags(anim, element, name, time, flags) }
}

// ufbx.c:33157 `ufbx_find_prop_texture` (impl: native/api.rs `find_prop_texture`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_find_prop_texture(
    material: *const crate::generated::Material,
    name: *const u8,
) -> *mut crate::generated::Texture {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::find_prop_texture(material, name) }
}

// ufbx.c:33158 `ufbx_find_shader_prop` (impl: native/api.rs `find_shader_prop`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_find_shader_prop(
    shader: *const crate::generated::Shader,
    name: *const u8,
) -> crate::prelude::String {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::find_shader_prop(shader, name) }
}

// ufbx.c:33159 `ufbx_find_shader_prop_bindings`
// (impl: native/api.rs `find_shader_prop_bindings`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_find_shader_prop_bindings(
    shader: *const crate::generated::Shader,
    name: *const u8,
) -> crate::prelude::List<crate::generated::ShaderPropBinding> {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::find_shader_prop_bindings(shader, name) }
}

// ufbx.c:33160 `ufbx_find_shader_texture_input`
// (impl: native/api.rs `find_shader_texture_input`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_find_shader_texture_input(
    shader: *const crate::generated::ShaderTexture,
    name: *const u8,
) -> *mut crate::generated::ShaderTextureInput {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::find_shader_texture_input(shader, name) }
}

// ufbx.c:33161 `ufbx_dom_find` (impl: native/api.rs `dom_find`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_dom_find(
    parent: *const crate::generated::DomNode,
    name: *const u8,
) -> *mut crate::generated::DomNode {
    // Same shape as `ufbx_dom_find_len`: unchecked `parent` (C parity),
    // read-only view in, correlated view out, mapped back to raw.
    // SAFETY: an ABI shim; the source pointer is bridged to a read-only
    // `View<_, Const>` (sound for any readable provenance) and the remaining
    // raw arguments carry this `unsafe fn`'s contract, forwarded to the native
    // impl unchanged.
    match unsafe {
        crate::native::api::dom_find(
        crate::native::view::View::<crate::generated::DomNode, crate::native::view::Const>::from_ptr(
            parent,
        ),
        name,
    )
    } {
        Some(node) => node.as_ptr() as *mut crate::generated::DomNode,
        None => core::ptr::null_mut(),
    }
}

// -- Catch API (ufbx.c:33163-33179): non-catch wrappers passing `panic == NULL`
// to their `ufbx_catch_*` counterparts. Each rides its impl's cfg / DEFERRED
// state.

// ufbx.c:33165-33167 `ufbx_triangulate_face` (impl: native/api.rs
// `triangulate_face`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_triangulate_face(
    indices: *mut u32,
    num_indices: usize,
    mesh: *const crate::generated::Mesh,
    face: crate::generated::Face,
) -> u32 {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::triangulate_face(indices, num_indices, mesh, face) }
}

// ufbx.c:33168-33170 `ufbx_compute_topology` (impl: native/api.rs
// `compute_topology`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_compute_topology(
    mesh: *const crate::generated::Mesh,
    topo: *mut crate::generated::TopoEdge,
    num_topo: usize,
) {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::compute_topology(mesh, topo, num_topo) }
}

// ufbx.c:33171-33173 `ufbx_topo_next_vertex_edge` (impl: native/api.rs
// `topo_next_vertex_edge`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_topo_next_vertex_edge(
    topo: *const crate::generated::TopoEdge,
    num_topo: usize,
    index: u32,
) -> u32 {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::topo_next_vertex_edge(topo, num_topo, index) }
}

// ufbx.c:33174-33176 `ufbx_topo_prev_vertex_edge` (impl: native/api.rs
// `topo_prev_vertex_edge`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_topo_prev_vertex_edge(
    topo: *const crate::generated::TopoEdge,
    num_topo: usize,
    index: u32,
) -> u32 {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::topo_prev_vertex_edge(topo, num_topo, index) }
}

// ufbx.c:33177-33179 `ufbx_get_weighted_face_normal` (impl: native/api.rs
// `get_weighted_face_normal`)
#[cfg_attr(feature = "c-abi", no_mangle)]
pub unsafe extern "C" fn ufbx_get_weighted_face_normal(
    positions: *const crate::generated::VertexVec3,
    face: crate::generated::Face,
) -> crate::generated::Vec3 {
    // SAFETY: an ABI shim; the raw-pointer arguments carry this `unsafe fn`'s
    // own raw-pointer contract and are forwarded unchanged to the native impl,
    // whose contract is identical.
    unsafe { crate::native::api::get_weighted_face_normal(positions, face) }
}
