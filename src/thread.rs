use crate::sched::{IntoSchedParams, Scheduler};
use std::thread;

/// Spawn a thread with the provided scheduler and params.
///
/// Params can either be a SchedParams struct, or an i32 representing the desired priority.
/// This function validates that the priority is between min and max for the scheduler before
/// attempting to set it. It panics if the priority is outside the allowed range or setting the
/// scheduler returns an error code.
pub fn spawn<F, T>(
    scheduler: Scheduler,
    params: impl IntoSchedParams,
    f: F,
) -> thread::JoinHandle<T>
where
    F: FnOnce() -> T,
    F: Send + 'static,
    T: Send + 'static,
{
    try_spawn(scheduler, params, move |set_result| {
        set_result.expect("failed to set scheduler");
        f()
    })
}

/// Spawn a thread and attempt to set the schedule of the current thread. The result of setting
/// the scheduler is provided to the thread closure as an argument.
///
/// Params can either be a SchedParams struct, or an i32 representing the desired priority.
/// This function validates that the priority is between min and max for the scheduler before
/// attempting to set it. Failures will continue execution and pass through the Result to the
/// thread closure.
#[cfg(target_os = "linux")]
pub fn try_spawn<F, T>(
    scheduler: Scheduler,
    params: impl IntoSchedParams,
    f: F,
) -> thread::JoinHandle<T>
where
    F: FnOnce(crate::sched::RtResult<()>) -> T,
    F: Send + 'static,
    T: Send + 'static,
{
    let params = params.into_sched_params();
    thread::spawn(move || {
        let set_result = scheduler
            .with_params(params)
            .and_then(|ps| ps.set_current());
        f(set_result)
    })
}

/// Spawn a thread and attempt to set the schedule of the current thread. The result of setting
/// the scheduler is provided to the thread closure as an argument.
///
/// Params can either be a SchedParams struct, or an i32 representing the desired priority.
/// This function validates that the priority is between min and max for the scheduler before
/// attempting to set it. Failures will continue execution and pass through the Result to the
/// thread closure.
///
/// This is a stub version of the function that always passes PreemptRtError::NonLinuxPlatform
/// to the thread closure.
#[cfg(all(feature = "non-linux-stubs", target_os = "macos"))]
pub fn try_spawn<F, T>(
    _scheduler: Scheduler,
    _params: impl IntoSchedParams,
    f: F,
) -> thread::JoinHandle<T>
where
    F: FnOnce(crate::sched::RtResult<()>) -> T,
    F: Send + 'static,
    T: Send + 'static,
{
    thread::spawn(move || f(Err(crate::sched::PreemptRtError::NonLinuxPlatform("macos"))))
}
