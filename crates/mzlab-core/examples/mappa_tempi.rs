//! Times every phase from a SCIEX `.wiff` (+ `.wiff.scan`) to the first RT x m/z map, for the profile reader and for the centroids of `lettori_centroidi`:
//! decode, scan table, `.mzidx` (with scratch), open, grid, full map cold/warm, region maps, tiles. `cargo run --release --example mappa_tempi -- X.wiff [SAMPLE] [--no-cen]`
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::time::Instant;

use mzlab_core::idxread::Index;
use mzlab_core::lettori::{self, FileSet, Lettore, Limiti, Membro};
use mzlab_core::maproute::map_route;
use mzlab_core::originroute::ItemMeta;
use mzlab_core::routes::{Query, Routes};
use mzlab_core::{lettori_centroidi, upload};

fn rss_mb() -> f64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("VmHWM:"))
                .and_then(|l| l.split_whitespace().nth(1)?.parse::<f64>().ok())
        })
        .map_or(f64::NAN, |kb| kb / 1024.0)
}

fn fileset(wiff: &str) -> FileSet {
    let m = |p: &str| {
        let dim = std::fs::metadata(p).unwrap().len();
        Membro {
            nome: p.rsplit('/').next().unwrap().to_string(),
            dim,
            dati: Box::new(File::open(p).unwrap()),
            sha1: None,
        }
    };
    FileSet {
        membri: vec![m(wiff), m(&format!("{wiff}.scan"))],
    }
}

fn tmp(name: &str) -> File {
    let p = std::env::temp_dir().join(format!("mappa_tempi_{}_{name}", std::process::id()));
    let f = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&p)
        .unwrap();
    let _ = std::fs::remove_file(&p);
    f
}

fn run(wiff: &str, c: usize, cen: bool) {
    println!(
        "=== {} sample {c} {}",
        wiff.rsplit('/').next().unwrap(),
        if cen { "CENTROIDS" } else { "PROFILE" }
    );
    let mut l = lettori::apri(fileset(wiff), &Limiti::sito()).expect("open");
    if cen {
        l = lettori_centroidi::centroidato(l);
    }
    let n = l.indice(c).unwrap().len();
    let t = Instant::now();
    let mut pts = 0usize;
    for k in 0..n {
        pts += l.scansione(c, k).unwrap().mz.len();
    }
    let dec = t.elapsed().as_secs_f64();
    println!(
        "decode every scan      {dec:>8.3} s  {n} scans {pts} points ({:.0}/scan)  {:.2} ms/scan",
        pts as f64 / n as f64,
        dec * 1e3 / n as f64
    );
    let (out, bytes) = index(&mut *l, c);
    maps(out, bytes);
}

/// Scan table + `.mzidx` with a spill file, as the site builds it; returns the index file and its size.
fn index(l: &mut dyn Lettore, c: usize) -> (File, u64) {
    let mut out = tmp("idx");
    let mut sp = tmp("spill");
    let t = Instant::now();
    let (st, _) = upload::indicizza_lettore_con(
        l,
        c,
        &mut || Ok(vec![]),
        "x.mzML",
        &mut out,
        192 << 20,
        Some(&mut sp),
        &mut |_| {},
    )
    .expect("index");
    let build = t.elapsed().as_secs_f64();
    println!(
        "table + .mzidx + spill {build:>8.3} s  {:.1} MB, {} passes   rss {:.0} MB",
        st.bytes as f64 / 1e6,
        st.passes,
        rss_mb()
    );
    (out, st.bytes)
}

/// The routes of the map on the index: full session map, regions, background.
fn maps(out: File, bytes: u64) {
    let t = Instant::now();
    let r = Routes::new(Index::open(out, bytes).expect("idx"));
    println!(
        "Index::open + Routes   {:>8.3} s",
        t.elapsed().as_secs_f64()
    );
    let mut routes: HashMap<u32, Routes<File>> = HashMap::new();
    routes.insert(0, r);
    let items = [ItemMeta {
        slot: 0,
        label: "a".into(),
        time: None,
        full: true,
    }];
    let mut ask = |what: &str, q: &str, rep: usize| {
        for i in 0..rep {
            let t = Instant::now();
            let s = map_route("/api/map", &items, &mut routes, &Query::parse(q))
                .unwrap()
                .unwrap_or_default();
            println!(
                "{what:<22} {:>8.1} ms  json {:.0} kB{}",
                t.elapsed().as_secs_f64() * 1e3,
                s.len() as f64 / 1e3,
                if i == 0 { "" } else { " (warm)" }
            );
        }
    };
    ask("map full (session)", "k=0&level=1", 2);
    ask(
        "region all m/z, all RT",
        "k=0&level=1&rt0=0&rt1=1000&mz0=100&mz1=1000&nrt=600&nmz=900",
        2,
    );
    ask(
        "region RT-only zoom",
        "k=0&level=1&rt0=10&rt1=15&mz0=100&mz1=1000&nrt=600&nmz=900",
        1,
    );
    ask("map full + bg (median)", "k=0&level=1&bg=1", 1);
    ask(
        "region + bg (median)",
        "k=0&level=1&rt0=0&rt1=1000&mz0=100&mz1=1000&nrt=600&nmz=900&bg=1",
        1,
    );
    ask(
        "region deep zoom",
        "k=0&level=1&rt0=14&rt1=16&mz0=300&mz1=302&nrt=600&nmz=400",
        1,
    );
    drop(routes);
    println!("rss peak {:.0} MB", rss_mb());
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let wiff = a.first().expect("X.wiff");
    let c: usize = a.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
    run(wiff, c, false);
    if !a.iter().any(|s| s == "--no-cen") {
        run(wiff, c, true);
    }
}
