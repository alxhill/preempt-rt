use crate::sched::{IntoSchedParams, ParameterizedScheduler, Scheduler};
use std::thread;

#[must_use = "must eventually spawn the thread"]
#[derive(Debug)]
pub struct Builder {
    name: Option<String>,
    stack_size: Option<usize>,
    parameterized_scheduler: ParameterizedScheduler,
}

impl Builder {
    pub fn new(scheduler: Scheduler, params: impl IntoSchedParams) -> Builder {
        Builder {
            name: None,
            stack_size: None,
            parameterized_scheduler: scheduler.with_params(params.into_sched_params()),
        }
    }

    pub fn name(mut self, name: &str) -> Builder {
        self.name = Some(name.into());
        self
    }

    pub fn stack_size(mut self, stack_size: usize) -> Builder {
        self.stack_size = Some(stack_size);
        self
    }

    pub fn try_spawn<F, T>(self, f: F) -> thread::JoinHandle<T>
    where
        F: FnOnce(crate::sched::RtResult<()>) -> T + Send + 'static,
        T: Send + 'static,
    {
        let mut tb = thread::Builder::new();

        if let Some(name) = self.name {
            tb = tb.name(name.clone());
        }

        if let Some(stack_size) = self.stack_size {
            tb = tb.stack_size(stack_size);
        }

        tb.spawn(|| {
            let sched_result = self.parameterized_scheduler.set_current();
            f(sched_result)
        })
        .expect("failed to spawn thread")
    }
}

/// Spawn a thread with the provided scheduler and params.
///
/// Params can either be a SchedParams struct, or an i32 representing the desired priority.
/// This function validates that the priority is between min and max for the scheduler before
/// attempting to set it. It panics if the priority is outside the allowed range or setting the
/// scheduler returns an error code.
///
/// This function will panic if the scheduler and priority are not correct or could not be set
/// for any reason (process does not have the correct permissions, the scheduler is
/// not supported on the target, or PREEMPT_RT is not enabled/not supported on this platform).
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
///
/// This function passes an RtResult to the provided closure which can be used to check if
/// the chosen scheduler has been enabled.
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
    Builder::new(scheduler, params).try_spawn(f)
}
