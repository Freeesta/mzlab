//! The MS2 spectrum of a row with its two options (`ms2spec::spectra_with`): the consensus of the scans of the peak and the background of the blanks taken
//! out. The files are synthetic `.mzidx` written here; off, the spectrum is the one of the best scan (`ms2spec::spectra`).

use mzlab_core::idxread::Index;
use mzlab_core::mzidx::build_mzml_to_vec;
use mzlab_core::mzml::Spectrum;
use mzlab_core::mzmlwrite::{Header, MzmlWriter};
use mzlab_core::upload::prepare;
use mzlab_feat::evid::producers::Peaks;
use mzlab_feat::evid::rowfacts::row_facts;
use mzlab_feat::export::Ctx;
use mzlab_feat::export_ms::Ms2;
use mzlab_feat::hypo::{Dir, Hypothesis, Outcome, Registry, SRC_RULE};
use mzlab_feat::legacy::ms2::consensus;
use mzlab_feat::ms2spec::{spectra, spectra_with, SpecOpts};
use mzlab_feat::samples::propose;
use mzlab_feat::table::{Aligned, RowQ, CELL_DETECTED};

const PREC: f64 = 300.0;

/// An index with one MS2 scan per `(rt, peaks)`, all of the precursor `prec`, positive.
fn file(name: &str, prec: f64, scans: &[(f64, Vec<(f64, f32)>)]) -> Vec<u8> {
    let mut w = MzmlWriter::new(
        Vec::new(),
        &Header {
            run: name.into(),
            ..Default::default()
        },
    )
    .unwrap();
    w.open_spectra(scans.len()).unwrap();
    for (i, (rt, peaks)) in scans.iter().enumerate() {
        w.spectrum(&Spectrum {
            index: i,
            id: format!("scan={}", i + 1),
            level: 2,
            rt: *rt,
            polarity: 1,
            profile: false,
            filter: "+ c ESI Full ms2".into(),
            precursor: Some(prec),
            mz: peaks.iter().map(|p| p.0).collect(),
            intensity: peaks.iter().map(|p| p.1).collect(),
            wide: None,
        })
        .unwrap();
    }
    w.close_spectra().unwrap();
    let bytes = w.finish().unwrap();
    let mut src = bytes.clone();
    let p = prepare(&mut src, bytes.len() as u64, name).unwrap();
    build_mzml_to_vec(&p.table, &bytes, 64 << 20).unwrap().0
}

/// A table of one row (m/z `PREC`, peak between 4.9 and 5.1 min, best MS2 in file 0) seen in the files `seen` of 3.
fn table(seen: &[usize]) -> Aligned {
    let mut t = Aligned::with_capacity(3, 1);
    t.mz.push(PREC);
    t.rt.push(5.0);
    for k in 0..3 {
        let a = if seen.contains(&k) { 1e6 } else { 0.0 };
        t.area.push(a);
        t.height.push((a / 10.0) as f32);
        t.apex_rt.push(5.0);
        t.width.push(0.2);
        t.state.push(CELL_DETECTED);
    }
    t.meta.push(&RowQ {
        rt_lo: 4.9,
        rt_hi: 5.1,
        ms2_file: 0,
        ..RowQ::NONE
    });
    t
}

fn run(
    files: &[Vec<u8>],
    t: &Aligned,
    o: &SpecOpts,
    blanks: &[usize],
) -> (Option<Ms2>, mzlab_feat::ms2spec::SpecInfo) {
    let names: Vec<(String, String)> = (0..3)
        .map(|k| (format!("f{k}.mzML"), "00".repeat(32)))
        .collect();
    let s = propose(&names);
    let c = Ctx {
        table: t,
        samples: &s,
        view: None,
        table_sha256: "ab",
        polarity: 1,
    };
    let mut open = |k: usize| {
        let b = files[k].clone();
        let n = b.len() as u64;
        Index::open(b, n)
    };
    let (mut v, info) = spectra_with(&c, &mut open, o, blanks).unwrap();
    (v.remove(0), info)
}

