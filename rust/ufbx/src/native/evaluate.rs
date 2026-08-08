//! Port of the corresponding `ufbx.c` section — see PORTING.md routing table.
//! NOT YET PORTED (Phase 1), except for the one refcount-header type below
//! that the already-ported API entry points need.
#![allow(dead_code)]

use core::mem::size_of;

use crate::generated::BakedAnim;
use crate::native::parse::Refcount;

// ufbx.c:26672-26676 `ufbxi_baked_anim_imp`
// C declares this OUTSIDE the `#if UFBXI_FEATURE_ANIMATION_BAKING` guard
// (opened at ufbx.c:26678) precisely so `ufbx_retain_baked_anim` /
// `ufbx_free_baked_anim` (ufbx.c:31291-31309) compile in both builds; those
// two are ported in `native::api`, so the type lands here — its C home module
// — ahead of the rest of the section. C declares no `ufbx_static_assert` for
// it (contrast `ufbxi_scene_imp`), but `ufbxi_get_imp(ufbxi_baked_anim_imp,
// bake)` (ufbx.c:31295) depends on the header-then-payload layout, so the
// offset is pinned here (same treatment as `ufbxi_anim_imp` in
// `native::scene_process`).
#[repr(C)]
pub(crate) struct BakedAnimImp {
    pub refcount: Refcount,
    pub bake: BakedAnim,
    pub magic: u32,
}

const _: () = assert!(core::mem::offset_of!(BakedAnimImp, bake) == size_of::<Refcount>());
