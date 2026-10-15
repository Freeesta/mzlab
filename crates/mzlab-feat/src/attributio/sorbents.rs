// Ported from Attributio sorbents.py, campaign.py.
//! The two per-sorbent feature lists behind a merged dual-SPE table. The merge carries ONE `blank_subtraction` text per row (copied from one of the lists), so
//! the blank of the presence rule is read from each sorbent's own list (`add_sorbent_blanks`); and the aligner may leave one compound in two rows, one per
//! list, which `collapse_twins` joins within the aligner's own tolerances. Both are switched on in `[mzmine]` (`sorbent_table`, `collapse_sorbent_twins`).

use super::campaign::Campaign;
use super::error::{CampaignError, Result};
use super::frame::Frame;
use super::io::{file_name, Io};
use std::collections::{HashMap, HashSet};

/// The blank abundance mzMine writes (`... (abundance 1.3E3)`), 0 when the text has none.
pub fn blank_abundance(text: &str) -> Result<f64> {
    const CLASS: fn(char) -> bool =
        |c| c.is_ascii_digit() || matches!(c, '.' | 'E' | 'e' | '+' | '-');
    let mut from = 0;
    while let Some(i) = text[from..].find("abundance ") {
        let start = from + i + "abundance ".len();
        let run = text[start..]
            .find(|c| !CLASS(c))
            .unwrap_or(text.len() - start);
        if run > 0 && text[start + run..].starts_with(')') {
            let num = &text[start..start + run];
            return num.parse::<f64>().map_err(|_| {
                CampaignError::value(format!("could not convert string to float: '{num}'"))
            });
        }
        from = start;
    }
    Ok(0.0)
}

/// Every per-sample column (any field) of one sorbent, for every site of the campaign that the table has.
pub fn sorbent_columns(c: &Campaign, f: &Frame, sorbent: &str, pol: &str) -> Vec<String> {
    let mut fields: Vec<&str> = f
        .columns()
        .iter()
        .filter(|n| n.starts_with("datafile:"))
        .filter_map(|n| n.rsplit_once(':').map(|(_, field)| field))
        .collect();
    fields.sort_unstable();
    fields.dedup();
    let mut out = Vec::new();
    for (site, _) in &c.sites {
        for field in &fields {
            let name = c.col(site, sorbent, pol, field);
            if f.col(&name).is_some() {
                out.push(name);
            }
        }
    }
    out
}

fn ends_with(cols: &[String], suffix: &str) -> Vec<String> {
    cols.iter()
        .filter(|n| n.ends_with(suffix))
        .cloned()
        .collect()
}

/// Rows that carry a feature of this sorbent's list: a per-sample state other than `UNKNOWN`, or a positive area.
fn has_sorbent(c: &Campaign, f: &Frame, sorbent: &str, pol: &str) -> Vec<bool> {
    let cols = sorbent_columns(c, f, sorbent, pol);
    let states: Vec<usize> = ends_with(&cols, ":feature_state")
        .iter()
        .filter_map(|n| f.col(n))
        .collect();
    let areas: Vec<usize> = ends_with(&cols, ":area")
        .iter()
        .filter_map(|n| f.col(n))
        .collect();
    (0..f.len())
        .map(|r| {
            states
                .iter()
                .any(|&k| f.str_na(r, k).is_some_and(|s| s != "UNKNOWN"))
                || areas.iter().any(|&k| f.num(r, k).unwrap_or(0.0) > 0.0)
        })
        .collect()
}

/// `numpy.round(x, 3)` (half to even on the scaled value) with a missing value as -1, as the bits of the result.
fn fingerprint_cell(v: Option<f64>) -> u64 {
    let r = v.map_or(-1.0, |x| (x * 1000.0).round_ties_even() / 1000.0);
    (if r == 0.0 { 0.0 } else { r }).to_bits()
}

/// The per-sample areas of a row: the same in a per-sorbent list and in the merged table, because the aligner copies the list's features unchanged.
fn fingerprint(f: &Frame, r: usize, cols: &[usize]) -> Vec<u64> {
    cols.iter()
        .map(|&k| fingerprint_cell(f.num(r, k)))
        .collect()
}

