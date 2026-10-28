//! Streaming helpers of `pipeline::run_file`: the peaks of a section as slabs of ADAP chromatograms, the reused trace buffer and the first pass that
//! collects the common mode G(t). Memory is one slab, not the file.

use mzlab_core::error::{Error, Result};
use mzlab_core::idxread::{Index, ReadAt};

use super::adap::{Chrom as AdapChrom, SlabStream};
use super::pipeline::UntParams;
use super::resolve::Chrom as Trace;
use crate::broad_gt::{share, TraceBank};

/// The peaks of the section at or above `noise` as slabs of m/z, one at a time: the peak table of the index is sorted by m/z, so the points reach the ADAP
/// builder in the order its cuts want and the memory is the slab (`UntParams::slab_bytes`), not the file. The chromatograms are those of `adap::each_slab`.
pub(super) fn each_slab<R: ReadAt>(
    ix: &mut Index<R>,
    si: usize,
    nscans: usize,
    p: &UntParams,
    mut f: impl FnMut(Vec<AdapChrom>),
) -> Result<()> {
    let n = ix.sections[si].n_peaks;
    let (noise, mut sorted) = (p.adap.noise, true);
    let mut stream = SlabStream::new(nscans, &p.adap, p.slab_bytes);
    ix.each_peak(si, 0, n, |pos, mz, inten| {
        if inten >= noise && pos < nscans {
            sorted &= stream.push(mz, inten as f32, pos as u32, &mut f);
        }
    })?;
    if !sorted {
        return Err(Error::new("error.badIndex"));
    }
    stream.finish(&mut f);
    Ok(())
}

/// The trace of the ADAP chromatograms on ALL the scans that the resolver wants (zero where there is no point), one buffer for the whole file: loading a
/// chromatogram clears only the scans of the previous one.
pub(super) struct TraceBuf {
    pub(super) t: Trace,
    touched: Vec<u32>,
}

impl TraceBuf {
    pub(super) fn new(nscans: usize) -> Self {
        TraceBuf {
            t: Trace {
                mz: vec![0.0; nscans],
                y: vec![0.0; nscans],
            },
            touched: Vec::new(),
        }
    }

    pub(super) fn load(&mut self, c: &AdapChrom) {
        for &s in &self.touched {
            self.t.mz[s as usize] = 0.0;
            self.t.y[s as usize] = 0.0;
        }
        for ((&s, &y), &m) in c.scan.iter().zip(&c.int).zip(&c.point_mz) {
            self.t.mz[s as usize] = m;
            self.t.y[s as usize] = y as f64;
        }
        self.touched.clear();
        self.touched.extend_from_slice(&c.scan);
    }
}

/// Bytes a chromatogram of the first pass costs while it waits for the second.
fn chrom_bytes(c: &AdapChrom) -> usize {
    std::mem::size_of::<AdapChrom>() + c.scan.len() * (4 + 4 + 8)
}

/// The chromatograms of the first pass of the common mode, kept for the second pass while they fit in a quarter of `slab_bytes` (a small file is then built
/// once); `None` once they do not, and the second pass builds them again.
pub(super) struct Kept(pub(super) Option<(Vec<Vec<AdapChrom>>, usize)>);

impl Kept {
    pub(super) fn keep(&mut self, chroms: Vec<AdapChrom>, limit: usize) {
        let Some((slabs, bytes)) = self.0.as_mut() else {
            return;
        };
        *bytes += chroms.iter().map(chrom_bytes).sum::<usize>();
        if *bytes > limit {
            self.0 = None;
        } else {
            slabs.push(chroms);
        }
    }
}

/// With the common-mode option of the broad pass on: the parameters with the gradient mode G(t) of the file in `broad.common`, found by a first pass
/// over the persistent chromatograms (the span between their first and last scan covers 400 scans or 30 % of the run). `None` when the option is off
/// or the file has too few of them. Only the chromatograms the bank samples are built as traces; `kept` receives the chromatograms for the second pass.
pub(super) fn common_mode_params<R: ReadAt>(
    ix: &mut Index<R>,
    si: usize,
    axis: &[f64],
    p: &UntParams,
    kept: &mut Kept,
) -> Result<Option<UntParams>> {
    let b = &p.broad;
    if !(b.enabled && b.gt) || b.common.is_some() {
        kept.0 = None;
        return Ok(None);
    }
    let mut bank = TraceBank::new(axis.len(), b.decimate);
    let mut buf = TraceBuf::new(axis.len());
    let mut failed = false;
    each_slab(ix, si, axis.len(), p, |chroms| {
        for c in &chroms {
            let (Some(&lo), Some(&hi)) = (c.scan.iter().min(), c.scan.iter().max()) else {
                continue;
            };
            let (lo, hi) = (lo as usize, hi as usize);
            if !bank.offer(lo, hi - lo + 1) {
                continue;
            }
            buf.load(c);
            let pushed = match &p.baseline {
                Some(bl) => bank.push_row(lo, &bl.apply(axis, &buf.t.y)[lo..=hi]),
                None => bank.push_row(lo, &buf.t.y[lo..=hi]),
            };
            failed |= pushed.is_err();
        }
        kept.keep(chroms, p.slab_bytes / 4);
    })?;
    let Some(cm) = bank.finish().ok().flatten().filter(|_| !failed) else {
        return Ok(None);
    };
    let mut q = p.clone();
    q.broad.common = share(Some(cm));
    Ok(Some(q))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chrom(scans: &[u32], base: f64) -> AdapChrom {
        AdapChrom {
            mz: 0.0,
            mz_mean: 0.0,
            scan: scans.to_vec(),
            int: scans.iter().map(|&s| (base + s as f64) as f32).collect(),
            point_mz: scans.iter().map(|&s| 100.0 + s as f64 / 8.0).collect(),
            points: scans.len(),
        }
    }

    /// The trace of one chromatogram on a fresh buffer: what every chromatogram got before the buffer was shared.
    fn fresh(c: &AdapChrom, n: usize) -> Trace {
        let mut b = TraceBuf::new(n);
        b.load(c);
        b.t
    }

    #[test]
    fn a_shared_trace_buffer_gives_the_trace_of_a_fresh_one() {
        let n = 40;
        let chroms = [
            chrom(&[3, 4, 5, 6, 7, 8], 10.0),
            chrom(&[0, 1, 2], 20.0),
            chrom(&[5, 6, 30, 31, 39], 30.0),
            chrom(&[], 0.0),
            chrom(&[4, 6, 8], 40.0),
        ];
        let mut shared = TraceBuf::new(n);
        for c in &chroms {
            shared.load(c);
            let want = fresh(c, n);
            assert_eq!(shared.t, want);
            for &s in &c.scan {
                assert_eq!(
                    shared.t.y[s as usize],
                    c.int[c.scan.iter().position(|&x| x == s).unwrap()] as f64
                );
            }
        }
    }
}
