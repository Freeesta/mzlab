//! The Agilent `.D` reader on synthetic folders written here with an encoder that shares no code with the reader (layout of `MSScan.xsd`); no laboratory
//! data. The real runs are in `agilent_real.rs` and `agilent_msconvert.rs`.

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
    assert_eq!(back.chromatograms.len(), 2, "TIC and BPC");
    assert_eq!(back.chromatograms[1].intensity[0], 500.0);
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

/// (MSScan.bin, MSPeak.bin) written for `XSD` (2 bytes more than the GC/MS runs, the uncompressed size, no commented-out field).
fn encode_for_xsd() -> (Vec<u8>, Vec<u8>) {
    let mut peak = vec![0u8; 68];
    let mut idx = vec![0u8; 160];
    idx[88..92].copy_from_slice(&160i32.to_le_bytes());
    for (k, s) in sample().iter().enumerate() {
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
    (idx, peak)
}

fn open_with(idx: &[u8], peak_len: usize, xsd: Option<&[u8]>) -> mzlab_core::Result<AgilentD> {
    let files = Files {
        ms_scan: idx,
        ms_scan_xsd: xsd,
        ms_peak_len: peak_len as u64,
        ..Default::default()
    };
    AgilentD::open("x.D", &files)
}

/// A folder whose xsd has other fields than the GC/MS runs: the records are read by that layout.
#[test]
fn the_layout_comes_from_the_xsd_of_the_folder() {
    let (idx, peak) = encode_for_xsd();
    assert_eq!(
        Layout::from_xsd(XSD).unwrap().record_len,
        4 + 8 + 4 + 8 + 2 + 4 + 8 + 4 + 4 + 4
    );
    let d = open_with(&idx, peak.len(), Some(XSD.as_bytes())).unwrap();
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
        open_with(&idx, peak.len(), None).is_err(),
        "the built-in layout does not fit this file"
    );
}

