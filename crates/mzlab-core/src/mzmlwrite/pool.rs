//! The encoding pool of `MzmlWriter` (native only): the calling thread reads the spectra and writes the text in order, a group of `Opzioni::thread` threads
//! (`std::thread::scope`, no dependency) builds the text of each spectrum (zlib, numpress, base64). The spectra travel in blocks of up to `BLOCK_SPECTRA`
//! (or `BLOCK_POINTS` points) and come back with the text, so the thread that allocated them frees them: a free on another thread contends for the
//! allocator's arena and made more threads slower than one. At most four times the threads' number of blocks are in flight, so the memory is that of the
//! reader plus a few blocks; the bytes written are those of `spectrum_with` whatever the number of threads.

use std::collections::BTreeMap;
use std::io::Write;
use std::sync::{mpsc, Mutex};

use super::{Encoding, MzmlWriter, SpectrumExtra};
use crate::error::{Error, Result};
use crate::mzml::Spectrum;

/// Spectra per message at most; the block also closes at `BLOCK_POINTS` points, so a dense spectrum travels alone and the memory in flight stays that of a few of them.
const BLOCK_SPECTRA: usize = 32;
const BLOCK_POINTS: usize = 100_000;

type Entry = (Spectrum, SpectrumExtra);

impl<W: Write> MzmlWriter<W> {
    /// Writes the spectra of `it` in order with `Opzioni::thread` encoding threads; the same bytes as `spectrum_with` on each. The first error in the order of the
    /// spectra (the reader's or an encoder's) is returned and nothing after it is written: the file stays open, never closed as valid.
    pub fn spettri<I>(&mut self, it: I) -> Result<()>
    where
        I: Iterator<Item = Result<(Spectrum, SpectrumExtra)>>,
    {
        self.spectra_with(it, &|enc, k, sp, extra, s| enc.spettro(k, sp, extra, s))
    }

    /// `spettri` with the encoder as a parameter, so that a test can make one spectrum fail.
    pub(super) fn spectra_with<I, F>(&mut self, mut it: I, encoding: &F) -> Result<()>
    where
        I: Iterator<Item = Result<(Spectrum, SpectrumExtra)>>,
        F: Fn(&mut Encoding, usize, &Spectrum, &SpectrumExtra, &mut String) -> Result<()> + Sync,
    {
        let n = self.opz.thread.max(1);
        let (opz, base, window) = (self.opz, self.spectra, 4 * n);
        let (tx_work, rx_work) = mpsc::channel::<(usize, Vec<Entry>)>();
        let (tx_ready, rx_ready) = mpsc::channel::<Ready>();
        let rx_lavoro = Mutex::new(rx_work);
        std::thread::scope(|sc| {
            for _ in 0..n {
                let (tx, rx) = (tx_ready.clone(), &rx_lavoro);
                sc.spawn(move || {
                    let mut enc = Encoding::new(opz);
                    // the text of the last block, to start the next one with room for it (a string that starts empty doubles a dozen times)
                    let mut capacita = 0usize;
                    loop {
                        // the lock is held only while waiting for the next block
                        let entry = rx.lock().unwrap_or_else(|e| e.into_inner()).recv();
                        let Ok((primo, blocco)) = entry else { break };
                        let ready = encode_block(&mut enc, base, primo, blocco, encoding, capacita);
                        capacita = ready.text.len() + ready.text.len() / 8;
                        if tx.send(ready).is_err() {
                            break;
                        }
                    }
                });
            }
            drop(tx_ready);
            // `dati` spectra sent in `lotti` blocks, `scritti` blocks written; `read_error` is the reader's error, kept until the spectra before it are written
            let (mut data, mut lotti, mut scritti, mut read_error, mut finished) =
                (0usize, 0usize, 0usize, None, false);
            let mut expectation: BTreeMap<usize, Ready> = BTreeMap::new();
            // an inner closure so that every way out, an error included, drops `tx_work` and lets the threads end
            let outcome = (|| -> Result<()> {
                loop {
                    while !finished && lotti - scritti < window {
                        let block = read_block(&mut it, &mut finished, &mut read_error);
                        if block.is_empty() {
                            continue;
                        }
                        let first = data;
                        data += block.len();
                        lotti += 1;
                        tx_work
                            .send((first, block))
                            .map_err(|_| Error::new("err.file.mzmlWrite"))?;
                    }
                    if scritti == lotti {
                        return Ok(());
                    }
                    let pronto = rx_ready
                        .recv()
                        .map_err(|_| Error::new("err.file.mzmlWrite"))?;
                    expectation.insert(pronto.primo, pronto);
                    while let Ok(pronto) = rx_ready.try_recv() {
                        expectation.insert(pronto.primo, pronto);
                    }
                    // the blocks come back in any order; they are written in the order of their first spectrum
                    while let Some(pronto) = expectation.remove(&self.spectra.saturating_sub(base))
                    {
                        self.write_block(&pronto)?;
                        scritti += 1;
                    }
                }
            })();
            drop(tx_work);
            outcome.and(read_error.map_or(Ok(()), Err))
        })
    }

