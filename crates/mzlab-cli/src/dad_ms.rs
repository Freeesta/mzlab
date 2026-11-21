//! The MS side of `dad ms-at|delay|peaks`: the routes and the mzML (or the shell of a vendor file) of a file, opened on a temporary index that lives while a step runs.

use std::fs::File;
use std::path::Path;

use mzlab_core::error::Result;
use mzlab_core::idxread::ReadAt;
use mzlab_core::mzmlhead::{Chrom, Mzml};
use mzlab_core::pdapeaks::MsOpts;
use mzlab_core::routes::{Query, Routes};
use mzlab_core::wiff::dad::DadData;
use mzlab_core::wiff::dad_peaks::PeakOpts;

use crate::dad_cmd::{idx_err, sample_at};

/// The routes and the mzML (or its shell) of `file`, for the MS side (`ms-at`, `delay`); `body` runs with them while the temporary index lives.
pub(crate) fn with_ms<T>(
    file: &Path,
    sample: Option<u32>,
    body: &mut dyn FnMut(&mut dyn MsSide) -> Result<T>,
) -> Result<T> {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let k = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let tmp = std::env::temp_dir().join(format!("mzlab-cli-{}-dad{k}", std::process::id()));
    std::fs::create_dir_all(&tmp).map_err(idx_err)?;
    let r = ms_inner(file, sample, &tmp, body);
    let _ = std::fs::remove_dir_all(&tmp);
    r
}

pub(crate) trait MsSide {
    fn spectrum(&mut self, q: &Query) -> Result<String>;
    fn tic(&mut self, q: &Query) -> Result<String>;
    fn delay(&mut self) -> Result<String>;
    /// `dad peaks`: the engine route, with the optical matrix read apart.
    fn peaks(&mut self, dad: &DadData, po: &PeakOpts, mo: &MsOpts) -> Result<String>;
    /// The times (minutes) of the scans of `level`, for the choice of the one nearest to a time.
    fn scan_times(&mut self, level: u8) -> Result<Vec<f64>>;
    /// Whether the file has an optical trace to compare with the TIC.
    fn has_uv(&self) -> bool;
}

struct Side<'a, R: ReadAt, M: ReadAt> {
    r: &'a mut Routes<R>,
    m: &'a mut Mzml<M>,
    chroms: &'a [Chrom],
}

impl<R: ReadAt, M: ReadAt> MsSide for Side<'_, R, M> {
    fn spectrum(&mut self, q: &Query) -> Result<String> {
        Ok(self
            .r
            .handle("/api/spectrum", q)?
            .unwrap_or_else(|| "null".into()))
    }
    fn tic(&mut self, q: &Query) -> Result<String> {
        Ok(self
            .r
            .handle("/api/chrom", q)?
            .unwrap_or_else(|| "null".into()))
    }
    fn delay(&mut self) -> Result<String> {
        self.r.delay(self.m, self.chroms)
    }
    fn peaks(&mut self, dad: &DadData, po: &PeakOpts, mo: &MsOpts) -> Result<String> {
        self.r.pda_peaks(self.m, self.chroms, dad, po, mo)
    }
    fn scan_times(&mut self, level: u8) -> Result<Vec<f64>> {
        let pol = self.r.pol;
        match self.r.idx.section_of(level, pol) {
            Some(si) => Ok(self.r.idx.arrays(si)?.rt.clone()),
            None => Ok(Vec::new()),
        }
    }
    fn has_uv(&self) -> bool {
        self.chroms.iter().any(|c| c.kind == "pda")
    }
}

fn ms_inner<T>(
    file: &Path,
    sample: Option<u32>,
    tmp: &Path,
    body: &mut dyn FnMut(&mut dyn MsSide) -> Result<T>,
) -> Result<T> {
    let name = file
        .file_name()
        .map_or(String::new(), |n| n.to_string_lossy().into_owned());
    let label = sample.map_or(String::new(), |id| format!("{name}@{id}"));
    let mzml = name.to_lowercase().ends_with(".mzml");
    let idx = tmp.join("0.mzidx");
    crate::flow_pace::build_index_of(
        file,
        &label,
        &idx,
        mzlab_core::budget_auto::native_index_budget(),
    )?;
    let mut r = Routes::new(crate::open(&idx)?);
    let end = r.idx.scans.iter().map(|s| s.end).max().unwrap_or(0);
    if mzml {
        let f = File::open(file).map_err(idx_err)?;
        let size = f.metadata().map_err(idx_err)?.len();
        let mut m = Mzml::new(f, size);
        let chroms = m.chromatograms(end)?;
        return body(&mut Side {
            r: &mut r,
            m: &mut m,
            chroms: &chroms,
        });
    }
    let mut l = mzlab_lettori::apri(
        crate::flow_set::file_set(file)?,
        &mzlab_core::readers::Limits::desktop(),
    )?;
    let ids: Vec<u32> = l.samples().iter().map(|s| s.id).collect();
    let k = sample_at(&ids, sample)?;
    let shell = mzlab_core::readers_mzml::guscio(l.as_mut(), k)?;
    let size = shell.len() as u64;
    let mut m = Mzml::new(shell, size);
    let chroms = m.chromatograms(end)?;
    body(&mut Side {
        r: &mut r,
        m: &mut m,
        chroms: &chroms,
    })
}
