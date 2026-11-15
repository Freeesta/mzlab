//! Tests of `update.rs`: manifest parsing (unsafe, non-canonical and colliding keys, sizes, desktop_min), diff and check, the patch of
//! `index.html`, `serve_override` and the HTTP source; the tests that change folders are in `update_apply_tests.rs`.
use super::*;
use std::sync::Mutex;

struct Mem(BTreeMap<String, Vec<u8>>, Mutex<Vec<String>>);
impl Source for Mem {
    fn get(&self, rel: &str) -> Result<Vec<u8>, UpdateError> {
        self.1.lock().unwrap().push(rel.to_string());
        self.0
            .get(rel)
            .cloned()
            .ok_or_else(|| UpdateError::new("update.error.http", "404"))
    }
}

fn manifest(version: &str, min: &str, files: &[(&str, &[u8])]) -> Manifest {
    Manifest {
        version: version.into(),
        desktop_min: min.into(),
        files: files
            .iter()
            .map(|(n, b)| {
                (
                    n.to_string(),
                    Entry {
                        sha256: sha256_hex(b),
                        size: b.len() as u64,
                    },
                )
            })
            .collect(),
    }
}
fn mem(files: &[(&str, &[u8])]) -> Mem {
    Mem(
        files
            .iter()
            .map(|(n, b)| (n.to_string(), b.to_vec()))
            .collect(),
        Mutex::new(vec![]),
    )
}

const INDEX: &str = "<meta content=\"default-src 'self'; connect-src 'self'; img-src data:\"><script src=\"static/browser.js\"></script>";

