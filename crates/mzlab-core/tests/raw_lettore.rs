#![cfg(feature = "thermo")]
//! The Thermo `.raw` read through the `Lettore` contract gives the same spectra as `raw::read`, with an index built without reading peaks, and is
//! recognised by the registry from its first bytes. Needs the private data repository (MZLAB_DATI or `mzlab-dati` beside this one); skipped without it.
#![allow(clippy::indexing_slicing)]
#![allow(clippy::disallowed_methods)] // tests compare against the standard sorts

use std::path::{Path, PathBuf};

use mzlab_core::lettori::{self, FileSet, Limiti, Membro};
use mzlab_core::{lettori_mzml, metodo, raw};

fn raw_files() -> Vec<PathBuf> {
    let mut cands: Vec<PathBuf> = std::env::var_os("MZLAB_DATI")
        .map(PathBuf::from)
        .into_iter()
        .collect();
    for up in Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .skip(1)
        .take(3)
    {
        cands.push(up.join("mzlab-dati"));
    }
    let Some(d) = cands.into_iter().find(|p| p.join("HRMS").is_dir()) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut stack = vec![d.join("HRMS")];
    while let Some(dir) = stack.pop() {
        for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("raw")) {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

fn set(path: &Path) -> FileSet {
    let bytes = std::fs::read(path).unwrap();
    let nome = path.file_name().unwrap().to_string_lossy().to_string();
    FileSet {
        membri: vec![Membro {
            nome,
            dim: bytes.len() as u64,
            dati: Box::new(bytes),
        }],
    }
}

#[test]
fn a_real_raw_reads_the_same_through_the_registry() {
    let files = raw_files();
    if files.is_empty() {
        eprintln!("SKIP raw_lettore: no data repository (MZLAB_DATI)");
        return;
    }
    for f in files.iter().take(2) {
        let want = raw::read(std::fs::File::open(f).unwrap()).unwrap();
        let mut l = lettori::apri(set(f), &Limiti::sito()).unwrap();
        assert_eq!(l.formato().id(), "thermo-raw");
        assert_eq!(l.campioni().len(), 1);
        let index = l.indice(0).unwrap().to_vec();
        assert!(index.len() >= want.spectra.len());
        let mut checked = 0;
        for w in &want.spectra {
            let k = index
                .iter()
                .position(|r| r.id == w.id)
                .unwrap_or_else(|| panic!("{}: not in the index", w.id));
            assert_eq!(
                (index[k].livello, index[k].rt, index[k].polarita),
                (w.level, w.rt, w.polarity),
                "{}",
                w.id
            );
            assert_eq!(index[k].filtro, w.filter);
            assert!(index[k].posizione.is_some_and(|p| p.lunghezza > 0));
            if checked % 7 == 0 {
                let got = l.scansione(0, k).unwrap();
                assert_eq!(
                    (
                        &got.id,
                        got.level,
                        got.profile,
                        got.precursor,
                        &got.mz,
                        &got.intensity
                    ),
                    (&w.id, w.level, w.profile, w.precursor, &w.mz, &w.intensity)
                );
                assert!(!l.intestazione(0, k).unwrap().is_empty());
            }
            checked += 1;
        }
        let tic = l.cromatogramma(0, "TIC").unwrap();
        assert_eq!(tic.time.len(), tic.intensity.len());
        assert!(!tic.time.is_empty());
        let doc = l.metodo(0).unwrap();
        assert!(doc.contains("thermo-raw"), "{doc}");
        eprintln!("{}: {} scans checked", f.display(), checked);
    }
}

#[test]
fn not_a_raw_is_refused() {
    let mut fs = FileSet {
        membri: vec![Membro {
            nome: "x.raw".into(),
            dim: 20,
            dati: Box::new(vec![7u8; 20]),
        }],
    };
    assert!(lettori::riconosci(&mut fs).is_none());
}

#[test]
fn the_noise_and_baseline_of_an_ft_scan_come_with_the_header_and_leave_the_peaks_alone() {
    let files = raw_files();
    if files.is_empty() {
        eprintln!("SKIP raw_lettore: no data repository (MZLAB_DATI)");
        return;
    }
    let mut seen = 0;
    for f in files.iter().take(2) {
        let direct = opentfraw::RawFileReader::open(std::io::BufReader::new(
            std::fs::File::open(f).unwrap(),
        ))
        .unwrap();
        let mut src = std::io::BufReader::new(std::fs::File::open(f).unwrap());
        let first = direct.run_header.sample_info.first_scan_number;
        let mut l = lettori::apri(set(f), &Limiti::sito()).unwrap();
        let n = l.indice(0).unwrap().len();
        for k in (0..n).step_by((n / 12).max(1)) {
            let head = l.intestazione(0, k).unwrap();
            let get = |name: &str| {
                head.iter()
                    .find(|c| c.nome == name)
                    .map(|c| c.valore.clone())
            };
            let nodes = direct
                .read_scan_labels(&mut src, first + k as u32)
                .map(|p| p.noise_nodes)
                .unwrap_or_default();
            if nodes.is_empty() {
                assert!(
                    get("thermo:noise_function").is_none(),
                    "{k}: no nodes, no field"
                );
                continue;
            }
            seen += 1;
            let want = nodes
                .iter()
                .map(|x| format!("{},{},{}", x.mz, x.noise, x.baseline))
                .collect::<Vec<_>>()
                .join(";");
            assert_eq!(
                get("thermo:noise_function").as_deref(),
                Some(want.as_str()),
                "scan {k}"
            );
            let noise: f32 = get("thermo:noise").unwrap().parse().unwrap();
            let (lo, hi) = nodes.iter().fold((f32::MAX, f32::MIN), |a, x| {
                (a.0.min(x.noise), a.1.max(x.noise))
            });
            assert!(
                noise >= lo && noise <= hi,
                "scan {k}: median outside the nodes"
            );
            assert!(get("thermo:baseline").is_some());
        }
    }
    eprintln!("scans with noise nodes checked: {seen}");
}

/// A sink that keeps the first bytes and refuses the rest: the header of the mzML is what the test reads.
struct Head(Vec<u8>);

impl std::io::Write for Head {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        if self.0.len() > (4 << 20) {
            return Err(std::io::Error::other("enough"));
        }
        self.0.extend_from_slice(b);
        Ok(b.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn the_mzml_of_a_raw_carries_the_instrument_the_hash_the_start_and_the_method() {
    let files = raw_files();
    let Some(f) = files.first() else {
        eprintln!("SKIP raw_lettore: no data repository (MZLAB_DATI)");
        return;
    };
    let mut l = lettori::apri(set(f), &Limiti::sito()).unwrap();
    let mut head = Head(Vec::new());
    let _ = lettori_mzml::scrivi(&mut *l, 0, &mut head, &mut |_, _| {});
    let xml = String::from_utf8_lossy(&head.0).into_owned();
    let top = &xml[..xml.find("<spectrumList").unwrap()];
    let want = mzlab_core::metodo::sha1_hex(
        &mut std::fs::read(f).unwrap(),
        std::fs::metadata(f).unwrap().len(),
    )
    .unwrap();
    assert!(
        top.contains(&format!(
            "accession=\"MS:1000569\" name=\"SHA-1\" value=\"{want}\""
        )),
        "{top}"
    );
    assert!(top.contains("MS:1000563"), "source file format");
    assert!(top.contains("startTimeStamp=\""), "start of the run");
    assert!(top.contains("opentfraw.computer_name"), "run terms");
    let doc = metodo::from_userparam(top).expect("metodo/1 in the run");
    assert!(doc.contains("\"schema\":\"metodo/1\""));
    assert!(
        doc.contains(&want),
        "the document names the file by its hash"
    );
    // the start is the one the method document gives as the creation of the file
    let start = top
        .split("startTimeStamp=\"")
        .nth(1)
        .and_then(|t| t.split('"').next())
        .unwrap();
    assert!(
        doc.contains(&format!("\"file_creato\":\"{start}\"")),
        "{start}"
    );
}
