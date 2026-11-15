//! Tests of `update.rs`: manifest parsing and canonical keys, diff and check, `apply` (only changed files, removed files dropped,
//! the folder as base only when it matches `local`), `recover` (interrupted swap, damaged or incompatible folder) and the HTTP source.
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

#[test]
fn apply_downloads_only_changed_files_and_swaps() {
    let tmp = tempfile::tempdir().unwrap();
    let data = tmp.path();
    let local = manifest(
        "a",
        "0.1.0",
        &[
            ("index.html", INDEX.as_bytes()),
            ("static/big.wasm", b"BIG"),
            ("static/a.js", b"old"),
        ],
    );
    let remote = manifest(
        "b",
        "0.1.0",
        &[
            ("index.html", INDEX.as_bytes()),
            ("static/big.wasm", b"BIG"),
            ("static/a.js", b"new"),
            ("static/n.js", b"n"),
        ],
    );
    let src = mem(&[
        ("static/a.js", b"new"),
        ("static/n.js", b"n"),
        ("static/big.wasm", b"BIG"),
    ]);
    let mut seen = vec![];
    let n = apply(data, &local, &remote, "0.1.0", &src, |p| {
        seen.push((p.done, p.total))
    })
    .unwrap();
    assert_eq!(n, 2);
    let mut asked = src.1.lock().unwrap().clone();
    asked.sort();
    assert_eq!(
        asked,
        vec!["static/a.js".to_string(), "static/n.js".to_string()]
    ); // the big file is not asked for
    assert_eq!(seen, vec![(1, 2), (2, 2)]);
    let web = data.join("web");
    assert_eq!(fs::read(web.join("static/a.js")).unwrap(), b"new");
    assert!(validate_web(&web));
    assert!(!data.join("web.next").exists() && !data.join("web.old").exists());
    // the manifest of the folder is the new one: a second check finds nothing to do
    let now = local_manifest(data, b"{}").unwrap();
    assert_eq!(
        check(&now, &remote, "0.1.0"),
        Check::Uptodate {
            version: "b".into()
        }
    );
    assert_eq!(serve_override(&web, "/static/a.js").unwrap().0, b"new");
    assert!(serve_override(&web, "/static/big.wasm").is_none());
}

#[test]
fn index_is_patched_and_files_of_an_earlier_update_are_kept() {
    let tmp = tempfile::tempdir().unwrap();
    let data = tmp.path();
    let v1 = manifest(
        "1",
        "0.1.0",
        &[("index.html", INDEX.as_bytes()), ("static/a.js", b"a1")],
    );
    let v2 = manifest(
        "2",
        "0.1.0",
        &[("index.html", INDEX.as_bytes()), ("static/a.js", b"a2")],
    );
    let v3 = manifest(
        "3",
        "0.1.0",
        &[
            ("index.html", b"<x>"),
            ("static/a.js", b"a2"),
            ("static/b.js", b"b3"),
        ],
    );
    let emb = manifest(
        "0",
        "0.1.0",
        &[("index.html", b"<0>"), ("static/a.js", b"a0")],
    );
    apply(
        data,
        &emb,
        &v1,
        "0.1.0",
        &mem(&[("index.html", INDEX.as_bytes()), ("static/a.js", b"a1")]),
        |_| {},
    )
    .unwrap();
    let idx = String::from_utf8(serve_override(&data.join("web"), "/").unwrap().0).unwrap();
    assert!(idx.contains("desktop-shim.js"));
    apply(
        data,
        &v1,
        &v2,
        "0.1.0",
        &mem(&[("static/a.js", b"a2")]),
        |_| {},
    )
    .unwrap();
    assert!(idx.contains("mzlab:") && serve_override(&data.join("web"), "/index.html").is_some());
    // v3 has an index.html without the markers: the update fails and nothing changes
    let err = apply(
        data,
        &v2,
        &v3,
        "0.1.0",
        &mem(&[("index.html", b"<x>"), ("static/b.js", b"b3")]),
        |_| {},
    )
    .unwrap_err();
    assert_eq!(err.key, "update.error.manifest");
    assert_eq!(fs::read(data.join("web/static/a.js")).unwrap(), b"a2");
    assert!(validate_web(&data.join("web")) && !data.join("web.next").exists());
}

