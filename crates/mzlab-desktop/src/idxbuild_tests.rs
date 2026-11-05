//! Tests of the index builder of the app.

use super::*;
use std::io::Write;
use std::sync::Barrier;

#[test]
fn concurrent_builds_of_one_name_write_their_own_part_files() {
    let d = tempfile::tempdir().unwrap();
    let b = IdxBuilder {
        dir: d.path().to_path_buf(),
        pool: rayon::ThreadPoolBuilder::new()
            .num_threads(2)
            .build()
            .unwrap(),
        stops: Default::default(),
    };
    let barrier = Barrier::new(2);
    let parts = |dir: &Path| {
        let named = |e: &std::fs::DirEntry| {
            let n = e.file_name().to_string_lossy().into_owned();
            n.ends_with(".part") && !n.contains(".sp.")
        };
        std::fs::read_dir(dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(named)
            .count()
    };
    let seen = std::sync::Mutex::new(Vec::new());
    std::thread::scope(|s| {
        for tag in *b"ab" {
            let (b, barrier, seen) = (&b, &barrier, &seen);
            s.spawn(move || {
                b.scrivi("same.mzML", |out, _, _| {
                    out.write_all(&[tag; 8]).unwrap();
                    barrier.wait();
                    seen.lock().unwrap().push(parts(&b.dir));
                    barrier.wait();
                    Ok(Stats::default())
                })
                .unwrap();
            });
        }
    });
    assert_eq!(
        *seen.lock().unwrap(),
        vec![2, 2],
        "each build has its own .part file"
    );
    let body = std::fs::read(b.path("same.mzML").unwrap()).unwrap();
    assert!(body == b"aaaaaaaa" || body == b"bbbbbbbb", "{body:?}");
}

/// «Cancel» in the middle of a build: the next write fails, the build gives `err.load.cancelled` and leaves no file (neither `.part` nor index);
/// a new upload of the name (`resume`) builds again.
#[test]
fn a_cancelled_build_stops_at_its_next_write_and_leaves_no_file() {
    let d = tempfile::tempdir().unwrap();
    let b = IdxBuilder::new(d.path().to_path_buf()).unwrap();
    let e = b
        .scrivi("x.mzML", |out, spill, _| {
            out.write_all(b"head").unwrap();
            spill.write_all(b"peaks").unwrap();
            b.cancel("x.mzML");
            let w = out.write_all(b"rest");
            assert!(w.is_err(), "a write past the cancel fails");
            w.map_err(|e| io_error(&e))?;
            Ok(Stats::default())
        })
        .unwrap_err();
    assert_eq!(e.key, "err.load.cancelled");
    let left: Vec<_> = std::fs::read_dir(d.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name())
        .collect();
    assert!(left.is_empty(), "{left:?}");
    // cancelled before it starts (it waited for a thread): it fails without creating a file
    let e = b
        .scrivi("x.mzML", |_, _, _| {
            unreachable!("a cancelled build does not run")
        })
        .unwrap_err();
    assert_eq!(e.key, "err.load.cancelled");
    b.resume("x.mzML");
    b.scrivi("x.mzML", |out, _, _| {
        out.write_all(b"ok").unwrap();
        Ok(Stats::default())
    })
    .unwrap();
    assert_eq!(std::fs::read(b.path("x.mzML").unwrap()).unwrap(), b"ok");
}

/// A build that ends after «Cancel» (the flag was set past its last write) leaves no index either: the page removes the file.
#[test]
fn a_build_that_ends_past_the_cancel_leaves_no_index() {
    let d = tempfile::tempdir().unwrap();
    let b = IdxBuilder::new(d.path().to_path_buf()).unwrap();
    let e = b
        .scrivi("y.mzML", |out, _, _| {
            out.write_all(b"all").unwrap();
            b.cancel("y.mzML");
            Ok(Stats::default())
        })
        .unwrap_err();
    assert_eq!(e.key, "err.load.cancelled");
    assert_eq!(std::fs::read_dir(d.path()).unwrap().count(), 0);
}

/// An empty scan table decodes with `declared == 0` (the body carries no count), which means no "missing scans" warning.
#[test]
fn an_empty_table_decodes_with_no_declared_count() {
    let mut body = Vec::new();
    for _ in 0..8 {
        body.extend_from_slice(&0u32.to_le_bytes());
    }
    let t = decode_inputs(&body).unwrap();
    assert_eq!((t.declared, t.level.len(), t.rt.len()), (0, 0, 0));
    assert!(!t.hr1);
    assert!(decode_inputs(&body[..31]).is_err());
}
