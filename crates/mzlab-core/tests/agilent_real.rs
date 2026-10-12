//! The Agilent `.D` reader on the real runs of the private data repository (`GC-MS/*.D`), against numbers the files carry themselves: the TIC of each
//! record, the scan counts of `MSTS.xml`, the end of `MSPeak.bin`, the ions of the method, the side files. Skipped without the data.
//! The comparison with msconvert is in `agilent_msconvert.rs`, the synthetic folders in `agilent_parity.rs`.

use std::path::{Path, PathBuf};

use mzlab_core::agilent::scan::Layout;
use mzlab_core::agilent::{AgilentD, Files};
use mzlab_core::mzml;

fn dati() -> Option<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .ok()?;
    let mut cands: Vec<PathBuf> = std::env::var_os("MZLAB_DATI")
        .map(PathBuf::from)
        .into_iter()
        .collect();
    cands.extend(
        root.ancestors()
            .skip(1)
            .take(3)
            .map(|up| up.join("mzlab-dati")),
    );
    cands.into_iter().find(|p| p.join("GC-MS").is_dir())
}

fn read(d: &Path, f: &str) -> Option<Vec<u8>> {
    std::fs::read(d.join(f)).ok()
}

/// The index and the points of one real run against the numbers the files carry.
fn check_index_and_points(d: &AgilentD, run: &mzlab_core::mzml::Run, peak_len: u64, label: &str) {
    assert!(
        d.records
            .iter()
            .enumerate()
            .all(|(k, r)| r.id == k as i32 + 1 && r.cycle == r.id),
        "{label}: ids"
    );
    assert!(
        d.records.windows(2).all(|w| w[0].time_min < w[1].time_min),
        "{label}: times"
    );
    let mut at = 68;
    for r in &d.records {
        assert_eq!(
            r.offset, at,
            "{label}: scan {} is not where the previous one ends",
            r.id
        );
        at += r.bytes as u64;
    }
    assert_eq!(at, peak_len, "{label}: MSPeak.bin has bytes no scan owns");
    // MSTS.xml counts by time, the records by segment: the totals agree, a segment can differ by the scan on the boundary
    assert_eq!(
        d.segments.iter().map(|s| s.scans).sum::<usize>(),
        d.records.len(),
        "{label}: scans of MSTS.xml"
    );
    for s in &d.segments {
        let n = d
            .records
            .iter()
            .filter(|r| r.time_min >= s.start_min && r.time_min <= s.end_min)
            .count();
        assert_eq!(n, s.scans, "{label}: segment {} by time", s.id);
    }
    let tree = d.tree.as_ref().unwrap();
    assert_eq!(
        (tree.scans, tree.device.as_str(), tree.polarity.as_str()),
        (d.records.len(), "SingleQuad", "Positive"),
        "{label}: MSTree2.bin"
    );
    // the points: TIC of the record = sum of the intensities, base peak = their maximum, m/z ascending
    assert_eq!(
        run.spectra.len(),
        d.records.iter().filter(|r| r.points > 0).count()
    );
    for (s, r) in run
        .spectra
        .iter()
        .zip(d.records.iter().filter(|r| r.points > 0))
    {
        let sum: f64 = (0..s.mz.len()).map(|i| s.intensity_f64(i)).sum();
        assert!(
            (sum - r.tic).abs() <= 3e-3 * r.tic + 1.0,
            "{label} scan {}: sum {sum} TIC {}",
            r.id,
            r.tic
        );
        let top = (0..s.mz.len())
            .map(|i| s.intensity_f64(i))
            .fold(0.0, f64::max);
        assert!(
            (top - r.base_value).abs() <= 1e-3 * top + 1.0,
            "{label} scan {}: base peak",
            r.id
        );
        assert!(
            s.mz.windows(2).all(|w| w[0] < w[1]) && s.polarity == 1 && s.level == 1,
            "{label} scan {}",
            r.id
        );
    }
}

/// A SIM run holds only the ions of its group, a scan run stays in its range.
fn check_method(d: &AgilentD, run: &mzlab_core::mzml::Run, label: &str) {
    let m = d.method.as_ref().unwrap();
    for (s, r) in run
        .spectra
        .iter()
        .zip(d.records.iter().filter(|r| r.points > 0))
    {
        if m.mode == "SIM" {
            // the scan time is the end of the scan: the group a scan belongs to is the one running at the time of the scan before it
            let before = d
                .records
                .iter()
                .rev()
                .find(|p| p.id < r.id)
                .map_or(r.time_min, |p| p.time_min);
            let g = m
                .groups
                .iter()
                .rev()
                .find(|g| before >= g.start_min)
                .unwrap();
            let ions: Vec<f64> = g.ions.iter().map(|i| i.0).collect();
            assert!(
                s.mz.len() == ions.len()
                    && s.mz.iter().zip(&ions).all(|(a, b)| (a - b).abs() < 0.05),
                "{label} scan {}: ions {:?} of the group {:?}",
                r.id,
                s.mz,
                ions
            );
        } else {
            assert!(
                s.mz[0] >= m.low_mass.unwrap() - 1.0
                    && *s.mz.last().unwrap() <= m.high_mass.unwrap() + 1.0,
                "{label} scan {}: range",
                r.id
            );
        }
    }
}

