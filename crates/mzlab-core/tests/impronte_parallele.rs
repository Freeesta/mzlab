//! The fingerprints taken by a second thread while the work goes on (`upload::with_parallel_prints`) are the ones `upload::impronte` gives, and the members
//! the work opens carry an empty SHA-1 (`upload::without_sha1`).
use mzlab_core::lettori::{FileSet, Member};
use mzlab_core::upload;

fn file_set(n: usize) -> FileSet {
    let data: Vec<u8> = (0..n).map(|i| (i * 31 % 251) as u8).collect();
    FileSet {
        membri: vec![
            Member {
                nome: "a.raw".into(),
                dim: data.len() as u64,
                dati: Box::new(data.clone()),
                sha1: None,
            },
            Member {
                nome: "b.raw".into(),
                dim: 7,
                dati: Box::new(vec![1u8, 2, 3, 4, 5, 6, 7]),
                sha1: None,
            },
        ],
    }
}

#[test]
fn same_prints_in_another_thread() {
    for n in [1usize, 1000, 3 << 20, 5_000_001] {
        let expected = upload::impronte(&mut file_set(n)).unwrap();
        let mut job = file_set(n);
        upload::without_sha1(&mut job);
        assert!(job.membri.iter().all(|m| m.sha1.as_deref() == Some("")));
        let avute = upload::with_parallel_prints(
            move || Ok(file_set(n)),
            |attendi| {
                let x = attendi()?;
                assert!(attendi().is_err(), "the fingerprints are given once");
                Ok(x)
            },
        )
        .unwrap();
        assert_eq!(avute, expected, "{n}");
    }
}
