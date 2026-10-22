//! The native index build and the counters it reports its progress with: a reader that counts the scans it decodes (`Scans`) and a `Read` that counts bytes
//! (`Counted`). They forward everything and only tell a `Pace` (`mzlab_core::pace`) how far the job is, so the index made is the same bytes.

use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;

use mzlab_core::error::Result;
use mzlab_core::lettori::Limiti;
use mzlab_core::mzidx::build_mzml_spill;
use mzlab_core::upload::{impronte, indicizza_lettore, prepare};

use crate::{flow_set, io};

use mzlab_core::lettori::{Campione, Campo, Formato, Lettore, VoceCromatogramma, VoceScansione};
use mzlab_core::mzml::{Chromatogram, Spectrum};
use mzlab_core::mzmlwrite::{Header, SpectrumExtra};
use mzlab_core::pace::Pace;

/// The `.mzidx` of one file (an mzML, or the sample `label` of any dataset a reader takes) at `idx`.
pub(crate) fn build_index_of(path: &Path, label: &str, idx: &Path, budget: usize) -> Result<()> {
    build_index_with(path, label, idx, budget, &mut |_, _| {})
}

/// `build_index_of` telling `step(done, total)` how far pass 1 has got (scans of the sample, bytes of an mzML), at most once per 1 %; same bytes with any `step`.
pub(crate) fn build_index_with(
    path: &Path,
    label: &str,
    idx: &Path,
    budget: usize,
    step: &mut dyn FnMut(usize, usize),
) -> Result<()> {
    let mut pace = mzlab_core::pace::Pace::new(step);
    let name = path
        .file_name()
        .map_or(String::new(), |n| n.to_string_lossy().into_owned());
    let mut out = File::create(idx).map_err(io)?;
    if !path.is_dir() && name.to_lowercase().ends_with(".mzml") {
        let size = std::fs::metadata(path).map_err(io)?.len();
        let p = prepare(&mut File::open(path).map_err(io)?, size, &name)?;
        build_mzml_spill(
            &p.table,
            BufReader::new(Counted::new(File::open(path).map_err(io)?, size, &mut pace)),
            &mut out,
            budget,
            None,
        )?;
    } else {
        let mut fs = flow_set::file_set(path)?;
        let imp = impronte(&mut fs)?;
        let l = mzlab_lettori::apri(fs, &Limiti::desktop())?;
        let ids: Vec<u32> = l.campioni().iter().map(|s| s.id).collect();
        let sample = flow_set::sample_index(label, &ids)?;
        let mut l = Scans::new(l, sample, &mut pace);
        indicizza_lettore(&mut l, sample, &imp, &name, &mut out, budget, &mut |_| {})?;
    }
    pace.at(1, 1);
    Ok(())
}

/// A reader that tells `pace` how many scans of sample `c` it has decoded (as many as the sample has in pass 1; a later pass counts past the total and is clamped).
pub(crate) struct Scans<'a, 'b> {
    l: Box<dyn Lettore>,
    c: usize,
    n: usize,
    done: usize,
    pace: &'a mut Pace<'b>,
}

impl<'a, 'b> Scans<'a, 'b> {
    pub(crate) fn new(mut l: Box<dyn Lettore>, c: usize, pace: &'a mut Pace<'b>) -> Self {
        let n = l.indice(c).map_or(0, |v| v.len());
        pace.at(0, n);
        Scans {
            l,
            c,
            n,
            done: 0,
            pace,
        }
    }

    fn count(&mut self, c: usize) {
        if c == self.c {
            self.done += 1;
            self.pace.at(self.done, self.n);
        }
    }
}

#[rustfmt::skip]
impl Lettore for Scans<'_, '_> {
    fn formato(&self) -> Formato { self.l.formato() }
    fn campioni(&self) -> &[Campione] { self.l.campioni() }
    fn indice(&mut self, c: usize) -> mzlab_core::Result<&[VoceScansione]> { self.l.indice(c) }
    fn scansione(&mut self, c: usize, k: usize) -> mzlab_core::Result<Spectrum> { let s = self.l.scansione(c, k); self.count(c); s }
    fn scansione_dati(&mut self, c: usize, k: usize) -> mzlab_core::Result<Spectrum> { let s = self.l.scansione_dati(c, k); self.count(c); s }
    fn scansione_profilo(&mut self, c: usize, k: usize) -> mzlab_core::Result<Option<Spectrum>> { self.l.scansione_profilo(c, k) }
    fn intestazione(&mut self, c: usize, k: usize) -> mzlab_core::Result<Vec<Campo>> { self.l.intestazione(c, k) }
    fn cromatogrammi(&mut self, c: usize) -> mzlab_core::Result<Vec<VoceCromatogramma>> { self.l.cromatogrammi(c) }
    fn cromatogramma(&mut self, c: usize, id: &str) -> mzlab_core::Result<Chromatogram> { self.l.cromatogramma(c, id) }
    fn dad_map(&mut self, c: usize) -> mzlab_core::Result<Option<String>> { self.l.dad_map(c) }
    fn metodo(&mut self, c: usize) -> mzlab_core::Result<String> { self.l.metodo(c) }
    fn testata_mzml(&mut self, c: usize) -> mzlab_core::Result<Option<Header>> { self.l.testata_mzml(c) }
    fn extra_mzml(&self, s: &Spectrum) -> SpectrumExtra { self.l.extra_mzml(s) }
    fn extra_scansione(&mut self, c: usize, k: usize, s: &Spectrum) -> SpectrumExtra { self.l.extra_scansione(c, k, s) }
    fn non_portati(&self) -> &'static [(&'static str, &'static str)] { self.l.non_portati() }
    fn mancanti(&mut self, c: usize) -> usize { self.l.mancanti(c) }
    fn variante(&mut self, c: usize) -> bool { self.l.variante(c) }
}

/// A `Read` that tells `pace` how many of the `size` bytes of the file it has handed out.
pub(crate) struct Counted<'a, 'b, R> {
    r: R,
    size: usize,
    read: usize,
    pace: &'a mut Pace<'b>,
}

impl<'a, 'b, R> Counted<'a, 'b, R> {
    pub(crate) fn new(r: R, size: u64, pace: &'a mut Pace<'b>) -> Self {
        Counted {
            r,
            size: size as usize,
            read: 0,
            pace,
        }
    }
}

impl<R: Seek> Seek for Counted<'_, '_, R> {
    fn seek(&mut self, to: SeekFrom) -> std::io::Result<u64> {
        let at = self.r.seek(to)?;
        self.read = at as usize;
        Ok(at)
    }
}

impl<R: Read> Read for Counted<'_, '_, R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = self.r.read(buf)?;
        self.read += n;
        self.pace.at(self.read, self.size);
        Ok(n)
    }
}