/// Three scans of the row: the fragments 100, 150, 200 in all, a different noise peak in each; the middle one is the strongest.
fn sample_scans() -> Vec<(f64, Vec<(f64, f32)>)> {
    let common = |k: f32, noise: f64| {
        vec![
            (100.0, 1000.0 * k),
            (150.0, 500.0 * k),
            (200.0, 2000.0 * k),
            (noise, 300.0 * k),
        ]
    };
    vec![
        (4.95, common(1.0, 111.0)),
        (5.0, common(2.0, 122.0)),
        (5.05, common(1.0, 133.0)),
    ]
}

fn blank_scans(extra: &[(f64, f32)]) -> Vec<(f64, Vec<(f64, f32)>)> {
    let p: Vec<(f64, f32)> = extra.to_vec();
    (0..3).map(|i| (4.9 + 0.1 * i as f64, p.clone())).collect()
}

#[test]
fn consensus_keeps_the_common_fragments_and_drops_the_noise() {
    let sc = sample_scans();
    let files = vec![
        file("a", PREC, &sc),
        file("b", PREC, &[]),
        file("c", PREC, &[]),
    ];
    let t = table(&[0]);
    let best = run(&files, &t, &SpecOpts::default(), &[]).0.unwrap();
    assert!(best.mz.contains(&122.0), "the best scan has its own noise");
    let cons = run(
        &files,
        &t,
        &SpecOpts {
            consensus: true,
            ..Default::default()
        },
        &[],
    )
    .0
    .unwrap();
    assert_eq!(cons.mz, vec![100.0, 150.0, 200.0]);
    assert_eq!(cons.files, 1);
    assert_eq!(cons.prec, PREC);
    // the same numbers as the Dig HR consensus of the same scans
    let arrays: Vec<(Vec<f64>, Vec<f64>)> = sc
        .iter()
        .map(|s| {
            (
                s.1.iter().map(|p| p.0).collect(),
                s.1.iter().map(|p| f64::from(p.1)).collect(),
            )
        })
        .collect();
    let legacy = consensus(&arrays);
    assert_eq!(cons.mz.len(), legacy.mz.len());
    for i in 0..cons.mz.len() {
        assert!(
            (cons.mz[i] - legacy.mz[i]).abs() < 1e-9 && (cons.it[i] - legacy.rel[i]).abs() < 1e-9
        );
    }
}

#[test]
fn the_default_options_give_the_spectrum_of_the_best_scan() {
    let files = vec![
        file("a", PREC, &sample_scans()),
        file("b", PREC, &[]),
        file("c", PREC, &[]),
    ];
    let t = table(&[0]);
    let names: Vec<(String, String)> = (0..3)
        .map(|k| (format!("f{k}.mzML"), "00".repeat(32)))
        .collect();
    let s = propose(&names);
    let c = Ctx {
        table: &t,
        samples: &s,
        view: None,
        table_sha256: "ab",
        polarity: 1,
    };
    let old = spectra(&c, |k| {
        let b = files[k].clone();
        let n = b.len() as u64;
        Index::open(b, n)
    })
    .unwrap();
    assert_eq!(old[0], run(&files, &t, &SpecOpts::default(), &[]).0);
    assert_eq!(
        old[0].as_ref().unwrap().mz,
        vec![100.0, 122.0, 150.0, 200.0]
    );
}

