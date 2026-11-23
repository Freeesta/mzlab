//! Speed study of a targeted extraction (`docs/studies/SPEED_STUDY.md`, driven by `tools/bench/speed_study.py`): N targets x M files, `peak auto-lr` each.
//! Modes: `calls` (per-call route in-process), `list` (one `trace xic --mz LIST` per file), `onepass` (one decode per file, threads over files), `phases`.
//! Each writes the `peak auto-lr` JSON of every (file, target) to OUT so the harness compares them byte for byte.
//! `cargo run --profile cli -p mzlab-cli --example speed_targets -- MODE TARGETS.csv OUT THREADS FILE...`
use std::fs::File;
use std::io::{BufReader, Cursor, Write};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use mzlab_core::area_unit::{lr_auto_json_in, AreaUnit};
use mzlab_core::idxread::Index;
use mzlab_core::peakq_lrauto::{lr_auto_integrate, LrAutoParams};
use mzlab_core::routes::Routes;
use mzlab_core::{json, mzidx, mzml, upload};

/// Half-window of every target, Da (unit resolution).
const DA: f64 = 0.5;

struct Target {
    mz: f64,
    lo: f64,
    hi: f64,
}

fn targets(path: &str) -> Vec<Target> {
    std::fs::read_to_string(path)
        .expect("targets")
        .lines()
        .skip(1)
        .filter_map(|l| {
            let v: Vec<f64> = l.split(',').filter_map(|s| s.parse().ok()).collect();
            (v.len() == 3).then(|| Target {
                mz: v[0],
                lo: v[1],
                hi: v[2],
            })
        })
        .collect()
}

/// A number as the CLI writes it (`--mz`, `--rt`).
fn num(v: f64) -> String {
    let mut o = json::Out::default();
    o.num(v);
    o.0
}

/// The `peak auto-lr` JSON of the points kept in the target window (what `trace xic --rt | peak auto-lr -` prints, without its newline).
fn integrate(rt: &[f64], y: &[f64], t: &Target) -> String {
    let (mut x, mut v) = (Vec::new(), Vec::new());
    for (&a, &b) in rt.iter().zip(y) {
        if a >= t.lo && a <= t.hi {
            x.push(a);
            v.push(b);
        }
    }
    if x.is_empty() {
        return "empty".into();
    }
    let r = lr_auto_integrate(&x, &v, x[0], x[x.len() - 1], LrAutoParams::default());
    lr_auto_json_in(&x, &r, AreaUnit::default())
}

/// One target through the two CLI commands, in-process: the bytes the pipe of two processes gives.
fn by_calls(file: &str, t: &Target, tmp: &std::path::Path) -> String {
    let s = |v: &[&str]| v.iter().map(|a| a.to_string()).collect::<Vec<_>>();
    let csv = tmp.join("t.csv");
    let rt = format!("{},{}", num(t.lo), num(t.hi));
    let a = s(&[
        "trace",
        "xic",
        file,
        "--mz",
        &num(t.mz),
        "--da",
        &num(DA),
        "--rt",
        &rt,
        "--format",
        "csv",
        "--out",
    ]);
    let mut a = a;
    a.push(csv.to_string_lossy().into_owned());
    let o = mzlab_cli::execute(&a);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let o = mzlab_cli::execute(&s(&["peak", "auto-lr", &csv.to_string_lossy()]));
    if o.code != 0 {
        return "empty".into();
    }
    String::from_utf8(o.stdout)
        .expect("utf-8")
        .trim_end()
        .to_string()
}

