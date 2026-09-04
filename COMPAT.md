# COMPAT.md — deviations from ufbx-rust 0.11.2 and from the C library

Registry of every intentional difference. Phase 3's API-diff against ufbx-rust
treats this file as the allowlist: any diff NOT listed here is a bug.

Doc convention for provenance (uniform, greppable; no name prefixes — if
extensions multiply they get grouped under an `ufbx::ext` module instead).
The FIRST LINE of the doc comment is a provenance tag matching this file's
sections, then a one-line summary; detail paragraphs follow:

| Tag | Meaning |
|---|---|
| `Native-port extension:` | Exists only in this crate (§1). |
| `ufbx-rust extension:` | Hand-written ufbx-rust addition with no C counterpart, reproduced verbatim for drop-in parity. |
| `Native-port divergence:` | Public shape differs from ufbx-rust (§2); the doc names the baseline shape and links here. |

Generated code (`generated.rs`) cannot carry hand-written doc tags — its
divergences are documented by `NOTE(ufbx-rs-native)` comments in the GENERATOR
plus an entry in §2 of this file.

## 1. Native-only additions (semver-additive; cannot break drop-in use)

| Item | Rationale |
|---|---|
| `ufbx::set_panic_handler(fn(&str))` | Runtime analogue of C's compile-time `#define ufbx_panic_handler` override; atomic fn-pointer, cost only on the panic path. Handler may return → graceful bail-out, matching C. |
| Cargo features beyond ufbx-rust's `mint`/`nightly` | Mirror ufbx.c's compile-time configuration; full mapping in §1b below. |
| `nightly` feature enables branch hints (`core::hint::likely/unlikely`) | In ufbx-rust the feature is declared but unused. Optimizer-only; API-invisible. |
| `prelude::RawEnum<T>` | New public type; see §2 for the signature it appears in. Pattern + name follow rustc's internal `RawEnum<T>` for LLVM C APIs. |
| `<Flags>::from_raw(u32)` / `<Flags>::raw()` on every generated flag type (`pub(crate)`) | C accumulates flag sets as plain `uint32_t` and casts once at the end, including shifted sub-fields no named constant covers (`flags \|= ((uint32_t)(next - '0') & 0xf) << 4;` ufbx.c:11818 → `prop->flags = (ufbx_prop_flags)flags;` ufbx.c:11866). The port needs the same u32 arithmetic. Crate-internal, so the public surface is unchanged; generator `emit_flag`, `NOTE(ufbx-rs-native)` there. |
| `Blob: Clone + Copy`, `Prop: Clone + Copy` | Extra trait impls (additive). C struct assignment is memcpy (PORTING.md checklist #15) and the reader copies `ufbx_prop` by value — the property sort (`ufbxi_macro_stable_sort(ufbx_prop, …)`, ufbx.c:11881) and `ufbxi_deduplicate_properties` (`ps[dst++] = ps[src++]`, ufbx.c:11894). `Prop` embeds `Blob`, so `Blob` needs it too. All fields trivially copyable; `Blob` is hand-written in `prelude.rs`, `Prop` via generator allowlist `copy_derive_types`. |
| `ShaderPropBinding: Clone + Copy` | Extra trait impls (additive). `ufbxi_sort_shader_prop_bindings` sorts the bindings by value (`ufbxi_macro_stable_sort(ufbx_shader_prop_binding, …)`, ufbx.c:14692), which is a plain C struct assignment through the sort scratch. Both fields are `ufbx_string`, trivially copyable. Generator allowlist `copy_derive_types`. |
| `Ref<T>: Clone + Copy` | Extra trait impls (additive). C struct assignment is memcpy (PORTING.md checklist #15) and the structs embedding `ufbx_element*` are copied by value through the sort scratch (see the two rows below). Hand-written manual impls in `prelude.rs` (not `derive`, which would add a spurious `T: Copy` bound); `Ref<T>` is `#[repr(transparent)]` over `NonNull<T>`, which is itself `Copy`. |
| `Connection: Clone + Copy` | Extra trait impls (additive). `ufbxi_sort_connections` sorts connections by value (`ufbxi_macro_stable_sort(ufbx_connection, …)`, ufbx.c:18651) and `ufbxi_resolve_connections` bulk-copies the src array into the dst array (ufbx.c:18769). Fields are `ufbx_element*` (→ `Ref<Element>`) and `ufbx_string`, all trivially copyable. Generator allowlist `copy_derive_types`. |
| `NameElement: Clone + Copy` | Extra trait impls (additive). `ufbxi_sort_name_elements` sorts by value (`ufbxi_macro_stable_sort(ufbx_name_element, …)`, ufbx.c:18587), a plain C struct assignment through the sort scratch. Generator allowlist `copy_derive_types`. |
| `AnimProp: Clone + Copy` | Extra trait impls (additive). `ufbxi_sort_anim_props` sorts by value (`ufbxi_macro_stable_sort(ufbx_anim_prop, …)`, ufbx.c:19301), a plain C struct assignment through the sort scratch. Fields are `ufbx_element*`/`ufbx_anim_value*` (→ `Ref<T>`), `uint32_t` and `ufbx_string`, all trivially copyable. Generator allowlist `copy_derive_types`. |
| `AllocatorOpts: Clone + Copy` | Extra trait impls (additive). The native `ufbxi_allocator` embeds it by value and stack-copies it (`ufbxi_release_ref`, ufbx.c:30289-30297); `derive(Copy)` requires field types to be `Copy`. All fields are trivially copyable. Generator allowlist `copy_derive_types`. |

## 1b. Cargo feature ↔ C configuration mapping

C's gates are opt-OUT macros (`UFBX_NO_*`, defined = feature removed); cargo
features are additive opt-IN. Parity therefore lives in the `default` list:
building with default features matches building ufbx.c with no `UFBX_NO_*`
defined.

| Cargo feature | C equivalent | Notes |
|---|---|---|
| `subdivision` (default) | absence of `UFBX_NO_SUBDIVISION` | |
| `tessellation` (default) | absence of `UFBX_NO_TESSELLATION` | |
| `geometry-cache` (default) | absence of `UFBX_NO_GEOMETRY_CACHE` | implies internal XML support (`UFBXI_FEATURE_XML`) |
| `scene-eval` (default) | absence of `UFBX_NO_SCENE_EVALUATION` | |
| `skinning-eval` (default) | absence of `UFBX_NO_SKINNING_EVALUATION` | |
| `baking` (default) | absence of `UFBX_NO_ANIMATION_BAKING` | |
| `triangulation` (default) | absence of `UFBX_NO_TRIANGULATION` | |
| `index-gen` (default) | absence of `UFBX_NO_INDEX_GENERATION` | |
| `obj` (default) | absence of `UFBX_NO_FORMAT_OBJ` | |
| `error-stack` | absence of `UFBX_NO_ERROR_STACK` | off by default: C only enables the stack in dev builds |
| `dev` | `UFBX_DEV` | implies `error-stack`; test builds use it |
| `regression` | `UFBX_REGRESSION` | changes algorithm constants, not just asserts |
| `real-is-f32` | `UFBX_REAL_IS_FLOAT` | `Real` = f32; Rust name spells the type where C names the intent |
| `c-abi` | n/a (linkage, not configuration) | exports the `ufbx_*` C symbols so the upstream C test suite links against the crate; test/validation only |
| `nightly` | n/a | branch hints via `core::hint::likely/unlikely` (in ufbx-rust the feature is declared but unused) |

## 2. Deliberate API divergences vs ufbx-rust 0.11.2

| Item | ufbx-rust | native port | Why |
|---|---|---|---|
| `RawProgressCb.fn_` return type | `ProgressResult` | `RawEnum<ProgressResult>` (`#[repr(transparent)]` u32; ABI-identical) | Materializing a two-variant Rust enum from an arbitrary C callback return is UB. Soundness fix; generator emits this for every enum-returning raw callback. |
| `MaterialMap.value_components` | `u32` | `u8` | Upstream `generate_rust.py` mapped `uint8_t`→`u32`, breaking `#[repr(C)]` layout (field offset 51). Caught by `tests/layout.rs`. ufbx-rust's version matches its older vendored ufbx.c; ours matches our `ufbx.h`. |
| `ShaderTexture.main_texture` | `Ref<Texture>` | `Option<Ref<Texture>>` | ufbx.h:2843 declares `ufbx_texture *main_texture;` without `ufbx_nullable`, but C leaves it NULL for every shader that is not `UFBX_SHADER_TEXTURE_SELECT_OUTPUT` (only ufbx.c:20531 assigns it) and explicitly re-nulls it in the cyclic-main-texture pass (`shader->main_texture = NULL;` ufbx.c:20723); C null-tests it at ufbx.c:20705/20708/20719/20735/20747, and the field's own doc comment says "Only specified if …". `Ref<T>` is `NonNull<T>`, so the un-overridden type would hold an invalid value in released scenes. Generator allowlist `nullable_field_overrides`, `NOTE(ufbx-rs-native)` there. Drop the override if upstream annotates the field. |
| `ShaderTextureInput.prop` | `Ref<Prop>` | `Option<Ref<Prop>>` | Construction-safety, NOT an upstream annotation gap — a full survey of ufbx.h's 69 pointer fields established that a *caller* never observes NULL here (ufbx.c:20658 assigns it unconditionally, and the one path that could re-null it at 20501 dereferences the result immediately at 20502), so upstream is right to leave it unannotated. The override exists because the port *builds* the struct, and construction passes through states the public contract never exposes. `ufbxi_update_shader_texture` re-assigns the field from `ufbx_find_prop_len` (ufbx.c:20501 → `native/scene_process.rs:4394`), which returns NULL when the property is absent: storing that into a `NonNull<Prop>` would materialize an invalid value at the store itself, and the line is not expressible 1:1 without a `transmute` that is UB exactly when it fires. The struct is also zeroed before the field is assigned (`memset`, ufbx.c:20651 → `ptr::write_bytes`, `scene_process.rs:4627`) and read back for a null test (`scene_process.rs:4387`). Unlike the `main_texture` row, this override does **not** go away if upstream annotates. Same generator allowlist. |
| `Unsafe<T>` unchanged | — | — | (No divergence; listed to record it was considered: migration to native `unsafe fields` deferred until stabilization — see rust-lang/rust#132922.) |
| `dom_find`, `dom_as_int32_list`, `dom_as_int64_list`, `dom_as_float_list`, `dom_as_double_list`, `dom_as_real_list`, `dom_as_blob_list`, `find_shader_texture_input` | `fn f<'a>(&DomNode, ..) -> &'a T` — the returned lifetime is bound to no argument | `fn f(&DomNode, ..) -> &T` — elided, so the result borrows the node it was looked up from | Soundness fix. An unbound `'a` lets the caller pick `'static` and keep a reference into the DOM / shader-texture storage after the scene is freed. The returned data lives in the scene that owns the node (`ufbx_dom_find_len` ufbx.c:32957, `ufbx_find_shader_texture_input_len` ufbx.c:31463 walk the node's own children/inputs), so the node borrow is the honest bound. Every call that compiles against 0.11.2 with a lifetime the scene actually satisfies still compiles. Unchanged in ufbx-rust 0.11.3. Generator `override_functions`, `NOTE(ufbx-rs-native)` there. |
| `find_anim_prop`, `find_anim_props` (free fns and `AnimLayer` methods), `find_shader_prop` (free fn and `Shader` method), `get_bone_pose` | the result shares `'a` with a secondary argument (`&'a Element`, `&'a str`, `&'a Node`) | `'a` binds only the container (`&'a AnimLayer` / `&'a Shader` / `&'a Pose`); the secondary argument is a lookup key with its own lifetime | Over-constrained baseline, loosened: the result never points into the key. `ufbx_find_anim_prop_len` (ufbx.c:30775) returns an entry of `layer->anim_props`; `ufbx_get_bone_pose` (ufbx.c:31405) an entry of `pose->bone_poses`; `ufbx_find_shader_prop_len` (ufbx.c:31425) returns a binding's `material_prop` string or the static `ufbx_empty_string`, never the `name` argument. Strictly more permissive for callers (a temporary key may be dropped while the result is held); nothing that compiled against 0.11.2 breaks. Unchanged in ufbx-rust 0.11.3. Generator `override_functions` / `override_member_functions`, `NOTE(ufbx-rs-native)` there. |
| `find_baked_node`, `find_baked_element`, `find_baked_node_by_typed_id`, `find_baked_element_by_element_id` | result lifetime bound to the `&'a mut Node` / `&'a mut Element` key, or (id variants) to nothing at all | result lifetime bound to the `&'a mut BakedAnim` it is found in; the `&mut` parameter types are kept verbatim | Soundness fix. The results are entries of `bake->nodes` / `bake->elements` (`ufbx_find_baked_node` ufbx.c:31320), so binding them to the key (or to `'static`) allows use after the bake is freed. The `&mut BakedAnim` / `&mut Node` parameters are a parity artifact of C's non-const signatures and stay as-is so drop-in callers do not change. Unchanged in ufbx-rust 0.11.3. Generator `override_functions`, `NOTE(ufbx-rs-native)` there. |
| `evaluate_prop_flags`, `evaluate_props_flags` | `evaluate_prop_flags(..) -> Prop`; `evaluate_props_flags(.., buffer: &mut Prop, buffer_size: usize, ..) -> Props` | `evaluate_prop_flags<'a, 'b>(&'a Anim, &'a Element, name: &'b str, f64, u32) -> ExternalRef<'b, Prop>`; `evaluate_props_flags<'a, 'b>(&'a Anim, &'a Element, f64, buffer: &'b mut [ExternalRef<'b, Prop>], u32) -> ExternalRef<'b, Props>` (both `where 'a: 'b`) | The flags variants take the exact shape ufbx-rust 0.11.2 already gives their flag-less twins `evaluate_prop` / `evaluate_props`; only the `_flags` entry points were left with raw bindgen output upstream. Soundness: on a miss C stores the caller's `name` pointer in the returned prop (`result.name.data = name;` ufbx.c:30965), so a by-value `Prop` outliving the name string dangles; `ExternalRef<'b, _>` pins the borrow. The `&mut Prop` + `usize` pair is a single-element reference plus an unchecked count (`buffer_size > 1` writes past the referent, ufbx.c:30996), and the returned `Props` aliases that buffer; the slice form carries its length and the result borrows it. `Prop` remains `Copy`, so callers can still copy out through `Deref` as with `evaluate_prop`. Unchanged in ufbx-rust 0.11.3. Generator `override_functions`, `NOTE(ufbx-rs-native)` there. |

| `pub static ufbx_empty_string` / `ufbx_empty_blob` | `String` / `Blob` | crate-internal, wrapped in `Sync` newtypes | ufbx-rust declares them in an `extern` block, where Rust does not require `Sync`; as native statics they must be `Sync`, and `String`/`Blob` hold raw pointers. The values are reachable as `String::default()`-style zero values; the remaining raw `ufbx_*` fn surface and the 11 plain-typed ABI statics stay public for drop-in use. |

## 3. Accepted behavioral divergences vs the C build

| Behavior | C | native port | Why |
|---|---|---|---|
| `ufbxi_f64_to_i64` at input exactly `+2^63` | UB; x86-64 gives `INT64_MIN`, AArch64 gives `INT64_MAX` | Rust `as` (saturating) = AArch64 behavior on all targets | No portable choice matches both C builds; documented in PORTING.md integer table. Divergence vs an x86-64-built C oracle on exactly this input. |
| `ufbx_error_frame.source_line` / stringified conditions | C `__LINE__` / `#cond` bytes | Rust `line!()` / `stringify!` (with verbatim-C override where parity matters) | Line numbers necessarily differ; the fuzz-check table is regenerated per-build (see UPSTREAM.md). |
| `ufbx_*` symbols in the final binary's symbol table | Always present (ufbx-rust links real C objects, so `dlsym("ufbx_load_file")` etc. always resolves) | Present only under the `c-abi` feature; by default the items exist under mangled Rust names | The raw surface is Rust-implemented; C-linkage names are emitted by `#[cfg_attr(feature = "c-abi", no_mangle)]`. Source-level use (calls, fn pointers, `ufbx_default_open_file` address compares) is unaffected — only linker/loader-level lookup (linking C objects against the crate, `dlsym`, FFI bridges by name) needs `c-abi`. |
