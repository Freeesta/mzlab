//! The Thermo `.raw` adapter of `metodo/1` (`metodo.rs`): the sequence row (vial, volume, id, comment, method), the instrument (model, serial number, versions of
//! the InstID block), the people and the computer, the LC and the mass spectrometer methods embedded in the file (`metodo_raw/script.rs`) and, in `campi`,
//! every other field of the container, of the scan trailer and of the method (`metodo_raw/campi.rs`). What is only guessed from position is marked in `docs/agenti/metodo.md`.
//! The status log is in `campi` (`campi/status.rs`); when its schema is not found `avvisi` says so. An anonymous document leaves out the user, the computer, the serial number and the paths, also inside the texts.

pub(crate) mod campi;
mod script;

use crate::metodo::{self, lc_sections, num, obj, text, Source, V};
use campi::Ident;
use opentfraw::RawFileReader;
use script::Block;
use std::io::{Read, Seek};

const GRAW_MAX: usize = 256 * 1024;

/// The last element of a Windows or Unix path.
fn leaf(p: &str) -> &str {
    p.rsplit(['\\', '/']).next().unwrap_or(p)
}

/// The source of the ionisation of the first scan, as its filter says (`FTMS + p ESI Full ms …`).
fn source_type(raw: &RawFileReader) -> Option<&'static str> {
    let f = raw.scan_filter(raw.run_header.sample_info.first_scan_number)?;
    ["ESI", "NSI", "APCI", "APPI", "MALDI", "EI", "CI"]
        .into_iter()
        .find(|t| f.split_whitespace().any(|w| w == *t))
}

fn positive(x: f64) -> Option<f64> {
    (x > 0.0).then_some(x)
}

fn campione(raw: &RawFileReader) -> Option<V> {
    let (s, i) = (&raw.seq_row, &raw.seq_row.injection);
    let vial = if s.vial.trim().is_empty() {
        &i.vial
    } else {
        &s.vial
    };
    obj(vec![
        ("vial", text(vial)),
        ("iniezione_uL", num(positive(i.injection_volume))),
        ("metodo_file", text(leaf(&s.inst_method))),
        ("id", text(&s.id)),
        ("commento", text(&s.comment)),
        ("diluizione", num(positive(i.dilution_factor))),
        ("peso", num(positive(i.sample_weight))),
        ("volume", num(positive(i.sample_volume))),
        ("istd", num(positive(i.istd_amount))),
    ])
}

fn strumento(raw: &RawFileReader, src: &Source) -> Option<V> {
    let id = |n: usize| {
        raw.inst_id_strings
            .get(n)
            .map(String::as_str)
            .and_then(text)
    };
    let model = raw
        .instrument_model
        .map(V::s)
        .or_else(|| id(1))
        .or_else(|| id(0));
    obj(vec![
        ("produttore", Some(V::s("Thermo Fisher Scientific"))),
        ("modello", model),
        (
            "serie",
            if src.anonymous {
                raw.inst_id_strings
                    .get(2)
                    .and_then(|s| src.alias("serial", s))
                    .map(V::s)
            } else {
                id(2)
            },
        ),
        ("hardware", id(4)),
        (
            "software",
            obj(vec![("nome", Some(V::s("Xcalibur"))), ("versione", id(3))])
                .map(|s| V::Arr(vec![s])),
        ),
    ])
}

/// The `ms` section: the source, and the scan events of the XML method.
fn ms(raw: &RawFileReader, exps: Vec<V>) -> Option<V> {
    obj(vec![
        ("sorgente", obj(vec![("tipo", source_type(raw).map(V::s))])),
        (
            "periodi",
            obj(vec![("esperimenti", Some(V::Arr(exps)))]).map(|p| V::Arr(vec![p])),
        ),
    ])
}

fn pressure(pairs: &[script::Pair]) -> Option<V> {
    let (v, u) = script::setting(pairs, "PumpModule.Pump.Pressure.UpperLimit")?;
    obj(vec![
        ("valore", num(Some(v))),
        ("unita", u.as_deref().and_then(text)),
    ])
}

/// The sampler settings of the script that are numbers.
fn sampler(pairs: &[script::Pair]) -> Option<V> {
    let params: Vec<V> = pairs
        .iter()
        .filter(|(t, k, _, _)| t.is_none() && k.starts_with("SamplerModule."))
        .filter_map(|(_, k, v, u)| {
            let n = v.replace(',', ".").parse::<f64>().ok()?;
            obj(vec![
                ("id", text(k)),
                ("valore", num(Some(n))),
                ("unita", u.as_deref().and_then(text)),
            ])
        })
        .collect();
    obj(vec![("parametri", Some(V::Arr(params)))])
}

