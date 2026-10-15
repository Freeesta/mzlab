//! The `.wiff` adapter of `metodo/1` (`metodo.rs`): the method of the sample (`methodinfo.rs`) plus what the container says about the sample and the run:
//! `SampleN/Log` (instrument, run status, LC modules in text), `SampleN/SampleDABE/DATA` (sample name, method file, vial), `CFR_INFO` (Analyst version, user, computer)
//! and the OLE property set `SummaryInformation`. Layouts measured on the 3200 QTRAP files; what is only guessed from position is marked in `docs/agenti/metodo.md`.

use crate::methodinfo::method_of_sample;
use crate::metodo::{lc_sections, ms_section, num, obj, text, Source, V};
use crate::ole::Ole;

const GRAW_MAX: usize = 256 * 1024;

fn u16s(b: &[u8]) -> impl Iterator<Item = u16> + '_ {
    b.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]]))
}

/// Strings stored as `u16 byte length` + UTF-16LE (the `SampleDABE` streams, `CFR_INFO`): every well-formed one, in order.
pub fn counted_strings(b: &[u8]) -> Vec<String> {
    let (mut out, mut p) = (vec![], 0);
    while p + 4 <= b.len() {
        let l = u16::from_le_bytes([b[p], b[p + 1]]) as usize;
        if l >= 2 && l.is_multiple_of(2) && l <= 2000 && p + 2 + l <= b.len() {
            let s = String::from_utf16(&u16s(&b[p + 2..p + 2 + l]).collect::<Vec<_>>()).ok();
            if let Some(s) = s.filter(|s| s.chars().all(|c| !c.is_control() || c == '\n')) {
                out.push(s);
                p += 2 + l;
                continue;
            }
        }
        p += 1;
    }
    out
}

/// The records of `SampleN/Log`: runs of Latin text between the binary headers, a record cut in blocks of 1024 characters joined again.
pub fn log_records(b: &[u8]) -> Vec<String> {
    let (mut runs, mut cur) = (Vec::<String>::new(), String::new());
    for c in u16s(b).chain(std::iter::once(0)) {
        if (0x20..0x7f).contains(&c) || c == 9 || c == 10 || (0xa0..0x100).contains(&c) {
            cur.push(char::from_u32(c as u32).unwrap_or(' '));
        } else {
            if cur.chars().count() >= 4 {
                runs.push(std::mem::take(&mut cur));
            }
            cur.clear();
        }
    }
    let mut recs: Vec<String> = vec![];
    let mut glue = false;
    for r in runs {
        if glue {
            if let Some(l) = recs.last_mut() {
                l.push_str(&r);
            }
        } else {
            // a length byte in front of the text reads as one stray character
            let mut cs = r.chars();
            let r = match (cs.next(), cs.next()) {
                (Some(a), Some(b)) if !a.is_alphabetic() && b.is_uppercase() => {
                    r.chars().skip(1).collect()
                }
                _ => r,
            };
            recs.push(r);
        }
        glue = recs.last().is_some_and(|l| l.chars().count() == 1024);
    }
    recs
}

fn field<'a>(rec: &'a str, key: &str) -> Option<&'a str> {
    let p = rec.find(key)?;
    let v = rec[p + key.len()..].split(',').next()?.trim();
    (!v.is_empty()).then_some(v)
}

/// `"0.25 mL/min"` -> (0.25, "mL/min"); a text without a number leading it -> None.
fn value_unit(s: &str) -> Option<(f64, String)> {
    let s = s.trim();
    let n = s.find(|c: char| c.is_whitespace()).unwrap_or(s.len());
    let x: f64 = s[..n].parse().ok()?;
    Some((x, s[n..].trim().to_string()))
}

/// The `Section (model)` / `key:value` entries of the LC driver record: (section, key, value); the part before any section has the section "".
fn driver_entries(rec: &str) -> Vec<(String, String, String)> {
    let mut sec = String::new();
    let mut out = vec![];
    for e in rec.split(',') {
        if e.trim().is_empty() {
            continue;
        }
        let indent = e.len() - e.trim_start().len();
        let t = e.trim();
        match t.split_once(':') {
            Some((k, v)) if indent > 0 => {
                out.push((sec.clone(), k.trim().to_string(), v.trim().to_string()))
            }
            _ if indent == 0 => sec = t.split(':').next().unwrap_or(t).trim().to_string(),
            _ => {}
        }
    }
    out
}