/// Nothing the side files say is dropped: every file of the list gave items, the tune and the sample fields are there, the unmapped record fields are kept.
fn check_metadata(d: &AgilentD) {
    let has = |origin: &str, name: &str| {
        d.metadata
            .iter()
            .any(|i| i.origin == origin && i.name == name)
    };
    assert!(
        has("acqmeth.txt", "tune EMVOLTS")
            && has("AcqData/Devices.xml", "Devices/Device/SerialNumber")
            && has("AcqData/sample_info.xml", "Dilution"),
        "tune {} serial {} sample fields {}",
        has("acqmeth.txt", "tune EMVOLTS"),
        has("AcqData/Devices.xml", "Devices/Device/SerialNumber"),
        has("AcqData/sample_info.xml", "Dilution")
    );
    for o in [
        "AcqData/MSTS.xml",
        "AcqData/Contents.xml",
        "AcqData/sequence.xml",
        "AcqData-CumulativeAuditTrail.xml",
        "GC.ini",
        "runstart.txt",
        "IndexedDataConverter.log",
        "checksum.xml",
        "PRE_POST.INI",
    ] {
        assert!(d.metadata.iter().any(|i| i.origin == o), "no item from {o}");
    }
    assert_eq!(d.extra_names.len(), d.records[0].extra.len());
    assert!(
        d.extra_names.iter().any(|n| n == "SamplingPeriod")
            && d.extra_names.iter().any(|n| n == "MinX")
    );
}

/// The bytes of the files of one `.D` folder.
struct Folder {
    name: String,
    scan: Vec<u8>,
    xsd: Vec<u8>,
    peak: Vec<u8>,
    small: Vec<(String, Vec<u8>)>,
}

const SMALL: [&str; 13] = [
    "AcqData/MSTS.xml",
    "AcqData/Devices.xml",
    "AcqData/Contents.xml",
    "AcqData/MSTree2.bin",
    "acqmeth.txt",
    "AcqData/sample_info.xml",
    "AcqData/sequence.xml",
    "AcqData-CumulativeAuditTrail.xml",
    "checksum.xml",
    "GC.ini",
    "PRE_POST.INI",
    "runstart.txt",
    "IndexedDataConverter.log",
];

impl Folder {
    fn load(f: &Path) -> Folder {
        let acq = f.join("AcqData");
        Folder {
            name: f.file_name().unwrap().to_string_lossy().into_owned(),
            scan: read(&acq, "MSScan.bin").unwrap(),
            xsd: read(&acq, "MSScan.xsd").unwrap(),
            peak: std::fs::read(acq.join("MSPeak.bin")).unwrap(),
            small: SMALL
                .iter()
                .filter_map(|n| read(f, n).map(|b| (n.to_string(), b)))
                .collect(),
        }
    }

    fn open(&self) -> AgilentD {
        let get = |n: &str| {
            self.small
                .iter()
                .find(|(k, _)| k == n)
                .map(|(_, b)| b.as_slice())
        };
        let others: Vec<(&str, &[u8])> = self
            .small
            .iter()
            .skip(5)
            .map(|(n, b)| (n.as_str(), b.as_slice()))
            .collect();
        let files = Files {
            ms_scan: &self.scan,
            data_ms: None,
            ms_scan_xsd: Some(&self.xsd),
            ms_peak_len: self.peak.len() as u64,
            ms_ts: get(SMALL[0]),
            devices: get(SMALL[1]),
            contents: get(SMALL[2]),
            ms_tree: get(SMALL[3]),
            acqmeth: get(SMALL[4]),
            others: &others,
        };
        AgilentD::open(&self.name, &files).unwrap()
    }
}

fn folders() -> Option<Vec<PathBuf>> {
    let data = dati()?;
    let mut v: Vec<PathBuf> = std::fs::read_dir(data.join("GC-MS"))
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "D"))
        .collect();
    v.sort();
    assert!(!v.is_empty());
    Some(v)
}

