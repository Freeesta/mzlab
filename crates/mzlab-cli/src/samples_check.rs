//! What `sample list` says of a `.wiff` pair against itself: the `--probe` sizes (`probe_json`) and the always-on verdict of the index against the
//! `.wiff.scan` (`mzlab_core::wiff::completeness`: `consistency` in the JSON, the warning on stderr). The numbers and the 10 s threshold are logged, never hidden.

use std::path::Path;

/// `{"scan_bytes", "indexed_end", "status"}` of the data file of a pair, `n/a` for a format without one.
pub(super) fn probe_json(scan: Option<(u64, u64)>) -> (String, &'static str) {
    let Some((bytes, end)) = scan else {
        return ("{\"status\":\"n/a\"}".into(), "n/a");
    };
    let status = match bytes.cmp(&end) {
        std::cmp::Ordering::Less => "short",
        std::cmp::Ordering::Equal => "ok",
        std::cmp::Ordering::Greater => "longer",
    };
    (
        format!("{{\"scan_bytes\":{bytes},\"indexed_end\":{end},\"status\":\"{status}\"}}"),
        status,
    )
}

/// The verdict of the `.wiff` index against its `.wiff.scan` (`mzlab_core::wiff::completeness`) when they disagree: the data file is shorter than the index
/// says, or holds sections after the indexed part (a copy made while the instrument was acquiring). `None` for a whole pair and for any other format.
pub(super) fn completeness(
    path: &Path,
    l: &mut dyn mzlab_core::readers::Reader,
) -> Option<mzlab_core::wiff::Completeness> {
    let listing = l.listing();
    let (scan_bytes, end) = listing.scan?;
    // a listed sample with nothing in it is a queued row whose section the index does not hold yet: only sections above the last non-empty sample are searched
    let highest = l
        .samples()
        .iter()
        .filter(|s| s.size != Some(0))
        .map(|s| s.id)
        .max()
        .unwrap_or(0);
    let unlisted = if scan_bytes > end {
        let mut f = crate::flow_set::scan_file(path)?;
        mzlab_core::wiff::unlisted_sections(&mut f, scan_bytes, end, highest)
    } else {
        0
    };
    let c = mzlab_core::wiff::completeness(
        (scan_bytes, end),
        listing.bytes.saturating_sub(scan_bytes),
        l.samples().len(),
        unlisted,
        crate::flow_set::wiff_age_s(path),
    );
    (!c.is_ok()).then_some(c)
}

/// The catalogue keys of the warning, by finding (short, unlisted) and state of the pair (writing: modified within the limit, idle, unknown time).
const KEYS: [[&str; 3]; 2] = [
    [
        "warn.wiff.short.writing",
        "warn.wiff.short.idle",
        "warn.wiff.short.unknown",
    ],
    [
        "warn.wiff.unlisted.writing",
        "warn.wiff.unlisted.idle",
        "warn.wiff.unlisted.unknown",
    ],
];

/// The notice for a pair that disagrees with itself: the finding, the numbers it rests on and the threshold, so the user decides what to do.
pub(super) fn consistency_notice(
    file: &str,
    c: &mzlab_core::wiff::Completeness,
) -> crate::convert::Notice {
    let short = c.status == "short";
    let state = match (c.recent(), c.age_s) {
        (true, Some(_)) => 0,
        (_, Some(_)) => 1,
        _ => 2,
    };
    let mut params = vec![("file", file.to_string())];
    if short {
        params.push(("scan_bytes", c.scan_bytes.to_string()));
        params.push(("indexed_end", c.indexed_end.to_string()));
    } else {
        params.push(("listed", c.listed.to_string()));
        params.push(("unlisted", c.unlisted.to_string()));
    }
    params.push(("wiff_bytes", c.wiff_bytes.to_string()));
    if let Some(age) = c.age_s.filter(|_| state < 2) {
        params.push(("age_s", age.to_string()));
        params.push(("recent_s", mzlab_core::wiff::RECENT_S.to_string()));
    }
    crate::convert::Notice {
        key: KEYS[usize::from(!short)][state],
        params,
    }
}

/// The stderr line of `consistency_notice`.
pub(super) fn consistency_warning(file: &str, c: &mzlab_core::wiff::Completeness) -> String {
    format!(
        "warning: {}\n",
        crate::error_text::warning(&consistency_notice(file, c))
    )
}

/// Adds `consistency` to the `samples_list/1` object `json`. The key exists only when the pair disagrees with itself, so a whole pair keeps the bytes of
/// the site's `native_samples`.
pub(super) fn attach(json: &mut String, c: &mzlab_core::wiff::Completeness) {
    json.pop();
    *json += &format!(",\"consistency\":{}}}", c.json());
}
