//! RV-01: the memory work on the `.raw` reader (trailers and events kept compact) changes no result. Proof on the real files: the three reads of a trailer
//! agree field by field, and `opentfraw` tests that the compact event table gives back every event. Regression pins: the SHA-256 of the `.mzidx`, the mzML,
//! the `method/1` document and the headers of a sample of scans; a change meant to change the output re-pins them (`RAW_EQ_PRINT=1`) and says which change.
//! Needs the private data repository (`MZLAB_DATI`; LAB-A by `dati/formati/raw_veloce_alias.tsv`; the big public DDA as `$MZLAB_RAW_BAF` or by the name its download row in `dati/formati/_grandi_thermo_aperti.tsv` gives); a missing file skips its case.
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
    let mut l: Box<dyn Reader> = readers::open(set(), &Limits::sito()).expect("open");
    let p = upload::prepare_reader(&mut *l, 0, &imp, &format!("{name}.mzML")).expect("table");
    let mut out = Cursor::new(Vec::new());
    mzidx::build_reader(&p.table, &mut *l, 0, &mut out, 192 << 20).expect("mzidx");
    let mzidx = hex(&out.into_inner());
    let mut mz = Sha256Sink::default();
    readers_mzml::write(&mut *l, 0, &mut mz, &mut |_, _| {}).expect("mzML");
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

/// (case, pinned hashes: mzidx, mzml, metodo, intestazioni); re-pinned on main after #3129.
const PINNED: &[(&str, [&str; 4])] = &[
    (
        "LAB-A",
        [
            "341c0443e439db21387b7e932eaf583663ce6b9d90fcee7006e497a27222fe69",
            "8ca8ce98b77fc41fbd1e28685edbc83759beb5bde064e470111a054e68209113",
            "660515dde622a5519f006b12951a59aba16377134494fb3a8b5318cfc89f86d3",
            "9aea18ce54aed30996187adbf32d15255a83ef102b2b4ca428a8b7b5b776d9ab",
        ],
    ),
    (
        "PXD077492",
        [
            "a7b720f76b31016966eb62becf950eb7f061465da67aaa8796ffd15b7b01547f",
            "6f26fe3338241676e215c7cbe36b442e008c0e71b5c7ef41ff2ba8c95925056e",
            "e8114cccd6fbf6b1f5f8cd0ad3fc07019803d87e5317fb12bb8962c8d270325f",
            "d146d4ece74e6fad473d0465d4cac9bfccdd5f416e4843779fd904b189b7e3bf",
        ],
    ),
    (
        "BAF",
        [
            "48d8960dd148675eee710f66997e66aa55459716f3884d128b7b0212ec6e7034",
            "e5807b218ae0d75a72612a4269c972f0e3836da75ac7eb12ec23797d712b5b97",
            "a92a6388f50c1fcf97600493d6448604bd49bcccc49658be671572e3c4a27535",
            "9a567e42e206f912e48d7187963bea9ec45ba090038cbb3f82dbda69571dc2a4",
        ],
    ),
];

fn file(case: &str) -> Option<PathBuf> {
    let p = match case {
        "LAB-A" => {
            let t =
                std::fs::read_to_string(dati_percorsi::path("dati/formati/raw_veloce_alias.tsv")?)
                    .ok()?;
            let r = t
                .lines()
                .find(|l| l.starts_with("LAB-A\t"))?
                .split('\t')
                .nth(1)?
                .to_string();
            dati_percorsi::path(&format!("dati/{r}"))?
        }
        "PXD077492" => std::fs::read_dir(dati_percorsi::path("dati/formati/thermo/raw")?)
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
            // the name of the file is the one its download row (accession PXD070375) gives, in the list of big files of the data repository
            None => {
                let list = std::fs::read_to_string(dati_percorsi::path(
                    "dati/formati/_grandi_thermo_aperti.tsv",
                )?)
                .ok()?;
                let url = list
                    .lines()
                    .find(|l| l.contains("PXD070375") && l.contains(".raw"))?
                    .split('\t')
                    .find(|c| c.starts_with("http"))?;
                let name = url.rsplit('/').next()?;
                dati_percorsi::path(&format!("dati/formati/thermo/raw/{name}"))?
            }
        },
    };
    p.is_file().then_some(p)
}

#[test]
fn the_outputs_keep_their_pinned_hashes() {
    let Some(_) = dati_percorsi::radice() else {
        eprintln!("SKIP raw_equivalence: no data repository (MZLAB_DATI)");
        return;
    };
    let print = std::env::var_os("RAW_EQ_PRINT").is_some();
    for (case, pinned) in PINNED {
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
            *pinned,
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
        let mut l = readers::open(fs, &Limits::sito()).unwrap();
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
            let t =
                mzlab_core::transport::decode_scansione(k, &mzlab_core::transport::encode_scan(&b))
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
    let Some(dir) = dati_percorsi::path("dati/formati/thermo/raw") else {
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
        let Ok(mut l) = readers::open(fs, &Limits::sito()) else {
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

/// The trailer records stay in the file and are decoded when asked: a whole record (`get`), a few fields (`get_fields`) and the walk over all of them
/// (`for_each`) give the same value for every field of every record, so the three ways the reader reads a trailer agree on the real files.
#[test]
fn the_trailer_reads_agree_on_every_field() {
    let Some(_) = dati_percorsi::radice() else {
        eprintln!("SKIP raw_equivalence: no data repository (MZLAB_DATI)");
        return;
    };
    for (case, _) in PINNED {
        let Some(p) = file(case) else {
            eprintln!("SKIP raw_equivalence {case}: file not found");
            continue;
        };
        let mut src = std::io::BufReader::new(std::fs::File::open(&p).unwrap());
        let raw = opentfraw::RawFileReader::open(&mut src).unwrap();
        let t = &raw.scan_parameters;
        assert!(!t.is_empty(), "{case}: no trailer");
        let step = (t.len() / 300).max(1);
        let mut walked = vec![None; t.len()];
        let n_fields = t.get(&mut src, 0).unwrap().values.len();
        assert!(t.for_each(&mut src, &mut |i, view| {
            if i % step == 0 {
                walked[i] = Some(
                    (0..n_fields)
                        .map(|f| format!("{:?}", view.value(f)))
                        .collect::<Vec<_>>(),
                );
            }
        }));
        for i in (0..t.len()).step_by(step) {
            let whole = t.get(&mut src, i).unwrap();
            let fields: Vec<usize> = (0..n_fields).collect();
            let some = t.get_fields(&mut src, i, &fields).unwrap();
            let from_walk = walked[i].as_ref().unwrap();
            assert_eq!(whole.values.len(), n_fields, "{case}: record {i}");
            assert_eq!(some.values.len(), n_fields, "{case}: record {i}");
            for (f, ((w, s), walk)) in whole
                .values
                .iter()
                .zip(&some.values)
                .zip(from_walk)
                .enumerate()
            {
                let a = format!("{:?}", Some(&w.1));
                assert_eq!(
                    a,
                    format!("{:?}", Some(&s.1)),
                    "{case}: record {i} field {f} (get_fields)"
                );
                assert_eq!(&a, walk, "{case}: record {i} field {f} (for_each)");
            }
        }
    }
}
