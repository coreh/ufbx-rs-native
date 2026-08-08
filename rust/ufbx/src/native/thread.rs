//! Port of the `// -- Threading` banner section (ufbx.c:5980-6173).
//! Phase 1: only the TYPE definitions are ported (ufbx.c:5982-6021) — they are
//! required by `ufbxi_context` (`native::parse`), which embeds
//! `ufbxi_thread_pool` by value. The thread-pool functions
//! (`ufbxi_thread_pool_execute` .. `ufbxi_thread_pool_run_task`,
//! ufbx.c:6023-6173) are NOT YET PORTED.
#![allow(dead_code)]

use core::ffi::c_void;

use crate::generated::{Error, RawThreadOpts};
use crate::native::allocator::Allocator;

// ufbx.h:170 `#define UFBX_THREAD_GROUP_COUNT 4`
pub(crate) const THREAD_GROUP_COUNT: usize = 4;

// ufbx.c:5982-5983 (forward typedefs collapse into the struct definitions)

// ufbx.c:5985 `typedef bool ufbxi_task_fn(ufbxi_task *task);`
// Internal fn-pointer typedef (PORTING.md "Callbacks"): C passes function
// designators — plain `extern "C"` fn pointers, never closures.
pub(crate) type TaskFn = unsafe extern "C" fn(task: *mut Task) -> bool;

// ufbx.c:5987-5990 `ufbxi_task`
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct Task {
    pub data: *mut c_void,
    pub error: *const u8,
}

// ufbx.c:5992-5995 `ufbxi_task_imp`
// C-parity: `fn` is a plain function pointer in C; `Option` only so the
// zero-initialized pool (C callers memset the containing context) is
// representable.
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct TaskImp {
    pub task: Task,
    pub fn_: Option<TaskFn>,
}

// ufbx.c:5997-6000 `ufbxi_task_group`
#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct TaskGroup {
    pub max_index: u32,
    pub wait_index: u32,
}

// ufbx.c:6002-6021 `ufbxi_thread_pool`
#[repr(C)]
pub(crate) struct ThreadPool {
    pub opts: RawThreadOpts,
    pub ator: *mut Allocator,
    pub error: *mut Error,
    pub user_ptr: *mut c_void,

    pub enabled: bool,
    pub failed: bool,
    pub error_desc: *const u8,

    pub start_index: u32,
    pub execute_index: u32,
    pub wait_index: u32,

    pub groups: [TaskGroup; THREAD_GROUP_COUNT],
    pub group: u32,

    pub num_tasks: u32,
    pub tasks: *mut TaskImp,
}
