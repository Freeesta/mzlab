//! Decoding of the arrays of an mzML on spare cores: the parser thread cuts the file into spectra and hands each one to a helper, then gives the
//! results to the caller in file order (a reorder queue), so `parse` calls its callback with the same items in the same order as without helpers.

use super::{finish_chromatogram, finish_spectrum, Chrom, Item, Mobility, Spec, Spectrum};
use crate::error::{Error, Result};
use crate::mzmlarray::Inflate;
use std::collections::VecDeque;
use std::io::Read;
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::sync::{Arc, Mutex};

/// How `parse` turns the pieces of the file into items.
pub(super) trait Decode {
    fn spectrum(&mut self, s: Spec, f: &mut dyn FnMut(Item)) -> Result<()>;
    fn chromatogram(&mut self, c: Chrom, f: &mut dyn FnMut(Item)) -> Result<()>;
    /// An item that needs no decoding, kept in its place in the order.
    fn item(&mut self, it: Item, f: &mut dyn FnMut(Item)) -> Result<()>;
    /// Everything handed so far has gone to `f`.
    fn finish(&mut self, f: &mut dyn FnMut(Item)) -> Result<()>;
}

/// One thread: the spectrum is decoded where it is cut.
pub(super) struct Plain(pub(super) Inflate);

fn deliver(sp: (Spectrum, Option<Mobility>), f: &mut dyn FnMut(Item)) {
    if let Some(m) = sp.1 {
        f(Item::Mobility(m));
    }
    f(Item::Spectrum(sp.0));
}

impl Decode for Plain {
    fn spectrum(&mut self, s: Spec, f: &mut dyn FnMut(Item)) -> Result<()> {
        deliver(finish_spectrum(s, &mut self.0)?, f);
        Ok(())
    }

    fn chromatogram(&mut self, c: Chrom, f: &mut dyn FnMut(Item)) -> Result<()> {
        f(Item::Chromatogram(finish_chromatogram(c, &mut self.0)?));
        Ok(())
    }

    fn item(&mut self, it: Item, f: &mut dyn FnMut(Item)) -> Result<()> {
        f(it);
        Ok(())
    }

    fn finish(&mut self, _: &mut dyn FnMut(Item)) -> Result<()> {
        Ok(())
    }
}

type Done = (u64, Result<Vec<Item>>);

/// The parser side of the helpers: jobs out, results in, the queue that puts them back in order.
pub(super) struct Helped {
    jobs: Sender<(u64, Spec)>,
    results: Receiver<Done>,
    /// The inflater of the pieces decoded on the parser thread (chromatograms).
    inf: Inflate,
    /// One entry per item handed, from sequence number `first`; `None` while a helper has it.
    queue: VecDeque<Option<Result<Vec<Item>>>>,
    first: u64,
    waiting: usize,
    limit: usize,
    /// An error has gone out: nothing after it is handed to `f`, as when one thread stops at the error.
    dead: bool,
}

/// The job queue the helpers share.
pub(super) type Jobs = Arc<Mutex<Receiver<(u64, Spec)>>>;

/// Body of a helper: decodes the spectra it is given until the parser drops its end.
pub(super) fn helper(jobs: &Jobs, out: &Sender<Done>) {
    let mut inf = Inflate::new();
    loop {
        let job = jobs.lock().unwrap_or_else(|e| e.into_inner()).recv();
        let Ok((seq, spec)) = job else { return };
        let r = ready_items(finish_spectrum(spec, &mut inf));
        if out.send((seq, r)).is_err() {
            return;
        }
    }
}

impl Helped {
    pub(super) fn new(jobs: Sender<(u64, Spec)>, results: Receiver<Done>, helpers: usize) -> Self {
        Helped {
            jobs,
            results,
            inf: Inflate::new(),
            queue: VecDeque::new(),
            first: 0,
            waiting: 0,
            limit: 4 * helpers.max(1),
            dead: false,
        }
    }

    fn place(&mut self, (seq, r): Done) {
        let at = usize::try_from(seq - self.first).unwrap_or(usize::MAX);
        if let Some(slot) = self.queue.get_mut(at) {
            *slot = Some(r);
            self.waiting -= 1;
        }
    }

    /// Hands to `f` the items at the front of the queue that are ready.
    fn drain(&mut self, f: &mut dyn FnMut(Item)) -> Result<()> {
        while let Some(Some(_)) = self.queue.front() {
            self.first += 1;
            if let Some(Some(r)) = self.queue.pop_front() {
                match r {
                    Ok(items) => items.into_iter().for_each(&mut *f),
                    Err(e) => {
                        self.dead = true;
                        self.queue.clear();
                        self.waiting = 0;
                        return Err(e);
                    }
                }
            }
        }
        Ok(())
    }
}

