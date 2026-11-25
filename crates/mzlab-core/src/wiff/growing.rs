//! Is a `.wiff` pair still being written? The decision from two readings of the sizes of its members (the `.wiff` and its `.wiff.scan`), shared by the
//! CLI (which reads the sizes of the disk twice) and any host that can read them twice: a pair whose size changed between the readings is refused with
//! one precise error (`err.file.wiffGrowing`), never opened half-written.

use crate::error::{Error, Result};

/// A member modified less than this many seconds ago may be a file an instrument is still writing (the documented threshold of the CLI: the size check of
/// `flow_set` and the `consistency` verdict of `sample list` use this one number, and every verdict logs it).
pub const RECENT_S: u64 = 10;

/// `Ok` when every member has the same size in `after` as in `before`; else `err.file.wiffGrowing` naming the first member that grew (or shrank), its
/// two sizes and the interval between the readings. `before` and `after` are `(member name, bytes)` in the same order.
pub fn check_stable(before: &[(String, u64)], after: &[(String, u64)], ms: u64) -> Result<()> {
    for (b, a) in before.iter().zip(after) {
        if b.1 != a.1 {
            return Err(Error::new("err.file.wiffGrowing")
                .with("name", &b.0)
                .with("from", b.1)
                .with("to", a.1)
                .with("ms", ms));
        }
    }
    Ok(())
}

/// What the `.wiff` index and the `.wiff.scan` say of each other, and the evidence for it (`sample list`, exit 8 or 1; never a refusal: the listing is still given).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Completeness {
    /// `ok`; `short` (the `.wiff.scan` ends before the index says: a cut copy); `unlisted` (it holds sections after the indexed part: a copy made while the
    /// instrument was acquiring, the index is saved when a sample ends). Padding after the last section without a section marker is `ok`.
    pub status: &'static str,
    pub scan_bytes: u64,
    pub indexed_end: u64,
    pub wiff_bytes: u64,
    /// Samples the index lists, and sections of the `.wiff.scan` it does not.
    pub listed: usize,
    pub unlisted: usize,
    /// Seconds since the newest of the two members was modified, if the system says.
    pub age_s: Option<u64>,
}

impl Completeness {
    pub fn is_ok(&self) -> bool {
        self.status == "ok"
    }

    /// A member was modified within `RECENT_S`: the instrument may still be writing.
    pub fn recent(&self) -> bool {
        self.age_s.is_some_and(|a| a < RECENT_S)
    }

    /// The `consistency` member of `sample list --json`, with every number the verdict rests on.
    pub fn json(&self) -> String {
        let age = self.age_s.map_or("null".to_string(), |a| a.to_string());
        format!(
            "{{\"status\":\"{}\",\"scan_bytes\":{},\"indexed_end\":{},\"wiff_bytes\":{},\"samples_listed\":{},\"sections_unlisted\":{},\"age_s\":{age},\"recent_s\":{RECENT_S},\"still_writing\":{}}}",
            self.status,
            self.scan_bytes,
            self.indexed_end,
            self.wiff_bytes,
            self.listed,
            self.unlisted,
            self.recent()
        )
    }
}

/// The verdict from the sizes the listing holds (`scan_bytes`, `indexed_end`), the samples listed, the sections found after the indexed part and the age of the
/// newest member. A pair is `ok` unless the data file is shorter than its index says or holds sections the index lacks.
pub fn completeness(
    (scan_bytes, indexed_end): (u64, u64),
    wiff_bytes: u64,
    listed: usize,
    unlisted: usize,
    age_s: Option<u64>,
) -> Completeness {
    let status = if scan_bytes < indexed_end {
        "short"
    } else if unlisted > 0 {
        "unlisted"
    } else {
        "ok"
    };
    Completeness {
        status,
        scan_bytes,
        indexed_end,
        wiff_bytes,
        listed,
        unlisted,
        age_s,
    }
}

/// Sections of the `.wiff.scan` after its indexed part that the `.wiff` index does not list (see `growing::completeness`).
pub fn unlisted_sections(
    scan: &mut dyn crate::idxread::ReadAt,
    scan_len: u64,
    indexed_end: u64,
    highest_id: u32,
) -> usize {
    super::unindexed::count_after(scan, scan_len, indexed_end, highest_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pair(w: u64, s: u64) -> Vec<(String, u64)> {
        vec![("a.wiff".into(), w), ("a.wiff.scan".into(), s)]
    }

    #[test]
    fn equal_sizes_pass_and_a_changed_member_is_named_with_both_sizes() {
        assert!(check_stable(&pair(10, 20), &pair(10, 20), 300).is_ok());
        let e = check_stable(&pair(10, 20), &pair(10, 25), 300)
            .expect_err("a changed size must be refused");
        assert_eq!(e.key, "err.file.wiffGrowing");
        let p: Vec<(String, String)> = e
            .params
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        assert!(p.contains(&("name".into(), "a.wiff.scan".into())), "{p:?}");
        assert!(
            p.contains(&("from".into(), "20".into())) && p.contains(&("to".into(), "25".into())),
            "{p:?}"
        );
    }

    #[test]
    fn the_verdict_is_short_unlisted_or_ok_and_recent_uses_the_one_threshold() {
        let c = |scan, unlisted, age| completeness((scan, 100), 5, 4, unlisted, age);
        assert!(
            c(100, 0, None).is_ok() && c(130, 0, Some(1)).is_ok(),
            "padding is not an alarm"
        );
        assert_eq!(c(99, 0, None).status, "short");
        assert_eq!(c(100, 3, None).status, "unlisted");
        assert_eq!(
            c(99, 3, None).status,
            "short",
            "a cut copy is the stronger finding"
        );
        assert!(c(100, 3, Some(RECENT_S - 1)).recent());
        assert!(!c(100, 3, Some(RECENT_S)).recent() && !c(100, 3, None).recent());
        let j = c(100, 3, Some(2)).json();
        assert!(
            j.contains("\"sections_unlisted\":3")
                && j.contains("\"samples_listed\":4")
                && j.contains("\"still_writing\":true"),
            "{j}"
        );
    }
}
