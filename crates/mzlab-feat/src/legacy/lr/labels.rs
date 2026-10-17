//! The criteria of a low-resolution ion on the cells of its row: a peak in the treated samples, absent in the blank and at time zero, growth
//! over time (Spearman rank correlation up to the maximum and rise), then score and label as mzFinder names them (`forte`, `possibile`,
//! `debole`). The peaks are those of the pipeline (state of the cell), not a second peak search.

use super::super::series::FileIn;

/// One file of a row: the cell of the table (zero area where the file has no peak).
#[derive(Debug, Clone, Default)]
pub struct Cell {
    pub area: f64,
    pub height: f64,
    pub apex_rt: Option<f64>,
    pub detected: bool,
    pub lo: f64,
    pub hi: f64,
}

/// One criterion: `pass`, `fail` or `n/a`, with its text.
#[derive(Debug, Clone)]
pub struct Crit {
    pub name: &'static str,
    pub status: &'static str,
    pub text: String,
}

pub fn crit(name: &'static str, status: &'static str, text: String) -> Crit {
    Crit { name, status, text }
}

/// `DEFAULT_THRESHOLDS` of the Python engine.
pub const RATIO_MAX: f64 = 0.20;
pub const SPEARMAN_MIN: f64 = 0.8;
pub const RISE_MIN: f64 = 2.0;
pub const STRONG: f64 = 75.0;
pub const POSSIBLE: f64 = 50.0;
pub const GROWS: &str = "grows over time";
pub const PERSISTS: &str = "persistent";

/// What counts as persistent: a peak in at least `min_files` treated samples (fewer when the series has fewer), or in two consecutive ones
/// whose apexes lie within `rt_tol` minutes. `min_files = 0` switches the criterion off.
#[derive(Debug, Clone, Copy)]
pub struct Persist {
    pub min_files: usize,
    pub rt_tol: f64,
}

#[derive(Debug, Clone)]
pub struct Score {
    pub score: Option<f64>,
    pub label: &'static str,
    pub criteria: Vec<Crit>,
}

