//! Port of the `// -- Threading` banner section (ufbx.c:5980-6173), complete:
//! the type definitions (ufbx.c:5982-6021) — required by `ufbxi_context`
//! (`native::parse`), which embeds `ufbxi_thread_pool` by value — plus the
//! whole task ring: `ufbxi_thread_pool_execute`, the wait/flush helpers used
//! by `ufbxi_read_objects_threaded` (`native::read`, ufbx.c:15129), `_init`,
//! `_free`, `_create_task` and `_run_task`. The public
//! `ufbx_thread_pool_run_task` entry point delegating to
//! `ufbxi_thread_pool_execute` lives in `native::api` with its `capi.rs` shim.
// Dead code with the full `c-abi` + `dev` surface enabled is a porting defect
// (an orphaned stub that no ported call site reaches); leaner feature sets
// legitimately strand items, so the lint is only armed for the full build.
#![cfg_attr(not(all(feature = "c-abi", feature = "dev")), allow(dead_code))]

use core::ffi::c_void;
use core::mem::{size_of, MaybeUninit};

use crate::generated::{Error, RawThreadOpts, ThreadPoolInfo};
use crate::native::allocator::{alloc, free, Allocator};
use crate::native::error::{strlen, ufbxi_check_err, ufbxi_fail_err, Fail, EMPTY_CHAR};
use crate::native::platform::{min_sz, ufbx_assert, ufbxi_ignore};
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

// ufbx.c:6023-6031 `ufbxi_thread_pool_execute`
pub(crate) unsafe fn thread_pool_execute(pool: *mut ThreadPool, index: u32) {
    let p = &*pool;
    let imp: *mut TaskImp = p.tasks.add((index % p.num_tasks) as usize);
    if ((*imp).fn_.unwrap())(&raw mut (*imp).task) {
        (*imp).task.error = core::ptr::null();
    } else if (*imp).task.error.is_null() {
        // C: `imp->task.error = "";`
        (*imp).task.error = EMPTY_CHAR.as_ptr();
    }
}

// ufbx.c:6033-6043 `ufbxi_thread_pool_update_finished`
#[inline(never)]
pub(crate) unsafe fn thread_pool_update_finished(pool: *mut ThreadPool, max_index: u32) {
    let p = &mut *pool;
    while p.wait_index < max_index {
        let task: *mut TaskImp = p.tasks.add((p.wait_index % p.num_tasks) as usize);
        if !p.failed && !(*task).task.error.is_null() {
            p.failed = true;
            p.error_desc = (*task).task.error;
        }
        p.wait_index = p.wait_index.wrapping_add(1);
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

// ufbx.c:6081-6105 `ufbxi_thread_pool_init`
#[inline(never)]
pub(crate) unsafe fn thread_pool_init(
    pool: *mut ThreadPool,
    error: *mut Error,
    ator: *mut Allocator,
    opts: *const RawThreadOpts,
) -> Result<(), Fail> {
    if !((*opts).pool.run_fn.is_some() && (*opts).pool.wait_fn.is_some()) {
        return Ok(());
    }
    (*pool).enabled = true;

    let mut num_tasks: u32 = min_sz((*opts).num_tasks, i32::MAX as usize) as u32;
    if num_tasks == 0 {
        num_tasks = 2048;
    }

    // C: `pool->opts = *opts;` — struct assignment is memcpy.
    (*pool).opts = core::ptr::read(opts);
    if (*pool).opts.pool.init_fn.is_some() {
        let mut info = MaybeUninit::<ThreadPoolInfo>::uninit(); // ufbxi_uninit
        let info: *mut ThreadPoolInfo = info.as_mut_ptr();
        (*info).max_concurrent_tasks = num_tasks;
        ufbxi_check_err!(
            error,
            ((*pool).opts.pool.init_fn.unwrap())(
                (*pool).opts.pool.user,
                pool as ThreadPoolContext,
                info
            ),
            "pool->opts.pool.init_fn(pool->opts.pool.user, (ufbx_thread_pool_context)pool, &info)"
        );
    }
    (*pool).ator = ator;
    (*pool).error = error;

    (*pool).num_tasks = num_tasks;
    (*pool).tasks = alloc::<TaskImp>(ator, num_tasks as usize);
    ufbxi_check_err!(error, !(*pool).tasks.is_null(), "pool->tasks");

    Ok(())
}

// ufbx.c:6107-6122 `ufbxi_thread_pool_free`
#[inline(never)]
pub(crate) unsafe fn thread_pool_free(pool: *mut ThreadPool) {
    if !(*pool).enabled {
        return;
    }

    // Wait for all pending tasks
    let mut i: u32 = 0;
    while (i as usize) < THREAD_GROUP_COUNT {
        (*pool).group = ((*pool).group + 1) % THREAD_GROUP_COUNT as u32;
        ufbxi_ignore!(thread_pool_wait_imp(pool, (*pool).group, false));
        i += 1;
    }

    if (*pool).opts.pool.free_fn.is_some() {
        ((*pool).opts.pool.free_fn.unwrap())((*pool).opts.pool.user, pool as ThreadPoolContext);
    }

    let p = &*pool;
    free::<TaskImp>(p.ator, p.tasks, p.num_tasks as usize);
}

// ufbx.c:6124-6127 `ufbxi_thread_pool_available_tasks`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn thread_pool_available_tasks(pool: *mut ThreadPool) -> u32 {
    let p = &*pool;
    p.num_tasks
        .wrapping_sub(p.start_index.wrapping_sub(p.wait_index))
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

// ufbx.c:6144-6165 `ufbxi_thread_pool_create_task`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn thread_pool_create_task(pool: *mut ThreadPool, fn_: TaskFn) -> *mut Task {
    let p = &*pool;
    let index: u32 = p.start_index;
    if index.wrapping_sub(p.wait_index) >= p.num_tasks {
        // C-parity: the C nests the same condition twice (ufbx.c:6147-6152) —
        // kept verbatim.
        if index.wrapping_sub(p.wait_index) >= p.num_tasks {
            // No space left
            return core::ptr::null_mut();
        }
    } else if index == i32::MAX as u32 {
        // TODO: Expand to 64 bits if possible?
        return core::ptr::null_mut();
    }

    let imp: *mut TaskImp = p.tasks.add((index % p.num_tasks) as usize);
    if index < p.num_tasks {
        core::ptr::write_bytes(imp as *mut u8, 0, size_of::<TaskImp>());
    }

    (*imp).fn_ = Some(fn_);

    &raw mut (*imp).task
}

// ufbx.c:6167-6173 `ufbxi_thread_pool_run_task`
pub(crate) unsafe fn thread_pool_run_task(pool: *mut ThreadPool, task: *mut Task) {
    // C: `(void)task;` — `task` is only read by the assert below.
    let _ = task;
    let p = &mut *pool;
    let index: u32 = p.start_index;
    ufbx_assert!(task == &raw mut (*p.tasks.add((index % p.num_tasks) as usize)).task);
    p.start_index = index.wrapping_add(1);
}