#[test]
fn removed_file_is_gone_after_update() {
    let tmp = tempfile::tempdir().unwrap();
    let data = tmp.path();
    let emb = manifest("0", "0.1.0", &[("static/a.js", b"a0")]);
    let v1 = manifest(
        "1",
        "0.1.0",
        &[
            ("index.html", INDEX.as_bytes()),
            ("static/a.js", b"a1"),
            ("static/old.js", b"o1"),
        ],
    );
    let v2 = manifest(
        "2",
        "0.1.0",
        &[("index.html", INDEX.as_bytes()), ("static/a.js", b"a2")],
    );
    let src = mem(&[
        ("index.html", INDEX.as_bytes()),
        ("static/a.js", b"a1"),
        ("static/old.js", b"o1"),
    ]);
    apply(data, &emb, &v1, "0.1.0", &src, |_| {}).unwrap();
    let web = data.join("web");
    assert!(web.join("static/old.js").is_file());
    apply(
        data,
        &v1,
        &v2,
        "0.1.0",
        &mem(&[("static/a.js", b"a2")]),
        |_| {},
    )
    .unwrap();
    assert!(!web.join("static/old.js").exists());
    assert!(serve_override(&web, "/static/old.js").is_none());
    assert!(validate_web(&web));
    let check: BTreeMap<String, Entry> =
        serde_json::from_slice(&fs::read(web.join(CHECK)).unwrap()).unwrap();
    assert!(!check.contains_key("static/old.js"));
    assert_eq!(serve_override(&web, "/static/a.js").unwrap().0, b"a2");
}

#[test]
fn fallback_manifest_does_not_keep_stale_files() {
    let tmp = tempfile::tempdir().unwrap();
    let data = tmp.path();
    let emb = manifest("0", "0.1.0", &[("static/a.js", b"a0")]);
    let v1 = manifest("1", "0.1.0", &[("static/a.js", b"a1")]);
    let v2 = manifest("2", "0.1.0", &[("static/a.js", b"a0")]);
    apply(
        data,
        &emb,
        &v1,
        "0.1.0",
        &mem(&[("static/a.js", b"a1")]),
        |_| {},
    )
    .unwrap();
    let web = data.join("web");
    assert_eq!(serve_override(&web, "/static/a.js").unwrap().0, b"a1");
    // `local` is the embedded manifest (the folder's one did not parse): the folder is not the base of the new one
    apply(
        data,
        &emb,
        &v2,
        "0.1.0",
        &mem(&[("static/a.js", b"a0")]),
        |_| {},
    )
    .unwrap();
    assert!(serve_override(&web, "/static/a.js").is_none_or(|(b, _)| b == b"a0"));
    assert!(validate_web(&web));
    // the same through `local_manifest` with a manifest of the folder that does not parse
    apply(
        data,
        &v2,
        &v1,
        "0.1.0",
        &mem(&[("static/a.js", b"a1")]),
        |_| {},
    )
    .unwrap();
    fs::write(web.join(MANIFEST), b"not json").unwrap();
    let local = local_manifest(data, &serde_json::to_vec(&emb).unwrap()).unwrap();
    assert_eq!(local, emb);
    apply(
        data,
        &local,
        &v2,
        "0.1.0",
        &mem(&[("static/a.js", b"a0")]),
        |_| {},
    )
    .unwrap();
    assert!(serve_override(&web, "/static/a.js").is_none_or(|(b, _)| b == b"a0"));
}

#[test]
fn corrupt_download_changes_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let data = tmp.path();
    let local = manifest("a", "0.1.0", &[("static/a.js", b"old")]);
    let remote = manifest("b", "0.1.0", &[("static/a.js", b"new")]);
    let err = apply(
        data,
        &local,
        &remote,
        "0.1.0",
        &mem(&[("static/a.js", b"nex")]),
        |_| {},
    )
    .unwrap_err();
    assert_eq!(err.key, "update.error.hash");
    assert!(!data.join("web").exists() && !data.join("web.next").exists());
}

#[test]
fn offline_changes_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let local = manifest("a", "0.1.0", &[("static/a.js", b"old")]);
    let remote = manifest("b", "0.1.0", &[("static/a.js", b"new")]);
    struct Down;
    impl Source for Down {
        fn get(&self, _: &str) -> Result<Vec<u8>, UpdateError> {
            Err(UpdateError::new("update.error.offline", "no route"))
        }
    }
    let err = apply(tmp.path(), &local, &remote, "0.1.0", &Down, |_| {}).unwrap_err();
    assert_eq!(err.key, "update.error.offline");
    assert!(!tmp.path().join("web").exists());
}

