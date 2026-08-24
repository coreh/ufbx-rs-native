//! Port of the `// -- Threading` banner section (ufbx.c:5980-6173), complete:
//! the type definitions (ufbx.c:5982-6021) — required by `ufbxi_context`
//! (`native::parse`), which embeds `ufbxi_thread_pool` by value — plus the
//! whole task ring: `ufbxi_thread_pool_execute`, the wait/flush helpers used
//! by `ufbxi_read_objects_threaded` (`native::read`, ufbx.c:15129), `_init`,
//! `_free`, `_create_task` and `_run_task`. The public
//! `ufbx_thread_pool_run_task` entry point delegating to
//! `ufbxi_thread_pool_execute` lives in `native::api` with its `capi.rs` shim.
// A full `c-abi` + `dev` build requires every ported item to be reachable;
// reduced feature sets legitimately leave gated helpers unused.
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
    // Runs on a POOL THREAD while the loader thread keeps writing other fields
    // of the same `ThreadPool` (`flush_group`, `run_task`, ...), so no
    // reference over the whole struct may be formed here: only the two fields
    // C reads, which are fixed by `thread_pool_init` for the pool's lifetime.
    // SAFETY: `pool` is a live initialized thread pool (fn raw-param contract);
    // `tasks`/`num_tasks` are its own init-time-constant fields.
    let tasks: *mut TaskImp = unsafe { (*pool).tasks };
    let num_tasks: u32 = unsafe { (*pool).num_tasks };
    // SAFETY: the index is reduced modulo `num_tasks`, so it addresses inside
    // the `tasks` run `thread_pool_init` allocated with that many entries.
    let imp: *mut TaskImp = unsafe { tasks.add((index % num_tasks) as usize) };
    // SAFETY: `imp` addresses a ring entry a `thread_pool_create_task` call
    // filled in, so `fn_` is `Some`; `&raw mut` takes the task field address
    // without forming a reference (C: `&imp->task`), which is the pointer the
    // C-callback contract expects.
    if unsafe { ((*imp).fn_.unwrap())(&raw mut (*imp).task) } {
        // SAFETY: writing the error slot of that same live ring entry.
        unsafe { (*imp).task.error = core::ptr::null() };
    } else if unsafe { (*imp).task.error }.is_null() {
        // C: `imp->task.error = "";`
        // SAFETY: writing the error slot of that same live ring entry;
        // `EMPTY_CHAR` is a NUL-terminated `'static` run.
        unsafe { (*imp).task.error = EMPTY_CHAR.as_ptr() };
    }
}

// ufbx.c:6033-6043 `ufbxi_thread_pool_update_finished`
#[inline(never)]
pub(crate) unsafe fn thread_pool_update_finished(pool: *mut ThreadPool, max_index: u32) {
    // Other groups' tasks may still be executing on pool threads, so this
    // touches the loader-owned scalar fields only (never a whole-struct
    // reference, which would retag the fields those threads read).
    // SAFETY (this fn's field ops): `pool` is a live initialized thread pool
    // (fn raw-param contract); every access is one of its own fields.
    while unsafe { (*pool).wait_index } < max_index {
        // SAFETY: the wait index is reduced modulo `num_tasks`, so it addresses
        // inside the `tasks` run allocated with that many entries.
        let task: *mut TaskImp = unsafe {
            (*pool)
                .tasks
                .add(((*pool).wait_index % (*pool).num_tasks) as usize)
        };
        // SAFETY (both reads): `task` addresses an in-bounds ring entry, and the
        // sole caller `thread_pool_wait_imp` reaches this only once `wait_fn`
        // has returned for every index below `max_index`, so the executing
        // thread's write to `task.error` (null, or the failure string
        // `thread_pool_execute` stored) happened-before these reads.
        if !unsafe { (*pool).failed } && !unsafe { (*task).task.error }.is_null() {
            unsafe {
                (*pool).failed = true;
                (*pool).error_desc = (*task).task.error;
            }
        }
        unsafe { (*pool).wait_index = (*pool).wait_index.wrapping_add(1) };
    }
}

