//! Is a `.wiff` pair still being written? The decision from two readings of the sizes of its members (the `.wiff` and its `.wiff.scan`), shared by the
//! CLI (which reads the sizes of the disk twice) and any host that can read them twice: a pair whose size changed between the readings is refused with
//! one precise error (`err.file.wiffGrowing`), never opened half-written.

use crate::error::{Error, Result};

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

#[cfg(test)]
mod tests {
    use super::*;

    fn pair(w: u64, s: u64) -> Vec<(String, u64)> {
        vec![("a.wiff".into(), w), ("a.wiff.scan".into(), s)]
    }

    #[test]
    fn equal_sizes_pass_and_a_changed_member_is_named_with_both_sizes() {
        assert!(check_stable(&pair(10, 20), &pair(10, 20), 300).is_ok());
        let e = check_stable(&pair(10, 20), &pair(10, 25), 300).unwrap_err();
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
}
