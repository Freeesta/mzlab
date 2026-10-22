//! `mzlab-cli explain-missing TABLE.mzfeat EXPECTED.csv`: for each expected product (id, m/z, RT) the numbers that say where the Tabui table loses
//! it: the nearest row, its peaks, its `tp/1` status and the first filter (`filtri/1`) that hides it. Counts and numbers only, no names; it reads
//! the `.mzfeat` of a run, so the same command serves the demo and the local series.

use std::fmt::Write;

use mzlab_feat::interp::filtri::{first_hidden, row_facts, STAGES};
use mzlab_feat::table::{Aligned, CELL_DETECTED, CELL_FILLED};

use crate::Outcome;

pub const HELP: &str = "mzlab-cli explain-missing TABLE.mzfeat EXPECTED.csv [--ppm N] [--rt-tol MIN] [--out FILE]
  TABLE.mzfeat   the table of a run (the output of tp.link or later)
  EXPECTED.csv   one expected product per line: id,mz,rt (a header line is skipped)
  --ppm N        mass window of a row for an expected m/z (default 5)
  --rt-tol MIN   RT window in minutes (default 0.3)
output: one CSV line per expected product: id, mz, rt, rows_mz (rows within the mass window), rows_in (also within the RT window), best_ppm, best_drt,
  detected, filled (cells of the best row), max_height, max_area, tp_status, stage (view | F1..F5 | no_row_mz | rt_off), nearest_ppm, nearest_drt,
  kinship (0-4), q_shape, q_snr (peak quality of the best row), rank_now (place of the best row in the view: kinship, then largest area), view_size, q_reason (bits 1 shape, 2 S/N, 4 points: why F1 hides the row), rank_k1..k3 (other orderings of the view: k1 kinship, peaks detected, area; k2 kinship, sources agreeing (`via`), peaks detected; k3 kinship, then log10 area + 1.5 x share of files detected + 0.5 x sources + peak shape)";

struct Args {
    table: String,
    expected: String,
    ppm: f64,
    rt_tol: f64,
    out: Option<String>,
}

fn parse(a: Vec<String>) -> Result<Args, String> {
    let (mut pos, mut ppm, mut rt_tol, mut out) = (vec![], 5.0, 0.3, None);
    let mut it = a.into_iter();
    while let Some(x) = it.next() {
        let mut val = |what: &str| it.next().ok_or(format!("{what} needs a value"));
        match x.as_str() {
            "--ppm" => ppm = val("--ppm")?.parse().map_err(|_| "--ppm is not a number")?,
            "--rt-tol" => {
                rt_tol = val("--rt-tol")?
                    .parse()
                    .map_err(|_| "--rt-tol is not a number")?
            }
            "--out" => out = Some(val("--out")?),
            _ => pos.push(x),
        }
    }
    let [table, expected]: [String; 2] = pos
        .try_into()
        .map_err(|_| "usage: explain-missing TABLE.mzfeat EXPECTED.csv")?;
    Ok(Args {
        table,
        expected,
        ppm,
        rt_tol,
        out,
    })
}

/// (id, m/z, RT) of each line; a line whose m/z is not a number (the header) is skipped.
pub fn read_expected(text: &str) -> Vec<(String, f64, f64)> {
    text.lines()
        .filter_map(|l| {
            let f: Vec<&str> = l.split([',', ';', '\t']).map(str::trim).collect();
            let (mz, rt) = (f.get(1)?.parse().ok()?, f.get(2)?.parse().ok()?);
            Some((f[0].to_string(), mz, rt))
        })
        .collect()
}

struct Best {
    row: usize,
    ppm: f64,
    drt: f64,
}

fn kin(t: &Aligned, r: usize) -> u8 {
    t.tp.kinship.get(r).copied().unwrap_or(0)
}

fn cells(t: &Aligned, row: usize) -> (usize, usize, f32, f64) {
    let (mut det, mut fil, mut h, mut a) = (0, 0, 0.0f32, 0.0f64);
    for f in 0..t.n_files {
        let k = row * t.n_files + f;
        match t.state[k] {
            CELL_DETECTED => det += 1,
            CELL_FILLED => fil += 1,
            _ => {}
        }
        h = h.max(t.height[k]);
        a = a.max(t.area[k]);
    }
    (det, fil, h, a)
}

