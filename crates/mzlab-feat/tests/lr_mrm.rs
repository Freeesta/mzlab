//! The pipeline on an MRM series (`mrm.rs`): six calibration levels and a sample, built here as mzML files with SRM chromatograms and read back with
//! `read_srm`. A quantifier grows with the concentration, its qualifier is 40 % of it, one transition does not depend on it and is too weak to be picked in one
//! file (its cell is integrated). The calibration table is long and ready for the line; the line is never computed. Synthetic data only.

use mzlab_core::mzml::{ChromKind, Chromatogram};
use mzlab_core::mzmlhead::Mzml;
use mzlab_core::mzmlwrite::{Header, MzmlWriter};
use mzlab_feat::export::Ctx;
use mzlab_feat::mrm::{
    align, csv_calibration, detect, fill_file, qualifier_ratios, read_srm, MrmParams, Srm,
};
use mzlab_feat::recipe::Recipe;
use mzlab_feat::samples::Sample;
use mzlab_feat::store::{read_file, read_table, write_file, write_table, Origin};
use mzlab_feat::table::{CELL_DETECTED, CELL_FILLED};

const CONC: [f64; 7] = [1.0, 2.0, 5.0, 10.0, 20.0, 50.0, 7.0];
/// Transitions: (Q1, Q3, RT, amplitude per unit of concentration, depends on the concentration).
const TR: [(f64, f64, f64, f64, bool); 3] = [
    (263.1, 181.1, 5.0, 60.0, true),
    (263.1, 152.0, 5.02, 24.0, true),
    (300.2, 120.1, 8.0, 400.0, false),
];

struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
}

fn gauss(t: f64, c: f64, s: f64, a: f64) -> f64 {
    a * (-0.5 * ((t - c) / s).powi(2)).exp()
}

/// The SRM chromatograms of file `k`: the quantifier of the lowest level is below the threshold, the flat transition is weak in file 3.
fn chromatograms(k: usize) -> Vec<Srm> {
    let mut rng = Lcg(5 + k as u64);
    let t: Vec<f64> = (0..600).map(|i| 0.5 + 0.02 * i as f64).collect();
    TR.iter()
        .map(|&(q1, q3, rt, amp, dep)| {
            let a = if dep {
                amp * CONC[k]
            } else if k == 3 {
                30.0
            } else {
                amp
            };
            let a = if dep && k == 0 && q3 > 170.0 { 4.0 } else { a };
            let y = t
                .iter()
                .map(|&x| gauss(x, rt, 0.06, a) + 2.0 * rng.next())
                .collect();
            Srm {
                q1,
                q3,
                name: String::new(),
                t: t.clone(),
                y,
            }
        })
        .collect()
}

fn params() -> MrmParams {
    MrmParams {
        min_height: 50.0,
        ..MrmParams::default()
    }
}

#[test]
fn the_srm_chromatograms_are_read_from_the_mzml() {
    let mut w = MzmlWriter::new(
        Vec::new(),
        &Header {
            run: "mrm".into(),
            ..Default::default()
        },
    )
    .unwrap();
    let src = chromatograms(2);
    w.open_chromatograms(src.len()).unwrap();
    for (i, s) in src.iter().enumerate() {
        let c = Chromatogram {
            index: i,
            id: format!(
                "SRM SIC Q1={} Q3={} sample=1 period=1 experiment={}",
                s.q1,
                s.q3,
                i + 1
            ),
            kind: ChromKind::Srm,
            q1: Some(s.q1),
            q3: Some(s.q3),
            unit: "number of detector counts".into(),
            time: s.t.clone(),
            intensity: s.y.clone(),
        };
        w.chromatogram(&c).unwrap();
    }
    w.close_chromatograms().unwrap();
    let bytes = w.finish().unwrap();
    let size = bytes.len() as u64;
    let got = read_srm(&mut Mzml::new(bytes, size), 0).unwrap();
    assert_eq!(got.len(), 3);
    assert_eq!(
        (got[0].q1, got[0].q3),
        (263.1, 152.0),
        "ordered by Q1 then Q3"
    );
    assert_eq!(got[1].y.len(), 600);
}

