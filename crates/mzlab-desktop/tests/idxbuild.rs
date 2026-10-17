//! The native builder of the desktop writes, from a memory map, the same `.mzidx` as the Python builder (`tools/mzidx_riferimento.py`), through the
//! routes `idx/build`, `idx/read`, `idx/remove`; several files are built at once on the thread pool; bad names and bad tables are refused.
//! Without python3 and numpy the parity test is skipped (`MZLAB_IDX_STRICT=1` fails instead).

use std::path::{Path, PathBuf};
use std::process::Command;

use mzlab_desktop::idxbuild::decode_inputs;
use mzlab_desktop::{handle, handle_body, State};
use serde::Deserialize;
use serde_json::value::RawValue;

#[derive(Deserialize)]
struct Meta {
    source: Box<RawValue>,
    run: Box<RawValue>,
    sections: Vec<Section>,
}
#[derive(Deserialize)]
struct Section {
    dtype: String,
}

fn u64_at(b: &[u8], at: usize) -> usize {
    u64::from_le_bytes(b[at..at + 8].try_into().unwrap()) as usize
}

/// The body the page posts, made of a reference file (what the Python reader knew).
fn body_of(reference: &[u8]) -> Vec<u8> {
    let (meta_off, meta_len, scans_off, scans_len) = (
        u64_at(reference, 16),
        u64_at(reference, 24),
        u64_at(reference, 32),
        u64_at(reference, 40),
    );
    let meta: Meta = serde_json::from_slice(&reference[meta_off..meta_off + meta_len]).unwrap();
    let scans = &reference[scans_off..scans_off + scans_len];
    let rows: Vec<Vec<serde_json::Value>> = serde_json::from_slice(scans).unwrap();
    let col = |c: usize| {
        rows.iter()
            .map(|r| r[c].as_f64().unwrap())
            .collect::<Vec<f64>>()
    };
    let f64b = |v: Vec<f64>| v.iter().flat_map(|x| x.to_le_bytes()).collect::<Vec<u8>>();
    let items: Vec<Vec<u8>> = vec![
        col(2).iter().map(|&v| v as u8).collect(),
        col(4).iter().map(|&v| (v as i8) as u8).collect(),
        f64b(col(3)),
        f64b(col(5)),
        scans.to_vec(),
        meta.source.get().as_bytes().to_vec(),
        meta.run.get().as_bytes().to_vec(),
        vec![meta.sections.iter().any(|s| s.dtype == "<f4") as u8],
    ];
    let mut out = Vec::new();
    for it in items {
        out.extend_from_slice(&(it.len() as u32).to_le_bytes());
        out.extend_from_slice(&it);
    }
    out
}

fn reference() -> Option<(PathBuf, Vec<String>)> {
    let out = std::env::temp_dir().join(format!("mzlab-desk-idx-{}", std::process::id()));
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let o = Command::new("python3")
        .current_dir(&root)
        .arg("tools/mzidx_riferimento.py")
        .arg(&out)
        .output();
    match o {
        Ok(o) if o.status.success() => Some((
            out,
            String::from_utf8(o.stdout)
                .unwrap()
                .lines()
                .map(|l| l.to_string())
                .collect(),
        )),
        other => {
            if std::env::var("MZLAB_IDX_STRICT").is_ok() {
                panic!("the reference builder did not run: {other:?}");
            }
            eprintln!("python3/numpy not available: test skipped");
            None
        }
    }
}

fn read_all(state: &State, name: &str, size: usize) -> Vec<u8> {
    let mut got = Vec::new();
    while got.len() < size {
        let r = handle(
            state,
            "idx/read",
            &format!("name={name}&off={}&len=65536", got.len()),
        );
        assert_eq!(r.status, 200);
        assert!(!r.body.is_empty());
        got.extend_from_slice(&r.body);
    }
    got
}

#[test]
fn native_index_is_identical_to_the_python_one_and_files_build_in_parallel() {
    let Some((dir, files)) = reference() else {
        return;
    };
    let state = State::new();
    state.set_index_folder(dir.join("cache")).unwrap();
    let picked: Vec<(u32, String, PathBuf)> = files
        .iter()
        .map(|f| {
            let p = PathBuf::from(f);
            let (id, _) = state.pick(p.clone()).unwrap();
            (id, p.file_stem().unwrap().to_string_lossy().to_string(), p)
        })
        .collect();
    assert!(picked.len() >= 8);
    // all the files at the same time, from threads: the pool of the builder takes them as threads become free
    std::thread::scope(|s| {
        for (id, stem, _) in &picked {
            let (state, dir) = (&state, &dir);
            s.spawn(move || {
                let reference = std::fs::read(dir.join(format!("{stem}.mzidx"))).unwrap();
                let r = handle_body(
                    state,
                    "idx/build",
                    &format!("id={id}&name={stem}.mzML"),
                    &body_of(&reference),
                );
                assert_eq!(
                    r.status,
                    200,
                    "{stem}: {}",
                    String::from_utf8_lossy(&r.body)
                );
                let got = read_all(state, &format!("{stem}.mzML"), reference.len());
                assert!(
                    got == reference,
                    "{stem}: the native index differs from the Python one"
                );
            });
        }
    });
    let some = &picked[0].1;
    assert!(dir
        .join("cache")
        .join(format!("{some}.mzML.mzidx"))
        .exists());
    assert!(
        !dir.join("cache")
            .join(format!("{some}.mzML.mzidx.part"))
            .exists(),
        "no half-written file stays"
    );
    let left: Vec<_> = std::fs::read_dir(dir.join("cache"))
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().ends_with(".sp.part"))
        .collect();
    assert!(left.is_empty(), "no scratch file stays: {left:?}");
    handle(&state, "idx/remove", &format!("name={some}.mzML"));
    assert!(!dir
        .join("cache")
        .join(format!("{some}.mzML.mzidx"))
        .exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn bad_names_and_tables_are_refused() {
    let state = State::new();
    state
        .set_index_folder(
            std::env::temp_dir().join(format!("mzlab-desk-bad-{}", std::process::id())),
        )
        .unwrap();
    let (id, _) = state
        .pick(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"))
        .unwrap();
    for name in ["..", "a/b", "a%2Fb", ""] {
        let r = handle_body(
            &state,
            "idx/build",
            &format!("id={id}&name={name}"),
            &[0; 64],
        );
        assert_ne!(r.status, 200, "{name}");
    }
    assert!(decode_inputs(&[1, 2, 3]).is_err());
    let r = handle_body(&state, "idx/build", &format!("id={id}&name=x.mzML"), &[]);
    assert_ne!(r.status, 200);
    let r = handle(&state, "idx/read", "name=nothere.mzML&off=0&len=10");
    assert_ne!(r.status, 200);
}
