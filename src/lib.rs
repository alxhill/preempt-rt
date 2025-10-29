//! Rust friendly bindings to the scheduler related functionality in libc.
//!
//! The `sched` module contains types and lightweight wrappers around libc functions. For example:
//! ```rust,no_run,ignore-apple,ignore-windows
//! use preempt_rt::sched;
//! use preempt_rt::sched::{Pid, Scheduler, SchedulerParams};
//! let sched = sched::get_scheduler(Pid::current_thread()).unwrap();
//! sched::set_scheduler(Pid::current_thread(), Scheduler::SCHED_FIFO, SchedulerParams {
//!     priority: 80
//! }).expect("failed to set scheduler");
//! ```
//!
//! The `thread` module has wrappers around `thread::spawn` for creating threads with a given
//! scheduler and priority.
//!
//! ```rust,no_run,ignore-apple,ignore-windows
//! use preempt_rt::sched::{RtResult, Scheduler};
//! use preempt_rt::thread;
//! thread::spawn(Scheduler::SCHED_FIFO, 80, move || {
//!     println!("this code is running on a thread with fifo scheduler & priority of 80");
//! });
//! // setting scheduler requires linux + preempt_rt kernel + appropriate permissions so may fail
//! thread::try_spawn(Scheduler::SCHED_FIFO, 80, move |sched_result| {
//!     match sched_result {
//!         Ok(()) => {}
//!         Err(e) => eprintln!("failed to set scheduler: {e}")
//!     }
//! });
//! ```
//!
//! Only SCHED_FIFO and SCHED_RR are meaningfully supported at the moment - SCHED_DEADLINE
//! requires additional parameters to be set, but on most platforms the Rust libc bindings don't
//! allow setting the additional attributes necessary to make this work, and the
//! `libc::sched_setattr` function has to be used to change the values rather than
//! `libc::sched_setscheduler`.
//!
//! This crate also includes stub methods for `preempt_rt::thread::try_spawn` that compile on
//! Windows and macOS (but does not attempt to set a scheduler). This is useful for
//! building/running tests on non-linux platforms.
pub mod sched;
pub mod thread;
