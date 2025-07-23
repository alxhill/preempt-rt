# preempt-rt

preempt-rt is a lightweight wrapper around the PREEMPT_RT libc functions, providing a rust-like interface to
the underlying libc functions.

There are some simple helper functions that can be used directly:

```rust
fn main() {
    let scheduler = get_scheduler(Pid::current_thread()).expect("could not get scheduler");
    set_scheduler(Scheduler::SCHED_FIFO, 50).expect("could not set scheduler to fifo with priority 50");
}
```

There are also lightweight wrappers for spawning threads with a given scheduler and priority. Because the FIFO, RR, and
DEADLINE schedulers will only work on a Linux kernel compiled with PREEMPT_RT, a fallible option is provided too
(useful when testing code on a non-rt host).

```rust
fn main() {
    preempt_rt::thread::spawn(Scheduler::SCHED_FIFO, 50, move || {
        println!("hello from an rt thread");
    });

    preempt_rt::thread::try_spawn(Scheduler::SCHED_FIFO, 50, move |sched_result| {
        match sched_result {
            Ok(()) => println!("hello from an rt thread"),
            Err(err) => eprintln!("failed to set thread scheduler: {err}")
        }
    });
}
```

Although SCHED_DEADLINE is included in the Scheduler enum, support for setting the deadline parameters is not currently
implemented in this library.