/// Every target of one file through one `trace xic --mz LIST --format csv` (chunks of 500, the CLI's limit), then `peak auto-lr` in-process per column.
/// The list has no `--rt`: the wide CSV is cut per target here, as `--rt` cuts it.
fn by_list(file: &str, ts: &[Target], tmp: &std::path::Path) -> Vec<String> {
    let mut out = Vec::new();
    for chunk in ts.chunks(500) {
        let csv = tmp.join("w.csv");
        let list = chunk
            .iter()
            .map(|t| num(t.mz))
            .collect::<Vec<_>>()
            .join(",");
        let a: Vec<String> = [
            "trace",
            "xic",
            file,
            "--mz",
            &list,
            "--da",
            &num(DA),
            "--format",
            "csv",
            "--out",
            &csv.to_string_lossy(),
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let o = mzlab_cli::execute(&a);
        assert_eq!(o.code, 0, "{}", o.stderr);
        let text = std::fs::read_to_string(&csv).expect("csv");
        let mut cols: Vec<Vec<f64>> = vec![Vec::new(); chunk.len() + 1];
        for line in text.lines().skip(1) {
            for (k, v) in line.split(',').enumerate() {
                cols[k].push(v.parse().expect("number"));
            }
        }
        for (k, t) in chunk.iter().enumerate() {
            out.push(integrate(&cols[0], &cols[k + 1], t));
        }
    }
    out
}

/// Every target of one file in one decode of its scans: each MS1 scan is walked once and every target window is cut from it by binary search
/// (the window of `Routes::xic`: lo <= m/z <= hi, intensities summed in m/z order), then the trace is rounded as the route writes it.
fn by_onepass(file: &str, ts: &[Target]) -> Vec<String> {
    let mut rt = Vec::new();
    let mut ys: Vec<Vec<f64>> = vec![Vec::new(); ts.len()];
    let f = BufReader::with_capacity(1 << 20, File::open(file).expect("open"));
    mzml::parse(f, |it| {
        let mzml::Item::Spectrum(s) = it else { return };
        if s.level != 1 {
            return;
        }
        rt.push(json::round(s.rt, 4));
        let wide = s.wide.as_deref();
        for (k, t) in ts.iter().enumerate() {
            let (lo, hi) = (t.mz - DA, t.mz + DA);
            let a = s.mz.partition_point(|&x| x < lo);
            let b = s.mz.partition_point(|&x| x <= hi);
            let mut sum = 0.0;
            for j in a..b {
                sum += wide.map_or(f64::from(s.intensity[j]), |w| w[j]);
            }
            ys[k].push(json::round(sum, 1));
        }
    })
    .expect("parse");
    ts.iter()
        .zip(&ys)
        .map(|(t, y)| integrate(&rt, y, t))
        .collect()
}

/// User + system CPU seconds, peak RSS (MB) and bytes read through syscalls (`rchar`) of this process.
fn usage() -> (f64, f64, u64) {
    let stat = std::fs::read_to_string("/proc/self/stat").unwrap_or_default();
    let tail = stat.rsplit(')').next().unwrap_or("");
    let f: Vec<&str> = tail.split_whitespace().collect();
    let tick = |i: usize| f.get(i).and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0) / 100.0;
    let cpu = tick(11) + tick(12);
    let status = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
    let hwm = status
        .lines()
        .find(|l| l.starts_with("VmHWM:"))
        .and_then(|l| l.split_whitespace().nth(1)?.parse::<f64>().ok())
        .unwrap_or(0.0)
        / 1024.0;
    let io = std::fs::read_to_string("/proc/self/io").unwrap_or_default();
    let rchar = io
        .lines()
        .find(|l| l.starts_with("rchar:"))
        .and_then(|l| l.split_whitespace().nth(1)?.parse().ok())
        .unwrap_or(0);
    (cpu, hwm, rchar)
}

/// The cost of each step behind one `trace xic` call on `file` (first target), in ms.
/// Times the three stages of building an index: full decode, scan table + hash, index build (ms each), plus the peak count and index bytes.
fn build_phases(file: &str, size: u64) -> ([f64; 3], usize, Vec<u8>) {
    let ms = |t0: Instant| t0.elapsed().as_secs_f64() * 1e3;
    let t0 = Instant::now();
    let mut n = 0usize;
    mzml::parse(BufReader::new(File::open(file).expect("open")), |it| {
        if let mzml::Item::Spectrum(s) = it {
            n += s.mz.len();
        }
    })
    .expect("parse");
    let decode = ms(t0);
    let t0 = Instant::now();
    let mut fh = File::open(file).expect("open");
    let p = upload::prepare(&mut fh, size, "x.mzML").expect("prepare");
    let prepare = ms(t0);
    let t0 = Instant::now();
    let mut out = Cursor::new(Vec::new());
    mzidx::build_mzml_spill(
        &p.table,
        BufReader::new(File::open(file).expect("open")),
        &mut out,
        192 << 20,
        None,
    )
    .expect("index");
    let build = ms(t0);
    ([decode, prepare, build], n, out.into_inner())
}