fn kv(k: &str, v: &str) -> Option<V> {
    let (valore, unita) = match value_unit(v) {
        Some((x, u)) => (Some(V::Num(x)), text(&u)),
        None => (None, None),
    };
    obj(vec![
        ("id", text(k)),
        ("etichetta", text(k)),
        ("valore", valore.clone()),
        ("testo", if valore.is_none() { text(v) } else { None }),
        ("unita", unita),
    ])
}

/// OLE property set (MS-OLEPS): (property id, text) of the string properties and the creation time (id 12) as `YYYY-MM-DDTHH:MM:SS`, local time of the instrument PC.
fn properties(b: &[u8]) -> Vec<(u32, String)> {
    let rd = |at: usize| {
        b.get(at..at + 4)
            .map(|x| u32::from_le_bytes([x[0], x[1], x[2], x[3]]))
    };
    let (Some(off), Some(_)) = (rd(0x2c).map(|o| o as usize), rd(0x2c)) else {
        return vec![];
    };
    let n = rd(off + 4).unwrap_or(0).min(64) as usize;
    let mut out = vec![];
    for i in 0..n {
        let (Some(pid), Some(po)) = (rd(off + 8 + 8 * i), rd(off + 12 + 8 * i)) else {
            break;
        };
        let at = off + po as usize;
        let s = match rd(at) {
            Some(30) => rd(at + 4)
                .and_then(|l| b.get(at + 8..at + 8 + l as usize))
                .map(|x| {
                    String::from_utf8_lossy(x)
                        .trim_end_matches('\0')
                        .to_string()
                }),
            Some(31) => rd(at + 4)
                .and_then(|l| b.get(at + 8..at + 8 + 2 * l as usize))
                .map(|x| {
                    String::from_utf16_lossy(&u16s(x).collect::<Vec<_>>())
                        .trim_end_matches('\0')
                        .to_string()
                }),
            Some(64) => b
                .get(at + 4..at + 12)
                .map(|x| filetime(u64::from_le_bytes(x.try_into().unwrap_or([0; 8])))),
            _ => None,
        };
        if let Some(s) = s {
            out.push((pid, s));
        }
    }
    out
}

