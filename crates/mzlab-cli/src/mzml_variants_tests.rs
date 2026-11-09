//! The mzML variants of `tools/mzml_varianti.py` (the conformance matrix, docs/agenti/mzml-matrice.md) through `canali`, `features` and `validate`: each opens
//! or fails with the key the matrix states. Skipped without `python3` + numpy (the generator is a tool, not a runtime dependency).
use std::path::{Path, PathBuf};
use std::process::Command;

fn generate(dir: &Path) -> bool {
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/mzml_varianti.py");
    Command::new("python3")
        .arg("-I")
        .arg(script)
        .arg(dir)
        .status()
        .is_ok_and(|s| s.success())
}

fn cli(args: &[&str]) -> crate::Outcome {
    crate::execute(&args.iter().map(|x| x.to_string()).collect::<Vec<_>>())
}

/// Variants that open: `canali` lists the channels and `features` builds the table.
const OPEN: [&str; 16] = [
    "plain_64_raw.mzML",
    "plain_64_z.mzML",
    "plain_32_raw.mzML",
    "plain_32_z.mzML",
    "indexed.mzML",
    "bigoffsets.mzML",
    "crlf.mzML",
    "bom.mzML",
    "ms3_4.mzML",
    "ms3_only_idx.mzML",
    "profile.mzML",
    "mobility.mzML",
    "rpg.mzML",
    "scansettings.mzML",
    "chroms_all.mzML",
    "gz.mzML.gz",
];

/// Variants that fail cleanly, with the key of the error.
const REFUSED: [(&str, &str); 3] = [
    ("empty.mzML", "err.file.notMzml"),
    ("notxml.mzML", "err.file.notMzml"),
    ("badb64.mzML", "err.file.base64"),
];

/// Opens with the spectra it has; its scan table says how many its list declares (`table_of_truncated_file_declares_more_than_it_holds`).
const PARTIAL: [&str; 1] = ["truncated.mzML"];

#[test]
fn every_variant_opens_or_fails_with_its_key() {
    let dir = std::env::temp_dir().join(format!("mzlab_cli_var_{}", std::process::id()));
    if !generate(&dir) {
        eprintln!("skipped: python3 with numpy not available");
        return;
    }
    let p = |n: &str| -> PathBuf { dir.join(n) };
    for n in OPEN {
        let o = cli(&["canali", p(n).to_str().unwrap()]);
        assert_eq!(o.code, 0, "{n}: {}", o.stderr);
        let tmp = dir.join(format!("t_{n}"));
        std::fs::create_dir_all(&tmp).unwrap();
        let csv = crate::features(&[p(n)], "{}", &tmp).unwrap_or_else(|e| panic!("{n}: {e:?}"));
        assert!(csv.starts_with("id,mz,rt"), "{n}: {csv}");
    }
    for (n, key) in REFUSED {
        let o = cli(&["canali", p(n).to_str().unwrap()]);
        assert_ne!(o.code, 0, "{n}");
        assert!(o.stderr.contains(key), "{n}: {}", o.stderr);
    }
    let o = cli(&["canali", p("truncated.mzML").to_str().unwrap()]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    assert_eq!(PARTIAL.len() + OPEN.len() + REFUSED.len(), 20);
    let _ = std::fs::remove_dir_all(&dir);
}

/// `validate` agrees with the matrix on the structure of the files that open.
#[test]
fn validate_flags_the_truncated_file_and_the_garbage() {
    let dir = std::env::temp_dir().join(format!("mzlab_cli_val_{}", std::process::id()));
    if !generate(&dir) {
        return;
    }
    for n in ["truncated.mzML", "empty.mzML", "notxml.mzML", "badb64.mzML"] {
        let o = cli(&["validate", dir.join(n).to_str().unwrap()]);
        let all = format!("{}{}", String::from_utf8_lossy(&o.stdout), o.stderr);
        assert!(all.contains("INVALID"), "{n}: {all}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// The warning of the page and of the CLI rests on this: the declared count of the list against the rows read.
#[test]
fn the_table_of_a_truncated_file_declares_more_than_it_holds() {
    let dir = std::env::temp_dir().join(format!("mzlab_cli_trunc_{}", std::process::id()));
    if !generate(&dir) {
        eprintln!("skipped: python3 with numpy not available");
        return;
    }
    let count = |n: &str| {
        let f = dir.join(n);
        let size = std::fs::metadata(&f).unwrap().len();
        let t = mzlab_core::scantable::read(&mut std::fs::File::open(&f).unwrap(), size).unwrap();
        (t.rows.len(), t.declared)
    };
    let (read, declared) = count("truncated.mzML");
    assert!(declared > read && read > 0, "{read} of {declared}");
    let (read, declared) = count("plain_64_z.mzML");
    assert_eq!(read, declared);
    let _ = std::fs::remove_dir_all(&dir);
}
