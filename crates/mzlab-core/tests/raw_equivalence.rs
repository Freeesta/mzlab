//! RV-01: the memory work on the `.raw` reader (trailers and indexes kept compact) changes no result. For each real file the `.mzidx` (as `raw_times` builds it), the mzML of `readers_mzml::scrivi`, the
//! `metodo/1` document and the header of an array of scans have the SHA-256 recorded before the change. Needs the private data repository (`MZLAB_DATI`, `dati/formati/raw_veloce_alias.tsv` for LAB-A);
//! the big DDA is looked for as `$MZLAB_RAW_BAF` or `dati/formati/thermo/raw/BAF_GEE_37.raw`. A file that is missing skips its case. `RAW_EQ_PRINT=1` prints the hashes instead of checking them.
use std::io::Cursor;
use std::path::{Path, PathBuf};

use mzlab_core::readers::{self, FileSet, Limits, Member, Reader};
use mzlab_core::{mzidx, readers_mzml, upload};
use sha2::{Digest, Sha256};

#[path = "../../../tools/dati_percorsi.rs"]
mod dati_percorsi;

fn hex(b: &[u8]) -> String {
    Sha256::digest(b)
        .iter()
        .map(|x| format!("{x:02x}"))
        .collect()
}

/// `(mzidx, mzml, metodo, intestazioni)` hashes of the file.
fn hashes(path: &Path) -> (String, String, String, String) {
    let name = path.file_name().unwrap().to_string_lossy().into_owned();
    let dim = std::fs::metadata(path).unwrap().len();
    let set = || FileSet {
        membri: vec![Member {
            nome: name.clone(),
            dim,
            dati: Box::new(std::fs::File::open(path).unwrap()),
            sha1: None,
        }],
    };
    let mut fs = set();
    let imp = upload::impronte(&mut fs).expect("fingerprint");
    let mut l: Box<dyn Reader> = readers::apri(set(), &Limits::sito()).expect("open");
    let p = upload::prepare_reader(&mut *l, 0, &imp, &format!("{name}.mzML")).expect("table");
    let mut out = Cursor::new(Vec::new());
    mzidx::build_reader(&p.table, &mut *l, 0, &mut out, 192 << 20).expect("mzidx");
    let mzidx = hex(&out.into_inner());
    let mut mz = Sha256Sink::default();
    readers_mzml::scrivi(&mut *l, 0, &mut mz, &mut |_, _| {}).expect("mzML");
    let metodo = hex(l.method(0).expect("metodo").as_bytes());
    let n = l.index(0).unwrap().len();
    let mut h = Sha256::new();
    for k in (0..n).step_by((n / 200).max(1)) {
        for c in l.scan_header(0, k).expect("header") {
            h.update(format!(
                "{}={}|{:?}|{:?}\n",
                c.nome, c.valore, c.unita, c.cv
            ));
        }
    }
    (mzidx, hex(&mz.0.finalize()), metodo, hex(&h.finalize()))
}

