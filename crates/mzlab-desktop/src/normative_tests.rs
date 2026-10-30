//! Tests of `normative.rs`: install, check, import and the refusals, with an in-memory `Source`.

use super::*;
use std::collections::HashMap;
use std::io::Write;

use crate::update::UpdateError;

fn mini() -> Vec<u8> {
    std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/normative/mini.mzsus"))
        .expect("fixture")
}

/// The site as a map: the catalogue and the pieces of `raw`, cut every `cut` bytes.
struct Site(HashMap<String, Vec<u8>>);

impl Site {
    fn new(raw: &[u8], cut: usize) -> Site {
        let mut e = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::new(9));
        e.write_all(raw).expect("zlib");
        let z = e.finish().expect("zlib");
        let mut m = HashMap::new();
        let mut pieces = Vec::new();
        for (i, b) in z.chunks(cut).enumerate() {
            let f = format!("{i:03}.bin");
            pieces.push(Piece {
                file: f.clone(),
                size: b.len() as u64,
                sha256: hex(b),
            });
            m.insert(format!("{PIECES}{f}"), b.to_vec());
        }
        let cat = Catalog {
            format: FORMAT.into(),
            size: raw.len() as u64,
            size_compressed: z.len() as u64,
            sha256: hex(raw),
            rows: 4,
            pieces,
        };
        m.insert(CATALOG.into(), serde_json::to_vec(&cat).expect("json"));
        Site(m)
    }
}

impl Source for Site {
    fn get(&self, rel: &str) -> Result<Vec<u8>, UpdateError> {
        self.0.get(rel).cloned().ok_or(UpdateError {
            key: "update.error.http",
            detail: "404".into(),
        })
    }
}

fn folder() -> (tempfile::TempDir, Folder) {
    let d = tempfile::tempdir().expect("dir");
    let f = Folder::new(d.path().to_path_buf());
    (d, f)
}

#[test]
fn install_reads_back_listings_and_check_compares_catalogues() {
    let (_d, f) = folder();
    let site = Site::new(&mini(), 200);
    assert!(site.0.len() > 3, "several pieces");
    assert_eq!(check(&f, &site), Ok(Check::None));
    assert_eq!(
        catalog_text(&f).unwrap_err().key,
        "normative.dl.err.missing"
    );
    let cancel = AtomicBool::new(false);
    let mut seen = 0;
    let st = install(&f, &site, &cancel, &mut |_, _| seen += 1).expect("install");
    assert!(st.installed && st.size == mini().len() as u64 && seen > 1);
    assert_eq!(check(&f, &site), Ok(Check::Current));
    let want = Suspects::open(&mini()).unwrap().listings_json().unwrap();
    assert_eq!(listings_json(&f).unwrap(), want);
    // the Library window reads the site's catalogue bytes back unchanged
    assert_eq!(catalog_text(&f).unwrap().as_bytes(), &site.0[CATALOG][..]);
    // another version on the site: an update is offered, the installed file is untouched
    let mut other = mini();
    other.extend_from_slice(&[0u8; 8]);
    assert_eq!(check(&f, &Site::new(&other, 200)), Ok(Check::Update));
}

#[test]
fn a_corrupt_piece_installs_nothing() {
    let (_d, f) = folder();
    let mut site = Site::new(&mini(), 200);
    site.0.get_mut(&format!("{PIECES}001.bin")).unwrap()[3] ^= 0xFF;
    let e = install(&f, &site, &AtomicBool::new(false), &mut |_, _| {}).unwrap_err();
    assert_eq!(e.key, "normative.dl.err.hash");
    assert!(!status(&f).installed && f.stat(FILE).is_err());
}

#[test]
fn cancel_and_unsafe_catalogues_are_refused() {
    let (_d, f) = folder();
    let site = Site::new(&mini(), 200);
    let e = install(&f, &site, &AtomicBool::new(true), &mut |_, _| {}).unwrap_err();
    assert_eq!(e.key, "normative.dl.err.cancelled");
    let c = String::from_utf8(site.0[CATALOG].clone())
        .unwrap()
        .replace("000.bin", "../x");
    assert_eq!(
        parse_catalog(c.as_bytes()).unwrap_err().key,
        "normative.dl.err.catalog"
    );
}