// ufbx.c:6045-6064 `ufbxi_thread_pool_wait_imp`
#[inline(never)]
pub(crate) unsafe fn thread_pool_wait_imp(
    pool: *mut ThreadPool,
    group: u32,
    can_fail: bool,
) -> Result<(), Fail> {
    // SAFETY (both reads): `pool` is a live thread pool (fn raw-param contract)
    // and `group` is one of its `THREAD_GROUP_COUNT` ring groups, so the array
    // index is in bounds.
    let max_index: u32 = unsafe { (*pool).groups[group as usize].max_index };

    if unsafe { (*pool).groups[group as usize].wait_index } < max_index {
        // SAFETY: the pool is only enabled — the state every caller reaches
        // this in — when `thread_pool_init` saw both `run_fn` and `wait_fn`, so
        // the unwrap holds; the context handle is the pool pointer itself, as
        // the C-callback contract specifies.
        unsafe {
            ((*pool).opts.pool.wait_fn.unwrap())(
                (*pool).opts.pool.user,
                pool as ThreadPoolContext,
                group,
                max_index,
            );
        }
        // SAFETY: writing the same in-bounds group slot of the live pool.
        unsafe { (*pool).groups[group as usize].wait_index = max_index };
    }
    // SAFETY: `pool` is live, which is the callee's whole raw-param contract.
    unsafe { thread_pool_update_finished(pool, max_index) };

    // SAFETY: reading scalar fields of the live pool.
    if unsafe { (*pool).failed } && can_fail {
        let error: *mut Error = unsafe { (*pool).error };
        if !unsafe { (*pool).error_desc }.is_null() {
            // SAFETY: `error` is the error slot `thread_pool_init` stored (the
            // load context outlives the pool), and `error_desc` is the
            // NUL-terminated `'static` string a failing task published.
            unsafe {
                (*error).description.data = (*pool).error_desc;
                (*error).description.length = strlen((*pool).error_desc);
            }
        }
        ufbxi_fail_err!(
            unsafe { crate::native::error::ErrorView::from_ptr(error) },
            "Task failed"
        );
    }
    Ok(())
}

// ufbx.c:6066-6070 `ufbxi_thread_pool_wait_group`
#[inline(never)]
pub(crate) unsafe fn thread_pool_wait_group(pool: *mut ThreadPool) -> Result<(), Fail> {
    // SAFETY: `pool` is live per this fn's raw-param contract, which is also
    // the callee's; `group` is read from that same pool.
    unsafe { thread_pool_wait_imp(pool, (*pool).group, true)? };
    Ok(())
}

