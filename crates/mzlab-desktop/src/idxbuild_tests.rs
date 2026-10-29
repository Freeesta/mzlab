//! Tests of the index builder of the app.

use super::*;
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
                b.scrivi("same.mzML", |out, _| {
                    use std::io::Write;
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
