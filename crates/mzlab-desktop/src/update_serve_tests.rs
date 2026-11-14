//! `serve_override` of the desktop update: the native files are never served from the updated folder, whatever the spelling of the path.
use super::*;

#[test]
fn serve_override_never_serves_the_native_files() {
    let tmp = tempfile::tempdir().unwrap();
    let web = tmp.path().join("web");
    fs::create_dir_all(web.join("static")).unwrap();
    for rel in NATIVE {
        fs::write(web.join(rel), b"planted").unwrap();
        assert!(serve_override(&web, &format!("/{rel}")).is_none(), "{rel}");
    }
    // the same files by another spelling of the path
    for req in [
        "/static//rust-worker.js",
        "/static/./desktop-shim.js",
        "/SW.JS",
        "/Static/Desktop-Shim.js",
        "//sw.js",
        "/static/rust-worker.js/",
    ] {
        assert!(serve_override(&web, req).is_none(), "{req}");
    }
    fs::write(web.join(CHECK), b"{}").unwrap();
    assert!(serve_override(&web, "/CHECK.JSON").is_none());
    // a downloaded file next to them is still served
    fs::write(web.join("static/a.js"), b"a").unwrap();
    assert!(serve_override(&web, "/static/a.js").is_some());
}