#[test]
fn background_takes_the_blank_fragments_out_and_keeps_the_precursor() {
    // 120 and 180 recur in the blank; so does 300.0 (the precursor)
    let bg = [(120.0, 20000.0), (180.0, 10000.0), (300.0, 9000.0)];
    let mut sc = sample_scans();
    for s in &mut sc {
        s.1.extend([(120.0, 4000.0), (180.0, 3000.0), (300.0, 800.0)]);
    }
    let files = vec![
        file("a", PREC, &sc),
        file("b", PREC, &[]),
        file("c", PREC, &blank_scans(&bg)),
    ];
    let t = table(&[0]);
    let off = run(&files, &t, &SpecOpts::default(), &[2]).0.unwrap();
    assert!(off.mz.contains(&120.0) && off.mz.contains(&180.0));
    let (on, info) = run(
        &files,
        &t,
        &SpecOpts {
            background: true,
            ..Default::default()
        },
        &[2],
    );
    let on = on.unwrap();
    assert_eq!(info.bg, "applied");
    assert_eq!(info.lost, vec![2]);
    assert!(!on.mz.contains(&120.0) && !on.mz.contains(&180.0));
    assert!(on.mz.contains(&300.0), "the precursor stays");
    assert!(on.mz.contains(&100.0) && on.mz.contains(&200.0));
    assert_eq!(on.mz.len(), on.it.len());
}

#[test]
fn background_without_blanks_leaves_the_spectrum_alone() {
    let files = vec![
        file("a", PREC, &sample_scans()),
        file("b", PREC, &[]),
        file("c", PREC, &[]),
    ];
    let t = table(&[0]);
    let plain = run(&files, &t, &SpecOpts::default(), &[]).0;
    let (asked, info) = run(
        &files,
        &t,
        &SpecOpts {
            background: true,
            ..Default::default()
        },
        &[],
    );
    assert_eq!(asked, plain);
    assert_eq!(info.bg, "no_blank");
    assert_eq!(info.lost, vec![0]);
}

#[test]
fn the_unit_profile_matches_the_background_within_half_a_dalton() {
    // the blank has 120.3, the sample 120.0
    let mut sc = sample_scans();
    for s in &mut sc {
        s.1.push((120.0, 4000.0));
    }
    let files = vec![
        file("a", PREC, &sc),
        file("b", PREC, &[]),
        file("c", PREC, &blank_scans(&[(120.3, 20000.0)])),
    ];
    let t = table(&[0]);
    let hr = run(
        &files,
        &t,
        &SpecOpts {
            background: true,
            ..Default::default()
        },
        &[2],
    )
    .0
    .unwrap();
    assert!(hr.mz.contains(&120.0), "0.3 Da is far at high resolution");
    let unit = run(
        &files,
        &t,
        &SpecOpts {
            background: true,
            unit: true,
            ..Default::default()
        },
        &[2],
    )
    .0
    .unwrap();
    assert!(!unit.mz.contains(&120.0));
}

#[test]
fn consensus_goes_on_across_the_files_where_the_row_has_a_peak() {
    // file 0: fragment 100 and 200 in all scans; file 1: 100 and 150; across the two files only 100 is in both (the others in 1 of 2 = 50 % >= 40 %)
    let f0: Vec<(f64, Vec<(f64, f32)>)> = (0..3)
        .map(|i| {
            (
                4.95 + 0.05 * i as f64,
                vec![(100.0, 1000.0), (200.0, 800.0)],
            )
        })
        .collect();
    let f1: Vec<(f64, Vec<(f64, f32)>)> = (0..3)
        .map(|i| (4.95 + 0.05 * i as f64, vec![(100.0, 900.0), (150.0, 400.0)]))
        .collect();
    let files = vec![
        file("a", PREC, &f0),
        file("b", PREC, &f1),
        file("c", PREC, &[]),
    ];
    let t = table(&[0, 1]);
    let m = run(
        &files,
        &t,
        &SpecOpts {
            consensus: true,
            ..Default::default()
        },
        &[],
    )
    .0
    .unwrap();
    assert_eq!(m.files, 2);
    assert_eq!(m.mz[0], 100.0);
    assert!((m.it.iter().copied().fold(0.0, f64::max) - 100.0).abs() < 1e-9);
}

