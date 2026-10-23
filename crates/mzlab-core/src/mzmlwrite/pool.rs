//! The encoding pool of `MzmlWriter` (native only): the calling thread reads the spectra and writes the text in order, a group of `Opzioni::thread` threads
//! (`std::thread::scope`, no dependency) builds the text of each spectrum (zlib, numpress, base64). The spectra travel in blocks of up to `SPETTRI_BLOCCO`
//! (or `PUNTI_BLOCCO` points) and come back with the text, so the thread that allocated them frees them: a free on another thread contends for the
//! allocator's arena and made more threads slower than one. At most four times the threads' number of blocks are in flight, so the memory is that of the
//! reader plus a few blocks; the bytes written are those of `spectrum_with` whatever the number of threads.

use std::collections::BTreeMap;
use std::io::Write;
use std::sync::{mpsc, Mutex};

use super::{Codifica, MzmlWriter, SpectrumExtra};
use crate::error::{Error, Result};
use crate::mzml::Spectrum;

/// Spectra per message at most; the block also closes at `PUNTI_BLOCCO` points, so a dense spectrum travels alone and the memory in flight stays that of a few of them.
const SPETTRI_BLOCCO: usize = 32;
const PUNTI_BLOCCO: usize = 100_000;

type Voce = (Spectrum, SpectrumExtra);

impl<W: Write> MzmlWriter<W> {
    /// Writes the spectra of `it` in order with `Opzioni::thread` encoding threads; the same bytes as `spectrum_with` on each. The first error in the order of the
    /// spectra (the reader's or an encoder's) is returned and nothing after it is written: the file stays open, never closed as valid.
    pub fn spettri<I>(&mut self, it: I) -> Result<()>
    where
        I: Iterator<Item = Result<(Spectrum, SpectrumExtra)>>,
    {
        self.spettri_con(it, &|enc, k, sp, extra, s| enc.spettro(k, sp, extra, s))
    }

    /// `spettri` with the encoder as a parameter, so that a test can make one spectrum fail.
    pub(super) fn spettri_con<I, F>(&mut self, mut it: I, codifica: &F) -> Result<()>
    where
        I: Iterator<Item = Result<(Spectrum, SpectrumExtra)>>,
        F: Fn(&mut Codifica, usize, &Spectrum, &SpectrumExtra, &mut String) -> Result<()> + Sync,
    {
        let n = self.opz.thread.max(1);
        let (opz, base, finestra) = (self.opz, self.spectra, 4 * n);
        let (tx_lavoro, rx_lavoro) = mpsc::channel::<(usize, Vec<Voce>)>();
        let (tx_pronti, rx_pronti) = mpsc::channel::<Pronto>();
        let rx_lavoro = Mutex::new(rx_lavoro);
        std::thread::scope(|sc| {
            for _ in 0..n {
                let (tx, rx) = (tx_pronti.clone(), &rx_lavoro);
                sc.spawn(move || {
                    let mut enc = Codifica::new(opz);
                    loop {
                        // the lock is held only while waiting for the next block
                        let voce = rx.lock().unwrap_or_else(|e| e.into_inner()).recv();
                        let Ok((primo, blocco)) = voce else { break };
                        if tx
                            .send(codifica_blocco(&mut enc, base, primo, blocco, codifica))
                            .is_err()
                        {
                            break;
                        }
                    }
                });
            }
            drop(tx_pronti);
            // `dati` spectra sent in `lotti` blocks, `scritti` blocks written; `errore_lettura` is the reader's error, kept until the spectra before it are written
            let (mut dati, mut lotti, mut scritti, mut errore_lettura, mut finito) =
                (0usize, 0usize, 0usize, None, false);
            let mut attesa: BTreeMap<usize, Pronto> = BTreeMap::new();
            // an inner closure so that every way out, an error included, drops `tx_lavoro` and lets the threads end
            let esito = (|| -> Result<()> {
                loop {
                    while !finito && lotti - scritti < finestra {
                        let blocco = leggi_blocco(&mut it, &mut finito, &mut errore_lettura);
                        if blocco.is_empty() {
                            continue;
                        }
                        let primo = dati;
                        dati += blocco.len();
                        lotti += 1;
                        tx_lavoro
                            .send((primo, blocco))
                            .map_err(|_| Error::new("err.file.mzmlWrite"))?;
                    }
                    if scritti == lotti {
                        return Ok(());
                    }
                    let pronto = rx_pronti
                        .recv()
                        .map_err(|_| Error::new("err.file.mzmlWrite"))?;
                    attesa.insert(pronto.primo, pronto);
                    while let Ok(pronto) = rx_pronti.try_recv() {
                        attesa.insert(pronto.primo, pronto);
                    }
                    // the blocks come back in any order; they are written in the order of their first spectrum
                    while let Some(pronto) = attesa.remove(&self.spectra.saturating_sub(base)) {
                        self.scrivi_blocco(&pronto)?;
                        scritti += 1;
                    }
                }
            })();
            drop(tx_lavoro);
            esito.and(errore_lettura.map_or(Ok(()), Err))
        })
    }

