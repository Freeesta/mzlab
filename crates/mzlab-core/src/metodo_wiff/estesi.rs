//! What `metodo/1` of a `.wiff` says more when the file is not from the 3200 QTRAP (feature `estesi`, the `sciex` readers module): the scheduled-MRM
//! flag and window of each experiment, the strings of `DabsInfo` / `SampleTable`, the tables of `GLPTables` (calibration and tuning of the instrument,
//! with the Analyst version that wrote the file) and the header of the DAD (points, channels and range). Layouts measured on the files of
//! `dati_formati/` (`docs/agenti/wiff.md`); nothing here is read from a position that no file confirms.

use super::counted_strings;
use crate::metodo::V;
use crate::metodo_campi::Campo;
use crate::ole::Ole;

/// Longest text of a field (a calibration table is a few hundred characters; the cut protects against a corrupt stream).
const VALUE_MAX: usize = 4096;
/// Most text runs taken from a `GLPTables`.
const RUNS_MAX: usize = 64;

fn u32_at(b: &[u8], at: usize) -> Option<u32> {
    b.get(at..at + 4)
        .map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

fn f32_at(b: &[u8], at: usize) -> Option<f64> {
    u32_at(b, at)
        .map(|u| f32::from_bits(u) as f64)
        .filter(|v| v.is_finite())
}

/// The printable UTF-16 runs of at least `min` characters (text with line ends and tabs), in order.
fn utf16_runs(b: &[u8], min: usize) -> Vec<String> {
    let (mut out, mut i) = (Vec::new(), 0);
    while i + 1 < b.len() && out.len() < RUNS_MAX {
        let mut j = i;
        while j + 1 < b.len()
            && b[j + 1] == 0
            && (b[j] == b'\n' || b[j] == b'\r' || b[j] == b'\t' || (0x20..=0x7e).contains(&b[j]))
        {
            j += 2;
        }
        if (j - i) / 2 >= min {
            out.push(b[i..j].chunks_exact(2).map(|c| c[0] as char).collect());
            i = j;
        } else {
            i += 1;
        }
    }
    out
}

/// A text cut to `VALUE_MAX` characters.
fn cut(t: &str) -> String {
    t.chars().take(VALUE_MAX).collect()
}

/// The fields of `GLPTables` of the method `no`: one per table, titled by its first line; the version of Analyst that wrote the file is mapped.
fn glp(ole: &Ole, no: u32) -> Vec<Campo> {
    let path = format!("MethodSubtree/Method{no}/GLPTables");
    let Some(b) = ole.read(&path) else {
        return vec![];
    };
    let mut out = Vec::new();
    for run in utf16_runs(&b, 8) {
        const VERSION: &str = "File was created with the software version:";
        if let Some(at) = run.find(VERSION) {
            let v = run[at + VERSION.len()..].trim();
            out.push(Campo::new(&path, "software", v).map("strumento.software"));
            continue;
        }
        let title = run
            .lines()
            .next()
            .unwrap_or("")
            .split('\t')
            .next()
            .unwrap_or("")
            .trim()
            .to_string();
        if !title.is_empty() {
            out.push(Campo::new(&path, &title, cut(&run)));
        }
    }
    out
}

/// The DAD header of sample `sample` (`.../DAD/DADSampleHeader`): the points, the channels and the wavelength range, kept only when they agree with each
/// other and with the index (`DADIdx`: 32 bytes of header, then 8 per point).
fn dad(ole: &Ole, sample: &str) -> Vec<Campo> {
    let Some(path) = ole
        .paths_list()
        .find(|p| p.starts_with(sample) && p.ends_with("/DAD/DADSampleHeader"))
        .map(String::from)
    else {
        return vec![];
    };
    let Some(h) = ole.read(&path) else {
        return vec![];
    };
    let (Some(points), Some(channels)) = (u32_at(&h, 34), u32_at(&h, 38)) else {
        return vec![];
    };
    let (Some(minutes), Some(from), Some(to), Some(step)) = (
        f32_at(&h, 46),
        f32_at(&h, 50),
        f32_at(&h, 54),
        f32_at(&h, 58),
    ) else {
        return vec![];
    };
    let idx = path.replace("DADSampleHeader", "DADIdx");
    let idx_points = ole.read(&idx).map(|b| b.len().saturating_sub(32) / 8);
    let consistent = step > 0.0
        && to > from
        && ((to - from) / step).round() as u64 + 1 == u64::from(channels)
        && idx_points == Some(points as usize);
    if !consistent {
        return vec![];
    }
    vec![
        Campo::new(&path, "punti", points),
        Campo::new(&path, "canali", channels),
        Campo::new(&path, "durata", minutes).unit("min"),
        Campo::new(&path, "lunghezza_onda_min", from).unit("nm"),
        Campo::new(&path, "lunghezza_onda_max", to).unit("nm"),
        Campo::new(&path, "passo", step).unit("nm"),
    ]
}

/// Scheduled MRM of the experiment `index` of the method `no`: `(window in seconds)` when the `sMRM` stream says the experiment is scheduled (u32 flag at
/// byte 36, u32 window at byte 40: 360 in a file named «6minWindow», 120 in the file whose mzML keeps each transition 120 s around its target).
fn scheduled(ole: &Ole, no: u32, index: usize) -> Option<u32> {
    let b = ole.read(&format!(
        "MethodSubtree/Method{no}/DeviceMethod0/Period0/Experiment{index}/sMRM"
    ))?;
    (u32_at(&b, 36)? == 1).then(|| u32_at(&b, 40)).flatten()
}

fn field_mut<'a>(v: &'a mut V, key: &str) -> Option<&'a mut V> {
    match v {
        V::Obj(f) => f.iter_mut().find(|(k, _)| *k == key).map(|(_, v)| v),
        _ => None,
    }
}

