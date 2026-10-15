//! The fingerprints taken by a second thread while the work goes on (`upload::con_impronte_parallele`) are the ones `upload::impronte` gives, and the members
//! the work opens carry an empty SHA-1 (`upload::senza_sha1`).
use mzlab_core::lettori::{FileSet, Membro};
use mzlab_core::upload;

fn insieme(n: usize) -> FileSet {
    let dati: Vec<u8> = (0..n).map(|i| (i * 31 % 251) as u8).collect();
    FileSet {
        membri: vec![
            Membro {
                nome: "a.raw".into(),
                dim: dati.len() as u64,
                dati: Box::new(dati.clone()),
                sha1: None,
            },
            Membro {
                nome: "b.raw".into(),
                dim: 7,
                dati: Box::new(vec![1u8, 2, 3, 4, 5, 6, 7]),
                sha1: None,
            },
        ],
    }
}

#[test]
fn stesse_impronte_in_un_altro_filo() {
    for n in [1usize, 1000, 3 << 20, 5_000_001] {
        let atteso = upload::impronte(&mut insieme(n)).unwrap();
        let mut lavoro = insieme(n);
        upload::senza_sha1(&mut lavoro);
        assert!(lavoro.membri.iter().all(|m| m.sha1.as_deref() == Some("")));
        let avute = upload::con_impronte_parallele(
            move || Ok(insieme(n)),
            |attendi| {
                let x = attendi()?;
                assert!(attendi().is_err(), "the fingerprints are given once");
                Ok(x)
            },
        )
        .unwrap();
        assert_eq!(avute, atteso, "{n}");
    }
}