fn filetime(ft: u64) -> String {
    let secs = (ft / 10_000_000) as i64 - 11_644_473_600;
    let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    // civil-from-days (Howard Hinnant)
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let (doy, y) = (doe - (365 * yoe + yoe / 4 - yoe / 100), yoe + era * 400);
    let mp = (5 * doy + 2) / 153;
    let (d, m) = (
        doy - (153 * mp + 2) / 5 + 1,
        if mp < 10 { mp + 3 } else { mp - 9 },
    );
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
        if m <= 2 { y + 1 } else { y },
        m,
        d,
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

/// The instrument model (`Component ID` of the mass spectrometer in the Log of sample `n`) and the creation time of the file
/// (`YYYY-MM-DDTHH:MM:SS`, local time of the instrument PC): what the mzML header says about the run.
pub fn run_facts(ole: &Ole, n: u32) -> (Option<String>, Option<String>) {
    let (log, _, _, props) = read_parts(ole, &format!("SampleSubtree/Sample{n}/"));
    let model = log
        .iter()
        .find(|r| r.starts_with("Mass Spectrometer:") && r.contains("Config Table"))
        .and_then(|r| field(r, "Component ID:"))
        .map(str::to_string);
    let created = props.into_iter().find(|(i, _)| *i == 12).map(|p| p.1);
    (model, created)
}

/// The document of sample `n` of the `.wiff` in `ole`; never an error: what is missing is absent, what cannot be read is in `avvisi`.
pub fn from_wiff(ole: &Ole, n: u32, src: &Source) -> String {
    let method = or_fallback(method_of_sample(ole, n), ole, n);
    let sample = format!("SampleSubtree/Sample{n}/");
    let (log, data, cfr, props) = read_parts(ole, &sample);
    let prop = |id| {
        props
            .iter()
            .find(|(i, _)| *i == id)
            .map(|(_, s)| s.as_str())
    };

    let ms_rec = |tag: &str| {
        log.iter()
            .find(|r| r.starts_with("Mass Spectrometer:") && r.contains(tag))
    };
    let cfg = ms_rec("Config Table");
    let driver = log.iter().find(|r| r.contains("driver:User="));
    let entries = driver.map(|r| driver_entries(r)).unwrap_or_default();
    let get = |sec: &str, k: &str| {
        entries
            .iter()
            .find(|(s, key, _)| s.starts_with(sec) && key == k)
            .map(|(_, _, v)| v.as_str())
    };
    let computer = driver.and_then(|r| field(r, "Computer="));
    let user = driver
        .and_then(|r| r.split("User=").nth(1))
        .and_then(|r| r.split(":Computer=").next())
        .map(|u| u.trim_matches('\\').trim())
        .filter(|u| !u.is_empty());
    let version = cfr.iter().find(|s| {
        !s.is_empty() && s.chars().all(|c| c.is_ascii_digit() || c == '.') && s.contains('.')
    });
    let domain_user = cfr
        .iter()
        .find(|s| s.contains('\\') && !s.contains(':'))
        .and_then(|s| s.rsplit('\\').next());

    let source_type = source_type(&log);
    let (lc, det) = lc_with_pressure(method.as_ref(), get("Pumps", "P.Max 1"));
    let autosampler = autosampler(&entries);

    let registro = run_records(&ms_rec);
    let raw = raw_log(&log);
    let warn = warnings(ole, &sample, log.is_empty());

    let doc = obj(vec![
        ("schema", Some(V::s(crate::metodo::SCHEMA))),
        ("origine", crate::metodo::origin(src, prop(12))),
        ("campione", campione(&data, get("", "Injection Volume"))),
        ("strumento", strumento(cfg, version)),
        ("operatore", operatore(user.or(domain_user), computer)),
        (
            "ms",
            method
                .as_ref()
                .and_then(|m| ms_section(m, source_type.as_deref())),
        ),
        ("lc", lc),
        (
            "campionatore",
            obj(vec![("parametri", Some(V::Arr(autosampler)))]),
        ),
        ("rivelatori", det),
        ("registro", Some(V::Arr(registro))),
        (
            "grezzo",
            obj(vec![("sezione", Some(V::s("Log"))), ("testo", text(&raw))])
                .map(|g| V::Arr(vec![g])),
        ),
        ("avvisi", Some(V::Arr(warn.into_iter().flatten().collect()))),
    ]);
    let doc = extended(doc, ole, n, &sample);
    doc.map_or_else(String::new, |d| d.json())
}

/// The strings of the log, of `DATA` and of `CFR_INFO` of the sample, and the properties of the summary information.
#[allow(clippy::type_complexity)]
fn read_parts(
    ole: &Ole,
    sample: &str,
) -> (Vec<String>, Vec<String>, Vec<String>, Vec<(u32, String)>) {
    let log = ole
        .read(&format!("{sample}Log"))
        .map(|b| log_records(&b))
        .unwrap_or_default();
    let data = ole
        .read(&format!("{sample}SampleDABE/DATA"))
        .map(|b| counted_strings(&b))
        .unwrap_or_default();
    let cfr = ole
        .read(&format!("{sample}SampleDABE/CFR_INFO"))
        .map(|b| counted_strings(&b))
        .unwrap_or_default();
    let props = ole
        .paths_list()
        .find(|p| p.ends_with("SummaryInformation") && !p.contains("Document"))
        .and_then(|p| ole.read(p))
        .map(|b| properties(&b))
        .unwrap_or_default();
    (log, data, cfr, props)
}

/// The `lc` section with the maximum pressure of the pumps (`P.Max 1` of the log, a value with its unit) and the detectors.
fn lc_with_pressure(
    method: Option<&crate::methodinfo::Method>,
    pmax: Option<&str>,
) -> (Option<V>, Option<V>) {
    let (mut lc, det) = method
        .and_then(|m| m.lc.as_ref())
        .map_or((None, None), lc_sections);
    if let (Some(pmax), Some(V::Obj(f))) = (pmax.and_then(value_unit), lc.as_mut()) {
        f.extend(
            obj(vec![
                ("valore", num(Some(pmax.0))),
                ("unita", text(&pmax.1)),
            ])
            .map(|p| ("pressione_max", p)),
        );
    }
    (lc, det)
}

/// The `campione` section: name and vial from the `DATA` strings (four or more), the method file, the injection volume.
fn campione(data: &[String], injection: Option<&str>) -> Option<V> {
    let named = data.len() >= 4;
    obj(vec![
        ("nome", data.first().filter(|_| named).and_then(|s| text(s))),
        ("vial", data.last().filter(|_| named).and_then(|s| text(s))),
        (
            "iniezione_uL",
            injection.and_then(value_unit).and_then(|v| num(Some(v.0))),
        ),
        (
            "metodo_file",
            data.iter()
                .find(|s| s.to_lowercase().ends_with(".dam"))
                .and_then(|s| text(s)),
        ),
    ])
}

/// The autosampler parameters of the driver entries.
fn autosampler(entries: &[(String, String, String)]) -> Vec<V> {
    entries
        .iter()
        .filter(|(s, ..)| s.starts_with("Autosampler"))
        .filter_map(|(_, k, v)| kv(k, v))
        .collect()
}

/// The kind of ion source as the instrument reported it («Source Type» of the log, e.g. «Turbo Spray»).
fn source_type(log: &[String]) -> Option<String> {
    log.iter()
        .find_map(|r| field(r, "Source Type:"))
        .map(String::from)
}

/// The `strumento` section from the "Config Table" record of the log and the software version of the `CFR_INFO` strings.
fn strumento(cfg: Option<&String>, version: Option<&String>) -> Option<V> {
    obj(vec![
        (
            "produttore",
            cfg.and_then(|r| field(r, "Manufacturer:")).and_then(text),
        ),
        (
            "modello",
            cfg.and_then(|r| field(r, "Component ID:")).and_then(text),
        ),
        (
            "serie",
            cfg.and_then(|r| field(r, "Serial Number:")).and_then(text),
        ),
        (
            "firmware",
            cfg.and_then(|r| field(r, "Firmware Version:"))
                .and_then(text),
        ),
        (
            "software",
            obj(vec![
                ("nome", Some(V::s("Analyst"))),
                ("versione", version.and_then(|v| text(v))),
            ])
            .map(|s| V::Arr(vec![s])),
        ),
    ])
}

/// The `operatore` section.
fn operatore(user: Option<&str>, computer: Option<&str>) -> Option<V> {
    obj(vec![
        ("utente", user.and_then(text)),
        ("computer", computer.and_then(text)),
    ])
}

/// The start and end records of the run, from the lookup of the Mass Spectrometer records of the log.
fn run_records<'a>(ms_rec: &impl Fn(&str) -> Option<&'a String>) -> Vec<V> {
    let record = |when: &str, r: Option<&String>| {
        r.and_then(|r| {
            obj(vec![
                ("quando", Some(V::s(when))),
                ("testo", text(r.split_once(":,").map_or(r, |x| x.1))),
            ])
        })
    };
    [
        record("inizio", ms_rec("Start of Run")),
        record("fine", ms_rec("End of Run")),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// The raw log joined and cut to `GRAW_MAX`.
fn raw_log(log: &[String]) -> String {
    let mut raw = log.join("\n");
    raw.truncate(
        raw.char_indices()
            .map(|(i, _)| i)
            .take_while(|&i| i <= GRAW_MAX)
            .last()
            .unwrap_or(0),
    );
    raw
}

/// What is missing or not decoded, for `avvisi`.
fn warnings(ole: &Ole, sample: &str, no_log: bool) -> Vec<Option<V>> {
    let mut warn = vec![];
    if ole
        .paths_list()
        .any(|p| p.contains("DADRealTimeData") && p.starts_with(sample))
    {
        warn.push(obj(vec![
            ("chiave", Some(V::s("meta.nonDecodificato"))),
            ("dove", Some(V::s("DAD"))),
        ]));
    }
    if no_log {
        warn.push(obj(vec![
            ("chiave", Some(V::s("meta.mancante"))),
            ("dove", Some(V::s("Log"))),
        ]));
    }
    warn
}

#[cfg(test)]
mod tests;

#[cfg(feature = "estesi")]
mod estesi;

/// The method of the sample, or (readers module) the nearest method below it when the file shares its methods between samples.
#[cfg(feature = "estesi")]
fn or_fallback(
    m: Option<crate::methodinfo::Method>,
    ole: &Ole,
    n: u32,
) -> Option<crate::methodinfo::Method> {
    m.or_else(|| crate::wiff::estesi::fallback_method(ole, n))
}

#[cfg(not(feature = "estesi"))]
fn or_fallback(
    m: Option<crate::methodinfo::Method>,
    _: &Ole,
    _: u32,
) -> Option<crate::methodinfo::Method> {
    m
}

/// The document with what only the readers module adds (`metodo_wiff/estesi.rs`).
#[cfg(feature = "estesi")]
fn extended(doc: Option<V>, ole: &Ole, n: u32, sample: &str) -> Option<V> {
    estesi::extend(doc, ole, n, sample)
}

#[cfg(not(feature = "estesi"))]
fn extended(doc: Option<V>, _: &Ole, _: u32, _: &str) -> Option<V> {
    doc
}