    /// The spectra of one encoded block in order, then the error that stopped it, if any.
    fn scrivi_blocco(&mut self, pronto: &Pronto) -> Result<()> {
        let mut da = 0;
        for (id, a) in &pronto.fine {
            self.spectra += 1;
            self.mark(true, id);
            self.put(&pronto.testo.as_bytes()[da..*a])?;
            da = *a;
        }
        pronto.errore.as_ref().map_or(Ok(()), |e| Err(e.clone()))
    }
}

/// What a thread sends back for a block: its first position, the text of the spectra that were encoded (one string, `fine` the end of each with its id) and the
/// error of the one that failed, whose text is not in `testo`.
struct Pronto {
    primo: usize,
    testo: String,
    fine: Vec<(String, usize)>,
    errore: Option<Error>,
    /// The spectra of the block, consumed: the writer thread drops them, so that the memory the reader allocated is freed by the thread that allocated it.
    usati: Vec<Voce>,
}

/// The text of the spectra of `blocco` (positions `base + primo` on) in one string; it stops at the first encoding error.
fn codifica_blocco<F>(
    enc: &mut Codifica,
    base: usize,
    primo: usize,
    mut blocco: Vec<Voce>,
    codifica: &F,
) -> Pronto
where
    F: Fn(&mut Codifica, usize, &Spectrum, &SpectrumExtra, &mut String) -> Result<()>,
{
    let mut pronto = Pronto {
        primo,
        testo: String::new(),
        fine: Vec::with_capacity(blocco.len()),
        errore: None,
        usati: Vec::new(),
    };
    for (i, (sp, extra)) in blocco.iter_mut().enumerate() {
        let prima = pronto.testo.len();
        if let Err(e) = codifica(enc, base + primo + i, sp, extra, &mut pronto.testo) {
            pronto.testo.truncate(prima);
            pronto.errore = Some(e);
            break;
        }
        pronto
            .fine
            .push((std::mem::take(&mut sp.id), pronto.testo.len()));
    }
    pronto.usati = blocco;
    pronto
}

/// The next block of spectra from `it`: up to `SPETTRI_BLOCCO` of them or `PUNTI_BLOCCO` points. The end of the input or the reader's error stops it and is
/// recorded in `finito` / `errore_lettura`, to be handled after the spectra before it.
fn leggi_blocco<I>(it: &mut I, finito: &mut bool, errore_lettura: &mut Option<Error>) -> Vec<Voce>
where
    I: Iterator<Item = Result<Voce>>,
{
    let (mut blocco, mut punti) = (Vec::new(), 0usize);
    while blocco.len() < SPETTRI_BLOCCO && punti < PUNTI_BLOCCO {
        match it.next() {
            None => *finito = true,
            Some(Err(e)) => {
                *errore_lettura = Some(e);
                *finito = true;
            }
            Some(Ok((sp, extra))) => {
                punti += sp.mz.len();
                blocco.push((sp, extra));
                continue;
            }
        }
        break;
    }
    blocco
}