#[test]
fn a_series_is_aligned_by_transition_and_the_cells_follow_the_concentration() {
    let files: Vec<Vec<Srm>> = (0..CONC.len()).map(chromatograms).collect();
    let feats: Vec<_> = files.iter().map(|f| detect(f, &params())).collect();
    let refs: Vec<_> = feats.iter().collect();
    let mut t = align(&refs, &params());
    assert_eq!(t.len(), 3, "one row per transition");
    assert_eq!((t.mz[0], t.q3[0]), (263.1, 152.0));
    let nf = t.n_files;
    // the quantifier is detected in every level above the lowest, with an area that grows with the concentration
    let q = (0..t.len()).find(|&r| t.q3[r] == 181.1).unwrap();
    let levels: Vec<usize> = (1..6).collect();
    assert!(levels.iter().all(|&k| t.state[q * nf + k] == CELL_DETECTED));
    assert!(levels
        .windows(2)
        .all(|w| t.area[q * nf + w[1]] > t.area[q * nf + w[0]]));
    // the flat transition is weak in file 3: not picked there, integrated in the pass of the gaps
    let flat = (0..t.len()).find(|&r| t.q3[r] == 120.1).unwrap();
    assert_ne!(t.state[flat * nf + 3], CELL_DETECTED);
    let n = fill_file(&mut t, 3, &files[3], &params());
    assert!(n >= 1 && t.state[flat * nf + 3] == CELL_FILLED && t.area[flat * nf + 3] > 0.0);
    // the qualifier over the quantifier is the same at every level (0.4), the quantifier is the larger of the pair
    let ratios = qualifier_ratios(&t);
    assert_eq!(ratios.len(), 1);
    assert_eq!(ratios[0].quantifier, q);
    for k in 2..6 {
        assert!(
            (ratios[0].ratio[k] - 0.4).abs() < 0.05,
            "ratio {} at level {k}",
            ratios[0].ratio[k]
        );
    }
}

#[test]
fn the_calibration_table_is_long_and_has_no_line() {
    let files: Vec<Vec<Srm>> = (0..CONC.len()).map(chromatograms).collect();
    let feats: Vec<_> = files.iter().map(|f| detect(f, &params())).collect();
    let refs: Vec<_> = feats.iter().collect();
    let t = align(&refs, &params());
    let samples: Vec<Sample> = (0..CONC.len())
        .map(|k| Sample {
            file: format!("cal{k}.mzML"),
            sha256: String::new(),
            kind: if k < 6 { "calibration" } else { "sample" }.into(),
            time_min: None,
            replicate: 1,
            group: String::new(),
            conc: (k < 6).then_some(CONC[k]),
            unit: (k < 6).then(|| "ng/mL".to_string()),
            order: k as u32 + 1,
            istd: false,
            note: String::new(),
        })
        .collect();
    let c = Ctx {
        table: &t,
        samples: &samples,
        view: None,
        table_sha256: "ab",
        polarity: 1,
    };
    let csv = csv_calibration(&c);
    let lines: Vec<&str> = csv.lines().collect();
    assert_eq!(
        lines[0],
        "transition,q1,q3,sample,type,concentration,unit,rt,area,height,state"
    );
    assert_eq!(
        lines.len(),
        1 + 3 * 6,
        "three transitions by six calibration files"
    );
    assert!(lines
        .iter()
        .any(|l| l.starts_with("263.1>181.1,263.1,181.1,\"cal5\",calibration,50,ng/mL,")));
    assert!(!csv.contains("slope") && !csv.contains("r2"));
}

#[test]
fn two_runs_give_the_same_bytes() {
    let go = || {
        let files: Vec<Vec<Srm>> = (0..CONC.len()).map(chromatograms).collect();
        let feats: Vec<_> = files.iter().map(|f| detect(f, &params())).collect();
        let bytes: Vec<u8> = feats.iter().flat_map(|f| f.to_bytes()).collect();
        let refs: Vec<_> = feats.iter().collect();
        let t = align(&refs, &params());
        (bytes, t)
    };
    let (a, ta) = go();
    let (b, tb) = go();
    assert_eq!(a, b);
    assert_eq!(format!("{ta:?}"), format!("{tb:?}"));
}

#[test]
fn the_q3_survives_the_files() {
    let files: Vec<Vec<Srm>> = (0..CONC.len()).map(chromatograms).collect();
    let feats: Vec<_> = files.iter().map(|f| detect(f, &params())).collect();
    let (back, _) = read_file(&write_file(
        &feats[2],
        &Recipe::default(),
        &Origin::default(),
    ))
    .unwrap();
    assert_eq!(back.q3, feats[2].q3);
    assert_eq!(back.mz, feats[2].mz);
    let refs: Vec<_> = feats.iter().collect();
    let t = align(&refs, &params());
    let (tb, _) = read_table(&write_table(&t, &Recipe::default(), &Origin::default())).unwrap();
    assert_eq!(tb.q3, t.q3);
}

#[test]
fn the_series_goes_through_the_files_and_the_table() {
    use mzlab_feat::mrm::{file_bytes, table_bytes};
    let p = params();
    let tables: Vec<_> = (0..CONC.len())
        .map(|k| {
            read_file(&file_bytes(
                &chromatograms(k),
                &format!("f{k}.mzML"),
                &format!("{k:064x}"),
                &p,
            ))
            .unwrap()
            .0
        })
        .collect();
    let bytes = table_bytes(&tables, &Origin::default(), &p, |k| Ok(chromatograms(k))).unwrap();
    let (t, _) = read_table(&bytes).unwrap();
    assert_eq!((t.len(), t.n_files), (3, 7));
    assert_eq!(t.q3, [152.0, 181.1, 120.1]);
    let flat = 2 * t.n_files + 3;
    assert_eq!(
        t.state[flat], CELL_FILLED,
        "the weak cell of the flat transition is integrated"
    );
    assert_eq!(
        bytes,
        table_bytes(&tables, &Origin::default(), &p, |k| Ok(chromatograms(k))).unwrap()
    );
}