#[test]
fn import_takes_a_mzsus_or_a_package_folder() {
    let (d, f) = folder();
    let pick = tempfile::tempdir().expect("dir");
    let file = pick.path().join("x.mzsus");
    std::fs::write(&file, mini()).unwrap();
    assert!(import(&f, &file).unwrap().installed);
    assert_eq!(
        listings_json(&f).unwrap(),
        Suspects::open(&mini()).unwrap().listings_json().unwrap()
    );
    // not a package: refused, the installed file stays
    let bad = pick.path().join("bad.mzsus");
    std::fs::write(&bad, b"MZSU nope").unwrap();
    assert!(import(&f, &bad).is_err() && status(&f).installed);
    // a folder whose catalogue disagrees with the file
    let site = Site::new(&mini(), 4096);
    let dir = pick.path().join("pkg");
    std::fs::create_dir(&dir).unwrap();
    std::fs::write(dir.join(FILE), mini()).unwrap();
    std::fs::write(dir.join("catalogo.json"), &site.0[CATALOG]).unwrap();
    assert!(import(&f, &dir).unwrap().installed);
    let mut other = mini();
    other[40] ^= 1;
    std::fs::write(dir.join(FILE), &other).unwrap();
    assert!(import(&f, &dir).is_err());
    drop(d);
}

const OFFICIAL_CSV: &str = include_str!("../../mzlab-feat/tests/data/official/candidate-list.csv");
const OFFICIAL_CAS: &str = include_str!("../../mzlab-feat/tests/data/official/cas.tsv");

/// The site with the CAS sidecar only (`static/mzfinder/sospetti/cas/`).
fn cas_site() -> Site {
    let raw = OFFICIAL_CAS.as_bytes();
    let mut e = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::new(9));
    e.write_all(raw).expect("zlib");
    let z = e.finish().expect("zlib");
    let cat = Catalog {
        format: CAS_FORMAT.into(),
        size: raw.len() as u64,
        size_compressed: z.len() as u64,
        sha256: hex(raw),
        rows: 2,
        pieces: vec![Piece {
            file: "000.bin".into(),
            size: z.len() as u64,
            sha256: hex(&z),
        }],
    };
    let mut m = HashMap::new();
    m.insert(
        CAS_CATALOG.to_string(),
        serde_json::to_vec(&cat).expect("json"),
    );
    m.insert(format!("{CAS_PIECES}000.bin"), z);
    Site(m)
}

#[test]
fn an_official_file_is_imported_with_the_sidecar_of_the_site_and_kept_private() {
    let (_d, f) = folder();
    let summary = import_official(
        &f,
        &cas_site(),
        ("svhc", "candidate-list.csv", "2026-10-30"),
        OFFICIAL_CSV.as_bytes(),
    )
    .expect("import");
    assert!(summary.contains("\"unresolved\":1") && summary.contains("candidate-list.csv"));
    // the same bytes as the function the CLI and the .wasm run
    let label = r#"{"date":"2026-10-30","file":"candidate-list.csv"}"#;
    let want = mzlab_feat::normative::official::import_official(
        "svhc",
        OFFICIAL_CSV.as_bytes(),
        OFFICIAL_CAS,
        label,
    )
    .expect("direct");
    let n = u32::from_le_bytes([want[0], want[1], want[2], want[3]]) as usize;
    let st = f.stat("normative-imported-SVHC.mzsus").expect("stored");
    assert_eq!(
        f.read("normative-imported-SVHC.mzsus", 0, st.size)
            .expect("read"),
        &want[4 + n..]
    );
    // the sidecar stays for the next import, offline
    assert!(f.stat(CAS_FILE).is_ok());
    let lists: serde_json::Value = serde_json::from_str(&imported(&f)).expect("json");
    assert_eq!(lists["lists"][0]["id"], "SVHC");
    assert_eq!(lists["lists"][0]["coverage"]["screenable"], 2);
}

#[test]
fn an_import_without_the_sidecar_offline_says_so() {
    let (_d, f) = folder();
    let e = import_official(
        &f,
        &Site(HashMap::new()),
        ("svhc", "a.csv", "2026-10-30"),
        b"x",
    )
    .unwrap_err();
    assert_eq!(e.key, "normative.dl.err.http");
}
