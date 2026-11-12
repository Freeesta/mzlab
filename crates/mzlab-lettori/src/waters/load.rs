//! One function of a folder as the reader keeps it (`Fun`): its index, its DAT and its side tables. A function whose index cannot be trusted, or whose
//! calibration line a cut `_HEADER.TXT` lost, is left out whole and counted (`warn.waters.damaged`): its records are never read by rows that do not fit,
//! nor its m/z uncalibrated. The first such part keeps the refusal of the folder, given only when nothing else in it can be read.

use super::cuts::{cuts, nel_file};
use super::dat::{Calibration, Encoding};
use super::folder::Folder;
use super::functions::Function;
use super::idx::{self, Row};
use super::inf::{Extern, FunctionExt, WatersHeader};
use super::reader::Fun;
use super::sides::{compound_names, Table};
use mzlab_core::error::{Error, Result};

/// What the functions of a folder are read with: the folder-wide sources, the index rows so far, and the refusal of the first part left out.
pub(super) struct Loading<'a> {
    pub(super) ext: &'a Extern,
    pub(super) tars: &'a WatersHeader,
    /// `_HEADER.TXT` emptied or cut inside a calibration line.
    pub(super) header_cut: bool,
    pub(super) spent: u64,
    pub(super) refusal: Option<Error>,
}

/// The index and DAT of a function that fit each other.
struct Parts {
    rows: Vec<Row>,
    ha_idx: bool,
    dat: Option<usize>,
    dat_len: u64,
    larg: Option<u64>,
    cod: Option<Encoding>,
    cut: usize,
}

impl Loading<'_> {
    /// Function `f` with its index, or left out whole (`left_out`) with the notice that says why.
    pub(super) fn load(&mut self, c: &mut Folder, f: Function) -> Result<(Fun, Option<String>)> {
        let n = f.indice;
        let name = |e: &str| format!("_FUNC{n:03}.{e}");
        let idx_b = c.read_run(&name("IDX"))?;
        let ha_idx = idx_b.is_some();
        let cut_idx = nel_file(c, "err.file.indexCut", &name("IDX"));
        let mut rows = match &idx_b {
            Some(b) => match idx::read(b, &c.limits)? {
                Some(r) => r,
                None => {
                    let why = format!("{} cut inside a row", name("IDX"));
                    return Ok(self.left_out(f, idx::whole_rows(b), why, cut_idx));
                }
            },
            None => Vec::new(),
        };
        self.spent += rows.len() as u64;
        c.limits.ok_entries(self.spent)?;
        let dat = c.search(&name("DAT"));
        let dat_len = dat.map_or(0, |i| c.fs.membri[i].dim);
        let larg = idx::width(&rows, dat_len);
        let cod = if f.mirato() {
            None
        } else {
            larg.and_then(Encoding::from_width)
        };
        if !ha_idx && dat_len > 0 {
            let why = format!("{} missing next to {}", name("IDX"), name("DAT"));
            let e = nel_file(c, "err.file.folderIncomplete", &name("IDX"));
            return Ok(self.left_out(f, 0, why, e));
        }
        let Some(cut) = cuts(&mut rows, larg, dat.is_some(), dat_len, ha_idx) else {
            let why = format!("{} does not match {}", name("IDX"), name("DAT"));
            return Ok(self.left_out(f, rows.len(), why, cut_idx));
        };
        let k = cut + rows.iter().filter(|r| r.count > 0).count();
        let lost = self.header_cut && !f.mirato() && dat.is_some();
        if lost && k > 0 && !self.tars.tarature.contains_key(&n) {
            let why = "no whole Cal Function line in _HEADER.TXT".to_string();
            let e = nel_file(c, "err.file.headerCut", "_HEADER.TXT");
            return Ok(self.left_out(f, k, why, e));
        }
        let p = Parts {
            rows,
            ha_idx,
            dat,
            dat_len,
            larg,
            cod,
            cut,
        };
        self.whole(c, f, p)
    }

    /// Function `f` whose index and DAT fit: its side tables and the notice of a member that is not there.
    fn whole(&self, c: &mut Folder, f: Function, p: Parts) -> Result<(Fun, Option<String>)> {
        let n = f.indice;
        let name = |e: &str| format!("_FUNC{n:03}.{e}");
        let lim = c.limits;
        let tab = |c: &mut Folder, e: &str| -> Result<_> {
            Ok(c.read_run(&name(e))?.and_then(|b| Table::read(&b, &lim)))
        };
        let (sts, ee) = (tab(c, "STS")?, tab(c, "EE")?);
        let names = c
            .read(&name("CMP"))?
            .map_or(Vec::new(), |b| compound_names(&b, &lim));
        // the member that is not there is named: a function with an index and no DAT is not the same loss as a function without scans
        let warning = if p.dat.is_some() != p.ha_idx {
            Some(format!(
                "function {n}: {} missing",
                name(if p.ha_idx { "DAT" } else { "IDX" })
            ))
        } else if p.dat.is_none() || p.rows.is_empty() {
            Some(format!("function {n}: no scans"))
        } else if !f.mirato() && p.cod.is_none() {
            Some(format!("function {n}: record width"))
        } else {
            None
        };
        let (ext, polarita) = self.ext_of(n);
        let fun = Fun {
            f,
            ext,
            righe: p.rows,
            dat: p.dat,
            dat_len: p.dat_len,
            larg: p.larg,
            cod: p.cod,
            tar: self.tars.tarature.get(&n).cloned().unwrap_or_default(),
            sts,
            ee,
            names,
            livello: 1,
            polarita,
            cut: p.cut,
            left_out: None,
        };
        Ok((fun, warning))
    }

    /// The `_extern.inf` section of function `n` and its polarity.
    fn ext_of(&self, n: u32) -> (FunctionExt, i8) {
        let x = self.ext.funzioni.get(&n).cloned().unwrap_or_default();
        let polarity = x.polarita.or(self.ext.polarita).unwrap_or(0);
        (x, polarity)
    }

    /// Function `f` left out whole as damaged: no rows and no DAT are kept, its `rows` index rows are counted and `why` is named in the notice; `rows` 0 is
    /// an index that cannot be counted, told as «at least 1». `e` is the refusal of the folder when nothing else in it can be read, kept if it is the first.
    fn left_out(
        &mut self,
        f: Function,
        rows: usize,
        why: String,
        e: Error,
    ) -> (Fun, Option<String>) {
        self.refusal.get_or_insert(e);
        let told = if rows == 0 {
            "at least 1".to_string()
        } else {
            rows.to_string()
        };
        let note = format!("function {}: {why}, {told} scans left out", f.indice);
        let (ext, polarita) = self.ext_of(f.indice);
        let fun = Fun {
            f,
            ext,
            righe: Vec::new(),
            dat: None,
            dat_len: 0,
            larg: None,
            cod: None,
            tar: Calibration::default(),
            sts: None,
            ee: None,
            names: Vec::new(),
            livello: 1,
            polarita,
            cut: 0,
            left_out: Some(rows.max(1)),
        };
        (fun, Some(note))
    }
}
