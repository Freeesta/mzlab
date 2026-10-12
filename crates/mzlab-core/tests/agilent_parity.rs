//! The Agilent `.D` reader: (1) synthetic folders written here with an encoder that shares no code with the reader (layout of `MSScan.xsd`),
//! (2) the real runs of the private data repository (`GC-MS/*.D`), checked against numbers the files carry themselves: the TIC of each record,
//! the scan counts of `MSTS.xml`, the end of `MSPeak.bin`, the ions of the method. Skipped without the data. There is no reference mzML.

use std::path::{Path, PathBuf};

use mzlab_core::agilent::scan::Layout;
use mzlab_core::agilent::{AgilentD, Files};
use mzlab_core::mzml;

fn put_i32(b: &mut Vec<u8>, v: i32) {
    b.extend(v.to_le_bytes());
}
fn put_f64(b: &mut Vec<u8>, v: f64) {
    b.extend(v.to_le_bytes());
}

struct Scan {
    time: f64,
    level: i32,
    kind: i32,
    pol: i32,
    precursor: f64,
    pts: Vec<(f64, f64)>,
}

/// (MSScan.bin, MSPeak.bin) for the scans.
fn encode(scans: &[Scan]) -> (Vec<u8>, Vec<u8>) {
    let mut peak = vec![0u8; 68];
    let mut idx = vec![0u8; 160];
    idx[0] = 1;
    idx[1] = 1;
    idx[88..92].copy_from_slice(&160i32.to_le_bytes());
    for (k, s) in scans.iter().enumerate() {
        let off = peak.len() as i64;
        s.pts.iter().for_each(|p| put_f64(&mut peak, p.0));
        s.pts.iter().for_each(|p| put_f64(&mut peak, p.1));
        let tic: f64 = s.pts.iter().map(|p| p.1).sum();
        let (bmz, bv) = s
            .pts
            .iter()
            .fold((0.0, 0.0), |a, p| if p.1 > a.1 { *p } else { a });
        let r = &mut idx;
        put_i32(r, k as i32 + 1);
        put_i32(r, 1);
        put_i32(r, 0);
        put_f64(r, s.time);
        put_i32(r, s.level);
        put_i32(r, s.kind);
        put_f64(r, tic);
        put_f64(r, bmz);
        put_f64(r, bv);
        put_i32(r, k as i32 + 1);
        put_i32(r, 0);
        put_i32(r, 2);
        put_i32(r, s.pol);
        for v in [0.0, 0.0, s.precursor, 0.0, 0.0, 1050.0, 0.0] {
            put_f64(r, v);
        }
        put_i32(r, 0);
        put_i32(r, 0);
        put_i32(r, 1);
        r.extend(off.to_le_bytes());
        put_i32(r, s.pts.len() as i32 * 16);
        put_i32(r, s.pts.len() as i32);
        for v in [f64::MAX, bv, 0.0, 0.0] {
            put_f64(r, v);
        }
    }
    (idx, peak)
}

fn sample() -> Vec<Scan> {
    let spec = |t: f64, k: f64| Scan {
        time: t,
        level: 1,
        kind: 1,
        pol: 0,
        precursor: 0.0,
        pts: vec![(50.5, 10.0 * k), (84.0, 500.0 * k), (86.25, 320.5 * k)],
    };
    vec![
        spec(3.0, 1.0),
        spec(3.01, 2.0),
        Scan {
            time: 3.02,
            level: 1,
            kind: 1,
            pol: 0,
            precursor: 0.0,
            pts: vec![],
        },
        spec(3.03, 0.5),
    ]
}

