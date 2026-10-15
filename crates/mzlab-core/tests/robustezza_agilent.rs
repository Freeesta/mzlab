//! The Agilent `.D` reader against folders that are not what they say: no `AcqData`, files of size zero, a declared size of 16 EB, a size that does not divide
//! by the record, `MSPeak.bin` shorter than the records say, a `DATA.MS` that is only its magic.

#![allow(clippy::disallowed_methods)] // tests compare against the standard sorts

mod agilent_synth;
mod robustezza;

#[cfg(feature = "estesi")]
use agilent_synth::lc;
use agilent_synth::{encode, sample};
use mzlab_core::lettori::{self, FileSet, Limiti, Membro};
use robustezza::*;

#[global_allocator]
static ALLOCATORE: Contatore = Contatore;

/// A member whose declared size (`dim`) can differ from its bytes.
fn membro(nome: &str, b: &[u8], dim: Option<u64>) -> Membro {
    Membro {
        nome: nome.into(),
        dim: dim.unwrap_or(b.len() as u64),
        dati: Box::new(Letture(b.to_vec())),
    }
}

fn legge(membri: Vec<(String, Vec<u8>, Option<u64>)>) -> Vec<String> {
    let mut k = Vec::new();
    let mut fs = FileSet {
        membri: membri.iter().map(|(n, b, d)| membro(n, b, *d)).collect(),
    };
    let _ = lettori::riconosci(&mut fs);
    let fs = FileSet {
        membri: membri.iter().map(|(n, b, d)| membro(n, b, *d)).collect(),
    };
    if let Some(mut l) = nota(lettori::apri(fs, &Limiti::sito()), &mut k) {
        if let Some(ix) = nota(l.indice(0), &mut k).map(<[_]>::to_vec) {
            (0..ix.len()).for_each(|j| drop(nota(l.scansione(0, j), &mut k)));
        }
        nota(l.metodo(0), &mut k);
    }
    k
}

fn caso(nome: &str, m: Vec<(&str, Vec<u8>, Option<u64>)>) -> Vec<String> {
    let totale = m.iter().map(|x| x.1.len()).sum();
    let m = m
        .into_iter()
        .map(|(n, b, d)| (n.to_string(), b, d))
        .collect();
    uno(nome, totale, Box::new(move || legge(m)))
}

#[test]
fn folders_that_are_not_what_they_say() {
    let (idx, peak) = encode(&sample());
    let (scan, pk) = ("x.D/AcqData/MSScan.bin", "x.D/AcqData/MSPeak.bin");
    let mut g = Vec::new();
    g.extend(caso(
        "no AcqData",
        vec![("x.D/readme.txt", b"hello".to_vec(), None)],
    ));
    g.extend(caso("no members", vec![]));
    g.extend(caso(
        "empty files",
        vec![(scan, vec![], None), (pk, vec![], None)],
    ));
    g.extend(caso(
        "declared 16 EB",
        vec![
            (scan, idx.clone(), Some(u64::MAX)),
            (pk, peak.clone(), None),
        ],
    ));
    g.extend(caso(
        "peak declared 16 EB",
        vec![
            (scan, idx.clone(), None),
            (pk, peak.clone(), Some(u64::MAX)),
        ],
    ));
    g.extend(caso(
        "peak declared 4 GB",
        vec![(scan, idx.clone(), None), (pk, peak.clone(), Some(1 << 32))],
    ));
    let mut storto = idx.clone();
    storto.truncate(idx.len() - 7);
    g.extend(caso(
        "size not a multiple of the record",
        vec![(scan, storto, None), (pk, peak.clone(), None)],
    ));
    g.extend(caso(
        "peak shorter than the records",
        vec![
            (scan, idx.clone(), None),
            (pk, peak[..peak.len() / 2].to_vec(), None),
        ],
    ));
    g.extend(caso(
        "peak with the header only",
        vec![(scan, idx.clone(), None), (pk, peak[..68].to_vec(), None)],
    ));
    g.extend(caso(
        "DATA.MS with its magic only",
        vec![("x.D/DATA.MS", vec![0x01, 0x32], None)],
    ));
    g.extend(caso(
        "DATA.MS declared 16 EB",
        vec![("x.D/DATA.MS", vec![0x01, 0x32, 0, 0], Some(u64::MAX))],
    ));
    verifica("agilent", 11, &g);
}

/// A folder of LC/MS files: (member name, bytes, declared size).
#[cfg(feature = "estesi")]
type Members<'a> = Vec<(&'a str, Vec<u8>, Option<u64>)>;

#[cfg(feature = "estesi")]
const LC_SCAN: &str = "x.D/AcqData/MSScan.bin";
#[cfg(feature = "estesi")]
const LC_PROFILE: &str = "x.D/AcqData/MSProfile.bin";
#[cfg(feature = "estesi")]
const LC_PEAK: &str = "x.D/AcqData/MSPeak.bin";
#[cfg(feature = "estesi")]
const LC_XSD: &str = "x.D/AcqData/MSScan.xsd";
#[cfg(feature = "estesi")]
const LC_CAL: &str = "x.D/AcqData/DefaultMassCal.xml";
#[cfg(feature = "estesi")]
const LC_METHOD: &str = "x.D/AcqData/AcqMethod.xml";

