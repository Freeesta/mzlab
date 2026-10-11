//! The download of the packages against a local HTTP server: pieces checked one by one, a wrong piece fetched again, an interruption resumed from the first
//! missing piece, the whole file checked, the old file kept until the new one is verified, the external sources only through the allow-list.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use mzlab_desktop::librerie::Folder;
use mzlab_desktop::scarica::*;
use mzlab_desktop::update::{sha256_hex, Http};

enum Reply {
    Body(Vec<u8>),
    Drop,
    Status(u16),
}

type Behave = Box<dyn Fn(&str, usize) -> Reply + Send + Sync>;

struct Server {
    port: u16,
    hits: Arc<Mutex<HashMap<String, usize>>>,
}

fn serve(behave: Behave) -> Server {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = l.local_addr().unwrap().port();
    let hits: Arc<Mutex<HashMap<String, usize>>> = Arc::default();
    let h2 = hits.clone();
    std::thread::spawn(move || {
        for mut c in l.incoming().flatten() {
            let mut buf = [0u8; 4096];
            let n = c.read(&mut buf).unwrap_or(0);
            let req = String::from_utf8_lossy(&buf[..n]).to_string();
            let path = req.split(' ').nth(1).unwrap_or("/").to_string();
            let hit = {
                let mut m = h2.lock().unwrap();
                let e = m.entry(path.clone()).or_insert(0);
                *e += 1;
                *e
            };
            match behave(&path, hit) {
                Reply::Body(b) => {
                    let _ = c.write_all(
                        format!(
                            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            b.len()
                        )
                        .as_bytes(),
                    );
                    let _ = c.write_all(&b);
                }
                Reply::Status(s) => {
                    let _ = c.write_all(
                        format!("HTTP/1.1 {s} X\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                            .as_bytes(),
                    );
                }
                Reply::Drop => {}
            }
        }
    });
    Server { port, hits }
}

/// A package of three pieces (a header of the format, then bytes) and its catalog.
fn package(version: &str, fill: u8) -> (Package, Vec<Vec<u8>>) {
    let mut whole = b"MZLB".to_vec();
    whole.extend_from_slice(&[1, 0, 1, 0]); // version 1, minimum reader 1
    whole.resize(36, 0);
    whole.resize(2500, fill);
    let pieces: Vec<Vec<u8>> = whole.chunks(1000).map(|c| c.to_vec()).collect();
    let pkg = Package {
        id: "massbank-pos".into(),
        nome: "MassBank".into(),
        versione: version.into(),
        sha256: sha256_hex(&whole),
        dimensione: whole.len() as u64,
        lettore_min: 1,
        spettri: 7,
        picchi: 70,
        polarita: 1,
        pezzi: pieces
            .iter()
            .enumerate()
            .map(|(i, p)| Piece {
                file: format!("massbank-pos/{i:03}.bin"),
                dimensione: p.len() as u64,
                sha256: sha256_hex(p),
            })
            .collect(),
    };
    (pkg, pieces)
}

fn no_cancel() -> AtomicBool {
    AtomicBool::new(false)
}

fn run(folder: &Folder, port: u16, pkg: &Package) -> Result<usize, DlError> {
    let src = Http::with_base(&format!("http://127.0.0.1:{port}/"));
    let mut events = 0;
    download_package(folder, &src, pkg, &no_cancel(), &mut |_| events += 1).map(|_| events)
}

fn pieces_of(pkg: &Package, bodies: Vec<Vec<u8>>) -> Behave {
    let map: HashMap<String, Vec<u8>> = pkg
        .pezzi
        .iter()
        .zip(bodies)
        .map(|(p, b)| (format!("/static/librerie/{}", p.file), b))
        .collect();
    Box::new(move |path, _| {
        map.get(path)
            .cloned()
            .map_or(Reply::Status(404), Reply::Body)
    })
}

#[test]
fn a_package_is_downloaded_checked_and_installed() {
    let (pkg, bodies) = package("2026.03", 7);
    let whole: Vec<u8> = bodies.concat();
    let srv = serve(pieces_of(&pkg, bodies));
    let t = tempfile::tempdir().unwrap();
    let folder = Folder::new(t.path().join("L"));
    assert_eq!(
        run(&folder, srv.port, &pkg).unwrap(),
        3,
        "one progress event per piece"
    );
    assert_eq!(
        std::fs::read(t.path().join("L/massbank-pos.mzlib")).unwrap(),
        whole
    );
    let side: serde_json::Value = serde_json::from_slice(
        &std::fs::read(t.path().join("L/.mzlab/massbank-pos.mzlib.pkg.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(side["versione"], "2026.03");
    assert_eq!(side["sha256"], pkg.sha256);
    assert_eq!(side["size"], 2500);
    assert_eq!(side["info"]["n"], 7);
    assert_eq!(side["info"]["pos"], 7);
    let left: Vec<String> = folder
        .list(".mzlab")
        .unwrap()
        .into_iter()
        .map(|e| e.name)
        .collect();
    assert_eq!(
        left,
        vec!["massbank-pos.mzlib.pkg.json"],
        "no piece, no partial file stays"
    );
}

#[test]
fn a_wrong_piece_is_fetched_again_and_a_piece_that_stays_wrong_fails() {
    let (pkg, bodies) = package("1", 3);
    let good = pieces_of(&pkg, bodies.clone());
    let bad = {
        let mut b = bodies[1].clone();
        b[10] ^= 0xff;
        b
    };
    let srv = serve(Box::new(move |path, hit| {
        if path.ends_with("/001.bin") && hit == 1 {
            Reply::Body(bad.clone())
        } else {
            good(path, hit)
        }
    }));
    let t = tempfile::tempdir().unwrap();
    let folder = Folder::new(t.path().join("L"));
    run(&folder, srv.port, &pkg).unwrap();
    assert_eq!(
        srv.hits.lock().unwrap()["/static/librerie/massbank-pos/001.bin"],
        2,
        "the corrupt piece was fetched again"
    );
    assert!(t.path().join("L/massbank-pos.mzlib").is_file());

    let (pkg2, bodies2) = package("1", 3);
    let srv2 = serve(Box::new(move |path, _| {
        let mut b = bodies2[2].clone();
        b[0] ^= 1;
        if path.ends_with("/002.bin") {
            Reply::Body(b)
        } else {
            Reply::Body(bodies2[0].clone())
        }
    }));
    let t2 = tempfile::tempdir().unwrap();
    let f2 = Folder::new(t2.path().join("L"));
    let e = run(&f2, srv2.port, &pkg2).unwrap_err();
    assert_eq!(e.key, "lib.dl.err.hash");
    assert!(
        !t2.path().join("L/massbank-pos.mzlib").exists(),
        "nothing is installed"
    );
}

#[test]
fn an_interruption_is_resumed_from_the_first_missing_piece() {
    let (pkg, bodies) = package("1", 5);
    let good = pieces_of(&pkg, bodies);
    let up = Arc::new(AtomicBool::new(false));
    let up2 = up.clone();
    let srv = serve(Box::new(move |path, hit| {
        if path.ends_with("/002.bin") && !up2.load(std::sync::atomic::Ordering::Relaxed) {
            Reply::Drop // the connection is cut on the last piece
        } else {
            good(path, hit)
        }
    }));
    let t = tempfile::tempdir().unwrap();
    let folder = Folder::new(t.path().join("L"));
    let e = run(&folder, srv.port, &pkg).unwrap_err();
    assert_eq!(e.key, "lib.dl.err.offline");
    assert!(!t.path().join("L/massbank-pos.mzlib").exists());
    assert_eq!(
        folder.list(".mzlab").unwrap().len(),
        2,
        "the two verified pieces stay"
    );
    up.store(true, std::sync::atomic::Ordering::Relaxed);
    run(&folder, srv.port, &pkg).unwrap();
    let h = srv.hits.lock().unwrap();
    assert_eq!(
        h["/static/librerie/massbank-pos/000.bin"], 1,
        "a piece already there is not fetched again"
    );
    assert_eq!(h["/static/librerie/massbank-pos/001.bin"], 1);
    assert!(t.path().join("L/massbank-pos.mzlib").is_file());
}

#[test]
fn a_damaged_piece_on_the_disk_is_fetched_again() {
    let (pkg, bodies) = package("1", 5);
    let srv = serve(pieces_of(&pkg, bodies));
    let t = tempfile::tempdir().unwrap();
    let folder = Folder::new(t.path().join("L"));
    folder
        .put(".mzlab/massbank-pos.000.piece", 0, 1000, &vec![0u8; 1000])
        .unwrap(); // right size, wrong bytes
    run(&folder, srv.port, &pkg).unwrap();
    assert_eq!(
        srv.hits.lock().unwrap()["/static/librerie/massbank-pos/000.bin"],
        1
    );
}

#[test]
fn the_old_file_stays_until_the_new_one_is_verified() {
    let (old, _) = package("1", 1);
    let (new, mut bodies) = package("2", 9);
    let t = tempfile::tempdir().unwrap();
    let folder = Folder::new(t.path().join("L"));
    let (old_pkg, old_bodies) = package("1", 1);
    let srv_old = serve(pieces_of(&old_pkg, old_bodies.clone()));
    run(&folder, srv_old.port, &old).unwrap();
    let before = std::fs::read(t.path().join("L/massbank-pos.mzlib")).unwrap();
    // the pieces are right one by one but the catalog's total is not: nothing is replaced
    let mut wrong_total = new.clone();
    wrong_total.sha256 = "0".repeat(64);
    let srv_new = serve(pieces_of(&new, bodies.clone()));
    let e = run(&folder, srv_new.port, &wrong_total).unwrap_err();
    assert_eq!(e.key, "lib.dl.err.hash");
    assert_eq!(
        std::fs::read(t.path().join("L/massbank-pos.mzlib")).unwrap(),
        before,
        "the old package is untouched"
    );
    assert!(
        folder
            .list(".mzlab")
            .unwrap()
            .iter()
            .all(|f| !f.name.ends_with(".new") && !f.name.ends_with(".piece")),
        "the failed attempt leaves nothing"
    );
    // now the good one replaces it
    run(&folder, srv_new.port, &new).unwrap();
    bodies.truncate(3);
    assert_eq!(
        std::fs::read(t.path().join("L/massbank-pos.mzlib")).unwrap(),
        bodies.concat()
    );
    let side: serde_json::Value = serde_json::from_slice(
        &std::fs::read(t.path().join("L/.mzlab/massbank-pos.mzlib.pkg.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(side["versione"], "2");
}

#[test]
fn a_cancelled_download_stops_between_pieces_and_a_newer_reader_is_refused() {
    let (pkg, bodies) = package("1", 5);
    let srv = serve(pieces_of(&pkg, bodies));
    let t = tempfile::tempdir().unwrap();
    let folder = Folder::new(t.path().join("L"));
    let cancel = AtomicBool::new(false);
    let src = Http::with_base(&format!("http://127.0.0.1:{}/", srv.port));
    let e = download_package(&folder, &src, &pkg, &cancel, &mut |p| {
        if p.piece == 1 {
            cancel.store(true, std::sync::atomic::Ordering::Relaxed);
        }
    })
    .unwrap_err();
    assert_eq!(e.key, "lib.dl.err.cancelled");
    assert!(!t.path().join("L/massbank-pos.mzlib").exists());
    let mut newer = pkg.clone();
    newer.lettore_min = READER + 1;
    assert_eq!(
        run(&folder, srv.port, &newer).unwrap_err().key,
        "lib.dl.err.version"
    );
    assert_eq!(
        READER,
        mzlab_lib::format::VERSION,
        "the reader of the app is the one of the crate"
    );
}

#[test]
fn the_catalog_is_read_and_checked() {
    let (pkg, bodies) = package("1", 5);
    let cat = serde_json::json!({"formato": "librerie/1", "pezzo": 4194304, "librerie": [pkg], "esterne": [{"id": "gnps-cc0", "url": "https://external.gnps2.org/x.mgf", "formato": "mgf"}]});
    let body = cat.to_string().into_bytes();
    let srv = serve(Box::new(move |path, _| {
        if path == "/static/librerie/catalogo.json" {
            Reply::Body(body.clone())
        } else {
            Reply::Status(404)
        }
    }));
    let src = Http::with_base(&format!("http://127.0.0.1:{}/", srv.port));
    let c = fetch_catalog(&src).unwrap();
    assert_eq!(c.librerie.len(), 1);
    assert_eq!(c.esterne[0].sha256, None);
    drop(bodies);
    for bad in [
        serde_json::json!({"formato": "librerie/9", "librerie": []}),
        serde_json::json!({"formato": "librerie/1", "librerie": [{"id": "../x", "versione": "1", "sha256": "", "dimensione": 0, "pezzi": []}]}),
        serde_json::json!({"formato": "librerie/1", "librerie": [{"id": "a", "versione": "1", "sha256": "", "dimensione": 5, "pezzi": [{"file": "a/000.bin", "dimensione": 4, "sha256": ""}]}]}),
        serde_json::json!({"formato": "librerie/1", "librerie": [{"id": "a", "versione": "1", "sha256": "", "dimensione": 4, "pezzi": [{"file": "b/../../000.bin", "dimensione": 4, "sha256": ""}]}]}),
    ] {
        assert_eq!(
            parse_catalog(bad.to_string().as_bytes()).unwrap_err().key,
            "lib.dl.err.catalog",
            "{bad}"
        );
    }
    let off = Http::with_base("http://127.0.0.1:1/");
    assert_eq!(fetch_catalog(&off).unwrap_err().key, "lib.dl.err.offline");
}

#[test]
fn external_sources_only_from_the_allow_list_and_checked() {
    let data = b"BEGIN IONS\nPEPMASS=300.1\n100.1 10\nEND IONS\n".to_vec();
    let body = data.clone();
    let srv = serve(Box::new(move |_, _| Reply::Body(body.clone())));
    let t = tempfile::tempdir().unwrap();
    let folder = Folder::new(t.path().join("L"));
    let local = Policy {
        hosts: vec!["127.0.0.1".into()],
        https_only: false,
    };
    let ext = External {
        id: "gnps-cc0".into(),
        nome: "GNPS".into(),
        url: format!("http://127.0.0.1:{}/x.mgf", srv.port),
        formato: "mgf".into(),
        sha256: Some(sha256_hex(&data)),
        dimensione: Some(data.len() as u64),
    };
    let sha = download_external(&folder, &ext, &local, &no_cancel(), &mut |_| {}).unwrap();
    assert_eq!(sha, sha256_hex(&data));
    assert_eq!(
        std::fs::read(t.path().join("L/gnps-cc0.mgf")).unwrap(),
        data
    );
    // an unknown SHA-256 (a source that changes) is accepted and returned
    let mut free = ext.clone();
    free.sha256 = None;
    free.dimensione = None;
    assert_eq!(
        download_external(&folder, &free, &local, &no_cancel(), &mut |_| {}).unwrap(),
        sha256_hex(&data)
    );
    // a wrong SHA-256: nothing is kept
    let mut wrong = ext.clone();
    wrong.id = "other".into();
    wrong.sha256 = Some("0".repeat(64));
    assert_eq!(
        download_external(&folder, &wrong, &local, &no_cancel(), &mut |_| {})
            .unwrap_err()
            .key,
        "lib.dl.err.hash"
    );
    assert!(!t.path().join("L/other.mgf").exists() && folder.list(".mzlab").unwrap().is_empty());
    // the production policy: https and the listed hosts only
    let prod = Policy::default();
    for ok in ["https://external.gnps2.org/gnpslibrary/GNPS-LIBRARY.mgf", "https://github.com/MassBank/MassBank-data/releases/download/2026.03/MassBank_NISTformat.msp", "https://release-assets.githubusercontent.com/x"] {
        assert!(prod.allows(ok), "{ok}");
    }
    for no in [
        "http://external.gnps2.org/x",
        "https://evil.example/x",
        "https://external.gnps2.org.evil.example/x",
        "https://user@external.gnps2.org/x",
        "ftp://zenodo.org/x",
        "https://127.0.0.1/x",
        "external.gnps2.org/x",
    ] {
        assert!(!prod.allows(no), "{no}");
    }
    let off = External {
        url: "https://evil.example/x.mgf".into(),
        ..ext
    };
    assert_eq!(
        download_external(&folder, &off, &prod, &no_cancel(), &mut |_| {})
            .unwrap_err()
            .key,
        "lib.dl.err.host"
    );
}
