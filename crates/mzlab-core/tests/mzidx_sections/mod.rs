//! Where two `.mzidx` built for the same sample first differ in their sections (helper of `readers_mzidx.rs`): the message of a failed byte comparison
//! names the section and the array, so a mismatch on a real file can be told without the file.
use mzlab_core::json::{self, Value};

fn u64_at(b: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(b[at..at + 8].try_into().unwrap())
}

fn json_at(b: &[u8], off: usize, len: usize) -> Value {
    json::parse(&b[u64_at(b, off) as usize..][..u64_at(b, len) as usize]).unwrap()
}

/// Where the sections of `a` and `b` first differ, as the meta of `a` names it: the shape of each section (level, dtype, counts), then the array
/// (`rt`, `tic`, …, `mz`, `inten`, `pos`) holding the first different byte, with its byte offset in that array.
pub fn first_difference(a: &[u8], b: &[u8], scans_off: usize) -> String {
    let fields = [
        "rt", "tic", "scan_ids", "sumint", "bp_mz", "bp_inten", "fence", "mz", "inten", "pos",
    ];
    let (sa, sb) = (json_at(a, 16, 24), json_at(b, 16, 24));
    let (sa, sb) = (
        sa.get("sections").unwrap().arr(),
        sb.get("sections").unwrap().arr(),
    );
    let shape = |s: &Value| {
        ["level", "dtype", "n_scans", "n_peaks"].map(|k| match s.get(k) {
            Some(Value::Str(x)) => x.clone(),
            v => format!("{}", v.and_then(Value::f64).unwrap_or(f64::NAN)),
        })
    };
    let mut out = format!("{} vs {} sections", sa.len(), sb.len());
    for (i, (x, y)) in sa.iter().zip(sb).enumerate() {
        if shape(x) != shape(y) {
            out.push_str(&format!(
                "; section {i}: mzML {:?}, reader {:?}",
                shape(x),
                shape(y)
            ));
        }
    }
    let Some(at) = (64..scans_off).find(|&i| a[i] != b[i]) else {
        return out;
    };
    // the array of `a` that starts last at or before `at`
    let mut best = (0usize, "header", 0usize);
    for (i, s) in sa.iter().enumerate() {
        for f in fields {
            if let Some(o) = s.get("off").and_then(|o| o.get(f)).and_then(Value::f64) {
                let o = o as usize;
                if o <= at && o >= best.2 {
                    best = (i, f, o);
                }
            }
        }
    }
    let (i, f, o) = best;
    out.push_str(&format!(
        "; first byte {at}: section {i} {:?} array {f} at byte {}",
        sa.get(i).map(shape),
        at - o
    ));
    out
}
