//! RV-01: the memory work on the `.raw` reader (trailers and indexes kept compact) changes no result. For each real file the `.mzidx` (as `raw_tempi` builds it), the mzML of `lettori_mzml::scrivi`, the
//! `metodo/1` document and the header of an array of scans have the SHA-256 recorded before the change. Needs the private data repository (`MZLAB_DATI`, `dati_formati/raw_veloce_alias.tsv` for LAB-A);
//! the big DDA is looked for as `$MZLAB_RAW_BAF` or `dati_formati/thermo_raw/BAF_GEE_37.raw`. A file that is missing skips its case. `RAW_EQ_PRINT=1` prints the hashes instead of checking them.
use std::io::Cursor;
use std::path::{Path, PathBuf};

use mzlab_core::lettori::{self, FileSet, Lettore, Limiti, Membro};
use mzlab_core::{lettori_mzml, mzidx, upload};
use sha2::{Digest, Sha256};

fn dati() -> Option<PathBuf> {
    let d = PathBuf::from(std::env::var_os("MZLAB_DATI")?);
    d.is_dir().then_some(d)
}

fn hex(b: &[u8]) -> String {
    Sha256::digest(b)
        .iter()
        .map(|x| format!("{x:02x}"))
        .collect()
}

/// `(mzidx, mzml, metodo, intestazioni)` hashes of the file.
fn hashes(path: &Path) -> (String, String, String, String) {
    let nome = path.file_name().unwrap().to_string_lossy().into_owned();
    let dim = std::fs::metadata(path).unwrap().len();
    let set = || FileSet {
        membri: vec![Membro {
            nome: nome.clone(),
            dim,
            dati: Box::new(std::fs::File::open(path).unwrap()),
            sha1: None,
        }],
    };
    let mut fs = set();
    let imp = upload::impronte(&mut fs).expect("fingerprint");
    let mut l: Box<dyn Lettore> = lettori::apri(set(), &Limiti::sito()).expect("open");
    let p = upload::prepare_lettore(&mut *l, 0, &imp, &format!("{nome}.mzML")).expect("table");
    let mut out = Cursor::new(Vec::new());
    mzidx::build_lettore(&p.table, &mut *l, 0, &mut out, 192 << 20).expect("mzidx");
    let mzidx = hex(&out.into_inner());
    let mut mz = Sha256Sink::default();
    lettori_mzml::scrivi(&mut *l, 0, &mut mz, &mut |_, _| {}).expect("mzML");
    let metodo = hex(l.metodo(0).expect("metodo").as_bytes());
    let n = l.indice(0).unwrap().len();
    let mut h = Sha256::new();
    for k in (0..n).step_by((n / 200).max(1)) {
        for c in l.intestazione(0, k).expect("header") {
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
const PRIMA: &[(&str, [&str; 4])] = &[
    (
        "LAB-A",
        [
            "c8ca0a6568a025ed1c9533edb02f39dd28b1088788e94b98e3e9e3a05d3760df",
            "d749c8f9331701663d46ac4562a6c208a3e503804d10d0d07373a25deba965b7",
            "6ac0e6ce68d207d42ac94eb35bbb3e21060657bb878b84fa3bc13a223dd36e76",
            "fdb802dc4346660bdeba3cddf1d7cbb8a23d4b99d3e6d3d57a3253d61d9517d4",
        ],
    ),
    (
        "PXD077492",
        [
            "1516465e726f8ab470f4f6e2e3655bc6ac2dd63a16b57824caa05e7d7530e09f",
            "13f991a1a8f1d019ceb6790215e031a8a02fbf29270af759aebf939e31a39a03",
            "1006c10f52ddcfcf4bc358ce4cd7773e6bb4e5257865e9aaa72e159c022d81a3",
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

fn file(d: &Path, caso: &str) -> Option<PathBuf> {
    let p = match caso {
        "LAB-A" => {
            let t = std::fs::read_to_string(d.join("dati_formati/raw_veloce_alias.tsv")).ok()?;
            let r = t
                .lines()
                .find(|l| l.starts_with("LAB-A\t"))?
                .split('\t')
                .nth(1)?
                .to_string();
            d.join(r)
        }
        "PXD077492" => std::fs::read_dir(d.join("dati_formati/thermo_raw"))
            .ok()?
            .flatten()
            .map(|e| e.path())
            .find(|p| {
                p.file_name().is_some_and(|n| {
                    n.to_string_lossy().starts_with("PXD077492_")
                        && n.to_string_lossy().ends_with(".raw")
                })
            })?,
        _ => std::env::var_os("MZLAB_RAW_BAF")
            .map(PathBuf::from)
            .unwrap_or_else(|| d.join("dati_formati/thermo_raw/BAF_GEE_37.raw")),
    };
    p.is_file().then_some(p)
}

#[test]
fn la_memoria_compatta_non_cambia_nessun_risultato() {
    let Some(d) = dati() else {
        eprintln!("SKIP raw_equivalenza: no data repository (MZLAB_DATI)");
        return;
    };
    let print = std::env::var_os("RAW_EQ_PRINT").is_some();
    for (caso, prima) in PRIMA {
        let Some(p) = file(&d, caso) else {
            eprintln!("SKIP raw_equivalenza {caso}: file not found");
            continue;
        };
        let (a, b, c, e) = hashes(&p);
        if print {
            println!("    (\"{caso}\", [\"{a}\", \"{b}\", \"{c}\", \"{e}\"]),");
            continue;
        }
        assert_eq!(
            [a.as_str(), b.as_str(), c.as_str(), e.as_str()],
            *prima,
            "{caso}"
        );
    }
}

/// RV-05: `scansione_dati` (what the module sends: points, level, polarity, profile, RT and precursor from the index row) gives the same scan as `scansione`, and the transport
/// (`trasporto`) returns it as it is, for a sample of the scans of each file.
#[test]
fn scansione_dati_e_il_trasporto_danno_la_stessa_scansione() {
    let Some(d) = dati() else {
        eprintln!("SKIP raw_equivalenza: no data repository (MZLAB_DATI)");
        return;
    };
    for caso in ["LAB-A", "PXD077492", "BAF"] {
        let Some(p) = file(&d, caso) else {
            eprintln!("SKIP raw_equivalenza {caso}: file not found");
            continue;
        };
        let nome = p.file_name().unwrap().to_string_lossy().into_owned();
        let dim = std::fs::metadata(&p).unwrap().len();
        let fs = FileSet {
            membri: vec![Membro {
                nome,
                dim,
                dati: Box::new(std::fs::File::open(&p).unwrap()),
                sha1: None,
            }],
        };
        let mut l = lettori::apri(fs, &Limiti::sito()).unwrap();
        let indice = l.indice(0).unwrap().to_vec();
        let riletto =
            mzlab_core::trasporto::decode_indice(&mzlab_core::trasporto::encode_indice(&indice))
                .unwrap();
        assert_eq!(riletto, indice, "{caso}: index through the transport");
        let passo = (indice.len() / 300).max(1);
        for k in (0..indice.len()).step_by(passo) {
            let a = l.scansione(0, k).unwrap();
            let b = l.scansione_dati(0, k).unwrap();
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
                "{caso}: scan {k}"
            );
            assert_eq!(
                (&a.id, &a.filter),
                (&indice[k].id, &indice[k].filtro),
                "{caso}: row {k}"
            );
            let t = mzlab_core::trasporto::decode_scansione(
                k,
                &mzlab_core::trasporto::encode_scansione(&b),
            )
            .unwrap();
            assert_eq!(
                (&t.mz, &t.intensity, t.rt, t.profile),
                (&b.mz, &b.intensity, b.rt, b.profile),
                "{caso}: transport {k}"
            );
        }
    }
}
