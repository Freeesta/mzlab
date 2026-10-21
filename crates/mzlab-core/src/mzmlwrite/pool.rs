//! The encoding pool of `MzmlWriter` (native only): the calling thread reads the spectra one by one and writes the text in order, a group of `Opzioni::thread`
//! threads (`std::thread::scope`, no dependency) builds the text of each spectrum (zlib, numpress, base64). At most four times the threads' number of spectra are
//! in flight, so the memory is that of the reader plus a few spectra; the bytes written are those of `spectrum_with` whatever the number of threads.

use std::collections::BTreeMap;
use std::io::Write;
use std::sync::{mpsc, Mutex};

use super::{Codifica, MzmlWriter, SpectrumExtra};
use crate::error::{Error, Result};
use crate::mzml::Spectrum;

/// What a thread sends back: the position in the sequence, the id of the spectrum and its text.
type Pronto = (usize, String, Result<String>);

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
        let (tx_lavoro, rx_lavoro) = mpsc::channel::<(usize, Spectrum, SpectrumExtra)>();
        let (tx_pronti, rx_pronti) = mpsc::channel::<Pronto>();
        let rx_lavoro = Mutex::new(rx_lavoro);
        std::thread::scope(|sc| {
            for _ in 0..n {
                let (tx, rx) = (tx_pronti.clone(), &rx_lavoro);
                sc.spawn(move || {
                    let mut enc = Codifica::new(opz);
                    loop {
                        let voce = rx.lock().unwrap_or_else(|e| e.into_inner()).recv();
                        let Ok((k, sp, extra)) = voce else { break };
                        let mut s = String::new();
                        let r = codifica(&mut enc, base + k, &sp, &extra, &mut s).map(|()| s);
                        if tx.send((k, sp.id, r)).is_err() {
                            break;
                        }
                    }
                });
            }
            drop(tx_pronti);
            // `dati` sent, `scritti` written; `errore_lettura` is the reader's error, kept until the spectra before it are written
            let (mut dati, mut scritti, mut errore_lettura, mut finito) =
                (0usize, 0usize, None, false);
            let mut attesa: BTreeMap<usize, (String, Result<String>)> = BTreeMap::new();
            // an inner closure so that every way out, an error included, drops `tx_lavoro` and lets the threads end
            let esito = (|| -> Result<()> {
                loop {
                    while !finito && dati - scritti < finestra {
                        match it.next() {
                            None => finito = true,
                            Some(Err(e)) => {
                                errore_lettura = Some(e);
                                finito = true;
                            }
                            Some(Ok((sp, extra))) => {
                                tx_lavoro
                                    .send((dati, sp, extra))
                                    .map_err(|_| Error::new("err.file.mzmlWrite"))?;
                                dati += 1;
                            }
                        }
                    }
                    if scritti == dati {
                        return Ok(());
                    }
                    let (k, id, r) = rx_pronti
                        .recv()
                        .map_err(|_| Error::new("err.file.mzmlWrite"))?;
                    attesa.insert(k, (id, r));
                    while let Ok((k, id, r)) = rx_pronti.try_recv() {
                        attesa.insert(k, (id, r));
                    }
                    while let Some((id, r)) = attesa.remove(&scritti) {
                        let testo = r?;
                        self.spectra += 1;
                        self.mark(true, &id);
                        self.put(testo.as_bytes())?;
                        scritti += 1;
                    }
                }
            })();
            drop(tx_lavoro);
            esito.and(errore_lettura.map_or(Ok(()), Err))
        })
    }
}
