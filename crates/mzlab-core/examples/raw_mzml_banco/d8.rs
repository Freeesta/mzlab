//! D8: why more threads are slower. Three experiments that isolate one cause each, without the converter around them: the memory traffic of the zlib
//! `reset`, the hand-over of small tasks through the channel of the encoding pool, and blocks freed on a thread other than the one that allocated them.
use std::hint::black_box;
use std::sync::{mpsc, Mutex};
use std::time::Instant;

use flate2::{Compress, Compression};
use serde_json::{json, Value};

use crate::micro::ns_per_op;
use crate::misura::arrotonda;

/// `ops` resets of a level-1 zlib stream on each of `t` threads at once; the time one reset takes when the threads run together.
fn reset_in_parallelo(t: usize, ops: usize) -> f64 {
    let inizio = Instant::now();
    std::thread::scope(|sc| {
        for _ in 0..t {
            sc.spawn(|| {
                let mut z = Compress::new(Compression::new(1), true);
                for _ in 0..ops {
                    z.reset();
                    black_box(&z);
                }
            });
        }
    });
    inizio.elapsed().as_secs_f64() / ops as f64 * 1e9
}

fn calibra_lavoro() -> f64 {
    let mut x = 1.0f64;
    ns_per_op(|| {
        for _ in 0..100 {
            x = black_box(x * 1.0000001 + 1e-9);
        }
    }) / 100.0
}

fn lavoro(iter: u64) -> f64 {
    let mut x = 1.0f64;
    for _ in 0..iter {
        x = black_box(x * 1.0000001 + 1e-9);
    }
    x
}

/// The pool of `mzmlwrite/pool.rs` reduced to its channels: a producer sends `compiti` tasks of about `us` microseconds to `t` workers that share the receiver
/// behind a mutex and answer on a second channel; `blocco` tasks travel together. Returns the wall seconds.
fn canale(t: usize, compiti: usize, us: f64, blocco: usize, ns_iter: f64) -> f64 {
    let iter = (us * 1e3 / ns_iter) as u64;
    let (tx, rx) = mpsc::channel::<Vec<usize>>();
    let (txr, rxr) = mpsc::channel::<Vec<usize>>();
    let rx = Mutex::new(rx);
    let inizio = Instant::now();
    std::thread::scope(|sc| {
        for _ in 0..t {
            let (txr, rx) = (txr.clone(), &rx);
            sc.spawn(move || loop {
                let Ok(v) = rx.lock().unwrap_or_else(|e| e.into_inner()).recv() else {
                    break;
                };
                for _ in &v {
                    black_box(lavoro(iter));
                }
                if txr.send(v).is_err() {
                    break;
                }
            });
        }
        drop(txr);
        let mut spediti = 0;
        let mut finiti = 0;
        let mut k = 0;
        while finiti < compiti {
            while spediti < compiti && spediti - finiti < 4 * t * blocco {
                let n = blocco.min(compiti - spediti);
                tx.send((k..k + n).collect()).expect("send");
                k += n;
                spediti += n;
            }
            let v = rxr.recv().expect("answer");
            finiti += v.len();
        }
        drop(tx);
    });
    inizio.elapsed().as_secs_f64()
}

/// Blocks of 8 KB allocated on `t` threads and freed either on the same thread or on the main thread (the text of a spectrum is built by a worker and
/// dropped by the writer). Nanoseconds per block.
fn allocazione_incrociata(t: usize, blocchi: usize, incrociata: bool) -> f64 {
    let inizio = Instant::now();
    let (tx, rx) = mpsc::sync_channel::<Vec<u8>>(1024);
    std::thread::scope(|sc| {
        for _ in 0..t {
            let tx = tx.clone();
            sc.spawn(move || {
                for i in 0..blocchi {
                    let mut v = Vec::with_capacity(8192);
                    v.push(i as u8);
                    if incrociata {
                        let _ = tx.send(v);
                    } else {
                        black_box(&v);
                    }
                }
            });
        }
        drop(tx);
        while let Ok(v) = rx.recv() {
            black_box(&v);
        }
    });
    inizio.elapsed().as_secs_f64() / (blocchi * t) as f64 * 1e9
}

pub fn tutti(thread: &[usize]) -> Value {
    let ns_iter = calibra_lavoro();
    let reset: Vec<Value> = thread
        .iter()
        .map(|&t| json!({"thread": t, "ns_per_reset": arrotonda(reset_in_parallelo(t, 40_000))}))
        .collect();
    let mut canali = Vec::new();
    for us in [10.0, 50.0, 200.0] {
        for &t in thread {
            for blocco in [1usize, 32] {
                let compiti = 20_000;
                let s = canale(t, compiti, us, blocco, ns_iter);
                let ideale = compiti as f64 * us * 1e-6 / t as f64;
                canali.push(json!({
                    "compito_us": us, "thread": t, "compiti_per_messaggio": blocco, "secondi": arrotonda(s), "ideale_s": arrotonda(ideale),
                    "sovraccosto_us_per_compito": arrotonda((s - ideale) / compiti as f64 * 1e6),
                }));
            }
        }
    }
    let alloc: Vec<Value> = thread
        .iter()
        .flat_map(|&t| {
            [false, true].map(|inc| json!({"thread": t, "liberato_su_altro_thread": inc, "ns_per_blocco": arrotonda(allocazione_incrociata(t, 100_000, inc))}))
        })
        .collect();
    json!({"ns_per_iterazione_lavoro": arrotonda(ns_iter), "reset_zlib_in_parallelo": reset, "canale_del_pool": canali, "allocazione_8kb": alloc})
}
