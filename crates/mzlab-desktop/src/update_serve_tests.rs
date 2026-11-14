//! The rule of `override_rel` for every spelling of a native path (checked on any platform), and `serve_override` on a real folder.
use super::*;

#[test]
fn the_native_files_and_check_json_have_no_spelling_that_is_served() {
    for req in [
        "/static/rust-worker.js",
        "/static/desktop-shim.js",
        "/sw.js",
        "/check.json",
        "/static//rust-worker.js",
        "/static/./desktop-shim.js",
        "//sw.js",
        "/static/rust-worker.js/",
        "/SW.JS",
        "/Static/Desktop-Shim.js",
        "/CHECK.JSON",
        // Windows: a trailing dot or space is dropped by the file system; 8.3 short names fold onto the long name
        "/sw.js.",
        "/static/desktop-shim.js.",
        "/static./rust-worker.js",
        "/sw.js ",
        "/static/DESKTO~1.JS",
        "/static/RUST-W~1.JS",
        "/CHECK~1.JSO",
        // never the folder's parent, a drive or a stream
        "/../sw.js",
        "/static/../sw.js",
        "/static\\sw.js",
        "/sw.js:x",
        "/C:/x",
    ] {
        assert_eq!(override_rel(req), None, "{req}");
    }
}

#[test]
fn an_ordinary_downloaded_path_is_kept_in_its_normal_form() {
    assert_eq!(override_rel("/").as_deref(), Some("index.html"));
    assert_eq!(override_rel("/static/a.js").as_deref(), Some("static/a.js"));
    assert_eq!(
        override_rel("/static//a.js").as_deref(),
        Some("static/a.js")
    );
    assert_eq!(
        override_rel("/static/rust-worker.js.map").as_deref(),
        Some("static/rust-worker.js.map")
    );
}

#[test]
fn serve_override_serves_a_plain_file_and_never_a_native_one_or_a_symlink() {
    let tmp = tempfile::tempdir().unwrap();
    let web = tmp.path().join("web");
    fs::create_dir_all(web.join("static")).unwrap();
    for rel in NATIVE {
        fs::write(web.join(rel), b"planted").unwrap();
        assert!(serve_override(&web, &format!("/{rel}")).is_none(), "{rel}");
    }
    fs::write(web.join("static/a.js"), b"a").unwrap();
    assert!(serve_override(&web, "/static/a.js").is_some());
    #[cfg(unix)]
    {
        fs::write(tmp.path().join("outside.js"), b"x").unwrap();
        std::os::unix::fs::symlink(tmp.path().join("outside.js"), web.join("static/link.js"))
            .unwrap();
        assert!(serve_override(&web, "/static/link.js").is_none());
        std::os::unix::fs::symlink(tmp.path(), web.join("up")).unwrap();
        assert!(serve_override(&web, "/up/outside.js").is_none());
    }
}