/// Compressed points are refused by name; an unknown type and a missing field are errors that say which.
#[test]
fn what_the_xsd_cannot_describe_is_an_error() {
    let (idx, peak) = encode_for_xsd();
    let mut d = open_with(&idx, peak.len(), Some(XSD.as_bytes())).unwrap();
    d.records
        .iter_mut()
        .for_each(|r| r.uncompressed = Some(r.bytes + 40));
    assert!(d
        .run(&mut peak.clone())
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

fn utf16(t: &str) -> Vec<u8> {
    let mut b = vec![0xff, 0xfe];
    t.encode_utf16().for_each(|u| b.extend(u.to_le_bytes()));
    b
}

#[test]
fn method_sim_groups_and_scan_range() {
    let sim = "oven\nHold Time 1 min\nMS Information\nAcquisition Mode         : SIM\nSolvent Delay (minutes)  : 4\nTune file                : D:\\x\\bfb.u\n\
[SIM Parameters]\nGroup 1               Group ID                  : A\nResolution                : 1\nGroup Start Time          : 4\nNumber of Ions            : 3\n\
Dwell In Group            :(  Mass,  Dwell) (  Mass,  Dwell)\n                           (  95.10,70  )  (  108.10,70  )\n                           (  135.10,50  )\n\n\
Group 2               Group ID                  : B\nResolution                : 1\nGroup Start Time          : 8.5\n                           (  111.00,70  )\n\n[MSZones]\nMS Source : 230 C\n";
    let m = mzlab_core::agilent::method::parse(&utf16(sim));
    assert_eq!(
        (m.mode.as_str(), m.solvent_delay_min, m.tune_file.as_str()),
        ("SIM", Some(4.0), "D:\\x\\bfb.u")
    );
    assert_eq!(m.groups.len(), 2);
    assert_eq!(
        (
            m.groups[0].id.as_str(),
            m.groups[0].start_min,
            m.groups[0].resolution
        ),
        ("A", 4.0, Some(1.0))
    );
    assert_eq!(
        m.groups[0].ions,
        vec![(95.1, 70.0), (108.1, 70.0), (135.1, 50.0)]
    );
    assert_eq!(
        (m.groups[1].start_min, m.groups[1].ions.clone()),
        (8.5, vec![(111.0, 70.0)])
    );
    let scan = "MS Information\nAcquisition Mode         : Scan\nSolvent Delay (minutes)  : 3\n[Scan Parameters]\nStart Time               : 3\nLow Mass                 : 50\nHigh Mass                : 500\nThreshold                : 100\n";
    let m = mzlab_core::agilent::method::parse(scan.as_bytes());
    assert_eq!(
        (
            m.mode.as_str(),
            m.start_min,
            m.low_mass,
            m.high_mass,
            m.threshold,
            m.groups.len()
        ),
        ("Scan", Some(3.0), Some(50.0), Some(500.0), Some(100.0), 0)
    );
}

/// A `data.ms` written here (big-endian records of whole twentieths and 2-bit-exponent abundances): a run with only that file opens.
fn encode_data_ms(scans: &[(u32, Vec<(u16, u16)>)]) -> Vec<u8> {
    let start = 0x140usize;
    let mut b = vec![0u8; start];
    b[0] = 1;
    b[1] = 0x32;
    for (o, s) in [
        (0x04usize, "GC / MS Data File"),
        (0x18, "run one"),
        (0xB2, "1 Jan 25  01:00 am"),
    ] {
        b[o] = s.len() as u8;
        b[o + 1..o + 1 + s.len()].copy_from_slice(s.as_bytes());
    }
    b[0x10A..0x10C].copy_from_slice(&(((start + 2) / 2) as u16).to_be_bytes());
    b[0x116..0x11A].copy_from_slice(&(scans.len() as u32).to_be_bytes());
    for (ms, peaks) in scans {
        let words = (24 + 4 * (peaks.len() + 1)) / 2;
        b.extend((words as u16).to_be_bytes());
        b.extend(ms.to_be_bytes());
        b.extend([0, 12, 0, 1, 0, 0]);
        b.extend((peaks.len() as u16).to_be_bytes());
        let top = peaks
            .iter()
            .max_by_key(|p| p.1 & 0x3fff)
            .copied()
            .unwrap_or((0, 0));
        b.extend(top.0.to_be_bytes());
        b.extend(top.1.to_be_bytes());
        for p in peaks.iter().rev() {
            b.extend(p.0.to_be_bytes());
            b.extend(p.1.to_be_bytes());
        }
        b.extend([0, 5, 0, 4, 0, 0]);
        b.extend(
            peaks
                .iter()
                .map(|p| (((p.1 & 0x3fff) as u32) << (3 * (p.1 >> 14))))
                .sum::<u32>()
                .to_be_bytes(),
        );
    }
    b.extend([9u8; 20]);
    b
}

#[test]
fn a_run_with_only_data_ms_opens() {
    let scans = vec![
        (
            60000u32,
            vec![(1019u16, 10u16), (1680, 500), (4138, 0x4000 | 40)],
        ),
        (60300, vec![(1680, 20)]),
    ];
    let bytes = encode_data_ms(&scans);
    let d = AgilentD::open(
        "old.D",
        &Files {
            data_ms: Some(&bytes),
            ..Default::default()
        },
    )
    .unwrap();
    let run = d.run(&mut Vec::new()).unwrap();
    assert_eq!(
        (run.spectra.len(), run.spectra[0].mz.clone()),
        (2, vec![50.95, 84.0, 206.9])
    );
    assert_eq!(
        run.spectra[0].intensity_f64(2),
        320.0,
        "exponent 1 multiplies by 8"
    );
    assert_eq!(
        (
            d.records[0].time_min,
            d.records[0].tic,
            d.records[1].time_min
        ),
        (1.0, 830.0, 1.005)
    );
    assert!(d
        .metadata
        .iter()
        .any(|i| i.origin == "data.ms" && i.name == "sample name" && i.value == "run one"));
    assert!(
        d.metadata
            .iter()
            .any(|i| i.origin == "data.ms#after-records" && i.name == "undecoded sha1"),
        "the bytes after the records are declared"
    );
    let mut mzml = Vec::new();
    d.to_mzml(&mut Vec::new(), 0, &mut mzml).unwrap();
    assert_eq!(mzlab_core::mzml::read(&mzml[..]).unwrap().spectra.len(), 2);
    // another record length is an error that names the numbers
    let mut odd = bytes.clone();
    odd[0x140 + 13] = 5;
    assert!(AgilentD::open(
        "old.D",
        &Files {
            data_ms: Some(&odd),
            ..Default::default()
        }
    )
    .err()
    .unwrap()
    .to_string()
    .contains("record 1 has"));
    let mut wrong = bytes;
    wrong[1] = 0x33;
    assert!(AgilentD::open(
        "old.D",
        &Files {
            data_ms: Some(&wrong),
            ..Default::default()
        }
    )
    .is_err());
}