/// What every line reads of the table: the facts of the filters and the place of each row in the view (rows of status 0 by kinship, then area).
struct Cx<'a> {
    t: &'a Aligned,
    facts: Vec<mzlab_feat::interp::filtri::RowFacts>,
    /// Place of each row of the view under `KEYS` (`None` = not in the view).
    rank: Vec<[Option<usize>; KEYS]>,
    view: usize,
}

/// Number of orderings compared: 0 = kinship then largest area (the order of the view), 1 = + peaks detected, 2 = + sources agreeing, 3 = a continuous score.
const KEYS: usize = 4;

/// The sort key of a row under ordering `k`, larger first; kinship always comes first.
fn key(t: &Aligned, r: usize, k: usize) -> [f64; 3] {
    let (det, _, _, area) = cells(t, r);
    let kn = f64::from(kin(t, r));
    let via = f64::from(t.tp.via.get(r).copied().unwrap_or(0).count_ones());
    let shape = if t.meta.is_empty() {
        0.0
    } else {
        f64::from(t.meta.get(r).q_shape).clamp(0.0, 1.0)
    };
    let seen = det as f64 / t.n_files.max(1) as f64;
    match k {
        0 => [kn, area, 0.0],
        1 => [kn, det as f64, area],
        2 => [kn, via, det as f64 + area.max(1.0).log10() * 1e-3],
        _ => [
            kn,
            area.max(1.0).log10() + 1.5 * seen + 0.5 * via + shape,
            0.0,
        ],
    }
}

impl<'a> Cx<'a> {
    fn new(t: &'a Aligned) -> Self {
        let rows: Vec<usize> = (0..t.len())
            .filter(|&r| t.tp.status.get(r) == Some(&0))
            .collect();
        let mut rank = vec![[None; KEYS]; t.len()];
        (0..KEYS).for_each(|k| {
            let mut o = rows.clone();
            o.sort_by(|&x, &y| {
                let (a, b) = (key(t, x, k), key(t, y, k));
                b.iter()
                    .zip(&a)
                    .map(|(bi, ai)| bi.total_cmp(ai))
                    .find(|c| c.is_ne())
                    .unwrap_or(x.cmp(&y))
            });
            for (i, &r) in o.iter().enumerate() {
                rank[r][k] = Some(i + 1);
            }
        });
        Cx {
            t,
            facts: row_facts(t),
            rank,
            view: rows.len(),
        }
    }
}

/// One line of the report for the expected product (`mz`, `rt`).
fn explain(cx: &Cx, id: &str, mz: f64, rt: f64, a: &Args) -> String {
    let (t, facts) = (cx.t, &cx.facts);
    let dist = |r: usize| ((t.mz[r] - mz) / mz * 1e6, t.rt[r] - rt);
    let in_mz: Vec<usize> = (0..t.len()).filter(|&r| dist(r).0.abs() <= a.ppm).collect();
    let rows_in: Vec<usize> = in_mz
        .iter()
        .copied()
        .filter(|&r| dist(r).1.abs() <= a.rt_tol)
        .collect();
    // the best row: the strongest one in both windows
    let best = rows_in
        .iter()
        .copied()
        .max_by(|&x, &y| cells(t, x).3.total_cmp(&cells(t, y).3))
        .map(|row| Best {
            row,
            ppm: dist(row).0,
            drt: dist(row).1,
        });
    let near = (0..t.len()).min_by(|&x, &y| dist(x).0.abs().total_cmp(&dist(y).0.abs()));
    let (np, nd) = near.map_or((f64::NAN, f64::NAN), dist);
    let mut s = format!("{id},{mz:.4},{rt:.2},{},{}", in_mz.len(), rows_in.len());
    match &best {
        Some(b) => {
            let (det, fil, h, ar) = cells(t, b.row);
            let stage = first_hidden(&facts[b.row]).map_or("view", |i| STAGES[i].0);
            let _ = write!(
                s,
                ",{:.1},{:.2},{det},{fil},{h:.0},{ar:.0},{},{stage}",
                b.ppm, b.drt, t.tp.status[b.row]
            );
        }
        None => {
            let stage = if in_mz.is_empty() {
                "no_row_mz"
            } else {
                "rt_off"
            };
            let _ = write!(s, ",,,,,,,,{stage}");
        }
    }
    let _ = write!(s, ",{np:.1},{nd:.2}");
    match &best {
        Some(b) => {
            let q = if t.meta.is_empty() {
                None
            } else {
                Some(t.meta.get(b.row))
            };
            let (sh, sn) = q.map_or((f32::NAN, f32::NAN), |q| (q.q_shape, q.q_snr));
            let rk = |k: usize| cx.rank[b.row][k].map_or(String::new(), |r| r.to_string());
            let _ = write!(
                s,
                ",{},{sh:.2},{sn:.1},{},{}",
                kin(t, b.row),
                rk(0),
                cx.view
            );
            // the bits of `quality::Q_*` that made F1 hide the row: 1 shape, 2 signal-to-noise, 4 points
            let _ = write!(s, ",{}", q.map_or(0, |q| q.qreason));
            let _ = write!(s, ",{},{},{}", rk(1), rk(2), rk(3));
        }
        None => s.push_str(&format!(",,,,,{},,,,", cx.view)),
    }
    s
}

