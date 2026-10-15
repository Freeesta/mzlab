//! Join aligner against the real series: the per-file lists of mzmine 3.9.0 (`risolutore_<file>.mzML_*.csv`) are aligned by `untargeted::join` and the
//! rows are compared with the table that mzmine aligned (`join_Aligned_feature_list.csv`, before the gap filling). Real data never enter the public
//! repository: the directory of the exports of one run of `tools/bench/motivi_mzmine.py` (its `mzmine/` folder) is `MZLAB_MZMINE_FASI`, else
//! `$MZLAB_DATI/HRMS/mzmine39_fasi/`. Without it the test is skipped. Threshold: at least 0.97 of the rows with the same set of cells.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use mzlab_feat::untargeted::join::{align, JoinParams};
use mzlab_feat::untargeted::Feature;

fn table(path: &std::path::Path) -> (Vec<String>, Vec<Vec<String>>) {
    let text = std::fs::read_to_string(path).unwrap();
    let mut rows = text.lines().map(|l| {
        l.split(',')
            .map(|x| x.trim_matches('"').to_string())
            .collect::<Vec<_>>()
    });
    (rows.next().unwrap(), rows.collect())
}

fn dir() -> Option<PathBuf> {
    let d = std::env::var_os("MZLAB_MZMINE_FASI")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("MZLAB_DATI").map(|p| PathBuf::from(p).join("HRMS/mzmine39_fasi"))
        })?;
    d.is_dir().then_some(d)
}

fn key(file: &str, mz: f64, rt: f64) -> String {
    format!("{file}|{mz:.5}|{rt:.3}")
}

/// The per-file lists of mzmine (`risolutore_<stem>.mzML_*.csv`) of the files `stems`.
fn load_lists(d: &std::path::Path, stems: &[String]) -> Vec<Vec<Feature>> {
    let mut lists: Vec<Vec<Feature>> = Vec::new();
    for s in stems {
        let p = std::fs::read_dir(&d)
            .unwrap()
            .flatten()
            .map(|e| e.path())
            .find(|p| {
                p.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with(&format!("risolutore_{s}.mzML_"))
            })
            .expect("list of the file");
        let (h, r) = table(&p);
        let c = |n: &str| h.iter().position(|x| x == n).unwrap();
        lists.push(
            r.iter()
                .map(|x| Feature {
                    mz: x[c("mz")].parse().unwrap(),
                    rt: x[c("rt")].parse().unwrap(),
                    rt_start: x[c("rt_range:min")].parse().unwrap(),
                    rt_end: x[c("rt_range:max")].parse().unwrap(),
                    height: x[c("height")].parse().unwrap(),
                    area: x[c("area")].parse().unwrap(),
                    filled: false,
                })
                .collect(),
        );
    }
    lists
}

#[test]
fn rows_like_mzmine_on_the_real_series() {
    let Some(d) = dir() else {
        eprintln!("SKIP untargeted_join_dati: no MZLAB_MZMINE_FASI / MZLAB_DATI");
        return;
    };
    let (head, rows) = table(&d.join("join_Aligned_feature_list.csv"));
    let col = |n: &str| {
        head.iter()
            .position(|h| h == n)
            .unwrap_or_else(|| panic!("column {n}"))
    };
    let stems: Vec<String> = head
        .iter()
        .filter_map(|h| {
            h.strip_prefix("datafile:")
                .and_then(|s| s.strip_suffix(".mzML:area"))
        })
        .map(String::from)
        .collect();
    let lists = load_lists(&d, &stems);
    let ours: Vec<BTreeSet<String>> = align(&lists, &JoinParams::default())
        .rows
        .iter()
        .map(|r| {
            r.cells
                .iter()
                .enumerate()
                .filter_map(|(k, c)| c.as_ref().map(|f| key(&stems[k], f.mz, f.rt)))
                .collect()
        })
        .collect();
    let mut gold: BTreeMap<usize, BTreeSet<String>> = BTreeMap::new();
    for (i, r) in rows.iter().enumerate() {
        for s in &stems {
            let (a, mz, rt) = (
                &r[col(&format!("datafile:{s}.mzML:area"))],
                &r[col(&format!("datafile:{s}.mzML:mz"))],
                &r[col(&format!("datafile:{s}.mzML:rt"))],
            );
            if let (Ok(_), Ok(mz), Ok(rt)) =
                (a.parse::<f64>(), mz.parse::<f64>(), rt.parse::<f64>())
            {
                gold.entry(i).or_default().insert(key(s, mz, rt));
            }
        }
    }
    let set: BTreeSet<&BTreeSet<String>> = ours.iter().collect();
    let same = gold.values().filter(|g| set.contains(g)).count();
    let share = same as f64 / gold.len().max(1) as f64;
    assert!(
        share >= 0.97,
        "rows with the same cells as mzmine: {same} of {} ({share:.3})",
        gold.len()
    );
}