/// One `trace xic` call with every target in the list (ms), the route an agent uses for many ions on one file.
fn list_call(file: &str, ts: &[Target], tmp: &std::path::Path) -> f64 {
    let t0 = Instant::now();
    let list = ts.iter().map(|u| num(u.mz)).collect::<Vec<_>>().join(",");
    let csv = tmp.join("all.csv");
    let a: Vec<String> = [
        "trace",
        "xic",
        file,
        "--mz",
        &list,
        "--da",
        &num(DA),
        "--format",
        "csv",
        "--out",
        &csv.to_string_lossy(),
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    assert_eq!(mzlab_cli::execute(&a).code, 0);
    t0.elapsed().as_secs_f64() * 1e3
}

fn phases(file: &str, ts: &[Target]) {
    let t = &ts[0];
    let ms = |t0: Instant| t0.elapsed().as_secs_f64() * 1e3;
    let size = std::fs::metadata(file).expect("file").len();
    let tmp = std::env::temp_dir().join(format!("speed-phases-{}", std::process::id()));
    std::fs::create_dir_all(&tmp).expect("tmp");
    let t0 = Instant::now();
    let whole = by_calls(file, t, &tmp);
    let call = ms(t0);
    let ([decode, prepare, build], n, bytes) = build_phases(file, size);
    let isize = bytes.len();
    let t0 = Instant::now();
    let ix = Index::open(bytes, isize as u64).expect("open index");
    let mut r = Routes::new(ix);
    let open = ms(t0);
    let t0 = Instant::now();
    let (x, y) = r.xic_raw(t.mz, DA, 1).expect("xic");
    let xic = ms(t0);
    let t0 = Instant::now();
    let mut o = json::Out::default();
    o.raw("{\"rt\":")
        .rounded(&x, 4)
        .raw(",\"y\":")
        .rounded(&y, 1)
        .raw("}");
    let render = ms(t0);
    let text = o.0;
    assert_eq!(text, r.xic(t.mz, DA, 1).expect("xic"), "the route's text");
    let t0 = Instant::now();
    for u in ts {
        std::hint::black_box(r.xic(u.mz, DA, 1).expect("xic"));
    }
    let xic_each = ms(t0) / ts.len() as f64;
    let list_call = list_call(file, ts, &tmp);
    let x: Vec<f64> = x.iter().map(|&v| json::round(v, 4)).collect();
    let y: Vec<f64> = y.iter().map(|&v| json::round(v, 1)).collect();
    let t0 = Instant::now();
    let j = integrate(&x, &y, t);
    let lr = ms(t0);
    assert_eq!(j, whole, "the steps give the bytes of the call");
    let _ = std::fs::remove_dir_all(&tmp);
    println!(
        "{{\"mode\":\"phases\",\"xic_text_each_ms\":{xic_each:.4},\"list_call_ms\":{list_call:.2},\"n_targets\":{},\"file_mb\":{:.3},\"peaks\":{n},\"index_mb\":{:.3},\"call_ms\":{call:.3},\"decode_ms\":{decode:.3},\"prepare_ms\":{prepare:.3},\"build_ms\":{build:.3},\"open_ms\":{open:.3},\"xic_ms\":{xic:.4},\"render_ms\":{render:.4},\"json_bytes\":{},\"lr_auto_ms\":{lr:.4}}}",
        ts.len(),
        size as f64 / 1e6,
        isize as f64 / 1e6,
        text.len()
    );
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let [mode, tpath, outp, threads, ..] = a.as_slice() else {
        eprintln!("usage: speed_targets calls|list|onepass|phases TARGETS.csv OUT THREADS FILE...");
        std::process::exit(2);
    };
    let ts = targets(tpath);
    let files: Vec<String> = a[4..].to_vec();
    if mode == "phases" {
        return phases(&files[0], &ts);
    }
    let threads: usize = threads.parse().expect("threads");
    let t0 = Instant::now();
    let next = AtomicUsize::new(0);
    let mut results: Vec<(usize, Vec<String>)> = Vec::new();
    std::thread::scope(|sc| {
        let hs: Vec<_> = (0..threads.max(1))
            .map(|w| {
                let (next, files, ts) = (&next, &files, &ts);
                sc.spawn(move || {
                    let tmp =
                        std::env::temp_dir().join(format!("speed-{}-{w}", std::process::id()));
                    std::fs::create_dir_all(&tmp).expect("tmp");
                    let mut mine = Vec::new();
                    loop {
                        let k = next.fetch_add(1, Ordering::Relaxed);
                        let Some(f) = files.get(k) else { break };
                        let r = match mode.as_str() {
                            "calls" => ts.iter().map(|t| by_calls(f, t, &tmp)).collect(),
                            "list" => by_list(f, ts, &tmp),
                            "onepass" => by_onepass(f, ts),
                            m => panic!("unknown mode {m}"),
                        };
                        mine.push((k, r));
                    }
                    let _ = std::fs::remove_dir_all(&tmp);
                    mine
                })
            })
            .collect();
        for h in hs {
            results.extend(h.join().expect("worker"));
        }
    });
    let wall = t0.elapsed().as_secs_f64();
    results.sort_by_key(|r| r.0);
    let mut w = std::io::BufWriter::new(File::create(outp).expect("out"));
    let mut found = 0usize;
    for (k, r) in &results {
        for (t, j) in ts.iter().zip(r) {
            found += usize::from(j.contains("\"rt\":"));
            writeln!(
                w,
                "{}\t{}\t{j}",
                files[*k].rsplit('/').next().unwrap_or(""),
                num(t.mz)
            )
            .expect("write");
        }
    }
    let (cpu, rss, rchar) = usage();
    let bytes: u64 = files
        .iter()
        .map(|f| std::fs::metadata(f).map_or(0, |m| m.len()))
        .sum();
    println!(
        "{{\"mode\":\"{mode}\",\"threads\":{threads},\"files\":{},\"targets\":{},\"wall_s\":{wall:.3},\"cpu_s\":{cpu:.2},\"peak_rss_mb\":{rss:.1},\"rchar\":{rchar},\"input_bytes\":{bytes},\"passes_per_file\":{:.2},\"peaks_found\":{found}}}",
        files.len(),
        ts.len(),
        rchar as f64 / bytes.max(1) as f64
    );
}
