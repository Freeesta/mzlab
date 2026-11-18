//! Which file is the companion of which in a SCIEX pair: `X.wiff` goes with `X.wiff.scan`, same base name, case-insensitive.
//! ONE rule for the app (`opening.rs`), the CLI (`pair`, `convert`, `channels`) and the site (`wiff-pair.js`, proved equal by `tests/test_wiff_pair.py`).
//! The rule is on names only: nothing here reads a file, and the folder listing (`companion_in_dir`) reads names, never contents.

/// A name that belongs to a SCIEX pair, and which half it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Half {
    Wiff,
    Scan,
}

impl Half {
    pub fn key(self) -> &'static str {
        match self {
            Half::Wiff => "wiff",
            Half::Scan => "scan",
        }
    }
}

const SCAN: &str = ".wiff.scan";
const WIFF: &str = ".wiff";

/// The half of a pair that `name` is (`.wiff` or `.wiff.scan`, any case), or `None` (any other `.scan` is another maker's file).
pub fn half_of(name: &str) -> Option<Half> {
    let low = name.to_ascii_lowercase();
    if low.ends_with(SCAN) {
        Some(Half::Scan)
    } else if low.ends_with(WIFF) {
        Some(Half::Wiff)
    } else {
        None
    }
}

/// The name the other half would have: `X.wiff` → `X.wiff.scan`, `X.wiff.scan` → `X.wiff` (the written case of `name` is kept).
pub fn companion_name(name: &str) -> Option<String> {
    match half_of(name)? {
        Half::Wiff => Some(format!("{name}.scan")),
        Half::Scan => Some(name[..name.len() - ".scan".len()].to_string()),
    }
}

/// The name among `names` that is the companion of `name`: the exact spelling wins, else, of those that equal it ignoring case, the first in byte order (so the answer never depends on the order the folder lists).
pub fn pick<'a>(name: &str, names: impl IntoIterator<Item = &'a str>) -> Option<&'a str> {
    let want = companion_name(name)?;
    let low = want.to_lowercase();
    let mut loose = None;
    for n in names {
        if n == want {
            return Some(n);
        }
        if n.to_lowercase() == low && loose.is_none_or(|l| n < l) {
            loose = Some(n);
        }
    }
    loose
}

/// Where the companion of a file stands, for the CLI and the tests: `found` has the file, `missing` the name that was looked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Companion {
    pub half: Half,
    pub wanted: String,
    pub found: Option<std::path::PathBuf>,
}

/// The companion of the file `path` in its own folder (names only, no file is opened); `None` when `path` is not a half of a pair.
pub fn companion_in_dir(path: &std::path::Path) -> Option<Companion> {
    let name = path.file_name()?.to_string_lossy().into_owned();
    let half = half_of(&name)?;
    let wanted = companion_name(&name)?;
    let dir = match path.parent() {
        Some(d) if !d.as_os_str().is_empty() => d,
        _ => std::path::Path::new("."),
    };
    let names: Vec<(String, std::path::PathBuf)> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().is_file())
        .map(|e| (e.file_name().to_string_lossy().into_owned(), e.path()))
        .collect();
    let found = pick(&name, names.iter().map(|(n, _)| n.as_str()))
        .and_then(|n| names.iter().find(|(m, _)| m == n))
        .map(|(_, p)| p.clone());
    Some(Companion {
        half,
        wanted,
        found,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn halves_and_companions_by_name() {
        assert_eq!(half_of("A.wiff"), Some(Half::Wiff));
        assert_eq!(half_of("A.WIFF.SCAN"), Some(Half::Scan));
        assert_eq!(half_of("A.scan"), None);
        assert_eq!(half_of("A.wiff2"), None);
        assert_eq!(
            companion_name("Run 1.wiff").as_deref(),
            Some("Run 1.wiff.scan")
        );
        assert_eq!(
            companion_name("Run 1.WIFF.SCAN").as_deref(),
            Some("Run 1.WIFF")
        );
        assert_eq!(companion_name("a.mzML"), None);
    }

    #[test]
    fn exact_spelling_wins_over_case_variants() {
        let names = ["x.WIFF.SCAN", "x.wiff.scan", "y.wiff.scan"];
        assert_eq!(pick("x.wiff", names), Some("x.wiff.scan"));
        assert_eq!(pick("X.wiff", names), Some("x.WIFF.SCAN"));
        assert_eq!(pick("z.wiff", names), None);
        assert_eq!(pick("x.wiff.scan", ["X.WIFF", "x.mzML"]), Some("X.WIFF"));
    }
}