/// The texts of the method as they are (`grezzo`), cut to `GRAW_MAX` in all, with the identity blanked in an anonymous document.
fn raw_texts(blocks: &[String], ident: &Ident, anon: bool) -> Vec<V> {
    let mut left = GRAW_MAX;
    let mut out = Vec::new();
    for b in blocks {
        let t = if anon { ident.scrub(b) } else { b.clone() };
        let mut end = t.len().min(left);
        while !t.is_char_boundary(end) {
            end -= 1;
        }
        let part = t.get(..end).unwrap_or("");
        left = left.saturating_sub(part.len());
        let name = match script::classify(b) {
            Block::Script => "Method LC",
            Block::Xml => "Method XML",
            Block::Text => "Method MS",
        };
        out.extend(obj(vec![
            ("sezione", Some(V::s(name))),
            ("testo", text(part)),
        ]));
    }
    out
}

/// The warning that the status log is there and was not decoded.
fn status_warning(raw: &RawFileReader) -> Vec<Option<V>> {
    let missing = raw.status_log.is_none() && raw.run_header.sample_info.inst_log_length > 0;
    missing
        .then(|| {
            obj(vec![
                ("chiave", Some(V::s("meta.nonDecodificato"))),
                ("dove", Some(V::s("Status log"))),
            ])
        })
        .into_iter()
        .collect()
}

/// The `metodo/1` document of a `.raw` already opened as `raw` (`source` is the same file, read again for the texts of the method). Never an error: what is missing is absent.
pub fn from_raw<R: Read + Seek>(raw: &RawFileReader, source: &mut R, src: &Source) -> String {
    let anon = src.anonymous;
    let s = &raw.seq_row;
    let ident = Ident {
        computer: raw.raw_file_info.computer_name.clone(),
        paths: [&s.path, &s.file_name, &s.inst_method, &s.proc_method]
            .iter()
            .map(|p| (*p).clone())
            .collect(),
        ..Default::default()
    };
    let blocks = raw.instrument_method_blocks(source);
    let traces = raw.analog_traces(source).unwrap_or_default();
    let by = |b: Block| blocks.iter().find(|t| script::classify(t) == b);
    let script_text = by(Block::Script).map(String::as_str).unwrap_or("");
    let pairs = script::script_pairs(script_text);
    let items = by(Block::Xml)
        .map(|x| script::xml_items(script::xml_of(x)))
        .unwrap_or_default();
    let (lc, _) = script::lc_of(script_text).map_or((None, None), |l| lc_sections(&l));
    let lc = lc.map(|mut v| {
        if let (V::Obj(f), Some(p)) = (&mut v, pressure(&pairs)) {
            f.push(("pressione_max", p));
        }
        v
    });
    let mut warn = status_warning(raw);
    if blocks.is_empty() {
        warn.push(obj(vec![
            ("chiave", Some(V::s("meta.mancante"))),
            ("dove", Some(V::s("Method"))),
        ]));
    }
    let doc = obj(vec![
        ("schema", Some(V::s(metodo::SCHEMA))),
        (
            "origine",
            metodo::origin(src, campi::clock(raw.header.audit_start.time).as_deref()),
        ),
        ("campione", campione(raw)),
        ("strumento", strumento(raw, src)),
        (
            "operatore",
            if anon {
                obj(vec![
                    (
                        "utente",
                        src.alias("user", &raw.header.audit_start.tag2).map(V::s),
                    ),
                    (
                        "computer",
                        src.alias("computer", &raw.raw_file_info.computer_name)
                            .map(V::s),
                    ),
                ])
            } else {
                obj(vec![
                    ("utente", text(&raw.header.audit_start.tag2)),
                    ("computer", text(&raw.raw_file_info.computer_name)),
                ])
            },
        ),
        ("ms", ms(raw, script::experiments(&items))),
        ("lc", lc),
        ("campionatore", sampler(&pairs)),
        (
            "campi",
            Some(V::Arr(campi::fields(
                raw, &traces, &pairs, &items, &ident, anon,
            ))),
        ),
        ("grezzo", Some(V::Arr(raw_texts(&blocks, &ident, anon)))),
        ("avvisi", Some(V::Arr(warn.into_iter().flatten().collect()))),
    ]);
    doc.map_or_else(String::new, |d| d.json())
}
