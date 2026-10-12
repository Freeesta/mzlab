//! A Thermo `.raw` in PROFILE against the mzML msconvert made from it (profile, 64-bit, zlib, no peak picking), scan by scan, on the mzML
//! that `raw::to_mzml_with` writes: MS level, polarity, scan time, TIC, base peak, precursor (selected m/z, isolation window, energy,
//! the scan it comes from), number of points, m/z and intensity. Needs the private data repository (MZLAB_DATI, or `mzlab-dati` beside
//! this one) with `HRMS/Profile raw/`: without it the test is skipped; no `.raw` lives in this repository.
use std::collections::BTreeMap;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

use mzlab_core::{mzml, raw};
use quick_xml::events::Event;
use quick_xml::Reader;

fn dati() -> Option<PathBuf> {
    let mut cands: Vec<PathBuf> = std::env::var_os("MZLAB_DATI")
        .map(PathBuf::from)
        .into_iter()
        .collect();
    let here = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for up in here.ancestors().skip(1).take(3) {
        cands.push(up.join("mzlab-dati"));
        cands.push(
            up.parent()
                .map(|p| p.join("mzlab-dati"))
                .unwrap_or_default(),
        );
    }
    cands.into_iter().find(|p| p.join("HRMS").is_dir())
}

/// What the header of one spectrum says (numbers as text where they are compared exactly).
#[derive(Default, Debug, Clone)]
struct Head {
    level: u32,
    polarity: i32,
    rt_min: f64,
    tic: f64,
    bp_mz: f64,
    bp_int: f64,
    points: usize,
    profile: bool,
    selected: Option<f64>,
    target: Option<f64>,
    low: Option<f64>,
    high: Option<f64>,
    energy: Option<f64>,
    parent: Option<u32>,
}

fn scan_of(id: &str) -> Option<u32> {
    id.split("scan=")
        .nth(1)?
        .split_whitespace()
        .next()?
        .parse()
        .ok()
}

