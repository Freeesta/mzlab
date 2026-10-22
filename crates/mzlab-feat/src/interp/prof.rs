//! Sub-phase timers of the interpretation pass, summed by name and printed on stderr when `MZLAB_PHASES` is set (for profiling real runs).
//! Native builds only: the `.wasm` has no clock, there `time` just runs the closure. `build` uses it.

#[cfg(not(target_arch = "wasm32"))]
mod imp {
    use std::sync::{Mutex, OnceLock};
    use std::time::{Duration, Instant};

    static TOTALS: Mutex<Vec<(&'static str, Duration)>> = Mutex::new(Vec::new());

    fn on() -> bool {
        static ON: OnceLock<bool> = OnceLock::new();
        *ON.get_or_init(|| std::env::var_os("MZLAB_PHASES").is_some())
    }

    pub fn time<T>(name: &'static str, f: impl FnOnce() -> T) -> T {
        if !on() {
            return f();
        }
        let t = Instant::now();
        let out = f();
        let d = t.elapsed();
        if let Ok(mut v) = TOTALS.lock() {
            match v.iter_mut().find(|e| e.0 == name) {
                Some(e) => e.1 += d,
                None => v.push((name, d)),
            }
        }
        out
    }

    pub fn report() {
        if !on() {
            return;
        }
        if let Ok(mut v) = TOTALS.lock() {
            for (name, d) in v.drain(..) {
                eprintln!("interp phase {name}: {:.2} s", d.as_secs_f64());
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod imp {
    pub fn time<T>(_name: &'static str, f: impl FnOnce() -> T) -> T {
        f()
    }

    pub fn report() {}
}

pub use imp::{report, time};