#[test]
fn sha256_known_value() {
    assert_eq!(
        sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn native_version_compare() {
    assert!(native_ok("0.1.0", "0.1.0"));
    assert!(native_ok("0.2.0", "0.1.9"));
    assert!(native_ok("1.0.0", "0.9.9"));
    assert!(!native_ok("0.1.0", "0.1.1"));
    assert!(!native_ok("0.9.0", "0.10.0"));
}

#[test]
fn diff_lists_only_changed_new_and_not_native() {
    let local = manifest(
        "a",
        "0.1.0",
        &[
            ("index.html", b"1"),
            ("static/a.js", b"a"),
            ("static/pyodide/p.wasm", b"big"),
        ],
    );
    let remote = manifest(
        "b",
        "0.1.0",
        &[
            ("index.html", b"2"),
            ("static/a.js", b"a"),
            ("static/pyodide/p.wasm", b"big"),
            ("static/n.js", b"n"),
            ("static/rust-worker.js", b"x"),
            ("sw.js", b"s"),
        ],
    );
    assert_eq!(
        diff(&local, &remote),
        vec!["index.html".to_string(), "static/n.js".to_string()]
    );
}

#[test]
fn check_cases() {
    let a = manifest("a", "0.1.0", &[("static/a.js", b"a")]);
    assert_eq!(
        check(&a, &a, "0.1.0"),
        Check::Uptodate {
            version: "a".into()
        }
    );
    let b = manifest("b", "0.1.0", &[("static/a.js", b"aa")]);
    assert_eq!(
        check(&a, &b, "0.1.0"),
        Check::Available {
            version: "b".into(),
            files: 1,
            bytes: 2
        }
    );
    let c = manifest("c", "0.2.0", &[("static/a.js", b"aa")]);
    assert!(matches!(check(&a, &c, "0.1.0"), Check::Newapp { .. }));
}

#[test]
fn rejects_unsafe_paths() {
    for bad in ["../x", "/etc/passwd", "a/../../b", "a\\b", "C:/x", ""] {
        assert!(safe_rel(bad).is_err(), "{bad}");
    }
    assert!(safe_rel("static/pyodide/pyodide.asm.wasm").is_ok());
}

fn manifest_json(min: &str, keys: &[&str]) -> Vec<u8> {
    let files: BTreeMap<&str, Entry> = keys
        .iter()
        .map(|k| {
            let e = Entry {
                sha256: sha256_hex(b"x"),
                size: 1,
            };
            (*k, e)
        })
        .collect();
    serde_json::to_vec(&serde_json::json!({ "version": "v", "desktop_min": min, "files": files }))
        .unwrap()
}

#[test]
fn rejects_non_canonical_and_colliding_keys() {
    assert!(parse_manifest(&manifest_json(
        "0.1.0",
        &["index.html", "static/a.js", "static/rust-worker.js"]
    ))
    .is_ok());
    for keys in [
        &["a/./b.js"][..],
        &["a//b.js"],
        &["a/"],
        &["index.html/."],
        &["static/./desktop-shim.js"],
        &["a/B.js", "a/b.js"],
        &["Index.html"],
        &["CHECK.JSON"],
        &["Static/Manifest.json"],
        &["static/Desktop-Shim.js"],
    ] {
        let err = parse_manifest(&manifest_json("0.1.0", keys)).unwrap_err();
        assert_eq!(err.key, "update.error.manifest", "{keys:?}");
    }
    for min in [
        "v2.0.0",
        "2.0",
        "2.0.0.1",
        "2..0",
        "99999999999999999999.0.0",
        "",
    ] {
        let err = parse_manifest(&manifest_json(min, &["static/a.js"])).unwrap_err();
        assert_eq!(err.key, "update.error.manifest", "{min}");
    }
}

#[test]
fn rejects_a_total_size_that_overflows() {
    let big = Entry {
        sha256: sha256_hex(b"x"),
        size: u64::MAX,
    };
    let json = serde_json::json!({ "version": "v", "desktop_min": "0.1.0",
        "files": { "static/a.js": big, "static/b.js": big } });
    let err = parse_manifest(&serde_json::to_vec(&json).unwrap()).unwrap_err();
    assert_eq!(err.key, "update.error.manifest");
    let one = serde_json::json!({ "version": "v", "desktop_min": "0.1.0", "files": { "static/a.js": big } });
    assert!(parse_manifest(&serde_json::to_vec(&one).unwrap()).is_ok());
}

#[test]
fn patch_index_matches_prepare_py() {
    let p = patch_index(INDEX).unwrap();
    assert!(
        p.contains("connect-src 'self' mzlab: http://mzlab.localhost ipc: http://ipc.localhost;")
    );
    assert!(p.contains("<script src=\"static/desktop-shim.js\"></script>\n<script src=\"static/browser.js\"></script>"));
    assert!(patch_index("<html></html>").is_err());
}

#[path = "update_apply_tests.rs"]
mod apply;

#[test]
fn serve_override_never_leaves_the_folder() {
    let tmp = tempfile::tempdir().unwrap();
    fs::create_dir(tmp.path().join("web")).unwrap();
    fs::write(tmp.path().join("secret.txt"), b"s").unwrap();
    assert!(serve_override(&tmp.path().join("web"), "/../secret.txt").is_none());
    assert!(serve_override(&tmp.path().join("web"), "/check.json").is_none());
}

#[test]
fn http_source_reads_a_file_and_reports_offline() {
    use std::io::{Read, Write};
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = l.local_addr().unwrap().port();
    let h = std::thread::spawn(move || {
        for _ in 0..2 {
            let (mut c, _) = l.accept().unwrap();
            let mut buf = [0u8; 2048];
            let n = c.read(&mut buf).unwrap();
            let first = String::from_utf8_lossy(&buf[..n])
                .lines()
                .next()
                .unwrap_or("")
                .to_string();
            let resp = if first.contains("static/a.js") {
                "HTTP/1.1 200 OK\r\nContent-Length: 3\r\nConnection: close\r\n\r\nabc"
            } else {
                "HTTP/1.1 302 Found\r\nLocation: http://elsewhere/\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            };
            c.write_all(resp.as_bytes()).unwrap();
        }
    });
    let src = Http::with_base(&format!("http://127.0.0.1:{port}/"));
    assert_eq!(src.get("static/a.js").unwrap(), b"abc");
    // a redirect is not followed: the host cannot change
    assert_eq!(
        src.get("static/other.js").unwrap_err().key,
        "update.error.http"
    );
    h.join().unwrap();
    let down = Http::with_base(&format!("http://127.0.0.1:{port}/"));
    assert_eq!(
        down.get("static/a.js").unwrap_err().key,
        "update.error.offline"
    );
    assert!(down.get("../x").is_err());
}