/// Two rows: the parent (m/z 300, RT 5, best MS2 in file 0) and a product (m/z 316, RT 4, best MS2 in file 1); file 2 is the blank.
fn parent_and_product() -> Aligned {
    let mut t = Aligned::with_capacity(3, 2);
    for (mz, rt, file) in [(300.0, 5.0, 0u32), (316.0, 4.0, 1)] {
        t.mz.push(mz);
        t.rt.push(rt);
        for k in 0..3 {
            let a = if k == file { 1e6 } else { 0.0 };
            t.area.push(a);
            t.height.push((a / 10.0) as f32);
            t.apex_rt.push(rt as f32);
            t.width.push(0.2);
            t.state.push(CELL_DETECTED);
        }
        t.meta.push(&RowQ {
            rt_lo: (rt - 0.1) as f32,
            rt_hi: (rt + 0.1) as f32,
            ms2_file: file,
            ..RowQ::NONE
        });
    }
    t
}

fn hypothesis(mz: f64) -> Hypothesis {
    Hypothesis {
        id: 1,
        sources: SRC_RULE,
        label: "r".into(),
        formula: String::new(),
        neutral: mz - 1.00727646688,
        mz,
        adduct: "[M+H]+",
        polarity: 1,
        steps: 1,
        level: 3,
        dir: Dir::Free,
        rt_ref: None,
        alternatives: Vec::new(),
    }
}

/// The network share of the product (row 1) with the spectra from `spectra_with`; the blank holds the background fragments 150 and 200.
fn network_of(product: &[(f64, f32)], o: &SpecOpts) -> Option<f64> {
    let parent = [(250.0, 900.0), (200.0, 800.0), (150.0, 700.0)];
    let scan = |rt: f64, p: &[(f64, f32)]| (rt, p.to_vec());
    let files = vec![
        file(
            "a",
            300.0,
            &[scan(4.95, &parent), scan(5.0, &parent), scan(5.05, &parent)],
        ),
        file(
            "b",
            316.0,
            &[scan(3.95, product), scan(4.0, product), scan(4.05, product)],
        ),
        file(
            "c",
            PREC,
            &blank_scans(&[(150.0, 20000.0), (200.0, 15000.0)]),
        ),
    ];
    let t = parent_and_product();
    let s = propose(
        &(0..3)
            .map(|k| (format!("f{k}.mzML"), "00".repeat(32)))
            .collect::<Vec<_>>(),
    );
    let c = Ctx {
        table: &t,
        samples: &s,
        view: None,
        table_sha256: "ab",
        polarity: 1,
    };
    let mut open = |k: usize| {
        let b = files[k].clone();
        let n = b.len() as u64;
        Index::open(b, n)
    };
    let (sp, _) = spectra_with(&c, &mut open, o, &[2]).unwrap();
    let spec: Vec<Option<(Peaks, f32)>> = sp
        .into_iter()
        .map(|m| m.map(|m| (Peaks { mz: m.mz, it: m.it }, 0.9)))
        .collect();
    let reg = Registry {
        hyps: vec![hypothesis(300.0), hypothesis(316.0)],
        outcomes: vec![Outcome::NotSearched; 2],
        rules_version: 1,
        polarity: 1,
        suspects_cut: 0,
    };
    row_facts(&t, &reg, Some((300.0, 5.0)), &spec, &[], false).0[1].network
}

#[test]
fn a_decoy_that_shares_only_background_fragments_loses_its_network_and_a_true_product_keeps_it() {
    let on = SpecOpts {
        background: true,
        ..Default::default()
    };
    // the decoy shares with the parent only 150 and 200, which the blank holds too
    let decoy = [(200.0, 800.0), (150.0, 700.0)];
    assert!(network_of(&decoy, &SpecOpts::default()).is_some_and(|n| n > 0.0));
    assert!(network_of(&decoy, &on).is_none_or(|n| n == 0.0));
    // the product shifted by 16 from the parent: 266 and 216 are not background
    let product = [(266.0, 900.0), (216.0, 800.0), (150.0, 700.0)];
    assert!(network_of(&product, &SpecOpts::default()).is_some_and(|n| n > 0.0));
    assert!(network_of(&product, &on).is_some_and(|n| n > 0.0));
}
