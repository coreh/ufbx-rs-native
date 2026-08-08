//! Port of the corresponding `ufbx.c` section — see PORTING.md routing table.
//! NOT YET PORTED (Phase 1), except for the single comparator below, which the
//! `// -- Reading the parsed data` unit needs through `ufbx_find_prop_len`.
#![allow(dead_code)]

use crate::generated::Prop;
use crate::native::string_pool::str_less;
use crate::prelude::String;

// -- Scene processing (ufbx.c:18545-...)
//
// PARTIAL: `ufbxi_cmp_prop_less_ref` only; the rest of the section is unported.

// ufbx.c:18572-18576 `ufbxi_cmp_prop_less_ref`
#[inline(always)]
pub(crate) unsafe fn cmp_prop_less_ref(a: *const Prop, name: String, key: u32) -> bool {
    if (*a)._internal_key != key {
        return (*a)._internal_key < key;
    }
    str_less((*a).name, name)
}