fn check_mzml_round_trip(
    d: &AgilentD,
    run: &mzlab_core::mzml::Run,
    peak: &mut Vec<u8>,
    label: &str,
) {
    let mut out = Vec::new();
    let n = peak.len() as u64;
    d.to_mzml(peak, n, &mut out).unwrap();
    let back = mzml::read(&out[..]).unwrap();
    assert_eq!(back.spectra.len(), run.spectra.len(), "{label}");
    for (a, b) in back.spectra.iter().zip(&run.spectra) {
        let same =
            a.mz == b.mz && (0..a.mz.len()).all(|i| a.intensity_f64(i) == b.intensity_f64(i));
        assert!(
            same && a.id == b.id && (a.rt - b.rt).abs() < 1e-12,
            "{label} {}",
            b.id
        );
    }
}

#[test]
fn real_runs_agree_with_their_own_numbers() {
    let Some(all) = folders() else {
        eprintln!("SKIP agilent_real: no data repository (MZLAB_DATI)");
        return;
    };
    for f in all {
        let mut fo = Folder::load(&f);
        let label = fo.name.clone();
        assert_eq!(
            Layout::from_xsd(&String::from_utf8_lossy(&fo.xsd)).unwrap(),
            Layout::builtin(),
            "{label}: the xsd of the folder and the built-in layout agree"
        );
        let d = fo.open();
        check_metadata(&d);
        let run = d.run(&mut fo.peak).unwrap();
        check_index_and_points(&d, &run, fo.peak.len() as u64, &label);
        check_method(&d, &run, &label);
        check_mzml_round_trip(&d, &run, &mut fo.peak, &label);
        eprintln!(
            "{label}: {} scans, {} segments, {}",
            d.records.len(),
            d.segments.len(),
            d.method.as_ref().unwrap().mode
        );
    }
}

/// The GC part of the method (oven, inlet, column, transfer line) is read for the real runs.
#[test]
fn the_gc_method_is_read() {
    let Some(all) = folders() else { return };
    for f in all {
        let g = Folder::load(&f).open().method.unwrap().gc;
        let ok = g.oven_initial_c.is_some()
            && !g.oven_ramps.is_empty()
            && g.inlet_heater_c.is_some()
            && g.inlet_pressure_psi.is_some()
            && g.column_flow_ml_min.is_some();
        assert!(
            ok && g.transfer_line_c.is_some()
                && g.run_time_min.is_some()
                && !g.column_dimensions.is_empty()
                && !g.inlet_mode.is_empty(),
            "{}: {g:?}",
            f.file_name().unwrap().to_string_lossy()
        );
        eprintln!("oven {:?} C hold {:?} min, ramps {:?}, inlet {} {:?} C, column {} {:?} mL/min, transfer {:?}", g.oven_initial_c, g.oven_hold_min, g.oven_ramps, g.inlet_mode, g.inlet_heater_c, g.column_dimensions, g.column_flow_ml_min, g.transfer_line_c);
    }
}

/// `data.ms` against the `AcqData` converted from it: same scans, times and TICs; the m/z are whole twentieths of the calibrated ones (up to 0.05 apart).
/// A run with only `data.ms` opens and converts.
#[test]
fn data_ms_agrees_with_acqdata_and_opens_alone() {
    let Some(all) = folders() else { return };
    for f in all {
        let label = f.file_name().unwrap().to_string_lossy().into_owned();
        let bytes = std::fs::read(f.join("data.ms")).unwrap();
        let mut fo = Folder::load(&f);
        let full = fo.open();
        let run = full.run(&mut fo.peak).unwrap();
        let alone = AgilentD::open(
            "x.D",
            &Files {
                data_ms: Some(&bytes),
                ..Default::default()
            },
        )
        .unwrap();
        let mut none = Vec::new();
        let got = alone.run(&mut none).unwrap();
        assert_eq!(
            (got.spectra.len(), alone.records.len()),
            (run.spectra.len(), full.records.len()),
            "{label}: scans"
        );
        for (a, b) in alone.records.iter().zip(&full.records) {
            assert!(
                (a.time_min - b.time_min).abs() < 1e-9 && a.tic == b.tic,
                "{label} scan {}: time/TIC",
                b.id
            );
        }
        for (a, b) in got.spectra.iter().zip(&run.spectra) {
            assert!(
                a.mz.len() == b.mz.len()
                    && a.mz.iter().zip(&b.mz).all(|(x, y)| (x - y).abs() <= 0.0501),
                "{label} {}: m/z",
                b.id
            );
        }
        let mut out = Vec::new();
        alone.to_mzml(&mut none, 0, &mut out).unwrap();
        assert_eq!(
            mzml::read(&out[..]).unwrap().spectra.len(),
            run.spectra.len(),
            "{label}: mzML of data.ms alone"
        );
        assert!(
            alone
                .metadata
                .iter()
                .any(|i| i.origin == "data.ms" && i.name == "sample name")
                && alone
                    .metadata
                    .iter()
                    .any(|i| i.origin == "data.ms#header" && i.name == "undecoded sha1")
        );
    }
}
