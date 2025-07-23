use crate::sched::PreemptRtError::{PriorityAboveMax, PriorityBelowMin};
use libc::{c_int, pid_t};
use std::mem::MaybeUninit;
use std::{fmt, result};
use thiserror::Error;

/// PreemptRt result type
pub type Result<T> = result::Result<T, PreemptRtError>;

#[derive(Debug, Error)]
pub enum PreemptRtError {
    #[error("c function returned errno: {0}")]
    Errno(c_int),
    #[error("unknown scheduler for value {0}")]
    UnknownScheduler(c_int),
    #[error("priority {0} is higher than max priority {1}")]
    PriorityAboveMax(c_int, c_int),
    #[error("priority {0} is lower than min priority {1}")]
    PriorityBelowMin(c_int, c_int),
}

fn handle_errno(result: c_int) -> Result<c_int> {
    if result == -1 {
        Err(PreemptRtError::Errno(unsafe { *libc::__errno_location() }))
    } else {
        Ok(result)
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Pid(pid_t);

impl Pid {
    pub const fn current_thread() -> Self {
        Pid(0)
    }
}

impl From<Pid> for pid_t {
    fn from(pid: Pid) -> Self {
        pid.0
    }
}

impl fmt::Display for Pid {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

#[repr(i32)]
#[allow(non_camel_case_types)] // intentionally matching with libc / linux docs
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
/// The type of scheduler for use with [`sched_getscheduler`] and [`sched_setscheduler`].
/// See [man_sched(7)](https://man7.org/linux/man-pages/man7/sched.7.html) for more details
/// on the differences in behavior.
pub enum Scheduler {
    /// The default scheduler on non-realtime linux - also known as SCHED_OTHER.
    SCHED_NORMAL = libc::SCHED_NORMAL,
    /// The realtime FIFO scheduler. All FIFO threads have priority higher than 0 and
    /// preempt SCHED_NORMAL threads. Threads are executed in priority order, using
    /// first-in-first-out lists to handle two threads with the same priority.
    SCHED_FIFO = libc::SCHED_FIFO,
    /// Round-robin scheduler, similar to SCHED_FIFO but with a time quantum.
    SCHED_RR = libc::SCHED_RR,
    /// Batch scheduler, similar to SCHED_OTHER but assumes the thread is CPU intensive.
    /// The kernel applies a mild penalty to switching to this thread.
    /// As of Linux 2.6.16, the only valid priority is 0.
    SCHED_BATCH = libc::SCHED_BATCH,
    /// The idle scheduler only executes the thread when there are idle CPUs. SCHED_IDLE
    /// threads have no progress guarantees.
    SCHED_IDLE = libc::SCHED_IDLE,
    /// Deadline scheduler, attempting to provide guaranteed latency for requests.
    /// See the [linux kernel docs](https://docs.kernel.org/scheduler/sched-deadline.html)
    /// for details.
    SCHED_DEADLINE = libc::SCHED_DEADLINE,
}

impl TryFrom<c_int> for Scheduler {
    type Error = PreemptRtError;

    fn try_from(value: c_int) -> Result<Self> {
        match value {
            libc::SCHED_NORMAL => Ok(Scheduler::SCHED_NORMAL),
            libc::SCHED_FIFO => Ok(Scheduler::SCHED_FIFO),
            libc::SCHED_RR => Ok(Scheduler::SCHED_RR),
            libc::SCHED_BATCH => Ok(Scheduler::SCHED_BATCH),
            libc::SCHED_IDLE => Ok(Scheduler::SCHED_IDLE),
            libc::SCHED_DEADLINE => Ok(Scheduler::SCHED_DEADLINE),
            _ => Err(PreemptRtError::UnknownScheduler(value)),
        }
    }
}

impl Scheduler {
    /// Get the highest priority value for a given scheduler.
    pub fn priority_max(&self) -> Result<c_int> {
        let res = unsafe { libc::sched_get_priority_max(*self as c_int) };
        handle_errno(res)
    }

    /// Get the lowest priority value for a given scheduler.
    pub fn priority_min(&self) -> Result<c_int> {
        let res = unsafe { libc::sched_get_priority_min(*self as c_int) };
        handle_errno(res)
    }

    pub fn with_priority(self, priority: c_int) -> Result<ParameterizedScheduler> {
        let max = self.priority_max()?;
        let min = self.priority_min()?;
        if priority > max {
            Err(PriorityAboveMax(priority, max).into())
        } else if priority < min {
            Err(PriorityBelowMin(priority, min).into())
        } else {
            Ok(ParameterizedScheduler {
                scheduler: self,
                params: SchedulerParams { priority },
            })
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
/// Schedule parameters for a thread. Priority is the only supported parameter by the kernel
/// at the moment. This is a wrapper around `libc::sched_param`
pub struct SchedulerParams {
    /// Priority of the current schedule.
    pub priority: c_int,
}

impl From<SchedulerParams> for libc::sched_param {
    #[cfg(not(any(target_env = "musl", target_env = "ohos")))]
    fn from(param: SchedulerParams) -> Self {
        libc::sched_param {
            sched_priority: param.priority,
        }
    }

    #[cfg(any(target_env = "musl", target_env = "ohos"))]
    fn from(param: SchedulerParams) -> Self {
        let ts_zero = libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        // musl and ohos have additional fields for SCHED_DEADLINE - this is not abstracted
        // in this library yet.
        libc::sched_param {
            sched_priority: param.priority,
            sched_ss_init_budget: ts_zero.clone(),
            sched_ss_low_priority: 0,
            sched_ss_repl_period: ts_zero.clone(),
            sched_ss_max_repl: 0,
        }
    }
}

impl From<libc::sched_param> for SchedulerParams {
    fn from(param: libc::sched_param) -> Self {
        SchedulerParams {
            priority: param.sched_priority,
        }
    }
}

pub trait IntoSchedParam {
    fn into_sched_param(self) -> SchedulerParams;
}

impl IntoSchedParam for i32 {
    fn into_sched_param(self) -> SchedulerParams {
        SchedulerParams {
            priority: self as c_int,
        }
    }
}

impl<T: IntoSchedParam> IntoSchedParam for Option<T> {
    fn into_sched_param(self) -> SchedulerParams {
        match self {
            None => SchedulerParams { priority: 0 },
            Some(param) => param.into_sched_param(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParameterizedScheduler {
    scheduler: Scheduler,
    params: SchedulerParams,
}

impl ParameterizedScheduler {
    pub fn set_on(self, pid: Pid) -> Result<()> {
        set_scheduler(pid, self.scheduler, self.params)
    }

    pub fn set_current(self) -> Result<()> {
        self.set_on(Pid::current_thread())
    }
}

/// Get the current scheduler in use for a given process or thread.
/// Using `Pid::from_raw(0)` will fetch the scheduler for the calling thread.
pub fn get_scheduler(pid: Pid) -> Result<Scheduler> {
    let res = unsafe { libc::sched_getscheduler(pid.into()) };
    handle_errno(res).and_then(Scheduler::try_from)
}

/// Set the scheduler and parameters for a given process or thread.
/// Using `Pid::from_raw(0)` will set the scheduler for the calling thread.
///
/// SCHED_OTHER, SCHED_IDLE and SCHED_BATCH only support a priority of `0`, and can be used
/// outside a Linux PREEMPT_RT context.
///
/// SCHED_FIFO and SCHED_RR allow priorities between the min and max inclusive.
///
/// SCHED_DEADLINE cannot be set with this function, `libc::sched_setattr` must be used instead.
pub fn set_scheduler(pid: Pid, scheduler: Scheduler, param: SchedulerParams) -> Result<()> {
    let param: libc::sched_param = param.into();
    let res = unsafe { libc::sched_setscheduler(pid.into(), scheduler as c_int, &param) };

    handle_errno(res).map(drop)
}

/// Get the schedule parameters (currently only priority) for a given thread.
/// Using `Pid::from_raw(0)` will return the parameters for the calling thread.
pub fn get_scheduler_params(pid: Pid) -> Result<SchedulerParams> {
    let mut param: MaybeUninit<libc::sched_param> = MaybeUninit::uninit();
    let res = unsafe { libc::sched_getparam(pid.into(), param.as_mut_ptr()) };

    handle_errno(res).map(|_| unsafe { param.assume_init() }.into())
}
/// Set the schedule parameters (currently only priority) for a given thread.
/// Using `Pid::from_raw(0)` will return the parameters for the calling thread.
///
/// Changing the priority to something other than `0` requires using a SCHED_FIFO or SCHED_RR
/// and using a Linux kernel with PREEMPT_RT enabled.
pub fn set_scheduler_params(pid: Pid, param: SchedulerParams) -> Result<()> {
    let param: libc::sched_param = param.into();
    let res = unsafe { libc::sched_setparam(pid.into(), &param) };
    handle_errno(res).map(drop)
}