#[test]
fn synthetic_folder_reads_back() {
    let (idx, peak) = encode(&sample());
    let d = AgilentD::open(
        "x.D",
        &Files {
            ms_scan: &idx,
            ms_peak_len: peak.len() as u64,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(d.records.len(), 4);
    let mut p = peak.clone();
    let run = d.run(&mut p).unwrap();
    assert_eq!(run.spectra.len(), 3, "the empty scan is left out");
    assert_eq!(run.spectra[1].mz, vec![50.5, 84.0, 86.25]);
    assert_eq!(run.spectra[1].intensity_f64(1), 1000.0);
    assert_eq!(
        (
            run.spectra[0].id.as_str(),
            run.spectra[0].level,
            run.spectra[0].polarity,
            run.spectra[0].profile
        ),
        ("scan=1", 1, 1, false)
    );
    let tic = &run.chromatograms[0];
    assert_eq!(tic.time, vec![3.0, 3.01, 3.02, 3.03]);
    assert_eq!(tic.intensity[1], 1661.0);
    // mzML written and read back
    let mut out = Vec::new();
    d.to_mzml(&mut peak.clone(), peak.len() as u64, &mut out)
        .unwrap();
    let back = mzml::read(&out[..]).unwrap();
    assert_eq!(back.spectra.len(), 3);
    for (a, b) in back.spectra.iter().zip(&run.spectra) {
        assert_eq!(
            (a.mz.clone(), a.level, a.polarity, a.id.clone()),
            (b.mz.clone(), b.level, b.polarity, b.id.clone())
        );
        assert!((0..a.mz.len()).all(|i| a.intensity_f64(i) == b.intensity_f64(i)));
        assert!((a.rt - b.rt).abs() < 1e-12);
    }
    assert_eq!(back.chromatograms.len(), 1);
}

#[test]
fn product_scans_keep_level_and_precursor() {
    let mut s = sample();
    s[1].level = 2;
    s[1].precursor = 84.0;
    s[1].pol = 1;
    let (idx, peak) = encode(&s);
    let d = AgilentD::open(
        "x.D",
        &Files {
            ms_scan: &idx,
            ms_peak_len: peak.len() as u64,
            ..Default::default()
        },
    )
    .unwrap();
    let run = d.run(&mut peak.clone()).unwrap();
    assert_eq!(
        (
            run.spectra[1].level,
            run.spectra[1].precursor,
            run.spectra[1].polarity
        ),
        (2, Some(84.0), -1)
    );
    assert_eq!(run.spectra[0].precursor, None);
    assert_eq!(
        run.chromatograms[0].time.len(),
        3,
        "the TIC is made of the survey scans"
    );
}

#[test]
fn what_it_cannot_read_is_an_error_with_the_numbers() {
    let (idx, peak) = encode(&sample());
    let open = |i: &[u8], n: u64| {
        AgilentD::open(
            "x.D",
            &Files {
                ms_scan: i,
                ms_peak_len: n,
                ..Default::default()
            },
        )
    };
    assert_eq!(open(&[], 0).err().unwrap().key, "err.file.notAgilentD");
    assert_eq!(
        open(&idx[..idx.len() - 7], peak.len() as u64)
            .err()
            .unwrap()
            .key,
        "err.file.agilent"
    );
    assert_eq!(
        open(&idx, peak.len() as u64 - 1).err().unwrap().key,
        "err.file.agilent",
        "a spectrum past the end of MSPeak.bin"
    );
    assert_eq!(
        open(&idx[..160], 0).err().unwrap().key,
        "err.file.agilent",
        "no scans"
    );
    let mut other = idx.clone();
    other[160 + 132..160 + 136].copy_from_slice(&3i32.to_le_bytes());
    let d = open(&other, peak.len() as u64).unwrap();
    let e = d.run(&mut peak.clone()).err().unwrap();
    assert!(e.to_string().contains("SpectrumFormatID 3"), "{e}");
}

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

#[test]
fn real_runs_agree_with_their_own_numbers() {
    let Some(data) = dati() else {
        eprintln!("SKIP agilent_parity: no data repository (MZLAB_DATI)");
        return;
    };
    let mut folders: Vec<PathBuf> = std::fs::read_dir(data.join("GC-MS"))
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "D"))
        .collect();
    folders.sort();
    assert!(!folders.is_empty());
    for f in folders {
        let acq = f.join("AcqData");
        let (scan, xsd) = (
            read(&acq, "MSScan.bin").unwrap(),
            read(&acq, "MSScan.xsd").unwrap(),
        );
        let mut peak = std::fs::read(acq.join("MSPeak.bin")).unwrap();
        let label = f.display().to_string();
        assert_eq!(
            Layout::from_xsd(&String::from_utf8_lossy(&xsd)).unwrap(),
            Layout::builtin(),
            "{label}: the xsd of the folder and the built-in layout agree"
        );
        let (ts, dev, contents, tree, meth) = (
            read(&acq, "MSTS.xml"),
            read(&acq, "Devices.xml"),
            read(&acq, "Contents.xml"),
            read(&acq, "MSTree2.bin"),
            read(&f, "acqmeth.txt"),
        );
        let files = Files {
            ms_scan: &scan,
            ms_scan_xsd: Some(&xsd),
            ms_peak_len: peak.len() as u64,
            ms_ts: ts.as_deref(),
            devices: dev.as_deref(),
            contents: contents.as_deref(),
            ms_tree: tree.as_deref(),
            acqmeth: meth.as_deref(),
        };
        let d = AgilentD::open(&f.file_name().unwrap().to_string_lossy(), &files).unwrap();
        let run = d.run(&mut peak).unwrap();
        check_index_and_points(&d, &run, peak.len() as u64, &label);
        check_method(&d, &run, &label);
        // mzML written and read back
        let mut out = Vec::new();
        let n = peak.len() as u64;
        d.to_mzml(&mut peak, n, &mut out).unwrap();
        let back = mzml::read(&out[..]).unwrap();
        assert_eq!(back.spectra.len(), run.spectra.len(), "{label}");
        for (a, b) in back.spectra.iter().zip(&run.spectra) {
            assert!(
                a.mz == b.mz
                    && (0..a.mz.len()).all(|i| a.intensity_f64(i) == b.intensity_f64(i))
                    && a.id == b.id
                    && (a.rt - b.rt).abs() < 1e-12,
                "{label} {}",
                b.id
            );
        }
        eprintln!(
            "{label}: {} scans, {} segments, {}",
            d.records.len(),
            d.segments.len(),
            d.method.as_ref().unwrap().mode
        );
    }
}