fn column_numbers(f: &Frame, name: &str) -> Vec<f64> {
    let k = f.col(name);
    (0..f.len())
        .map(|r| k.and_then(|k| f.num(r, k)).unwrap_or(f64::NAN))
        .collect()
}

/// The blank of every row of a per-sorbent list.
fn list_blanks(lst: &Frame) -> Result<Vec<f64>> {
    let k = lst.col("blank_subtraction");
    (0..lst.len())
        .map(|r| {
            k.and_then(|k| lst.str_na(r, k))
                .map_or(Ok(0.0), blank_abundance)
        })
        .collect()
}

/// Adds `blank_abundance:<sorbent>` for every sorbent of the campaign (missing where the row has no feature of that sorbent). `warnings` receives the
/// texts of the warnings Attributio raises.
pub fn add_sorbent_blanks(
    c: &Campaign,
    f: &mut Frame,
    io: &Io,
    pol: &str,
    warnings: &mut Vec<String>,
) -> Result<()> {
    for sb in &c.sorbents {
        let values = if f.is_empty() {
            Vec::new()
        } else {
            sorbent_blank_column(c, f, io, pol, sb, warnings)?
        };
        f.add_column(&format!("blank_abundance:{sb}"), &values);
    }
    Ok(())
}

fn load_sorbent_list(c: &Campaign, io: &Io, pol: &str, sb: &str) -> Result<(String, Frame)> {
    let path = c.sorbent_table_path(pol, sb).unwrap_or_default();
    let text = io.text(&path).ok_or_else(|| {
        CampaignError::campaign(format!(
            "[{}] per-sorbent feature list not found: {path}",
            c.id
        ))
    })?;
    Ok((path, Frame::parse(&text, ',')?))
}