fn disconnected(e: TryRecvError) -> bool {
    e == TryRecvError::Disconnected
}

impl Helped {
    /// Takes the results that are in (waiting for one when `block`), and turns what no helper will ever answer into errors.
    fn collect(&mut self, block: bool) {
        loop {
            let got = if block && self.waiting > 0 {
                self.results.recv().map_err(|_| true)
            } else {
                self.results.try_recv().map_err(disconnected)
            };
            match got {
                Ok(d) => {
                    self.place(d);
                    if block {
                        return;
                    }
                }
                Err(closed) => {
                    if closed {
                        self.fail_waiting();
                    }
                    return;
                }
            }
        }
    }

    /// No helper is left (one stopped): the pieces still waiting are lost.
    fn fail_waiting(&mut self) {
        for slot in self.queue.iter_mut().filter(|s| s.is_none()) {
            *slot = Some(Err(Error::memory_budget()));
        }
        self.waiting = 0;
    }

    fn push_ready(&mut self, r: Result<Vec<Item>>, f: &mut dyn FnMut(Item)) -> Result<()> {
        self.queue.push_back(Some(r));
        self.collect(false);
        self.drain(f)
    }
}

/// The items of a decoded spectrum: its mobility (when it has one) and then the spectrum.
fn ready_items(r: Result<(Spectrum, Option<Mobility>)>) -> Result<Vec<Item>> {
    r.map(|(sp, m)| {
        m.map(Item::Mobility)
            .into_iter()
            .chain([Item::Spectrum(sp)])
            .collect()
    })
}

impl Decode for Helped {
    fn spectrum(&mut self, s: Spec, f: &mut dyn FnMut(Item)) -> Result<()> {
        let seq = self.first + self.queue.len() as u64;
        self.queue.push_back(None);
        self.waiting += 1;
        if let Err(e) = self.jobs.send((seq, s)) {
            // no helper is left to take it: decode it here
            self.queue.pop_back();
            self.waiting -= 1;
            let r = ready_items(finish_spectrum(e.0 .1, &mut self.inf));
            return self.push_ready(r, f);
        }
        self.collect(false);
        while self.waiting >= self.limit {
            self.collect(true);
        }
        self.drain(f)
    }

    fn chromatogram(&mut self, c: Chrom, f: &mut dyn FnMut(Item)) -> Result<()> {
        let r = finish_chromatogram(c, &mut self.inf).map(|c| vec![Item::Chromatogram(c)]);
        self.push_ready(r, f)
    }

    fn item(&mut self, it: Item, f: &mut dyn FnMut(Item)) -> Result<()> {
        self.push_ready(Ok(vec![it]), f)
    }

    fn finish(&mut self, f: &mut dyn FnMut(Item)) -> Result<()> {
        if self.dead {
            return Ok(());
        }
        while self.waiting > 0 {
            self.collect(true);
        }
        self.drain(f)
    }
}

/// `parse` with `helpers` threads decoding the arrays (none: on the calling thread).
pub(crate) fn parse_helped<R: Read>(
    reader: R,
    f: &mut dyn FnMut(Item),
    helpers: usize,
) -> Result<()> {
    if helpers == 0 {
        return parse_with(reader, f, &mut Plain(Inflate::new()));
    }
    let (jobs, queue) = std::sync::mpsc::channel();
    let (out, results) = std::sync::mpsc::channel();
    let queue = std::sync::Arc::new(std::sync::Mutex::new(queue));
    std::thread::scope(|s| {
        for _ in 0..helpers {
            let (queue, out) = (queue.clone(), out.clone());
            s.spawn(move || helper(&queue, &out));
        }
        drop(out);
        let mut helped = Helped::new(jobs, results, helpers);
        // the helpers stop when `helped` drops the job end
        parse_with(reader, f, &mut helped)
    })
}

/// Helper threads at most: the arrays are decoded about as fast as the file is cut into spectra with a few of them.
pub(super) const HELPERS: usize = 3;

/// `parse` with the decoding done by `out`.
fn parse_with<R: Read, D: Decode>(reader: R, f: &mut dyn FnMut(Item), out: &mut D) -> Result<()> {
    let r = super::parse_loop(reader, f, out);
    // what is in flight goes to `f` before an error is told, as without helpers
    out.finish(f).and(r)
}
