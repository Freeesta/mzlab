//! The one rule for the display names of the injections of a `.wiff`: a name written in the file is kept, an unnamed injection of a
//! file with several is `<stem>_s<N>` (N = 1-based position in the file). Used by `lettore` for the sample list; the page and the exports read that name.

use std::collections::HashSet;

/// Display names of the injections of `file` (`Run.wiff`), in file order. `samples` = (sample number in the container, name from its DATA, if any).
/// Named samples keep their name (trimmed, even when repeated: the exports number their files). An unnamed sample of a file with several becomes
/// `<stem>_s<N>`, with `_2`, `_3`... added only when a name already in use (any case) has that text; an unnamed lone sample stays `<file> #<number>`.
pub fn display_names(file: &str, samples: &[(u32, Option<String>)]) -> Vec<String> {
    let stem = file_stem(file);
    let given: Vec<Option<&str>> = samples
        .iter()
        .map(|(_, n)| n.as_deref().map(str::trim).filter(|n| !n.is_empty()))
        .collect();
    let mut used: HashSet<String> = given.iter().flatten().map(|n| n.to_lowercase()).collect();
    samples
        .iter()
        .zip(&given)
        .enumerate()
        .map(|(i, ((id, _), name))| match name {
            Some(n) => (*n).to_string(),
            None if samples.len() == 1 => format!("{file} #{id}"),
            None => {
                let base = format!("{stem}_s{}", i + 1);
                let mut free = base.clone();
                for k in 2.. {
                    if used.insert(free.to_lowercase()) {
                        break;
                    }
                    free = format!("{base}_{k}");
                }
                free
            }
        })
        .collect()
}

/// The name of the file without folders and without a `.wiff` extension (any case).
fn file_stem(file: &str) -> &str {
    let name = file.rsplit(['/', '\\']).next().unwrap_or(file);
    match name.len().checked_sub(5) {
        Some(at) if name.is_char_boundary(at) && name[at..].eq_ignore_ascii_case(".wiff") => {
            &name[..at]
        }
        _ => name,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(items: &[(u32, Option<&str>)]) -> Vec<(u32, Option<String>)> {
        items
            .iter()
            .map(|(i, n)| (*i, n.map(String::from)))
            .collect()
    }

    #[test]
    fn named_samples_keep_their_names() {
        let got = display_names("Run.wiff", &s(&[(1, Some("A")), (2, Some(" B "))]));
        assert_eq!(got, ["A", "B"]);
    }

    #[test]
    fn unnamed_samples_get_the_stem_and_their_position() {
        let got = display_names("Run.wiff", &s(&[(1, None), (2, None), (3, Some(""))]));
        assert_eq!(got, ["Run_s1", "Run_s2", "Run_s3"]);
    }

    #[test]
    fn position_not_container_number_and_mixed_names() {
        let got = display_names(
            "dir/Run.WIFF",
            &s(&[(4, Some("Blank")), (7, None), (9, Some("   "))]),
        );
        assert_eq!(got, ["Blank", "Run_s2", "Run_s3"]);
    }

    #[test]
    fn duplicate_names_stay_and_generated_ones_do_not_collide() {
        let got = display_names(
            "Run.wiff",
            &s(&[
                (1, Some("X")),
                (2, Some("X")),
                (3, Some("run_S3")),
                (4, None),
                (5, None),
            ]),
        );
        assert_eq!(got, ["X", "X", "run_S3", "Run_s4", "Run_s5"]);
        let got = display_names("Run.wiff", &s(&[(1, Some("Run_s2")), (2, None)]));
        assert_eq!(got, ["Run_s2", "Run_s2_2"]);
    }

    #[test]
    fn single_sample_file_is_unchanged() {
        assert_eq!(display_names("Run.wiff", &s(&[(1, None)])), ["Run.wiff #1"]);
        assert_eq!(
            display_names("Run.wiff", &s(&[(3, Some("Only"))])),
            ["Only"]
        );
    }
}