// ufbx.c:6072-6079 `ufbxi_thread_pool_wait_all`
#[inline(never)]
pub(crate) unsafe fn thread_pool_wait_all(pool: *mut ThreadPool) -> Result<(), Fail> {
    let mut i: u32 = 0;
    while (i as usize) < THREAD_GROUP_COUNT {
        // SAFETY: `pool` is live per this fn's raw-param contract, which is also
        // the callee's; `group` is read from that same pool.
        unsafe { thread_pool_wait_imp(pool, (*pool).group, true)? };
        // SAFETY: rotating the pool's own group counter, kept in range by the
        // modulo (C: `pool->group = (pool->group + 1) % UFBX_THREAD_GROUP_COUNT`).
        unsafe { (*pool).group = ((*pool).group + 1) % THREAD_GROUP_COUNT as u32 };
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
    // SAFETY (both reads): `opts` points at a live `RawThreadOpts` the caller
    // owns for the duration of the call (fn raw-param contract).
    if !(unsafe { (*opts).pool.run_fn }.is_some() && unsafe { (*opts).pool.wait_fn }.is_some()) {
        return Ok(());
    }
    // SAFETY: `pool` is the caller's live (zero-initialized) pool storage.
    unsafe { (*pool).enabled = true };

    // SAFETY: reading a scalar field of the live `opts`.
    let mut num_tasks: u32 = min_sz(unsafe { (*opts).num_tasks }, i32::MAX as usize) as u32;
    if num_tasks == 0 {
        num_tasks = 2048;
    }

    // C: `pool->opts = *opts;` — struct assignment is memcpy.
    // SAFETY: `RawThreadOpts` is plain data (scalars and fn pointers), so a
    // bitwise read of the live `opts` into the live pool duplicates no
    // ownership; both pointers are valid and distinct objects.
    unsafe { (*pool).opts = core::ptr::read(opts) };
    // SAFETY: reading the copy stored in the live pool.
    if unsafe { (*pool).opts.pool.init_fn }.is_some() {
        let mut info = MaybeUninit::<ThreadPoolInfo>::uninit(); // ufbxi_uninit
        let info: *mut ThreadPoolInfo = info.as_mut_ptr();
        // SAFETY: `info` is the address of the live stack `MaybeUninit` above;
        // writing the field initializes it in place (C: `ufbxi_uninit` leaves
        // the rest for the callback to fill).
        unsafe { (*info).max_concurrent_tasks = num_tasks };
        ufbxi_check_err!(
            // SAFETY: `error` is the caller's live error slot.
            unsafe { crate::native::error::ErrorView::from_ptr(error) },
            // SAFETY: `init_fn` is `Some` per the check above; the context
            // handle is the pool pointer itself and `info` the stack struct
            // just initialized, as the C-callback contract specifies.
            unsafe {
                ((*pool).opts.pool.init_fn.unwrap())(
                    (*pool).opts.pool.user,
                    pool as ThreadPoolContext,
                    info,
                )
            },
            "pool->opts.pool.init_fn(pool->opts.pool.user, (ufbx_thread_pool_context)pool, &info)"
        );
    }
    // SAFETY (all three stores): writing scalar fields of the live pool.
    unsafe { (*pool).ator = ator };
    unsafe { (*pool).error = error };

    unsafe { (*pool).num_tasks = num_tasks };
    // SAFETY: `ator` is the caller's live allocator; `alloc` returns null on
    // failure, which the check below handles.
    unsafe { (*pool).tasks = alloc::<TaskImp>(ator, num_tasks as usize) };
    ufbxi_check_err!(
        // SAFETY: `error` is the caller's live error slot.
        unsafe { crate::native::error::ErrorView::from_ptr(error) },
        // SAFETY: reading the pointer field just stored in the live pool.
        !unsafe { (*pool).tasks }.is_null(),
        "pool->tasks"
    );

    Ok(())
}

// ufbx.c:6107-6122 `ufbxi_thread_pool_free`
#[inline(never)]
pub(crate) unsafe fn thread_pool_free(pool: *mut ThreadPool) {
    // SAFETY: `pool` is the caller's live (at minimum zero-initialized) pool
    // storage — `free_temp` also reaches this for a pool whose init bailed at
    // the `run_fn`/`wait_fn` check, which reads `enabled == false` and returns.
    if !unsafe { (*pool).enabled } {
        return;
    }

    // Wait for all pending tasks
    let mut i: u32 = 0;
    while (i as usize) < THREAD_GROUP_COUNT {
        // SAFETY: rotating the pool's own group counter, kept in range by the
        // modulo.
        unsafe { (*pool).group = ((*pool).group + 1) % THREAD_GROUP_COUNT as u32 };
        // SAFETY: `pool` is live per this fn's raw-param contract, which is also
        // the callee's; `group` is read from that same pool.
        ufbxi_ignore!(unsafe { thread_pool_wait_imp(pool, (*pool).group, false) });
        i += 1;
    }

    // SAFETY: reading the callback slot of the live pool.
    if unsafe { (*pool).opts.pool.free_fn }.is_some() {
        // SAFETY: `free_fn` is `Some` per the check above; the context handle is
        // the pool pointer itself, as the C-callback contract specifies.
        unsafe {
            ((*pool).opts.pool.free_fn.unwrap())((*pool).opts.pool.user, pool as ThreadPoolContext);
        }
    }

    // SAFETY: `pool` is live; on a completed init `tasks` is the `num_tasks`-entry
    // run `thread_pool_init` allocated from `ator`, so this frees it with the
    // matching allocator and count. If init failed earlier `tasks` is null: with
    // `num_tasks == 0` (`init_fn` failure) `free_size` early-returns, and with
    // `num_tasks != 0` (task-run allocation failure) it stops at its non-null
    // assert — C-parity with ufbx.c:6121; no memory is accessed either way.
    unsafe { free::<TaskImp>((*pool).ator, (*pool).tasks, (*pool).num_tasks as usize) };
}

// ufbx.c:6124-6127 `ufbxi_thread_pool_available_tasks`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn thread_pool_available_tasks(pool: *mut ThreadPool) -> u32 {
    // SAFETY: `pool` is a live initialized thread pool (fn raw-param contract);
    // scalar field reads only (pool threads may be executing concurrently).
    unsafe {
        (*pool)
            .num_tasks
            .wrapping_sub((*pool).start_index.wrapping_sub((*pool).wait_index))
    }
}

