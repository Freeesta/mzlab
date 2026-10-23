//! Test aid: the `mzlab://` protocol over plain HTTP on 127.0.0.1 (no Tauri), so that `tests/e2e_shim.py` can drive the page, the
//! shim and the worker in Chromium against the real native engine. `serve PORT FILE...` registers the files, prints one line
//! `FILE <id> <name> <size>` per file and then serves until it is killed. `MZLAB_LIBDIR` is the folder of the libraries (POST bodies are read for `lib/put`), `MZLAB_DATA` the folder of the app for the files, the notebook and the indexes.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::Arc;

use mzlab_desktop::{handle_body, State};

fn main() {
    let mut args = std::env::args().skip(1);
    let port: u16 = args
        .next()
        .and_then(|p| p.parse().ok())
        .expect("usage: serve PORT FILE...");
    let state = Arc::new(State::new());
    if let Some(d) = std::env::var_os("MZLAB_LIBDIR") {
        state.libs.set_root(d.into());
    }
    // the folder of the app for the files, the notebook and the indexes (`MZLAB_DATA`; a folder of the system temp dir otherwise)
    let data = std::env::var_os("MZLAB_DATA").map(std::path::PathBuf::from).unwrap_or_else(|| std::env::temp_dir().join(format!("mzlab-serve-{}", std::process::id())));
    state.set_data_folder(data.clone());
    state.set_index_folder(data.join("mzidx")).expect("index folder");
    for f in args {
        let (id, p) = state.pick(f.into()).expect("file");
        println!("FILE {id} {} {}", p.name, p.size);
    }
    let listener = TcpListener::bind(("127.0.0.1", port)).expect("port");
    println!("READY");
    std::io::stdout().flush().ok();
    for stream in listener.incoming().flatten() {
        let state = state.clone();
        std::thread::spawn(move || {
            let mut line = String::new();
            let mut reader = BufReader::new(&stream);
            if reader.read_line(&mut line).is_err() {
                return;
            }
            let mut len = 0usize;
            loop {
                let mut h = String::new();
                if reader.read_line(&mut h).is_err() || h.trim().is_empty() {
                    break;
                }
                if let Some(v) = h.to_ascii_lowercase().strip_prefix("content-length:") {
                    len = v.trim().parse().unwrap_or(0);
                }
            }
            let mut body = vec![0u8; len];
            if reader.read_exact(&mut body).is_err() {
                return;
            }
            let target = line.split(' ').nth(1).unwrap_or("/");
            let (path, query) = target.split_once('?').unwrap_or((target, ""));
            let r = handle_body(&state, path, query, &body);
            let head = format!(
                "HTTP/1.1 {} X\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                r.status,
                r.content_type,
                r.body.len()
            );
            let mut out = &stream;
            let _ = out.write_all(head.as_bytes());
            let _ = out.write_all(&r.body);
        });
    }
}
