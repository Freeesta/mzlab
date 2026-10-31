//! `Library::verify`: reads every block and page of a library and checks it against its CRC and the table.

use super::*;

impl<S: Source> Library<S> {
    /// Reads and checks everything (every block and page against its CRC, the peak counts against the table): `Ok` only for a sound file.
    pub fn verify(&self) -> Result<()> {
        let n = self.header.n_spectra;
        let mut peaks = 0u64;
        let mut prev: Option<(u8, f64)> = None;
        for i in 0..n {
            let r = self.rec(i)?;
            let k = (pol_key(r.pol), r.prec);
            if let Some(p) = prev
                && (p.0 > k.0 || (p.0 == k.0 && p.1 > k.1) || r.first_peak as u64 != peaks)
            {
                return Err(Error::corrupt("table"));
            }
            prev = Some(k);
            peaks += r.n_peaks as u64;
            let (mz, _) = self.peaks_q(i)?;
            if !mz.is_sorted_by(|a, b| a < b) {
                return Err(Error::corrupt("peaks"));
            }
            self.meta(i)?;
        }
        if peaks != self.header.n_peaks {
            return Err(Error::corrupt("peaks"));
        }
        self.inchikeys()?;
        for nl in [false, true] {
            let d = self.index_dir(nl);
            let mut total = 0u64;
            let mut last = 0u32;
            for (p, pg) in d.pages.iter().enumerate() {
                let b = self.page(nl, to_u32(p)?)?;
                let c = pg.1 as usize;
                for j in 0..c {
                    let (mz, sp) = (le_u32(&b, j * 4), le_u32(&b, c * 4 + j * 4));
                    if mz < last || sp >= n {
                        return Err(Error::corrupt("index"));
                    }
                    last = mz;
                }
                total += c as u64;
            }
            if total != d.n_entries {
                return Err(Error::corrupt("index"));
            }
        }
        Ok(())
    }
}