/// The whole report (header and one line per expected product).
pub fn report(t: &Aligned, expected: &[(String, f64, f64)], ppm: f64, rt_tol: f64) -> String {
    let a = Args {
        table: String::new(),
        expected: String::new(),
        ppm,
        rt_tol,
        out: None,
    };
    let cx = Cx::new(t);
    let mut out = String::from("id,mz,rt,rows_mz,rows_in,best_ppm,best_drt,detected,filled,max_height,max_area,tp_status,stage,nearest_ppm,nearest_drt,kinship,q_shape,q_snr,rank_now,view_size,q_reason,rank_k1,rank_k2,rank_k3\n");
    for (id, mz, rt) in expected {
        out.push_str(&explain(&cx, id, *mz, *rt, &a));
        out.push('\n');
    }
    out
}

pub fn run(args: Vec<String>) -> Outcome {
    if args.iter().any(|a| a == "--help") {
        return Outcome::ok(format!("{HELP}\n").into_bytes());
    }
    let go = || -> Result<Outcome, String> {
        let a = parse(args)?;
        let bytes = std::fs::read(&a.table).map_err(|e| e.to_string())?;
        let (t, _) = mzlab_feat::store::read_table(&bytes).map_err(|e| e.to_string())?;
        let text = std::fs::read_to_string(&a.expected).map_err(|e| e.to_string())?;
        let r = report(&t, &read_expected(&text), a.ppm, a.rt_tol);
        match &a.out {
            None => Ok(Outcome::ok(r.into_bytes())),
            Some(p) => std::fs::write(p, r)
                .map(|_| Outcome::ok(Vec::new()))
                .map_err(|e| e.to_string()),
        }
    };
    go().unwrap_or_else(|e| Outcome::fail(&e))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table() -> Aligned {
        let mut t = Aligned::with_capacity(2, 3);
        t.n_files = 2;
        // row 0: in view; row 1: same mass, wrong RT; row 2: a control row
        t.mz = vec![300.0, 400.0, 500.0];
        t.rt = vec![5.0, 9.0, 7.0];
        t.area = vec![10.0, 0.0, 5.0, 6.0, 7.0, 8.0];
        t.height = vec![1.0, 0.0, 2.0, 3.0, 4.0, 5.0];
        t.state = vec![
            CELL_DETECTED,
            3,
            CELL_DETECTED,
            CELL_FILLED,
            CELL_DETECTED,
            CELL_DETECTED,
        ];
        t.tp.status = vec![0, 0, 1];
        t.tp.kinship = vec![2, 3, 0];
        t
    }

    #[test]
    fn stages_are_named_by_the_first_filter() {
        let e = read_expected("id,mz,rt\na,300.0005,5.1\nb,400.0,5.0\nc,500.0,7.0\nd,600.0,5.0\n");
        let r = report(&table(), &e, 5.0, 0.3);
        let col = |l: usize| {
            r.lines()
                .nth(l)
                .unwrap()
                .split(',')
                .nth(12)
                .unwrap()
                .to_string()
        };
        assert_eq!(
            [col(1), col(2), col(3), col(4)],
            ["view", "rt_off", "F3", "no_row_mz"]
        );
        let n = r.lines().next().unwrap().split(',').count();
        assert!(r.lines().all(|l| l.split(',').count() == n));
        // rank_now of the first expected product: kinship 3 outranks kinship 2
        assert_eq!(r.lines().nth(1).unwrap().split(',').nth(18), Some("2"));
    }
}