/// The experiments of the `ms` section, with `programmato` / `finestra_s` for the scheduled ones. In a scheduled experiment the number the method
/// keeps in the dwell field of a transition is its target retention time (min): the field is renamed `rt_min`.
fn mark_scheduled(doc: &mut V, ole: &Ole, no: u32) {
    let Some(V::Arr(periods)) = field_mut(doc, "ms").and_then(|m| field_mut(m, "periodi")) else {
        return;
    };
    for p in periods {
        let Some(V::Arr(exps)) = field_mut(p, "esperimenti") else {
            continue;
        };
        for e in exps {
            let Some(V::Num(i)) = e.get("indice").cloned() else {
                continue;
            };
            let Some(window) = scheduled(ole, no, i as usize) else {
                continue;
            };
            if let V::Obj(f) = e {
                f.push(("programmato", V::Bool(true)));
                f.push(("finestra_s", V::Num(window as f64)));
                if let Some((_, V::Arr(t))) = f.iter_mut().find(|(k, _)| *k == "transizioni") {
                    for tr in t {
                        if let V::Obj(tf) = tr {
                            tf.iter_mut()
                                .filter(|(k, _)| *k == "dwell_ms")
                                .for_each(|(k, _)| *k = "rt_min");
                        }
                    }
                }
            }
        }
    }
}

/// `doc` with the scheduled experiments marked and `campi` (the strings of `DabsInfo` and `SampleTable`, the tables of `GLPTables`, the DAD header).
pub(super) fn extend(doc: Option<V>, ole: &Ole, n: u32, sample: &str) -> Option<V> {
    let mut doc = doc?;
    let no = crate::wiff::estesi::method_no(ole, n);
    mark_scheduled(&mut doc, ole, no);
    let strings = |path: &str| -> Vec<Campo> {
        ole.read(path)
            .map(|b| counted_strings(&b))
            .unwrap_or_default()
            .iter()
            .enumerate()
            .map(|(i, s)| Campo::new(path, &format!("s{}", i + 1), s))
            .collect()
    };
    let mut campi = strings("SampleSubtree/DabsInfo");
    campi.extend(strings("SampleSubtree/SampleTable"));
    campi.extend(glp(ole, no));
    campi.extend(dad(ole, sample));
    let list: Vec<V> = campi.iter().filter_map(Campo::json).collect();
    if let (V::Obj(f), false) = (&mut doc, list.is_empty()) {
        f.push(("campi", V::Arr(list)));
    }
    Some(doc)
}