#[cfg(feature = "estesi")]
fn lc_cal() -> Vec<u8> {
    b"<DefaultMassCal><DefaultCalibration DefaultCalibrationID=\"1\"><Step StepNumber=\"1\"><CalibrationFormula>Traditional</CalibrationFormula><Value Number=\"1\">0.1</Value><Value Number=\"2\">0</Value></Step></DefaultCalibration></DefaultMassCal>".to_vec()
}

/// One record (scan type, format, offset, bytes, points, uncompressed size), the data member and the xsd and calibration of a Q-TOF run.
#[cfg(feature = "estesi")]
fn lc_run(
    kind: i32,
    f: (i32, i64, i32, i32, i32),
    data: (&'static str, Vec<u8>),
) -> Members<'static> {
    let r = lc::record(1, kind, f.0, f.1, f.2, f.3, f.4);
    vec![
        (LC_SCAN, lc::index(&[r]), None),
        (data.0, data.1, None),
        (LC_XSD, lc::xsd("DataDependentScanParamType"), None),
        (LC_CAL, lc_cal(), None),
    ]
}

/// Profile blocks and record numbers that lie: a cut LZF stream, a run-length start index past the end, absurd counts and offsets.
#[cfg(feature = "estesi")]
fn lc_profile_cases() -> Vec<(&'static str, Members<'static>)> {
    let mut rl = Vec::new();
    rl.extend_from_slice(&170.0f64.to_le_bytes());
    rl.extend_from_slice(&0.05f64.to_le_bytes());
    rl.extend_from_slice(&[0x00, 0xff, 0x02, 0x90]);
    rl.extend_from_slice(&(-1_000_000i32).to_le_bytes());
    rl.extend_from_slice(&[5, 0, 0, 0, 6, 0, 0, 0]);
    let mut v = vec![
        (
            "truncated LZF stream",
            lc_run(1, (1, 0, 4, 100, 416), (LC_PROFILE, vec![31u8, 1, 2, 3])),
        ),
        (
            "run-length start index past the end",
            lc_run(1, (1, 0, rl.len() as i32, 10, 56), (LC_PROFILE, rl)),
        ),
    ];
    for (name, format, points, unc, offset) in [
        ("profile with i32::MAX points", 1, i32::MAX, i32::MAX, 0i64),
        ("profile with negative points", 1, -5, 16, 0),
        ("MRM with i32::MAX points", 3, i32::MAX, 0, 0),
        ("centroids with an offset past the file", 2, 4, 0, i64::MAX),
        ("unknown spectrum format", 9, 4, 0, 0),
    ] {
        let kind = if format == 3 { 256 } else { 1 };
        let member = if format == 1 { LC_PROFILE } else { LC_PEAK };
        v.push((
            name,
            lc_run(
                kind,
                (format, offset, 48, points, unc),
                (member, vec![0u8; 64]),
            ),
        ));
    }
    v
}

/// Layouts and methods that lie: an unknown nested type, no calibration, a method of 16 EB, with 50 000 open sections or cut, detector files of 16 EB and 4 GB.
#[cfg(feature = "estesi")]
fn lc_file_cases() -> Vec<(&'static str, Members<'static>)> {
    let r = lc::record(1, 1, 3, 0, 12, 1, 0);
    let base = || -> Members<'static> {
        vec![
            (LC_SCAN, lc::index(&[r.clone()]), None),
            (LC_PEAK, vec![0u8; 64], None),
            (LC_XSD, lc::xsd("DataDependentScanParamType"), None),
        ]
    };
    let with = |extra: Members<'static>| -> Members<'static> {
        let mut m = base();
        m.extend(extra);
        m
    };
    let mut unknown = base();
    unknown[2].1 = lc::xsd("NoSuchType");
    let mut no_cal = lc_run(1, (1, 0, 4, 100, 416), (LC_PROFILE, vec![0u8; 4]));
    no_cal.pop();
    vec![
        ("xsd with an unknown nested type", unknown),
        ("profile without a mass calibration", no_cal),
        (
            "AcqMethod.xml declared 16 EB",
            with(vec![(LC_METHOD, b"<x/>".to_vec(), Some(u64::MAX))]),
        ),
        (
            "AcqMethod.xml with 50 000 open sections",
            with(vec![(
                LC_METHOD,
                "<RCDevicesXml>&lt;Section&gt;".repeat(50_000).into_bytes(),
                None,
            )]),
        ),
        (
            "AcqMethod.xml cut inside a value",
            with(vec![(
                LC_METHOD,
                b"<RCDevicesXml>&lt;Section&gt;&lt;ParameterName&gt;x".to_vec(),
                None,
            )]),
        ),
        (
            "detector files declared 16 EB and 4 GB",
            with(vec![
                ("x.D/AcqData/DAD1.cd", vec![0u8; 16], Some(u64::MAX)),
                ("x.D/AcqData/DAD1.sp", vec![0u8; 16], Some(1 << 32)),
            ]),
        ),
    ]
}

/// LC/MS folders with every number of the file hostile: the reader answers with an error or with what is there, never with a panic or an allocation sized by the file.
#[cfg(feature = "estesi")]
#[test]
fn lc_ms_folders_that_are_not_what_they_say() {
    let cases: Vec<_> = lc_profile_cases()
        .into_iter()
        .chain(lc_file_cases())
        .collect();
    let n = cases.len();
    let g: Vec<String> = cases
        .into_iter()
        .flat_map(|(name, m)| caso(name, m))
        .collect();
    verifica("agilent", n, &g);
}
