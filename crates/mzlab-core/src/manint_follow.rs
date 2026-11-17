//! The windows of `manint` applied to the other samples: where a window drawn on one sample lands on the same channel of the others (the apex searched
//! within the window's width on each side, an apex outside moves the window, none leaves it), the items of the table (drawn + applied, grouped by peak)
//! and `follow_text`, the page's `api/manfollow` and `mzlab-cli manual-table --set shifts=true`.

use super::{
    channel_of, key_of, parse, samples_and_keys, Item, Key, Result, Sample, Trace, Value,
    FOLLOW_SN, SN_POINTS, SN_SMOOTH,
};
use crate::peak1d::find_peak;
use crate::peakq_lr::near_idx;

/// The window `a..b` of the channel `src` applied to the same channel `dst` of another sample: the apex is searched within the window's width on each
/// side (`peak1d::find_peak`, S/N ≥ `FOLLOW_SN`); an apex inside the window keeps its edges (`applied`), one outside moves the window by the apex's offset
/// from the source's (`shifted`), none leaves it where it was (`no_peak`). Returns the shift in min and the origin.
fn follow(src: &Trace, dst: &Trace, a: f64, b: f64) -> (f64, &'static str) {
    let width = b - a;
    let at_src = (near_idx(&src.t, a)..=near_idx(&src.t, b))
        .fold(None::<usize>, |m, j| match m {
            Some(k) if src.y[k] >= src.y[j] => Some(k),
            _ => Some(j),
        })
        .map_or((a + b) / 2.0, |k| src.t[k]);
    let found = find_peak(
        &dst.t,
        &dst.y,
        Some((a + b) / 2.0),
        width / 2.0 + width,
        SN_SMOOTH,
        SN_POINTS,
    )
    .filter(|f| f.ok && f.snr.is_finite() && f.snr >= FOLLOW_SN);
    match found {
        None => (0.0, "no_peak"),
        Some(f) if f.apex_rt >= a && f.apex_rt <= b => (0.0, "applied"),
        Some(f) => (f.apex_rt - at_src, "shifted"),
    }
}

/// The windows the source `src` (at `at`) gives the other samples that hold its channel and have no window of their own on it (`own`: the samples
/// with one, by channel): one item per sample, in the order of the samples.
pub(super) fn followers(src: &Item, samples: &[Sample], own: &[(usize, Key)]) -> Vec<Item> {
    let tr = &samples[src.file].1[src.tr];
    let k = key_of(tr);
    let mut out = Vec::new();
    for (f, (_, trs)) in samples.iter().enumerate() {
        let Some(dst) = trs.iter().find(|t| key_of(t) == k) else {
            continue;
        };
        if f == src.file || dst.t.len() < 2 || own.iter().any(|(of, ok)| *of == f && *ok == k) {
            continue;
        }
        let (shift, origin) = follow(tr, dst, src.a, src.b);
        out.push(Item {
            file: f,
            tr: dst.pos,
            a: src.a + shift,
            b: src.b + shift,
            shift,
            origin,
            ..*src
        });
    }
    out
}

/// The other samples' windows for one window of the request (`{"file", "tr", "a", "b"}`) as JSON, `[{"file", "tr", "a", "b", "shift", "origin"}]` in the
/// order of the samples: the edges and the origin `table_text` gives the rows of a window with `follow`. A sample with no such channel is left out.
pub fn follow_text(files: &str, window: &str) -> Result<String> {
    let (samples, _) = samples_and_keys(files)?;
    let w = parse(window.as_bytes())?;
    let (file, tr, a, b) = channel_of(&w, &samples)?;
    let src = Item {
        w: 0,
        file,
        tr: tr.pos,
        a,
        b,
        shift: 0.0,
        origin: "manual",
        gkey: a,
    };
    let out: Vec<String> = followers(&src, &samples, &[])
        .iter()
        .map(|f| {
            format!(
                "{{\"file\":{},\"tr\":{},\"a\":{},\"b\":{},\"shift\":{},\"origin\":\"{}\"}}",
                f.file, f.tr, f.a, f.b, f.shift, f.origin
            )
        })
        .collect();
    Ok(format!("[{}]", out.join(",")))
}

/// The items of the table: the windows drawn (`list`, as `table_text` reads them) and, for those with `follow`, the windows the engine applies to the
/// other samples; a window drawn on another sample over the peak of a source joins that peak's group (it overrides the applied one there).
pub(super) fn items_of(list: &Value, samples: &[Sample]) -> Result<Vec<Item>> {
    let mut items: Vec<Item> = Vec::new();
    for (i, w) in list.arr().iter().enumerate() {
        let (file, tr, a, b) = channel_of(w, samples)?;
        items.push(Item {
            w: i,
            file,
            tr: tr.pos,
            a,
            b,
            shift: 0.0,
            origin: "manual",
            gkey: a,
        });
    }
    // a sample with a window of its own on a channel keeps it: the ones applied from another sample skip that channel there
    let own: Vec<(usize, Key)> = items
        .iter()
        .map(|it| (it.file, key_of(&samples[it.file].1[it.tr])))
        .collect();
    let drawn = items.clone();
    let sources: Vec<&Item> = drawn
        .iter()
        .filter(|it| list.arr()[it.w].get("follow").and_then(Value::bool) == Some(true))
        .collect();
    for it in &sources {
        items.extend(followers(it, samples, &own));
    }
    // a window drawn in another sample over the peak of a source is a row of that peak's group (it overrides the applied one there)
    for it in items.iter_mut().filter(|it| it.origin == "manual") {
        let k = key_of(&samples[it.file].1[it.tr]);
        let near = |s: &&&Item| {
            let w = s.b - s.a;
            s.file != it.file
                && key_of(&samples[s.file].1[s.tr]) == k
                && it.a <= s.b + w
                && it.b >= s.a - w
        };
        if let Some(s) = sources.iter().find(near) {
            it.gkey = s.gkey;
        }
    }
    Ok(items)
}
