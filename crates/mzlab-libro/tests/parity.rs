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

/// Every editable block of every chapter, handed back unchanged, must give the same source: otherwise the preview would offer an edit it then refuses.
#[test]
fn every_editable_block_of_every_chapter_survives_an_unchanged_edit() {
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
    let (mut total, mut bad) = (0, Vec::new());
    for n in &names {
        let md = fs::read_to_string(src.join(n)).unwrap();
        let opt = Options {
            teoria: true,
            raw_ok: imported.contains(&n.as_str()),
            ..Options::default()
        };
        let page = mzlab_libro::preview(&md, n, &opt).unwrap();
        let mut rest = page.as_str();
        while let Some(at) = rest.find(" data-src=\"") {
            let after = &rest[at + 11..];
            let key = &after[..after.find('"').unwrap()];
            let open = rest[..at].rfind('<').unwrap();
            let tag: String = rest[open + 1..]
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric())
                .collect();
            let start = at + 11 + after.find('>').unwrap() + 1;
            let inner = &rest[start..start + rest[start..].find(&format!("</{tag}>")).unwrap()];
            total += 1;
            match mzlab_libro::edit_block(&md, n, &opt, key, inner) {
                Ok(new) if new == md => {}
                Ok(_) => bad.push(format!("{n} {key}: source changed by an unchanged edit")),
                Err(e) => bad.push(format!(
                    "{n} {key}: {e} | {}",
                    inner.chars().take(80).collect::<String>()
                )),
            }
            rest = &rest[start..];
        }
    }
    assert!(total > 200, "editable blocks found: {total}");
    assert!(
        bad.is_empty(),
        "{} of {total} blocks:\n{}",
        bad.len(),
        bad.iter().take(15).cloned().collect::<Vec<_>>().join("\n")
    );
}
