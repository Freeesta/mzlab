//! The folder of the libraries through the `mzlab://` routes `lib/*` (what the page and its worker call): write in pieces, read memory-mapped,
//! list, rename, remove, and never leave the folder.

use mzlab_desktop::librerie::{default_dir, read_config, write_config, Config, Folder};
use mzlab_desktop::{handle, handle_body, State};

fn state(dir: &std::path::Path) -> State {
    let s = State::new();
    s.libs.set_root(dir.to_path_buf());
    s
}

fn put(s: &State, name: &str, off: usize, total: usize, data: &[u8]) -> u16 {
    handle_body(s, "/lib/put", &format!("name={name}&off={off}&total={total}"), data).status
}

fn json(r: mzlab_desktop::Response) -> serde_json::Value {
    serde_json::from_slice(&r.body).unwrap()
}

#[test]
fn a_file_written_in_pieces_is_read_back_and_listed() {
    let t = tempfile::tempdir().unwrap();
    let dir = t.path().join("Librerie");
    let s = state(&dir);
    let data: Vec<u8> = (0..100_000u32).map(|i| (i % 251) as u8).collect();
    for (i, c) in data.chunks(30_000).enumerate() {
        assert_eq!(put(&s, "a.mzlib", i * 30_000, data.len(), c), 200);
        if (i + 1) * 30_000 < data.len() {
            // while it is incomplete it is not listed (only `a.mzlib.part` exists)
            assert!(json(handle(&s, "/lib/list", "sub="))["files"].as_array().unwrap().is_empty());
        }
    }
    let l = json(handle(&s, "/lib/list", "sub="));
    assert_eq!(l["files"][0]["name"], "a.mzlib");
    assert_eq!(l["files"][0]["size"], 100_000);
    assert!(l["files"][0]["mtime"].as_i64().unwrap() > 1_600_000_000_000);
    let r = handle(&s, "/lib/read", "name=a.mzlib&off=99990&len=50");
    assert_eq!(r.status, 200);
    assert_eq!(r.body, data[99_990..]);
    let r = handle(&s, "/lib/read", "name=a.mzlib&off=10&len=5");
    assert_eq!(r.body, data[10..15]);
    assert_eq!(json(handle(&s, "/lib/stat", "name=a.mzlib"))["size"], 100_000);
}

#[test]
fn the_meta_folder_and_the_atomic_replacement() {
    let t = tempfile::tempdir().unwrap();
    let s = state(t.path());
    assert_eq!(put(&s, ".mzlab%2Fx.msp.mzlib.tmp", 0, 3, b"new"), 200);
    assert_eq!(put(&s, ".mzlab%2Fx.msp.mzlib", 0, 3, b"old"), 200);
    assert_eq!(handle(&s, "/lib/rename", "from=.mzlab%2Fx.msp.mzlib.tmp&to=.mzlab%2Fx.msp.mzlib").status, 200);
    assert_eq!(handle(&s, "/lib/read", "name=.mzlab%2Fx.msp.mzlib&off=0&len=10").body, b"new");
    let meta = json(handle(&s, "/lib/list", "sub=.mzlab"));
    assert_eq!(meta["files"].as_array().unwrap().len(), 1);
    assert!(json(handle(&s, "/lib/list", "sub="))["files"].as_array().unwrap().is_empty(), "the top level has no file");
    assert_eq!(handle(&s, "/lib/remove", "name=.mzlab%2Fx.msp.mzlib").status, 200);
    assert_eq!(handle(&s, "/lib/remove", "name=.mzlab%2Fx.msp.mzlib").status, 200, "removing what is not there is fine");
    assert!(json(handle(&s, "/lib/list", "sub=.mzlab"))["files"].as_array().unwrap().is_empty());
}

#[test]
fn an_empty_file_and_a_folder_that_does_not_exist() {
    let t = tempfile::tempdir().unwrap();
    let s = state(&t.path().join("not-yet"));
    assert!(json(handle(&s, "/lib/list", "sub="))["files"].as_array().unwrap().is_empty());
    assert_eq!(put(&s, "e.msp", 0, 0, b""), 200);
    assert_eq!(json(handle(&s, "/lib/stat", "name=e.msp"))["size"], 0);
    assert_eq!(handle(&s, "/lib/read", "name=e.msp&off=0&len=10").body.len(), 0);
}

#[test]
fn nothing_outside_the_folder_is_reachable() {
    let t = tempfile::tempdir().unwrap();
    std::fs::write(t.path().join("secret.txt"), "x").unwrap();
    let dir = t.path().join("Librerie");
    let s = state(&dir);
    for bad in ["..%2Fsecret.txt", "%2Fetc%2Fpasswd", "a%2Fb%2Fc", "..", ".mzlab%2F..%2F..%2Fsecret.txt", "x%5Cy", "C%3Ax", "other%2Fx", ""] {
        assert_eq!(handle(&s, "/lib/read", &format!("name={bad}&off=0&len=10")).status, 400, "{bad}");
        assert_eq!(put(&s, bad, 0, 1, b"x"), 400, "{bad}");
        assert_eq!(handle(&s, "/lib/remove", &format!("name={bad}")).status, 400, "{bad}");
    }
    assert_eq!(handle(&s, "/lib/rename", "from=a&to=..%2Fsecret.txt").status, 400);
    assert_eq!(handle(&s, "/lib/list", "sub=..").status, 400);
    assert_eq!(std::fs::read_to_string(t.path().join("secret.txt")).unwrap(), "x");
}

#[test]
fn the_default_folder_and_the_configuration() {
    let d = default_dir();
    assert!(d.ends_with("Documents/mzLab/Librerie") || d.ends_with("Documents\\mzLab\\Librerie"), "{d:?}");
    assert_eq!(Folder::default().root(), d);
    let t = tempfile::tempdir().unwrap();
    assert_eq!(read_config(t.path()), Config::default());
    assert!(!Config::default().esterne, "the external downloads are off by default");
    let c = Config { cartella: Some("/x/y".into()), esterne: true };
    write_config(t.path(), &c).unwrap();
    assert_eq!(read_config(t.path()), c);
    std::fs::write(t.path().join("config.json"), "not json").unwrap();
    assert_eq!(read_config(t.path()), Config::default(), "a damaged file is the default");
}