#[test]
fn newer_web_part_than_the_binary_is_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let local = manifest("a", "0.1.0", &[("static/a.js", b"old")]);
    let remote = manifest("b", "0.3.0", &[("static/a.js", b"new")]);
    let err = apply(
        tmp.path(),
        &local,
        &remote,
        "0.1.0",
        &mem(&[("static/a.js", b"new")]),
        |_| {},
    )
    .unwrap_err();
    assert_eq!(err.key, "update.error.newapp");
}

#[test]
fn damaged_folder_is_set_aside_and_embedded_copy_takes_over() {
    let tmp = tempfile::tempdir().unwrap();
    let data = tmp.path();
    let local = manifest("a", "0.1.0", &[("static/a.js", b"old")]);
    let remote = manifest("b", "0.1.0", &[("static/a.js", b"new")]);
    apply(
        data,
        &local,
        &remote,
        "0.1.0",
        &mem(&[("static/a.js", b"new")]),
        |_| {},
    )
    .unwrap();
    fs::write(data.join("web/static/a.js"), b"corrupt").unwrap();
    assert!(!validate_web(&data.join("web")));
    recover(data, "0.1.0");
    assert!(!data.join("web").exists());
    assert!(serve_override(&data.join("web"), "/static/a.js").is_none());
    // the manifest falls back to the embedded one, so the next check downloads the file again
    let emb = serde_json::to_vec(&local).unwrap();
    assert_eq!(local_manifest(data, &emb).unwrap(), local);
}

#[test]
fn web_part_for_a_newer_binary_is_set_aside() {
    let tmp = tempfile::tempdir().unwrap();
    let data = tmp.path();
    let local = manifest("a", "0.1.0", &[("static/a.js", b"old")]);
    let v1 = manifest("1", "0.1.0", &[("static/a.js", b"new")]);
    apply(
        data,
        &local,
        &v1,
        "0.1.0",
        &mem(&[("static/a.js", b"new")]),
        |_| {},
    )
    .unwrap();
    recover(data, "0.1.0");
    assert!(validate_web(&data.join("web")));
    recover(data, "0.0.9");
    assert!(!data.join("web").exists());
    assert!(serve_override(&data.join("web"), "/static/a.js").is_none());
    // a folder whose manifest does not parse is set aside as well (check.json is rewritten so that only the manifest is wrong)
    apply(
        data,
        &local,
        &v1,
        "0.1.0",
        &mem(&[("static/a.js", b"new")]),
        |_| {},
    )
    .unwrap();
    fs::write(data.join("web").join(MANIFEST), b"not json").unwrap();
    write_check(&data.join("web")).unwrap();
    assert!(validate_web(&data.join("web")));
    recover(data, "0.1.0");
    assert!(!data.join("web").exists());
}

#[test]
fn interrupted_swap_is_finished_or_undone() {
    let tmp = tempfile::tempdir().unwrap();
    let data = tmp.path();
    let local = manifest("a", "0.1.0", &[("static/a.js", b"old")]);
    let remote = manifest("b", "0.1.0", &[("static/a.js", b"new")]);
    apply(
        data,
        &local,
        &remote,
        "0.1.0",
        &mem(&[("static/a.js", b"new")]),
        |_| {},
    )
    .unwrap();
    fs::rename(data.join("web"), data.join("web.old")).unwrap(); // the app died between the two renames
    fs::create_dir(data.join("web.next")).unwrap();
    recover(data, "0.1.0");
    assert!(
        validate_web(&data.join("web"))
            && !data.join("web.old").exists()
            && !data.join("web.next").exists()
    );
}

#[cfg(unix)]
#[test]
fn recover_keeps_old_when_restore_fails() {
    let tmp = tempfile::tempdir().unwrap();
    let data = tmp.path();
    let local = manifest("a", "0.1.0", &[("static/a.js", b"old")]);
    let remote = manifest("b", "0.1.0", &[("static/a.js", b"new")]);
    apply(
        data,
        &local,
        &remote,
        "0.1.0",
        &mem(&[("static/a.js", b"new")]),
        |_| {},
    )
    .unwrap();
    fs::rename(data.join("web"), data.join("web.old")).unwrap();
    // a plain file named `web`: renaming the folder onto it fails (ENOTDIR)
    fs::write(data.join("web"), b"x").unwrap();
    recover(data, "0.1.0");
    assert!(!data.join("web").is_dir());
    assert!(validate_web(&data.join("web.old")));
}

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
