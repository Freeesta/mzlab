//! The state the checks of `tabcheck/1` share: example caps, the proof object under construction and the table by file role.

use super::*;

/// How many example rows and pairs the text carries.
#[derive(Debug, Clone, Copy)]
pub(super) struct Caps {
    pub(super) rows: usize,
    pub(super) pairs: usize,
}

/// A JSON object under construction (the proof of a check).
pub(super) struct Proof(Out, bool);

impl Proof {
    pub(super) fn new() -> Proof {
        Proof(Out::default(), false)
    }

    pub(super) fn key(&mut self, k: &str) -> &mut Out {
        self.0.raw(if self.1 { "," } else { "{" }).str(k).raw(":");
        self.1 = true;
        &mut self.0
    }

    pub(super) fn n(&mut self, k: &str, v: f64) -> &mut Proof {
        self.key(k).num(round(v, 5));
        self
    }

    pub(super) fn i(&mut self, k: &str, v: usize) -> &mut Proof {
        self.key(k).int(v as i64);
        self
    }

    pub(super) fn s(&mut self, k: &str, v: &str) -> &mut Proof {
        self.key(k).str(v);
        self
    }

    pub(super) fn raw(&mut self, k: &str, v: &str) -> &mut Proof {
        self.key(k).raw(v);
        self
    }

    pub(super) fn done(mut self) -> String {
        self.0.raw(if self.1 { "}" } else { "{}" });
        self.0 .0
    }
}

/// The table and what the checks share: the rows considered, the files by role and the RT window of one peak.
pub(super) struct Env<'a> {
    pub(super) t: &'a Aligned,
    pub(super) s: &'a [Sample],
    pub(super) opts: &'a Options,
    pub(super) caps: Caps,
    /// Indices of the rows considered, ascending.
    pub(super) rows: Vec<usize>,
    /// Files that are not `dark`.
    pub(super) live: Vec<usize>,
    pub(super) blanks: Vec<usize>,
    pub(super) qcs: Vec<usize>,
    /// Live files that are neither blank nor QC.
    pub(super) samples: Vec<usize>,
    /// Median FWHM of the peaks, minutes: the window inside which two rows co-elute.
    pub(super) fwhm: f64,
}

impl Env<'_> {
    pub(super) fn id(&self, row: usize) -> u32 {
        self.t.id(row)
    }

    pub(super) fn area(&self, row: usize, file: usize) -> Option<f64> {
        measured(self.t, row, file)
    }

    /// Areas of `row` in `files`, 0 where the file has no peak.
    pub(super) fn areas(&self, row: usize, files: &[usize]) -> Vec<f64> {
        files
            .iter()
            .map(|&k| self.area(row, k).unwrap_or(0.0))
            .collect()
    }

    pub(super) fn present_in_all(&self, row: usize, files: &[usize]) -> bool {
        files.iter().all(|&k| self.area(row, k).is_some())
    }
}

impl Env<'_> {
    /// Ids of `rows` (indices), cut to the example cap.
    pub(super) fn ids(&self, rows: &[usize]) -> Vec<u32> {
        rows.iter()
            .take(self.caps.rows)
            .map(|&r| self.id(r))
            .collect()
    }

    pub(super) fn names(&self, files: &[usize]) -> Vec<String> {
        files
            .iter()
            .take(self.caps.rows)
            .map(|&k| self.s[k].file.clone())
            .collect()
    }

    /// Whether the table is read as unit resolution.
    pub(super) fn unit_resolution(&self) -> bool {
        self.opts.mz_tol_da >= UNIT_TOL_DA
    }

    /// Rows considered by ascending m/z, then id.
    pub(super) fn by_mz(&self) -> Vec<usize> {
        let mut v = self.rows.clone();
        crate::sortc::by(&mut v, |&a, &b| {
            self.t.mz[a].total_cmp(&self.t.mz[b]).then(a.cmp(&b))
        });
        v
    }

    pub(super) fn rt_close(&self, a: usize, b: usize) -> bool {
        (self.t.rt[a] - self.t.rt[b]).abs() <= self.fwhm
    }
}

/// The state of the checks for `t`: rows considered, files by role and the RT window of one peak.
pub(super) fn build<'a>(
    t: &'a Aligned,
    s: &'a [Sample],
    view: Option<&View>,
    opts: &'a Options,
    caps: Caps,
) -> Env<'a> {
    let rows: Vec<usize> = view.map_or_else(
        || (0..t.len()).collect(),
        |v| v.rows.iter().map(|&r| r as usize).collect(),
    );
    let of =
        |f: &dyn Fn(&Sample) -> bool| (0..s.len()).filter(|&k| f(&s[k])).collect::<Vec<usize>>();
    let live = of(&|x| x.kind != "dark");
    let blanks = of(&|x| x.kind == "blank");
    let qcs = of(&|x| x.kind == "qc");
    let samples = of(&|x| !matches!(x.kind.as_str(), "blank" | "dark" | "qc"));
    let widths: Vec<f64> = rows
        .iter()
        .flat_map(|&r| (0..t.n_files).map(move |k| (r, k)))
        .filter(|&(r, k)| t.state[r * t.n_files + k] == crate::aligned::CELL_DETECTED)
        .map(|(r, k)| f64::from(t.width[r * t.n_files + k]))
        .filter(|w| w.is_finite() && *w > 0.0)
        .collect();
    let fwhm = if widths.is_empty() {
        FALLBACK_FWHM
    } else {
        median(widths) * FWHM_OF_WIDTH
    };
    Env {
        t,
        s,
        opts,
        caps,
        rows,
        live,
        blanks,
        qcs,
        samples,
        fwhm,
    }
}
