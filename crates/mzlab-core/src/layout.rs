//! The mzML layout the open routes share: the chromatogram list and the facts of the file, read once per index
//! (`lr_entry`, `lr_items` and `canali` each scanned the whole tail of the file for them). The slots of one file share one cache.
use crate::idxread::ReadAt;
use crate::iteminfo::{add_mrm, facts_with, Facts};
use crate::mzmlhead::{Chrom, Mzml};
use crate::Result;
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct Held {
    chroms: Option<Arc<Vec<Chrom>>>,
    /// `facts_from(.., need_mrm = false)` (its `n_chromatograms` is the argument of each ask), and the header polarity and SRM range `need_mrm` adds.
    base: Option<Facts>,
    mrm: Option<Facts>,
}

/// Shared by clones (`Index::over`); a rebuilt index starts empty.
#[derive(Clone, Default)]
pub struct Layout(Arc<Mutex<Held>>);

/// The layouts of the files opened lately, by what identifies their bytes (`Layout::of`): a second index of the same file (the slot of a panel next to the
/// one of the list, or one opened again) finds what the first one read. Content-addressed, so a changed file never finds a stale one.
static RECENT: Mutex<Vec<(String, Layout)>> = Mutex::new(Vec::new());
const KEEP: usize = 8;

impl Layout {
    /// `of` for an index: the `sha256` of its `source` and its scan table.
    pub fn of_index(source: &crate::json::Value, scans: &[crate::idxread::Scan]) -> Layout {
        let sha = source
            .get("sha256")
            .and_then(crate::json::Value::str)
            .unwrap_or("");
        Layout::of(sha, scans.len(), scans.last().map_or(0, |s| s.end))
    }

    /// The layout for the file with this `sha256` (of `source` in the index), `n_scans` scans ending at `scans_end`; a fresh one without a hash.
    pub fn of(sha: &str, n_scans: usize, scans_end: u64) -> Layout {
        if sha.is_empty() {
            return Layout::default();
        }
        let key = format!("{sha}:{n_scans}:{scans_end}");
        let mut v = RECENT.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(i) = v.iter().position(|(k, _)| *k == key) {
            let hit = v.remove(i);
            let l = hit.1.clone();
            v.insert(0, hit);
            return l;
        }
        let l = Layout::default();
        v.insert(0, (key, l.clone()));
        v.truncate(KEEP);
        l
    }

    /// The chromatogram list an index of an mzML keeps in its `run` (`chromlist`), unless this layout already has one; `None` when an mzML index has no
    /// valid list (it is damaged: there is no fallback that scans the file). The index of a vendor file has none and seeds nothing.
    pub fn seed(&self, run: &crate::json::Value, vendor: bool) -> Option<()> {
        if vendor {
            return Some(());
        }
        let list = crate::chromlist::of_run(run)?;
        let mut h = self.lock();
        if h.chroms.is_none() {
            h.chroms = Some(Arc::new(list));
        }
        Some(())
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Held> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// `Mzml::chromatograms(scans_end)`, scanned the first time only (`scans_end` is a property of the file).
    pub fn chromatograms<R: ReadAt>(
        &self,
        m: &mut Mzml<R>,
        scans_end: u64,
    ) -> Result<Arc<Vec<Chrom>>> {
        if let Some(c) = &self.lock().chroms {
            return Ok(c.clone());
        }
        let c = Arc::new(m.chromatograms(scans_end)?);
        self.lock().chroms = Some(c.clone());
        Ok(c)
    }

    /// `facts_from`: the base is worked out the first time, the part `need_mrm` adds the first time it is asked.
    pub fn facts<R: ReadAt>(
        &self,
        m: &mut Mzml<R>,
        scans_end: u64,
        n_chrom: i64,
        need_mrm: bool,
    ) -> Result<Facts> {
        let cached = self.lock().base.clone();
        let mut f = match cached {
            Some(f) => f,
            None => {
                let chroms = self.chromatograms(m, scans_end)?;
                let f = facts_with(m, &chroms, scans_end, 0, false)?;
                self.lock().base = Some(f.clone());
                f
            }
        };
        f.n_chromatograms = n_chrom;
        if need_mrm {
            let held = self.lock().mrm.clone();
            let x = match held {
                Some(x) => x,
                None => {
                    let chroms = self.chromatograms(m, scans_end)?;
                    let mut x = f.clone();
                    add_mrm(m, &chroms, scans_end, &mut x)?;
                    self.lock().mrm = Some(x.clone());
                    x
                }
            };
            f.header_polarity = x.header_polarity;
            f.srm_rt = x.srm_rt;
        }
        Ok(f)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use crate::error::Result as R;

    struct Counting {
        data: Vec<u8>,
        bytes: usize,
    }
    impl ReadAt for Counting {
        fn read_at(&mut self, off: u64, out: &mut [u8]) -> R<()> {
            self.bytes += out.len();
            self.data.read_at(off, out)
        }
    }

    fn file() -> Vec<u8> {
        let mut s = String::from(
            "<?xml version=\"1.0\"?><mzML><run id=\"r\"><chromatogramList count=\"2\">\n",
        );
        for i in 0..2 {
            s.push_str(&format!("<chromatogram index=\"{i}\" id=\"SRM SIC Q1=100.0 Q3={}.0\" defaultArrayLength=\"0\"></chromatogram>\n", 50 + i));
        }
        s.push_str("</chromatogramList></run></mzML>");
        s.into_bytes()
    }

    #[test]
    fn the_second_ask_reads_nothing_and_answers_the_same() {
        let data = file();
        let size = data.len() as u64;
        let mut m = Mzml::new(Counting { data, bytes: 0 }, size);
        let lay = Layout::default();
        let a = lay.chromatograms(&mut m, 0).unwrap();
        let fa = lay.facts(&mut m, 0, a.len() as i64, true).unwrap();
        let after_first = m.r.bytes;
        assert!(after_first > 0);
        let b = lay.clone().chromatograms(&mut m, 0).unwrap();
        let fb = lay.facts(&mut m, 0, b.len() as i64, true).unwrap();
        assert_eq!(
            m.r.bytes, after_first,
            "the cached layout is not read again"
        );
        assert_eq!(format!("{fa:?}"), format!("{fb:?}"));
        let plain = crate::iteminfo::facts_from(&mut m, 0, a.len() as i64, true).unwrap();
        assert_eq!(
            format!("{plain:?}"),
            format!("{fa:?}"),
            "the cache gives what facts_from gives"
        );
        assert_eq!(a.len(), 2);
    }

    #[test]
    fn two_hashes_never_share_a_layout_and_no_hash_shares_nothing() {
        let a = Layout::of("aa11", 3, 900);
        let b = Layout::of("bb22", 3, 900);
        assert!(
            !Arc::ptr_eq(&a.0, &b.0),
            "different sha256, different layout"
        );
        assert!(Arc::ptr_eq(&a.0, &Layout::of("aa11", 3, 900).0));
        assert!(!Arc::ptr_eq(&a.0, &Layout::of("aa11", 4, 900).0));
        let (x, y) = (Layout::of("", 3, 900), Layout::of("", 3, 900));
        assert!(!Arc::ptr_eq(&x.0, &y.0));
    }
}