const XSD: &str = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema">
<xs:complexType name="ScanRecordType"><xs:sequence>
<xs:element name="ScanID" type="xs:int"/><xs:element name="ScanTime" type="xs:double"/>
<!-- <xs:element name="Hidden" type="xs:double"/> -->
<xs:element name="MSLevel" type="xs:int"/><xs:element name="TIC" type="xs:double"/><xs:element name="Extra" type="xs:short" minOccurs="0"/>
<xs:element name="SpectrumParamValues" type="SpectrumParamsType" maxOccurs="unbounded"/>
</xs:sequence></xs:complexType>
<xs:complexType name="SpectrumParamsType"><xs:sequence>
<xs:element name="SpectrumFormatID" type="xs:int"/><xs:element name="SpectrumOffset" type="xs:long"/><xs:element name="ByteCount" type="xs:int"/>
<xs:element name="PointCount" type="xs:int"/><xs:element name="UncompressedByteCount" type="xs:int" minOccurs="0"/>
</xs:sequence></xs:complexType></xs:schema>"#;

/// A folder whose xsd has other fields than the GC/MS runs (2 bytes more, the uncompressed size, no commented-out field): the records are read by that layout.
#[test]
fn the_layout_comes_from_the_xsd_of_the_folder() {
    let sp = sample();
    let mut peak = vec![0u8; 68];
    let mut idx = vec![0u8; 160];
    idx[88..92].copy_from_slice(&160i32.to_le_bytes());
    for (k, s) in sp.iter().enumerate() {
        let off = peak.len() as i64;
        s.pts.iter().for_each(|p| put_f64(&mut peak, p.0));
        s.pts.iter().for_each(|p| put_f64(&mut peak, p.1));
        let n = s.pts.len() as i32 * 16;
        put_i32(&mut idx, k as i32 + 1);
        put_f64(&mut idx, s.time);
        put_i32(&mut idx, s.level);
        put_f64(&mut idx, s.pts.iter().map(|p| p.1).sum());
        idx.extend(7i16.to_le_bytes());
        put_i32(&mut idx, 1);
        idx.extend(off.to_le_bytes());
        put_i32(&mut idx, n);
        put_i32(&mut idx, s.pts.len() as i32);
        put_i32(&mut idx, n);
    }
    let layout = Layout::from_xsd(XSD).unwrap();
    assert_eq!(layout.record_len, 4 + 8 + 4 + 8 + 2 + 4 + 8 + 4 + 4 + 4);
    let open = |xsd: Option<&[u8]>| {
        AgilentD::open(
            "x.D",
            &Files {
                ms_scan: &idx,
                ms_scan_xsd: xsd,
                ms_peak_len: peak.len() as u64,
                ..Default::default()
            },
        )
    };
    let d = open(Some(XSD.as_bytes())).unwrap();
    let run = d.run(&mut peak.clone()).unwrap();
    assert_eq!(
        (
            d.records.len(),
            run.spectra.len(),
            run.spectra[1].mz.clone()
        ),
        (4, 3, vec![50.5, 84.0, 86.25])
    );
    assert_eq!(d.records[1].time_min, 3.01);
    assert!(
        open(None).is_err(),
        "the built-in layout does not fit this file"
    );
    // compressed points are refused by name, an unknown type and a missing field are errors
    let mut z = peak.clone();
    let d = AgilentD {
        records: d
            .records
            .iter()
            .cloned()
            .map(|mut r| {
                r.uncompressed = Some(r.bytes + 40);
                r
            })
            .collect(),
        ..open(Some(XSD.as_bytes())).unwrap()
    };
    assert!(d
        .run(&mut z)
        .err()
        .unwrap()
        .to_string()
        .contains("compressed"));
    assert!(Layout::from_xsd(&XSD.replace("xs:short", "xs:dateTime"))
        .err()
        .unwrap()
        .to_string()
        .contains("dateTime"));
    assert!(
        Layout::from_xsd(&XSD.replace("name=\"PointCount\"", "name=\"Points\""))
            .err()
            .unwrap()
            .to_string()
            .contains("PointCount")
    );
}
