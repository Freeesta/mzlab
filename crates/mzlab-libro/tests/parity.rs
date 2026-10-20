//! Byte-for-byte parity with the Python converter: every Markdown chapter of mzlab/web/teoria/src must convert to exactly the committed HTML next to it.
use std::fs;
use std::path::PathBuf;

use mzlab_libro::{convert, Options};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn every_chapter_matches_the_committed_html() {
    let src = root().join("mzlab/web/teoria/src");
    let listed = fs::read_to_string(root().join("tools/libro/importati.txt")).unwrap();
    let imported: Vec<&str> = listed
        .lines()
        .map(|l| l.split('#').next().unwrap().trim())
        .collect();
    let mut names: Vec<_> = fs::read_dir(&src)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .filter(|n| n.ends_with(".md"))
        .collect();
    names.sort();
    assert!(names.len() >= 23, "chapters found: {}", names.len());
    let mut bad = Vec::new();
    for n in &names {
        let md = fs::read_to_string(src.join(n)).unwrap();
        let want = fs::read_to_string(
            root()
                .join("mzlab/web/teoria")
                .join(n.replace(".md", ".html")),
        )
        .unwrap();
        let opt = Options {
            teoria: true,
            raw_ok: imported.contains(&n.as_str()),
            ..Options::default()
        };
        match convert(&md, n, &opt) {
            Ok(got) if got == want => {}
            Ok(got) => {
                let at = got
                    .bytes()
                    .zip(want.bytes())
                    .position(|(a, b)| a != b)
                    .unwrap_or(got.len().min(want.len()));
                let ctx = |s: &str| {
                    s.chars()
                        .skip(at.saturating_sub(40))
                        .take(100)
                        .collect::<String>()
                };
                bad.push(format!(
                    "{n}: differs at byte {at}\n  got:  {:?}\n  want: {:?}",
                    ctx(&got),
                    ctx(&want)
                ));
            }
            Err(e) => bad.push(format!("{n}: {e}")),
        }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}