#[derive(Default)]
struct Sha256Sink(Sha256);
impl std::io::Write for Sha256Sink {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        self.0.update(b);
        Ok(b.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// (case, hashes before the change: mzidx, mzml, metodo, intestazioni).
const BEFORE: &[(&str, [&str; 4])] = &[
    (
        "LAB-A",
        [
            "a4ac51c69f03843d300e6ad21a984c71d9077b6c3eff7cf12c2e47c3f941b468",
            "0b6d6c7bb20c07d1b684e929b26baa32340e48e6ed3237df3ab98007edc2a58f",
            "bbd8e719a87d0e85ef64d731a8d68b65d1cc55f777fae707fa44e1995edeae32",
            "fdb802dc4346660bdeba3cddf1d7cbb8a23d4b99d3e6d3d57a3253d61d9517d4",
        ],
    ),
    (
        "PXD077492",
        [
            "1516465e726f8ab470f4f6e2e3655bc6ac2dd63a16b57824caa05e7d7530e09f",
            "97df227f539ec3928e5f6c2d71a36d84f75ca2407c7a8f40a9a7a0c6e3810200",
            "783ea8ef3683eb2914a5912a71ed0b45da92ed5889f9f2bdb572de21367bb840",
            "41be13e4d786327c12cbb8e8ce7c5af7b539ec228957534da5473d1169544d6b",
        ],
    ),
    (
        "BAF",
        [
            "e35a63eba81e3a872f74dd9903d83a4f9bc9748076c471c536a0250c0cd43cd6",
            "1da5d58af5ca8cec238230d4857553296726cbee9642b7d18308c84b153ef2bc",
            "4f9312ccd9e290b25cb0d305e933e1a102855f200a5a53435439bfa9f822a36a",
            "648a3fded6940230011583d50681cbcd96940f7e1a2f20605751e626751b8e7a",
        ],
    ),
];

fn file(case: &str) -> Option<PathBuf> {
    let p = match case {
        "LAB-A" => {
            let t = std::fs::read_to_string(dati_percorsi::percorso(
                "dati/formati/raw_veloce_alias.tsv",
            )?)
            .ok()?;
            let r = t
                .lines()
                .find(|l| l.starts_with("LAB-A\t"))?
                .split('\t')
                .nth(1)?
                .to_string();
            dati_percorsi::percorso(&format!("dati/{r}"))?
        }
        "PXD077492" => std::fs::read_dir(dati_percorsi::percorso("dati/formati/thermo/raw")?)
            .ok()?
            .flatten()
            .map(|e| e.path())
            .find(|p| {
                p.file_name().is_some_and(|n| {
                    n.to_string_lossy().starts_with("PXD077492_")
                        && n.to_string_lossy().ends_with(".raw")
                })
            })?,
        _ => match std::env::var_os("MZLAB_RAW_BAF") {
            Some(v) => PathBuf::from(v),
            None => dati_percorsi::percorso("dati/formati/thermo/raw/BAF_GEE_37.raw")?,
        },
    };
    p.is_file().then_some(p)
}

#[test]
fn the_compact_memory_changes_no_result() {
    let Some(_) = dati_percorsi::radice() else {
        eprintln!("SKIP raw_equivalence: no data repository (MZLAB_DATI)");
        return;
    };
    let print = std::env::var_os("RAW_EQ_PRINT").is_some();
    for (case, before) in BEFORE {
        let Some(p) = file(case) else {
            eprintln!("SKIP raw_equivalence {case}: file not found");
            continue;
        };
        let (a, b, c, e) = hashes(&p);
        if print {
            println!("    (\"{case}\", [\"{a}\", \"{b}\", \"{c}\", \"{e}\"]),");
            continue;
        }
        assert_eq!(
            [a.as_str(), b.as_str(), c.as_str(), e.as_str()],
            *before,
            "{case}"
        );
    }
}

/// RV-05: `scansione_dati` (what the module sends: points, level, polarity, profile, RT and precursor from the index row) gives the same scan as `scansione`, and the transport
/// (`trasporto`) returns it as it is, for a sample of the scans of each file.
#[test]
fn scan_data_and_transport_give_the_same_scan() {
    let Some(_) = dati_percorsi::radice() else {
        eprintln!("SKIP raw_equivalence: no data repository (MZLAB_DATI)");
        return;
    };
    for case in ["LAB-A", "PXD077492", "BAF"] {
        let Some(p) = file(case) else {
            eprintln!("SKIP raw_equivalence {case}: file not found");
            continue;
        };
        let name = p.file_name().unwrap().to_string_lossy().into_owned();
        let dim = std::fs::metadata(&p).unwrap().len();
        let fs = FileSet {
            membri: vec![Member {
                nome: name,
                dim,
                dati: Box::new(std::fs::File::open(&p).unwrap()),
                sha1: None,
            }],
        };
        let mut l = readers::apri(fs, &Limits::sito()).unwrap();
        let index = l.index(0).unwrap().to_vec();
        let riletto =
            mzlab_core::transport::decode_index(&mzlab_core::transport::encode_index(&index))
                .unwrap();
        assert_eq!(riletto, index, "{case}: index through the transport");
        let step = (index.len() / 300).max(1);
        for k in (0..index.len()).step_by(step) {
            let a = l.scan(0, k).unwrap();
            let b = l.scan_data(0, k).unwrap();
            assert_eq!(
                (
                    a.index,
                    a.level,
                    a.rt,
                    a.polarity,
                    a.profile,
                    a.precursor,
                    &a.mz,
                    &a.intensity,
                    &a.wide
                ),
                (
                    b.index,
                    b.level,
                    b.rt,
                    b.polarity,
                    b.profile,
                    b.precursor,
                    &b.mz,
                    &b.intensity,
                    &b.wide
                ),
                "{case}: scan {k}"
            );
            assert_eq!(
                (&a.id, &a.filter),
                (&index[k].id, &index[k].filtro),
                "{case}: row {k}"
            );
            let t = mzlab_core::transport::decode_scansione(
                k,
                &mzlab_core::transport::encode_scansione(&b),
            )
            .unwrap();
            assert_eq!(
                (&t.mz, &t.intensity, t.rt, t.profile),
                (&b.mz, &b.intensity, b.rt, b.profile),
                "{case}: transport {k}"
            );
        }
    }
}

/// The light header of the scan table (`testata_tabella`) has the fields `Head::da_testata` reads exactly as the full header of the mzML (`testata_mzml`), so the
/// table is the same, for every `.raw` of the data repository.
#[test]
fn the_table_header_has_the_fields_of_the_full_header() {
    let Some(dir) = dati_percorsi::percorso("dati/formati/thermo/raw") else {
        eprintln!("SKIP raw_equivalence: no data repository (MZLAB_DATI)");
        return;
    };
    let mut seen = 0;
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if p.extension().is_none_or(|x| x != "raw") {
            continue;
        }
        let name = p.file_name().unwrap().to_string_lossy().into_owned();
        let fs = FileSet {
            membri: vec![Member {
                nome: name.clone(),
                dim: std::fs::metadata(&p).unwrap().len(),
                dati: Box::new(std::fs::File::open(&p).unwrap()),
                sha1: None,
            }],
        };
        let Ok(mut l) = readers::apri(fs, &Limits::sito()) else {
            continue;
        };
        let full = l.mzml_header(0).unwrap().expect("full header");
        let leggera = l.table_header(0).unwrap().expect("light header");
        let fields = |h: &mzlab_core::mzmlwrite::Header| {
            format!(
                "{:?}",
                (
                    &h.components,
                    &h.source_cv,
                    &h.sources,
                    &h.instrument,
                    &h.content,
                    &h.extra_software
                )
            )
        };
        assert_eq!(fields(&leggera), fields(&full), "{name}");
        seen += 1;
    }
    assert!(seen > 0, "no .raw opened");
}
