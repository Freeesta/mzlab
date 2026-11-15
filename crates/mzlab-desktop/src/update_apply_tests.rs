//! Tests of `update.rs` that change folders: `apply` (only changed files, removed files dropped, the folder as base only when it matches
//! `local`, errors change nothing) and `recover` (interrupted swap, damaged or incompatible folder, a restore that fails).
use super::*;

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
