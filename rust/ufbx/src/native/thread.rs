//! Port of the `// -- Threading` banner section (ufbx.c:5980-6173).
//! Phase 1: the TYPE definitions (ufbx.c:5982-6021) — they are required by
//! `ufbxi_context` (`native::parse`), which embeds `ufbxi_thread_pool` by
//! value — plus the six wait/flush helpers that
//! `ufbxi_read_objects_threaded` (`native::read`, ufbx.c:15129) calls:
//! `ufbxi_thread_pool_update_finished`, `_wait_imp`, `_wait_group`,
//! `_wait_all`, `_available_tasks` and `_flush_group`.
//!
//! Still NOT PORTED from this section: `ufbxi_thread_pool_execute` (6023),
//! `_init` (6081), `_free` (6107), `_create_task` (6144) and `_run_task`
//! (6167) — port them here, in C order, when this section's own unit lands.
#![allow(dead_code)]

use core::ffi::c_void;

use crate::generated::{Error, RawThreadOpts};
use crate::native::allocator::Allocator;
use crate::native::error::{strlen, ufbxi_fail_err, Fail};
use crate::prelude::ThreadPoolContext;

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

// ufbx.c:6033-6043 `ufbxi_thread_pool_update_finished`
#[inline(never)]
pub(crate) unsafe fn thread_pool_update_finished(pool: *mut ThreadPool, max_index: u32) {
    while (*pool).wait_index < max_index {
        let task: *mut TaskImp = (*pool)
            .tasks
            .add(((*pool).wait_index % (*pool).num_tasks) as usize);
        if !(*pool).failed && !(*task).task.error.is_null() {
            (*pool).failed = true;
            (*pool).error_desc = (*task).task.error;
        }
        (*pool).wait_index = (*pool).wait_index.wrapping_add(1);
    }
}

// ufbx.c:6045-6064 `ufbxi_thread_pool_wait_imp`
#[inline(never)]
pub(crate) unsafe fn thread_pool_wait_imp(
    pool: *mut ThreadPool,
    group: u32,
    can_fail: bool,
) -> Result<(), Fail> {
    let max_index: u32 = (*pool).groups[group as usize].max_index;

    if (*pool).groups[group as usize].wait_index < max_index {
        ((*pool).opts.pool.wait_fn.unwrap())(
            (*pool).opts.pool.user,
            pool as ThreadPoolContext,
            group,
            max_index,
        );
        (*pool).groups[group as usize].wait_index = max_index;
    }
    thread_pool_update_finished(pool, max_index);

    if (*pool).failed && can_fail {
        let error: *mut Error = (*pool).error;
        if !(*pool).error_desc.is_null() {
            (*error).description.data = (*pool).error_desc;
            (*error).description.length = strlen((*pool).error_desc);
        }
        ufbxi_fail_err!(error, "Task failed");
    }
    Ok(())
}

// ufbx.c:6066-6070 `ufbxi_thread_pool_wait_group`
#[inline(never)]
pub(crate) unsafe fn thread_pool_wait_group(pool: *mut ThreadPool) -> Result<(), Fail> {
    thread_pool_wait_imp(pool, (*pool).group, true)?;
    Ok(())
}

// ufbx.c:6072-6079 `ufbxi_thread_pool_wait_all`
#[inline(never)]
pub(crate) unsafe fn thread_pool_wait_all(pool: *mut ThreadPool) -> Result<(), Fail> {
    let mut i: u32 = 0;
    while (i as usize) < THREAD_GROUP_COUNT {
        thread_pool_wait_imp(pool, (*pool).group, true)?;
        (*pool).group = ((*pool).group + 1) % THREAD_GROUP_COUNT as u32;
        i += 1;
    }
    Ok(())
}

// ufbx.c:6124-6127 `ufbxi_thread_pool_available_tasks`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn thread_pool_available_tasks(pool: *mut ThreadPool) -> u32 {
    (*pool)
        .num_tasks
        .wrapping_sub((*pool).start_index.wrapping_sub((*pool).wait_index))
}

// ufbx.c:6129-6142 `ufbxi_thread_pool_flush_group`
#[inline(never)]
pub(crate) unsafe fn thread_pool_flush_group(pool: *mut ThreadPool) {
    let group: u32 = (*pool).group;
    let start_index: u32 = (*pool).execute_index;
    let count: u32 = (*pool).start_index.wrapping_sub(start_index);
    if count > 0 {
        if (*pool).opts.pool.run_fn.is_some() {
            ((*pool).opts.pool.run_fn.unwrap())(
                (*pool).opts.pool.user,
                pool as ThreadPoolContext,
                group,
                start_index,
                count,
            );
        }
        (*pool).groups[group as usize].max_index = start_index.wrapping_add(count);
        (*pool).execute_index = start_index.wrapping_add(count);
    }
    (*pool).group = (group + 1) % THREAD_GROUP_COUNT as u32;
}
