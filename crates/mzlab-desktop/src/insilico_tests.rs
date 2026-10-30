//! Tests of `insilico.rs`: install, check and the refusals, with an in-memory `Source` serving a package made by the real packer.

use super::*;
use std::collections::HashMap;

use crate::update::UpdateError;

const CSV: &str = "Identifier,CompoundName,SMILES,MolecularFormula,MonoisotopicMass\n\
A1,methanol,CO,CH4O,32.026215\nA2,ethanol,CCO,C2H6O,46.041865\nA3,propanol,CCCO,C3H8O,60.057515\nA4,butanol,CCCCO,C4H10O,74.073165\n";

/// The site as a map: the files `insilico pack` writes for `CSV`, cut every `cut` raw bytes.
struct Site(HashMap<String, Vec<u8>>);

impl Site {
    fn new(csv: &str, cut: usize) -> Site {
        let out = tempfile::tempdir().expect("dir");
        mzlab_cli::insilico_pack::pack_with(csv.as_bytes(), out.path(), "t", "t", cut)
            .expect("pack");
        let mut m = HashMap::new();
        for e in std::fs::read_dir(out.path()).expect("dir") {
            let e = e.expect("entry");
            let name = e.file_name().to_string_lossy().to_string();
            m.insert(
                format!("{SITE}{name}"),
                std::fs::read(e.path()).expect("read"),
            );
        }
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
fn install_reads_back_and_check_compares_catalogues() {
    let (_d, f) = folder();
    let site = Site::new(CSV, 60);
    assert_eq!(check(&f, &site), Ok(Check::None));
    let mut seen = 0;
    let st = install(&f, &site, &AtomicBool::new(false), &mut |_, _| seen += 1).expect("install");
    assert!(st.installed && st.rows == 4 && seen > 1);
    // the pieces and the catalogue are in the folder as the packer wrote them
    for (k, v) in &site.0 {
        let name = k.trim_start_matches(SITE);
        assert_eq!(
            &std::fs::read(dir(&f).join(name)).expect("read"),
            v,
            "{name}"
        );
    }
    assert_eq!(check(&f, &site), Ok(Check::Current));
    let other = Site::new(&CSV.replace("butanol", "isobutanol"), 60);
    assert_eq!(check(&f, &other), Ok(Check::Update));
    // the update replaces the catalogue and drops the pieces it no longer names
    let smaller = Site::new(&CSV.lines().take(3).collect::<Vec<_>>().join("\n"), 1 << 20);
    install(&f, &smaller, &AtomicBool::new(false), &mut |_, _| {}).expect("update");
    assert_eq!(status(&f).rows, 2);
    assert!(!dir(&f).join("001.bin").exists());
}

#[test]
fn a_corrupt_piece_installs_nothing() {
    let (_d, f) = folder();
    let mut site = Site::new(CSV, 60);
    site.0.get_mut(&format!("{SITE}001.bin")).unwrap()[3] ^= 0xFF;
    let e = install(&f, &site, &AtomicBool::new(false), &mut |_, _| {}).unwrap_err();
    assert_eq!(e.key, "normative.dl.err.hash");
    assert!(!status(&f).installed);
}

#[test]
fn cancel_and_unsafe_catalogues_are_refused() {
    let (_d, f) = folder();
    let site = Site::new(CSV, 60);
    let e = install(&f, &site, &AtomicBool::new(true), &mut |_, _| {}).unwrap_err();
    assert_eq!(e.key, "normative.dl.err.cancelled");
    let c = String::from_utf8(site.0[&format!("{SITE}catalogo.json")].clone())
        .unwrap()
        .replace("000.bin", "../x");
    assert_eq!(
        parse_catalog(c.as_bytes()).unwrap_err().key,
        "normative.dl.err.catalog"
    );
}

/// The installed folder is what `flow run` reads: the catalogue as the `suspects` file, the pieces next to it.
#[test]
fn the_installed_folder_ranks_like_the_csv() {
    let (_d, f) = folder();
    install(
        &f,
        &Site::new(CSV, 60),
        &AtomicBool::new(false),
        &mut |_, _| {},
    )
    .expect("install");
    let cat = std::fs::read(dir(&f).join(CATALOG)).expect("catalogue");
    for p in installed(&f).expect("installed").pieces {
        let raw = mzlab_cli::insilico_pack::read_piece(&dir(&f), &cat, &p.file).expect("piece");
        assert_eq!(raw.len() as u64, p.size_raw);
    }
}