// ufbx.c:6129-6142 `ufbxi_thread_pool_flush_group`
#[inline(never)]
pub(crate) unsafe fn thread_pool_flush_group(pool: *mut ThreadPool) {
    // SAFETY (all three reads): `pool` is a live initialized thread pool (fn
    // raw-param contract); these are scalar fields.
    let group: u32 = unsafe { (*pool).group };
    let start_index: u32 = unsafe { (*pool).execute_index };
    let count: u32 = unsafe { (*pool).start_index }.wrapping_sub(start_index);
    if count > 0 {
        // SAFETY: reading the callback slot of the live pool.
        if unsafe { (*pool).opts.pool.run_fn }.is_some() {
            // SAFETY: `run_fn` is `Some` per the check above; the context handle
            // is the pool pointer itself, as the C-callback contract specifies.
            unsafe {
                ((*pool).opts.pool.run_fn.unwrap())(
                    (*pool).opts.pool.user,
                    pool as ThreadPoolContext,
                    group,
                    start_index,
                    count,
                );
            }
        }
        // SAFETY: `group` came from the pool itself and the modulo below keeps
        // it below `THREAD_GROUP_COUNT`, so the array index is in bounds.
        unsafe { (*pool).groups[group as usize].max_index = start_index.wrapping_add(count) };
        // SAFETY: writing a scalar field of the live pool.
        unsafe { (*pool).execute_index = start_index.wrapping_add(count) };
    }
    // SAFETY: rotating the pool's own group counter, kept in range by the
    // modulo.
    unsafe { (*pool).group = (group + 1) % THREAD_GROUP_COUNT as u32 };
}

// ufbx.c:6144-6165 `ufbxi_thread_pool_create_task`
#[inline(never)]
#[must_use]
pub(crate) unsafe fn thread_pool_create_task(pool: *mut ThreadPool, fn_: TaskFn) -> *mut Task {
    // SAFETY (this fn's pool reads): `pool` is a live initialized thread pool
    // (fn raw-param contract); scalar field reads only, as pool threads may be
    // executing concurrently.
    let index: u32 = unsafe { (*pool).start_index };
    let wait_index: u32 = unsafe { (*pool).wait_index };
    let num_tasks: u32 = unsafe { (*pool).num_tasks };
    if index.wrapping_sub(wait_index) >= num_tasks {
        // C-parity: the C nests the same condition twice (ufbx.c:6147-6152) —
        // kept verbatim.
        if index.wrapping_sub(wait_index) >= num_tasks {
            // No space left
            return core::ptr::null_mut();
        }
    } else if index == i32::MAX as u32 {
        // TODO: Expand to 64 bits if possible?
        return core::ptr::null_mut();
    }

    // SAFETY: the index is reduced modulo `num_tasks`, so it addresses inside
    // the `tasks` run `thread_pool_init` allocated with that many entries.
    let imp: *mut TaskImp = unsafe { (*pool).tasks.add((index % num_tasks) as usize) };
    if index < num_tasks {
        // SAFETY: `imp` addresses one whole `TaskImp` of that allocation, so
        // zeroing exactly `size_of::<TaskImp>()` bytes stays inside it; all-zero
        // is a valid `TaskImp` (`fn_` is `Option<fn>`, `None` when null).
        unsafe { core::ptr::write_bytes(imp as *mut u8, 0, size_of::<TaskImp>()) };
    }

    // SAFETY: writing the callback slot of that same ring entry.
    unsafe { (*imp).fn_ = Some(fn_) };

    // SAFETY: `&raw mut` takes the task field address without forming a
    // reference (C: `&imp->task`), keeping the entry's provenance.
    unsafe { &raw mut (*imp).task }
}

// ufbx.c:6167-6173 `ufbxi_thread_pool_run_task`
pub(crate) unsafe fn thread_pool_run_task(pool: *mut ThreadPool, task: *mut Task) {
    // C: `(void)task;` — `task` is only read by the assert below.
    let _ = task;
    // SAFETY (this fn's pool ops): `pool` is a live initialized thread pool
    // (fn raw-param contract); scalar field accesses only, as pool threads may
    // be executing concurrently.
    let index: u32 = unsafe { (*pool).start_index };
    // SAFETY: the index is reduced modulo `num_tasks`, so it addresses inside
    // the `tasks` run allocated with that many entries; `&raw mut` takes the
    // task field address without forming a reference (C: `&...->task`).
    ufbx_assert!(
        task == unsafe { &raw mut (*(*pool).tasks.add((index % (*pool).num_tasks) as usize)).task }
    );
    unsafe { (*pool).start_index = index.wrapping_add(1) };
}