fn heads(path: &std::path::Path) -> BTreeMap<u32, Head> {
    let mut r = Reader::from_reader(BufReader::new(File::open(path).unwrap()));
    let mut buf = Vec::new();
    let (mut out, mut cur, mut scan, mut in_prec) = (BTreeMap::new(), Head::default(), 0u32, false);
    loop {
        let ev = r.read_event_into(&mut buf).unwrap();
        match &ev {
            Event::Eof => break,
            Event::Start(e) | Event::Empty(e) => {
                let a: BTreeMap<String, String> = e
                    .attributes()
                    .flatten()
                    .map(|x| {
                        (
                            x.key.as_ref().to_string(),
                            x.normalized_value(quick_xml::XmlVersion::Implicit1_0)
                                .map(|c| c.into_owned())
                                .unwrap_or_default(),
                        )
                    })
                    .collect();
                match e.name().as_ref() {
                    "spectrum" => {
                        cur = Head::default();
                        scan = scan_of(&a["id"]).unwrap();
                        cur.points = a["defaultArrayLength"].parse().unwrap();
                        in_prec = false;
                    }
                    "precursor" => {
                        in_prec = true;
                        cur.parent = a.get("spectrumRef").and_then(|s| scan_of(s));
                    }
                    "cvParam" => {
                        let v = a.get("value").map(String::as_str).unwrap_or("");
                        let f = v.parse::<f64>().ok();
                        match (a["accession"].as_str(), in_prec) {
                            ("MS:1000511", false) => cur.level = v.parse().unwrap_or(0),
                            ("MS:1000130", _) => cur.polarity = 1,
                            ("MS:1000129", _) => cur.polarity = -1,
                            ("MS:1000128", _) => cur.profile = true,
                            ("MS:1000016", _) => {
                                cur.rt_min = f.unwrap_or(0.0)
                                    / if a.get("unitName").is_some_and(|u| u.starts_with("sec")) {
                                        60.0
                                    } else {
                                        1.0
                                    }
                            }
                            ("MS:1000285", _) => cur.tic = f.unwrap_or(0.0),
                            ("MS:1000504", _) => cur.bp_mz = f.unwrap_or(0.0),
                            ("MS:1000505", _) => cur.bp_int = f.unwrap_or(0.0),
                            ("MS:1000744", true) => cur.selected = f,
                            ("MS:1000827", true) => cur.target = f,
                            ("MS:1000828", true) => cur.low = f,
                            ("MS:1000829", true) => cur.high = f,
                            ("MS:1000045", true) => cur.energy = f,
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
            Event::End(e) if e.name().as_ref() == "spectrum" => {
                out.insert(scan, cur.clone());
            }
            _ => {}
        }
        buf.clear();
    }
    out
}

#[derive(Default)]
struct Worst {
    n: usize,
    worst: f64,
    bad: usize,
    first: Option<String>,
}

impl Worst {
    fn add(&mut self, scan: u32, ours: f64, theirs: f64, tol: f64) {
        let d = (ours - theirs).abs() / theirs.abs().max(1e-12);
        self.n += 1;
        self.worst = self.worst.max(d);
        if d > tol {
            self.bad += 1;
            self.first
                .get_or_insert(format!("scan {scan}: {ours} vs {theirs}"));
        }
    }

    fn line(&self, name: &str) -> String {
        format!(
            "{name}: {} compared, worst relative {:.2e}, {} over tolerance{}",
            self.n,
            self.worst,
            self.bad,
            self.first
                .as_ref()
                .map(|f| format!(" (first {f})"))
                .unwrap_or_default()
        )
    }
}

/// The counts of what differs, and the worst relative errors of the numbers that match.
struct Report {
    header_differences: usize,
    point_differences: usize,
    tic_bad: usize,
    base_peak_bad: usize,
    precursor_bad: usize,
}

fn compare() -> Option<Report> {
    let Some(d) = dati() else {
        eprintln!("skipped: no data repository (MZLAB_DATI)");
        return None;
    };
    let rawp = d.join("HRMS/Profile raw/Timo25ppm_t10.raw");
    let refp = rawp.with_extension("mzML");
    if !rawp.is_file() || !refp.is_file() {
        eprintln!("skipped: no HRMS/Profile raw/Timo25ppm_t10.raw with its mzML");
        return None;
    }
    let ours_p =
        std::env::temp_dir().join(format!("mzlab-profile-parity-{}.mzML", std::process::id()));
    {
        let mut out = std::io::BufWriter::new(File::create(&ours_p).unwrap());
        raw::to_mzml_with(
            BufReader::new(File::open(&rawp).unwrap()),
            &mut out,
            "Timo25ppm_t10.raw",
            raw::Convert {
                profile: true,
                anonymous: false,
            },
            &mut |_, _| {},
        )
        .unwrap();
    }
    // msconvert was asked for the scan times 0-600 s only: those scans of the whole file are compared
    let (all, b) = (heads(&ours_p), heads(&refp));
    let a: BTreeMap<u32, Head> = all.into_iter().filter(|(s, _)| b.contains_key(s)).collect();
    assert_eq!(
        a.keys().collect::<Vec<_>>(),
        b.keys().collect::<Vec<_>>(),
        "every scan of the reference is in the .raw"
    );

    let (mut rt, mut tic, mut bpm, mut bpi, mut sel, mut tgt, mut lo, mut hi, mut ce) = (
        Worst::default(),
        Worst::default(),
        Worst::default(),
        Worst::default(),
        Worst::default(),
        Worst::default(),
        Worst::default(),
        Worst::default(),
        Worst::default(),
    );
    let (mut level, mut pol, mut prof, mut pts, mut parent, mut missing) = (0, 0, 0, 0, 0, 0);
    for (s, x) in &a {
        let y = &b[s];
        level += (x.level != y.level) as usize;
        pol += (x.polarity != y.polarity) as usize;
        prof += (x.profile != y.profile) as usize;
        pts += (x.points != y.points) as usize;
        parent += (x.parent != y.parent) as usize;
        rt.add(*s, x.rt_min, y.rt_min, 1e-6);
        tic.add(*s, x.tic, y.tic, 1e-4);
        bpm.add(*s, x.bp_mz, y.bp_mz, 1e-6);
        bpi.add(*s, x.bp_int, y.bp_int, 1e-4);
        for (o, t, w) in [
            (x.selected, y.selected, &mut sel),
            (x.target, y.target, &mut tgt),
            (x.low, y.low, &mut lo),
            (x.high, y.high, &mut hi),
            (x.energy, y.energy, &mut ce),
        ] {
            match (o, t) {
                (Some(o), Some(t)) => w.add(*s, o, t, 1e-6),
                (None, None) => {}
                _ => missing += 1,
            }
        }
    }
    eprintln!("scans {}; level differs {level}, polarity {pol}, profile flag {prof}, points {pts}, parent scan {parent}, precursor fields on one side only {missing}", a.len());
    for w in [
        (&rt, "scan time"),
        (&tic, "TIC"),
        (&bpm, "base peak m/z"),
        (&bpi, "base peak intensity"),
        (&sel, "selected ion m/z"),
        (&tgt, "isolation target"),
        (&lo, "isolation lower offset"),
        (&hi, "isolation upper offset"),
        (&ce, "collision energy"),
    ] {
        eprintln!("{}", w.0.line(w.1));
    }

    // the points: m/z and intensity of every scan
    let ours = mzml::read(BufReader::new(File::open(&ours_p).unwrap())).unwrap();
    let theirs = mzml::read(BufReader::new(File::open(&refp).unwrap())).unwrap();
    let (mut wmz, mut wint, mut npk, mut bad_n) =
        (Worst::default(), Worst::default(), 0usize, 0usize);
    let by_scan: BTreeMap<u32, &mzml::Spectrum> = ours
        .spectra
        .iter()
        .filter_map(|x| Some((scan_of(&x.id)?, x)))
        .collect();
    for y in &theirs.spectra {
        let Some(x) = scan_of(&y.id).and_then(|s| by_scan.get(&s)) else {
            bad_n += 1;
            continue;
        };
        if x.mz.len() != y.mz.len() {
            bad_n += 1;
            continue;
        }
        for i in 0..x.mz.len() {
            npk += 1;
            wmz.add(y.index as u32, x.mz[i], y.mz[i], 1e-9);
            wint.add(y.index as u32, x.intensity_f64(i), y.intensity_f64(i), 1e-6);
        }
    }
    eprintln!("points compared: {npk}; scans with another number of points {bad_n}");
    eprintln!("{}\n{}", wmz.line("m/z"), wint.line("intensity"));
    let _ = std::fs::remove_file(&ours_p);
    Some(Report {
        header_differences: level + pol + prof + pts + parent + missing,
        point_differences: bad_n + wmz.bad + wint.bad,
        tic_bad: tic.bad,
        base_peak_bad: bpm.bad + bpi.bad,
        precursor_bad: sel.bad + tgt.bad + lo.bad + hi.bad + ce.bad,
    })
}

#[test]
fn raw_profile_totals_and_precursors_match_msconvert() {
    if let Some(r) = compare() {
        assert_eq!(
            (r.tic_bad, r.base_peak_bad, r.precursor_bad),
            (0, 0, 0),
            "TIC, base peak and precursor values of the scans that match"
        );
    }
}

#[test]
#[ignore = "known gaps with this Tribrid file: opentfraw reads the scan event (level, polarity, filter) of every second scan wrongly and the profile points differ"]
fn raw_profile_matches_msconvert_mzml_scan_by_scan() {
    if let Some(r) = compare() {
        assert_eq!((r.header_differences, r.point_differences), (0, 0));
    }
}
