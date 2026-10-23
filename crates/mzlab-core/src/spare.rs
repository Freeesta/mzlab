//! Cores that no thread of the program is using, for the passes that can split their own work in two (`join2`). Native only: the
//! module in the browser has one thread. The count is a hint, never a lock: a pass that finds no spare core runs its halves one after the other.

#[cfg(not(target_family = "wasm"))]
mod imp {
    use std::sync::atomic::{AtomicIsize, Ordering};
    use std::sync::OnceLock;

    fn free() -> &'static AtomicIsize {
        static FREE: OnceLock<AtomicIsize> = OnceLock::new();
        FREE.get_or_init(|| {
            AtomicIsize::new(std::thread::available_parallelism().map_or(1, |n| n.get()) as isize)
        })
    }

    /// A core in use by the calling thread until the guard drops.
    pub struct Busy(isize);

    impl Drop for Busy {
        fn drop(&mut self) {
            free().fetch_add(self.0, Ordering::Relaxed);
        }
    }

    /// Marks `n` cores as in use (a thread of a pool that runs a job); they come back when the guard drops.
    pub fn busy(n: usize) -> Busy {
        free().fetch_sub(n as isize, Ordering::Relaxed);
        Busy(n as isize)
    }

    /// Up to `max` spare cores, taken together; they come back when the lease drops.
    pub struct Lease(Vec<Busy>);

    impl Lease {
        pub fn count(&self) -> usize {
            self.0.len()
        }
    }

    pub fn lease(max: usize) -> Lease {
        Lease((0..max).map_while(|_| take()).collect())
    }

    /// Takes one spare core if there is one.
    fn take() -> Option<Busy> {
        let f = free();
        let mut now = f.load(Ordering::Relaxed);
        while now > 0 {
            match f.compare_exchange_weak(now, now - 1, Ordering::Relaxed, Ordering::Relaxed) {
                Ok(_) => return Some(Busy(1)),
                Err(seen) => now = seen,
            }
        }
        None
    }

    /// `(a(), b())`, `b` on another thread when a core is spare.
    pub fn join2<A: Send, B: Send>(
        a: impl FnOnce() -> A + Send,
        b: impl FnOnce() -> B + Send,
    ) -> (A, B) {
        let Some(lease) = take() else {
            return (a(), b());
        };
        let out = std::thread::scope(|s| {
            let h = s.spawn(b);
            let ra = a();
            (ra, h.join())
        });
        drop(lease);
        match out {
            (ra, Ok(rb)) => (ra, rb),
            (_, Err(panic)) => std::panic::resume_unwind(panic),
        }
    }
}

#[cfg(target_family = "wasm")]
mod imp {
    pub struct Busy;

    pub fn busy(_: usize) -> Busy {
        Busy
    }

    pub struct Lease;

    impl Lease {
        pub fn count(&self) -> usize {
            0
        }
    }

    pub fn lease(_: usize) -> Lease {
        Lease
    }

    pub fn join2<A, B>(a: impl FnOnce() -> A, b: impl FnOnce() -> B) -> (A, B) {
        (a(), b())
    }
}

pub use imp::{busy, join2, lease, Busy, Lease};