    /// The spectra of one encoded block in order, then the error that stopped it, if any.
    fn write_block(&mut self, ready: &Ready) -> Result<()> {
        let mut da = 0;
        for (id, a) in &ready.fine {
            self.spectra += 1;
            self.mark(true, id);
            self.put(&ready.text.as_bytes()[da..*a])?;
            da = *a;
        }
        ready.error.as_ref().map_or(Ok(()), |e| Err(e.clone()))
    }
}

/// What a thread sends back for a block: its first position, the text of the spectra that were encoded (one string, `fine` the end of each with its id) and the
/// error of the one that failed, whose text is not in `testo`.
struct Ready {
    primo: usize,
    text: String,
    fine: Vec<(String, usize)>,
    error: Option<Error>,
    /// The spectra of the block, consumed: the writer thread drops them, so that the memory the reader allocated is freed by the thread that allocated it.
    used: Vec<Entry>,
}

/// The text of the spectra of `blocco` (positions `base + primo` on) in one string; it stops at the first encoding error.
fn encode_block<F>(
    enc: &mut Encoding,
    base: usize,
    first: usize,
    mut block: Vec<Entry>,
    encoding: &F,
    capacita: usize,
) -> Ready
where
    F: Fn(&mut Encoding, usize, &Spectrum, &SpectrumExtra, &mut String) -> Result<()>,
{
    let mut ready = Ready {
        primo: first,
        text: String::with_capacity(capacita),
        fine: Vec::with_capacity(block.len()),
        error: None,
        used: Vec::new(),
    };
    for (i, (sp, extra)) in block.iter_mut().enumerate() {
        let before = ready.text.len();
        if let Err(e) = encoding(enc, base + first + i, sp, extra, &mut ready.text) {
            ready.text.truncate(before);
            ready.error = Some(e);
            break;
        }
        ready
            .fine
            .push((std::mem::take(&mut sp.id), ready.text.len()));
    }
    ready.used = block;
    ready
}

/// The next block of spectra from `it`: up to `BLOCK_SPECTRA` of them or `BLOCK_POINTS` points. The end of the input or the reader's error stops it and is
/// recorded in `finito` / `read_error`, to be handled after the spectra before it.
fn read_block<I>(it: &mut I, finished: &mut bool, read_error: &mut Option<Error>) -> Vec<Entry>
where
    I: Iterator<Item = Result<Entry>>,
{
    let (mut block, mut points) = (Vec::new(), 0usize);
    while block.len() < BLOCK_SPECTRA && points < BLOCK_POINTS {
        match it.next() {
            None => *finished = true,
            Some(Err(e)) => {
                *read_error = Some(e);
                *finished = true;
            }
            Some(Ok((sp, extra))) => {
                points += sp.mz.len();
                block.push((sp, extra));
                continue;
            }
        }
        break;
    }
    block
}
