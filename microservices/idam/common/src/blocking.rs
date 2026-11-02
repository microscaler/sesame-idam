//! Blocking work kept off the may request workers.
//!
//! A may worker runs many request coroutines; a blocking call on one (a std-socket Redis round trip, an argon2id hash)
//! holds that worker and every coroutine queued on it until the call returns. [`io`] and [`cpu`] hand the work to a
//! small set of OS threads and park only the calling coroutine until the result comes back, so the worker keeps
//! serving other requests meanwhile.
//!
//! - [`io`]: blocking network I/O (the pooled Redis connections). `SESAME_BLOCKING_IO_THREADS`, default 8.
//! - [`cpu`]: CPU-bound work (argon2id verification and hashing). `SESAME_BLOCKING_CPU_THREADS`, default 2: this also
//!   bounds how many argon2id computations (19 MiB each at the stored m=19456) run at once in a pod.
//!
//! Outside a coroutine (tests, start-up, plain threads) both run the closure inline. A panic in the closure is
//! re-raised in the caller; the pool thread survives it.

use std::panic::{self, AssertUnwindSafe};
use std::sync::mpsc;
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;

type Job = Box<dyn FnOnce() + Send + 'static>;

const DEFAULT_IO_THREADS: usize = 8;
const DEFAULT_CPU_THREADS: usize = 2;

/// A fixed set of named OS threads taking jobs from one queue.
pub struct Pool {
    tx: mpsc::Sender<Job>,
}

impl Pool {
    /// Start `threads` (at least one) threads named `{name}-{i}`.
    ///
    /// # Panics
    ///
    /// Panics if the OS refuses to start a thread.
    #[must_use]
    pub fn new(name: &str, threads: usize) -> Self {
        let (tx, rx) = mpsc::channel::<Job>();
        let rx = Arc::new(Mutex::new(rx));
        for i in 0..threads.max(1) {
            let rx = Arc::clone(&rx);
            thread::Builder::new()
                .name(format!("{name}-{i}"))
                .spawn(move || loop {
                    let job = match rx.lock() {
                        Ok(queue) => queue.recv(),
                        Err(_) => return,
                    };
                    match job {
                        Ok(job) => job(),
                        Err(_) => return,
                    }
                })
                .unwrap_or_else(|e| panic!("failed to start blocking thread {name}-{i}: {e}"));
        }
        Self { tx }
    }

    /// Run `f` on a pool thread and wait for its result: a coroutine parks (its worker stays free), a plain thread
    /// blocks.
    pub fn run<T, F>(&self, f: F) -> T
    where
        T: Send + 'static,
        F: FnOnce() -> T + Send + 'static,
    {
        let (done_tx, done_rx) = may::sync::mpsc::channel();
        let job: Job = Box::new(move || {
            let _ = done_tx.send(panic::catch_unwind(AssertUnwindSafe(f)));
        });
        if let Err(mpsc::SendError(job)) = self.tx.send(job) {
            // No pool thread left (cannot happen while the process lives): do it here rather than not at all.
            job();
        }
        match done_rx.recv() {
            Ok(Ok(value)) => value,
            Ok(Err(panic_payload)) => panic::resume_unwind(panic_payload),
            Err(_) => unreachable!("a blocking job always reports back"),
        }
    }
}

fn threads_from_env(var: &str, default: usize) -> usize {
    std::env::var(var)
        .ok()
        .and_then(|v| v.trim().parse::<usize>().ok())
        .filter(|&n| n > 0)
        .unwrap_or(default)
}

fn io_pool() -> &'static Pool {
    static POOL: OnceLock<Pool> = OnceLock::new();
    POOL.get_or_init(|| {
        Pool::new(
            "sesame-io",
            threads_from_env("SESAME_BLOCKING_IO_THREADS", DEFAULT_IO_THREADS),
        )
    })
}

fn cpu_pool() -> &'static Pool {
    static POOL: OnceLock<Pool> = OnceLock::new();
    POOL.get_or_init(|| {
        Pool::new(
            "sesame-cpu",
            threads_from_env("SESAME_BLOCKING_CPU_THREADS", DEFAULT_CPU_THREADS),
        )
    })
}

/// Blocking network I/O: on the I/O threads when called from a coroutine, inline otherwise.
pub fn io<T, F>(f: F) -> T
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    if may::coroutine::is_coroutine() {
        io_pool().run(f)
    } else {
        f()
    }
}

/// CPU-bound work: on the CPU threads when called from a coroutine, inline otherwise.
pub fn cpu<T, F>(f: F) -> T
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    if may::coroutine::is_coroutine() {
        cpu_pool().run(f)
    } else {
        f()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    #[test]
    fn outside_a_coroutine_runs_inline() {
        let here = thread::current().id();
        assert!(io(move || thread::current().id() == here));
        assert!(cpu(move || thread::current().id() == here));
    }

    #[test]
    fn inside_a_coroutine_runs_on_a_pool_thread() {
        let name = may::go!(|| io(|| thread::current().name().map(str::to_string)))
            .join()
            .unwrap();
        assert!(name.unwrap_or_default().starts_with("sesame-io-"));
        let name = may::go!(|| cpu(|| thread::current().name().map(str::to_string)))
            .join()
            .unwrap();
        assert!(name.unwrap_or_default().starts_with("sesame-cpu-"));
    }

    #[test]
    fn the_pool_bounds_concurrency() {
        let pool: &'static Pool = Box::leak(Box::new(Pool::new("test-bound", 2)));
        let running: &'static AtomicUsize = Box::leak(Box::new(AtomicUsize::new(0)));
        let peak: &'static AtomicUsize = Box::leak(Box::new(AtomicUsize::new(0)));
        let handles: Vec<_> = (0..8)
            .map(|_| {
                may::go!(move || pool.run(move || {
                    let now = running.fetch_add(1, Ordering::SeqCst) + 1;
                    peak.fetch_max(now, Ordering::SeqCst);
                    thread::sleep(Duration::from_millis(20));
                    running.fetch_sub(1, Ordering::SeqCst);
                }))
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }
        assert!(peak.load(Ordering::SeqCst) <= 2);
    }

    #[test]
    fn a_panic_reaches_the_caller_and_the_thread_survives() {
        let pool: &'static Pool = Box::leak(Box::new(Pool::new("test-panic", 1)));
        let caught = thread::spawn(move || {
            panic::catch_unwind(AssertUnwindSafe(|| pool.run(|| panic!("boom"))))
        })
        .join()
        .unwrap();
        assert!(caught.is_err());
        assert_eq!(pool.run(|| 7), 7);
    }
}