fn sorbent_blank_column(
    c: &Campaign,
    f: &Frame,
    io: &Io,
    pol: &str,
    sb: &str,
    warnings: &mut Vec<String>,
) -> Result<Vec<String>> {
    let (path, lst) = load_sorbent_list(c, io, pol, sb)?;
    let area_names: Vec<String> = ends_with(&sorbent_columns(c, f, sb, pol), ":area");
    let cols: Vec<&String> = area_names.iter().filter(|n| lst.col(n).is_some()).collect();
    if cols.is_empty() {
        return Err(CampaignError::campaign(format!(
            "[{}] {}: no sample area column of sorbent {sb}",
            c.id,
            file_name(&path)
        )));
    }
    let lst_cols: Vec<usize> = cols.iter().filter_map(|n| lst.col(n)).collect();
    let df_cols: Vec<usize> = cols.iter().filter_map(|n| f.col(n)).collect();
    let mut index: HashMap<Vec<u64>, Vec<usize>> = HashMap::new();
    for k in 0..lst.len() {
        index
            .entry(fingerprint(&lst, k, &lst_cols))
            .or_default()
            .push(k);
    }
    let (lmz, lrt) = (column_numbers(&lst, "mz"), column_numbers(&lst, "rt"));
    let (mz, rt) = (column_numbers(f, "mz"), column_numbers(f, "rt"));
    let lab = list_blanks(&lst)?;
    let has = has_sorbent(c, f, sb, pol);
    let mut out = vec![f64::NAN; f.len()];
    let mut unmatched = 0;
    for i in (0..f.len()).filter(|&i| has[i]) {
        let mut cand: Vec<usize> = index
            .get(&fingerprint(f, i, &df_cols))
            .cloned()
            .unwrap_or_default();
        if cand.is_empty() {
            cand = (0..lst.len())
                .filter(|&k| {
                    (lmz[k] - mz[i]).abs() / mz[i] * 1e6 <= c.twin_mz_ppm
                        && (lrt[k] - rt[i]).abs() <= c.twin_rt_min
                })
                .collect();
        }
        let best = cand.iter().copied().min_by(|&a, &b| {
            let key = |j: usize| ((lrt[j] - rt[i]).abs(), (lmz[j] - mz[i]).abs());
            key(a)
                .partial_cmp(&key(b))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        match best {
            Some(k) => out[i] = lab[k],
            None => unmatched += 1,
        }
    }
    if unmatched > 0 {
        warnings.push(format!(
            "[{}] {pol} {sb}: {unmatched} merged rows not found in {}; their blank falls back on the merged text",
            c.id,
            file_name(&path)
        ));
        let fb = list_blanks(f)?;
        for i in 0..f.len() {
            if has[i] && out[i].is_nan() {
                out[i] = fb[i];
            }
        }
    }
    Ok(numbers_as_text(&out))
}

/// The blank abundances as the text of the new column (missing = empty).
fn numbers_as_text(v: &[f64]) -> Vec<String> {
    v.iter()
        .map(|v| {
            if v.is_nan() {
                String::new()
            } else {
                format!("{v}")
            }
        })
        .collect()
}

/// The ids of the dropped rows and the row each one was joined to.
pub type TwinMap = Vec<(i64, i64)>;

fn candidate_pairs(
    f: &Frame,
    ia: &[usize],
    ib: &[usize],
    tol_p: f64,
    tol_r: f64,
) -> Vec<(f64, usize, usize)> {
    let (mz, rt) = (column_numbers(f, "mz"), column_numbers(f, "rt"));
    let mut cand = Vec::new();
    for &i in ia {
        for &j in ib {
            let dp = (mz[j] - mz[i]).abs() / mz[i] * 1e6;
            let dr = (rt[j] - rt[i]).abs();
            if dp <= tol_p && dr <= tol_r {
                cand.push((dp / tol_p + dr / tol_r, i, j));
            }
        }
    }
    cand.sort_by(|a, b| {
        a.0.partial_cmp(&b.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.1.cmp(&b.1))
            .then(a.2.cmp(&b.2))
    });
    cand
}

/// One-to-one pairs, best match first.
fn pair_up(cand: &[(f64, usize, usize)]) -> Vec<(usize, usize)> {
    let (mut used_a, mut used_b) = (HashSet::new(), HashSet::new());
    let mut pairs = Vec::new();
    for &(_, i, j) in cand {
        if !used_a.contains(&i) && !used_b.contains(&j) {
            used_a.insert(i);
            used_b.insert(j);
            pairs.push((i, j));
        }
    }
    pairs
}

fn max_area(f: &Frame, r: usize, cols: &[usize]) -> f64 {
    cols.iter()
        .filter_map(|&k| f.num(r, k))
        .fold(None, |m: Option<f64>, v| Some(m.map_or(v, |x| x.max(v))))
        .unwrap_or(0.0)
}

/// Joins the rows that are one compound split by the merge. The kept row is the one with more MS2 scans, then the larger area, then the lower id; it takes
/// the other row's per-sample columns of the other sorbent, its blank, the sum of the MS2 scans and `twin_id`. Returns the dropped id and the kept id.
pub fn collapse_twins(c: &Campaign, f: &mut Frame, pol: &str) -> Result<TwinMap> {
    if f.is_empty() {
        f.add_column("twin_id", &[]);
        return Ok(Vec::new());
    }
    if c.sorbents.len() < 2 {
        return Err(CampaignError::value("IndexError: list index out of range"));
    }
    let (a, b) = (&c.sorbents[0], &c.sorbents[1]);
    let (ca, cb) = (sorbent_columns(c, f, a, pol), sorbent_columns(c, f, b, pol));
    let (has_a, has_b) = (has_sorbent(c, f, a, pol), has_sorbent(c, f, b, pol));
    let ia: Vec<usize> = (0..f.len()).filter(|&r| has_a[r] && !has_b[r]).collect();
    let ib: Vec<usize> = (0..f.len()).filter(|&r| has_b[r] && !has_a[r]).collect();
    let pairs = pair_up(&candidate_pairs(f, &ia, &ib, c.twin_mz_ppm, c.twin_rt_min));
    if pairs.is_empty() {
        f.add_column("twin_id", &[]);
        return Ok(Vec::new());
    }
    let area_cols: Vec<usize> = ends_with(&ca, ":area")
        .iter()
        .chain(&ends_with(&cb, ":area"))
        .filter_map(|n| f.col(n))
        .collect();
    let fs_col = f.col("fragment_scans");
    let fs: Vec<f64> = (0..f.len())
        .map(|r| fs_col.and_then(|k| f.num(r, k)).unwrap_or(0.0))
        .collect();
    let areas: Vec<f64> = (0..f.len()).map(|r| max_area(f, r, &area_cols)).collect();
    let id_col = f
        .col("id")
        .ok_or_else(|| CampaignError::value("KeyError: 'id'"))?;
    let ids: Vec<i64> = (0..f.len())
        .map(|r| f.int(r, id_col).unwrap_or(0))
        .collect();
    let mut twin = vec![String::new(); f.len()];
    let mut drop = vec![false; f.len()];
    let mut mapping = Vec::new();
    for (i, j) in pairs {
        let key = |r: usize| (fs[r], areas[r], -(ids[r] as f64));
        let (ki, kj) = (key(i), key(j));
        let i_wins =
            ki.0 > kj.0 || (ki.0 == kj.0 && (ki.1 > kj.1 || (ki.1 == kj.1 && ki.2 >= kj.2)));
        let (keep, gone) = if i_wins { (i, j) } else { (j, i) };
        let (cols, sb_gone) = if gone == j { (&cb, b) } else { (&ca, a) };
        for name in cols {
            if let Some(k) = f.col(name) {
                let v = f.cell(gone, k).to_string();
                f.set(keep, k, &v);
            }
        }
        if let Some(k) = f.col(&format!("blank_abundance:{sb_gone}")) {
            let v = f.cell(gone, k).to_string();
            f.set(keep, k, &v);
        }
        if let Some(k) = fs_col {
            f.set(keep, k, &format!("{}", fs[i] + fs[j]));
        }
        twin[keep] = ids[gone].to_string();
        drop[gone] = true;
        mapping.push((ids[gone], ids[keep]));
    }
    f.add_column("twin_id", &twin);
    let keep: Vec<bool> = drop.iter().map(|d| !d).collect();
    f.retain_rows(&keep);
    Ok(mapping)
}

/// Rows of a `(pol, id)` source table moved from dropped twins to the kept row; when both rows of a pair have a hit, the best by `score` stays (the kept row's
/// own hit on ties). The rows keep their original order. `score` returns the sort keys of a row (missing last); each key sorts descending unless `ascending`.
pub fn remap_ids<T>(
    rows: Vec<T>,
    mapping: &[(i64, i64)],
    pol: &str,
    key: impl Fn(&T) -> (&str, i64),
    set_id: impl Fn(&mut T, i64),
    score: impl Fn(&T) -> Vec<Option<f64>>,
    ascending: bool,
) -> Vec<T> {
    let mut rows = rows;
    let mut from_twin = vec![false; rows.len()];
    let mut any = false;
    for (n, r) in rows.iter_mut().enumerate() {
        let (p, id) = key(r);
        if p != pol {
            continue;
        }
        if let Some((_, to)) = mapping.iter().find(|(from, _)| *from == id) {
            set_id(r, *to);
            from_twin[n] = true;
            any = true;
        }
    }
    if !any {
        return rows;
    }
    let mut order: Vec<usize> = (0..rows.len()).collect();
    let keys: Vec<Vec<Option<f64>>> = rows.iter().map(&score).collect();
    order.sort_by(|&x, &y| {
        for (a, b) in keys[x].iter().zip(&keys[y]) {
            let o = match (a, b) {
                (Some(p), Some(q)) => {
                    let o = p.partial_cmp(q).unwrap_or(std::cmp::Ordering::Equal);
                    if ascending {
                        o
                    } else {
                        o.reverse()
                    }
                }
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            };
            if o != std::cmp::Ordering::Equal {
                return o;
            }
        }
        from_twin[x].cmp(&from_twin[y])
    });
    let mut seen: HashSet<(String, i64)> = HashSet::new();
    let mut kept: Vec<usize> = order
        .into_iter()
        .filter(|&n| {
            let (p, id) = key(&rows[n]);
            seen.insert((p.to_string(), id))
        })
        .collect();
    kept.sort_unstable();
    let mut slots: Vec<Option<T>> = rows.into_iter().map(Some).collect();
    kept.into_iter().filter_map(|n| slots[n].take()).collect()
}
