//! Progress of an alignment: `cb(done, total)` after every slab, weak-tier file and gap-filled file. The total is known once the slabs are cut
//! (before the first call) and the callback never changes the result.

pub(crate) struct Prog<'a> {
    cb: Option<&'a mut dyn FnMut(usize, usize)>,
    done: usize,
    total: usize,
}

impl<'a> Prog<'a> {
    pub(crate) fn off() -> Self {
        Prog {
            cb: None,
            done: 0,
            total: 0,
        }
    }

    pub(crate) fn new(cb: &'a mut dyn FnMut(usize, usize)) -> Self {
        Prog {
            cb: Some(cb),
            done: 0,
            total: 0,
        }
    }

    /// `n` more units of work to come; silent until `announce` or the first `tick`.
    pub(crate) fn add_total(&mut self, n: usize) {
        self.total += n;
    }

    /// Tells the caller the total, with nothing done yet.
    pub(crate) fn announce(&mut self) {
        self.tick(0);
    }

    /// `k` more units done.
    pub(crate) fn tick(&mut self, k: usize) {
        self.done = (self.done + k).min(self.total);
        if let Some(cb) = self.cb.as_mut() {
            cb(self.done, self.total);
        }
    }
}