/// `f"{v:g}"` for a time in minutes.
pub fn g(v: f64) -> String {
    if v.fract() == 0.0 && v.abs() < 1e15 {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}

fn treated(f: &FileIn) -> bool {
    f.kind == "sample" && f.time.is_none_or(|t| t > 0.0)
}

/// Average ranks (ties share the mean of their positions), 1-based.
fn ranks(v: &[f64]) -> Vec<f64> {
    let mut o: Vec<usize> = (0..v.len()).collect();
    mzlab_core::sortc::by(&mut o, |&a, &b| v[a].total_cmp(&v[b]));
    let mut r = vec![0.0; v.len()];
    let mut i = 0;
    while i < o.len() {
        let mut j = i;
        while j + 1 < o.len() && v[o[j + 1]] == v[o[i]] {
            j += 1;
        }
        for &k in &o[i..=j] {
            r[k] = (i + j) as f64 / 2.0 + 1.0;
        }
        i = j + 1;
    }
    r
}

/// Spearman rank correlation, `None` when one of the series is constant.
pub fn spearman(x: &[f64], y: &[f64]) -> Option<f64> {
    let (rx, ry) = (ranks(x), ranks(y));
    let n = rx.len() as f64;
    let (mx, my) = (rx.iter().sum::<f64>() / n, ry.iter().sum::<f64>() / n);
    let (mut sxy, mut sxx, mut syy) = (0.0, 0.0, 0.0);
    for (a, b) in rx.iter().zip(&ry) {
        sxy += (a - mx) * (b - my);
        sxx += (a - mx) * (a - mx);
        syy += (b - my) * (b - my);
    }
    (sxx > 0.0 && syy > 0.0).then(|| sxy / (sxx * syy).sqrt())
}

fn ratio_crit(name: &'static str, what: &str, refs: &[f64], amax: f64) -> Crit {
    let ratio = if amax > 0.0 {
        refs.iter().copied().fold(0.0, f64::max) / amax
    } else {
        1.0
    };
    let st = if ratio <= RATIO_MAX { "pass" } else { "fail" };
    crit(
        name,
        st,
        format!("{what} = {ratio:.2} (max {})", g(RATIO_MAX)),
    )
}

/// `distinct` = how many different times the samples must span for growth to be judged (0 = the three samples of today, whatever their times).
fn growth(files: &[FileIn], cells: &[Cell], distinct: usize) -> Option<Crit> {
    let mut timed: Vec<usize> = (0..files.len())
        .filter(|&k| files[k].kind == "sample" && files[k].time.is_some())
        .collect();
    mzlab_core::sortc::by(&mut timed, |&a, &b| files[a].t.total_cmp(&files[b].t));
    let mut times: Vec<f64> = timed.iter().map(|&k| files[k].t).collect();
    times.dedup();
    if timed.len() < 3 || times.len() < distinct {
        return None;
    }
    let a: Vec<f64> = timed
        .iter()
        .map(|&k| {
            if cells[k].detected {
                cells[k].area
            } else {
                0.0
            }
        })
        .collect();
    let imax = (0..a.len()).fold(0, |m, i| if a[i] > a[m] { i } else { m });
    let t_up: Vec<f64> = timed[..=imax].iter().map(|&k| files[k].t).collect();
    let rho = match imax + 1 {
        2 => Some(if a[1] > a[0] { 1.0 } else { 0.0 }),
        n if n > 2 => spearman(&t_up, &a[..=imax]),
        _ => None,
    };
    let low = a[..imax].iter().copied().fold(f64::INFINITY, f64::min);
    let rise = if imax > 0 && low > 0.0 {
        a[imax] / low
    } else {
        f64::INFINITY
    };
    let good = imax > 0 && rho.is_some_and(|r| r >= SPEARMAN_MIN) && rise >= RISE_MIN;
    let rho_t = rho.map_or("n/a".to_string(), |r| format!("{r:.2}"));
    let rise_t = if rise.is_infinite() {
        "rise from nothing".to_string()
    } else {
        format!("rise x{rise:.1}")
    };
    let text = format!(
        "maximum at {} min; Spearman rho = {rho_t} up to it (minimum {}), {rise_t} (minimum x{})",
        g(files[timed[imax]].t),
        g(SPEARMAN_MIN),
        g(RISE_MIN)
    );
    Some(crit(GROWS, if good { "pass" } else { "fail" }, text))
}

/// The persistence criterion over the treated samples `tr` (those with a peak).
fn persistence(files: &[FileIn], cells: &[Cell], tr: &[usize], p: Persist) -> Crit {
    if p.min_files == 0 {
        return crit(PERSISTS, "n/a", "switched off".into());
    }
    let mut on: Vec<usize> = tr.iter().copied().filter(|&k| cells[k].detected).collect();
    mzlab_core::sortc::by(&mut on, |&a, &b| {
        let t = |k: usize| files[k].time.unwrap_or(f64::MAX);
        t(a).total_cmp(&t(b))
    });
    let need = p.min_files.min(tr.len());
    let close = on.windows(2).any(|w| {
        let (a, b) = (cells[w[0]].apex_rt, cells[w[1]].apex_rt);
        a.zip(b).is_some_and(|(a, b)| (a - b).abs() <= p.rt_tol) && tr.len() >= 2
    });
    let ok = on.len() >= need || (on.len() >= 2 && close);
    let text = format!(
        "peak in {} of {} treated samples (needs {need}, or two consecutive ones with the apex within {} min)",
        on.len(),
        tr.len(),
        g(p.rt_tol)
    );
    crit(PERSISTS, if ok { "pass" } else { "fail" }, text)
}

/// `score_candidate` on the cells of a row (one per column of the table); with several conditions, the best condition's (`cond::best`).
pub fn score(files: &[FileIn], cells: &[Cell], p: Persist) -> Score {
    super::cond::best(files, cells, p).0
}

/// `score_candidate` on the cells of a row of one condition (or of a series without conditions).
pub fn score_one(files: &[FileIn], cells: &[Cell], p: Persist) -> Score {
    score_in(files, cells, p, 0)
}

/// `score_one`, with growth judged only when the samples span `distinct` different times (the view of one condition among several: its
/// replicates of one time are one point of the kinetic, not three).
pub fn score_in(files: &[FileIn], cells: &[Cell], p: Persist, distinct: usize) -> Score {
    let pick = |sel: &dyn Fn(&FileIn) -> bool| -> Vec<usize> {
        (0..files.len()).filter(|&k| sel(&files[k])).collect()
    };
    let tr = pick(&treated);
    let t0 = pick(&|f| f.kind == "sample" && f.time == Some(0.0));
    let blanks = pick(&|f| f.kind == "blank" || f.kind == "control");
    let amax = tr
        .iter()
        .filter(|&&k| cells[k].detected)
        .map(|&k| cells[k].area)
        .fold(0.0, f64::max);
    let best = tr
        .iter()
        .copied()
        .filter(|&k| cells[k].detected)
        .max_by(|&a, &b| cells[a].area.total_cmp(&cells[b].area));
    let ok = best.is_some();
    let mut c = vec![crit(
        "peak in the treated samples",
        if ok { "pass" } else { "fail" },
        match best {
            Some(k) => format!(
                "peak of the pipeline in {} (height {:.0})",
                files[k].label, cells[k].height
            ),
            None if tr.is_empty() => "no treated sample".into(),
            None => "no peak of the pipeline in the treated samples".into(),
        },
    )];
    if ok {
        c.push(persistence(files, cells, &tr, p));
    }
    let areas = |ks: &[usize]| -> Vec<f64> { ks.iter().map(|&k| cells[k].area).collect() };
    c.push(match (blanks.is_empty(), ok) {
        (false, true) => ratio_crit(
            "absent in the blank",
            "blank / treated",
            &areas(&blanks),
            amax,
        ),
        (true, _) => crit(
            "absent in the blank",
            "n/a",
            "no blank in the experiment".into(),
        ),
        _ => crit("absent in the blank", "n/a", "no peak".into()),
    });
    c.push(match (t0.is_empty(), ok) {
        (false, true) => ratio_crit("absent at time zero", "t0 / treated", &areas(&t0), amax),
        (true, _) => crit(
            "absent at time zero",
            "n/a",
            "no sample at time zero".into(),
        ),
        _ => crit("absent at time zero", "n/a", "no peak".into()),
    });
    let gr = if ok {
        growth(files, cells, distinct)
    } else {
        None
    };
    c.push(gr.unwrap_or_else(|| {
        crit(
            GROWS,
            "n/a",
            "at least three samples with a time and a peak are required".into(),
        )
    }));
    finish(c, ok)
}

/// Score (share of the criteria passed, at most 20 without a peak) and label.
pub fn finish(criteria: Vec<Crit>, ok: bool) -> Score {
    let ev: Vec<&Crit> = criteria.iter().filter(|c| c.status != "n/a").collect();
    if ev.is_empty() {
        return Score {
            score: None,
            label: "n/a",
            criteria,
        };
    }
    let mut sc = 100.0 * ev.iter().filter(|c| c.status == "pass").count() as f64 / ev.len() as f64;
    if !ok {
        sc = sc.min(20.0);
    }
    let label = if sc >= STRONG {
        "forte"
    } else if sc >= POSSIBLE {
        "possibile"
    } else {
        "debole"
    };
    Score {
        score: Some(sc.round()),
        label,
        criteria,
    }
}

/// The status of a criterion by name (`""` when absent).
pub fn status(c: &[Crit], name: &str) -> &'static str {
    c.iter().find(|x| x.name == name).map_or("", |x| x.status)
}
