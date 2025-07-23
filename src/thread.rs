use crate::sched::{IntoSchedParams, Scheduler};
use std::thread;

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
