//! The `metodo/1` document (`docs/agenti/metodo.md`) and the header of the mzML of a MassLynx folder: the sample and the instrument of `_HEADER.TXT`, the functions of the method (range,
//! collision energy, channels Q1/Q3/dwell/CE), the LC of `_INLET.INF`, the lock mass (declared, never applied). The user and the paths of the acquisition computer are marked `identita`.
//! Every line of the text files goes to `campi` with the section it comes from; what the reader does not decode is named in `avvisi`.

use super::cartella::{base, errore};
use super::inf::Entry;
use super::lettore::{Fun, WatersReader};
use mzlab_core::error::Result;
use mzlab_core::metodo::{self, num, obj, sha1_hex, text, Source, SrcFile, V};
use mzlab_core::metodo_campi::Field;
use mzlab_core::mzmlwrite::Header;

/// The most files named in `origine` and the most fields kept.
const FILE_MAX: usize = 200;
const FIELDS_MAX: usize = 4000;
const FORMAT: &str = "waters-raw";

/// The SHA-1 of every member, computed once (the members are read through, none is held).
fn impronte(l: &mut WatersReader) -> &[Option<String>] {
    if l.sha1.is_none() {
        let v =
            l.c.fs
                .membri
                .iter_mut()
                .map(|m| sha1_hex(&mut m.dati, m.dim).ok())
                .collect();
        l.sha1 = Some(v);
    }
    l.sha1.as_deref().unwrap_or_default()
}

/// `30-Apr-2022` and `00:26:58` as `2022-04-30T00:26:58` (the clock of the acquisition PC, no zone).
fn iso(data: &str, now: &str) -> Option<String> {
    let mut p = data.trim().split('-');
    let (g, m, a) = (p.next()?, p.next()?, p.next()?);
    let mese = [
        "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
    ]
    .iter()
    .position(|x| m.to_ascii_lowercase().starts_with(x))?
        + 1;
    let (g, a): (u32, u32) = (g.parse().ok()?, a.parse().ok()?);
    let now = now.trim();
    (now.len() == 8 && (1..=31).contains(&g)).then(|| format!("{a:04}-{mese:02}-{g:02}T{now}"))
}

fn acquisito(l: &WatersReader) -> Option<String> {
    iso(
        l.testata.get("Acquired Date")?,
        l.testata.get("Acquired Time")?,
    )
}

/// «XEVO-TQXS#WBA0927» → (model, serial); «NotSet» is no serial.
fn strumento_di(l: &WatersReader) -> (Option<String>, Option<String>) {
    let Some(i) = l.testata.get("Instrument") else {
        return (None, None);
    };
    let (m, s) = i.split_once('#').unwrap_or((i, ""));
    let s = s.trim();
    let series = (!s.is_empty() && !s.eq_ignore_ascii_case("NotSet")).then(|| s.to_string());
    (Some(m.trim().to_string()).filter(|m| !m.is_empty()), series)
}

fn tipo(f: &Fun) -> &'static str {
    if f.f.mirato() {
        "MRM"
    } else if f.f.riferimento() {
        "lock mass"
    } else if f.livello == 2 && f.ext.rampa.is_some() {
        "MSe"
    } else if f.livello == 2 {
        "MS2"
    } else {
        "MS"
    }
}

fn polarita_v(p: i8) -> Option<V> {
    match p {
        1 => Some(V::s("positive")),
        -1 => Some(V::s("negative")),
        _ => None,
    }
}

fn param(id: &str, label: &str, value: f64, unita: &str) -> Option<V> {
    obj(vec![
        ("id", text(id)),
        ("etichetta", text(label)),
        (
            "valore",
            num(Some(value)).filter(|_| value.is_finite() && value != 0.0),
        ),
        ("unita", text(unita)),
    ])
}

fn experiment(f: &Fun) -> Option<V> {
    let transizioni: Vec<V> =
        f.f.channels()
            .iter()
            .enumerate()
            .filter_map(|(j, &(q1, q3, dw))| {
                obj(vec![
                    ("q1", num(Some(f64::from(q1)))),
                    ("q3", num(Some(f64::from(q3)))),
                    ("dwell_ms", num(Some(f64::from(dw) * 1000.0))),
                    (
                        "ce_eV",
                        num(f.ee.as_ref().and_then(|t| t.value("Collision Energy", j))),
                    ),
                    ("nome", f.names.get(j).and_then(|n| text(n))),
                    ("polarita", polarita_v(f.polarita)),
                ])
            })
            .collect();
    let mirato = f.f.mirato();
    let mz =
        f.f.interval()
            .filter(|_| !mirato)
            .map(|(a, b)| V::Arr(vec![V::Num(a), V::Num(b)]));
    let ce = match (f.ext.rampa, f.ext.ce) {
        (Some((a, b)), _) => Some(V::Arr(vec![V::Num(a), V::Num(b)])),
        (None, Some(c)) => Some(V::Arr(vec![V::Num(c)])),
        _ => None,
    };
    obj(vec![
        ("indice", Some(V::Num(f64::from(f.f.indice)))),
        ("tipo", Some(V::s(tipo(f)))),
        ("mz", mz),
        ("ce_eV", ce.filter(|_| !mirato)),
        (
            "parametri",
            Some(V::Arr(
                [
                    param("cycle_time", "Cycle time", f64::from(f.f.cycle_s), "s"),
                    param(
                        "tof_depth",
                        "TOF bins per push",
                        f64::from(f.f.tof_depth),
                        "",
                    ),
                ]
                .into_iter()
                .flatten()
                .collect(),
            )),
        ),
        ("transizioni", Some(V::Arr(transizioni))),
    ])
}

fn ms(l: &WatersReader) -> Option<V> {
    let experiments: Vec<V> = l.funs.iter().filter_map(experiment).collect();
    let reference = l.funs.iter().find(|f| f.f.riferimento());
    let lock = reference.and_then(|f| {
        obj(vec![
            (
                "modo",
                Some(V::s(
                    "reference function; the correction is not applied to the m/z",
                )),
            ),
            ("numero_nel_trailer", Some(V::Num(f64::from(f.f.indice)))),
        ])
    });
    obj(vec![
        (
            "sorgente",
            obj(vec![("tipo", source(l).as_deref().and_then(text))]),
        ),
        (
            "periodi",
            obj(vec![("esperimenti", Some(V::Arr(experiments)))]).map(|p| V::Arr(vec![p])),
        ),
        ("calibrazione", obj(vec![("lock_mass", lock)])),
    ])
}

/// «Type - ESI» of the section «Source Information».
fn source(l: &WatersReader) -> Option<String> {
    l.ext
        .voci
        .iter()
        .find(|v| v.section == "Source Information" && v.chiave == "Type")
        .map(|v| v.valore.clone())
}

fn lc(l: &WatersReader) -> Option<V> {
    let i = l.inlet.as_ref()?;
    let grad: Vec<V> = i
        .gradiente
        .iter()
        .filter_map(|&(t, stream, b)| {
            obj(vec![
                ("t_min", num(Some(t))),
                ("B_pct", num(b)),
                ("flusso_mL_min", num(stream)),
            ])
        })
        .collect();
    obj(vec![
        ("gradiente", Some(V::Arr(grad))),
        ("flusso_mL_min", num(i.gradiente.first().and_then(|g| g.1))),
        ("B_iniziale_pct", num(i.gradiente.first().and_then(|g| g.2))),
        ("forno_C", num(i.oven_c)),
        ("durata_min", num(i.duration_min)),
    ])
}

/// A value that names the acquisition computer or one of its folders.
fn identifier(key: &str, value: &str) -> bool {
    value.contains(":\\")
        || value.starts_with("\\\\")
        || matches!(
            key,
            "User Name" | "Submitter" | "Laboratory Name" | "Computer Name"
        )
}

fn fields_of(origin: &str, entries: &[Entry], dest: &mut Vec<Field>) {
    for v in entries {
        if dest.len() >= FIELDS_MAX {
            return;
        }
        let o = if v.section.is_empty() {
            origin.to_string()
        } else {
            format!("{origin}#{}", v.section)
        };
        let c = Field::new(&o, &v.chiave, &v.valore);
        dest.push(if identifier(&v.chiave, &v.valore) {
            c.id()
        } else {
            c
        });
    }
}

fn fields(l: &WatersReader) -> Vec<Field> {
    let mut out = Vec::new();
    for (k, v) in &l.testata.entries {
        if out.len() >= FIELDS_MAX {
            break;
        }
        let mappa = match k.as_str() {
            "Sample Description" => Some("campione.nome"),
            "Bottle Number" => Some("campione.vial"),
            "SampleID" => Some("campione.id"),
            "User Name" => Some("operatore.utente"),
            "Instrument" => Some("strumento.modello"),
            _ => None,
        };
        let c = Field::new("waters:_HEADER.TXT", k, v);
        let c = if identifier(k, v) { c.id() } else { c };
        out.push(match mappa {
            Some(m) => c.map(m),
            None => c,
        });
    }
    fields_of("waters:_extern.inf", &l.ext.voci, &mut out);
    if let Some(i) = &l.inlet {
        fields_of("waters:_INLET.INF", &i.entries, &mut out);
    }
    for a in &l.analogiche {
        out.push(Field::new("waters:_CHROMS.INF", &a.name, &a.raw_value).unit(&a.unita));
    }
    out
}

fn origin(l: &mut WatersReader) -> Source {
    let sha = impronte(l).to_vec();
    let mut files: Vec<SrcFile> =
        l.c.fs
            .membri
            .iter()
            .zip(sha)
            .map(|(m, s)| SrcFile {
                name: base(&m.nome).to_string(),
                sha1: s,
                bytes: m.dim,
            })
            .collect();
    files.sort_by(|a, b| a.name.cmp(&b.name));
    files.truncate(FILE_MAX);
    Source {
        format: FORMAT,
        files,
        sample: None,
    }
}

pub(super) fn document(l: &mut WatersReader) -> Result<String> {
    let src = origin(l);
    let (model, series) = strumento_di(l);
    let h = |k: &str| l.testata.get(k).and_then(text);
    let metodo_file = l.testata.get("MS Method").map(|p| base(p).to_string());
    let doc = obj(vec![
        ("schema", Some(V::s(metodo::SCHEMA))),
        ("origine", metodo::origin(&src, acquisito(l).as_deref())),
        (
            "campione",
            obj(vec![
                ("nome", h("Sample Description")),
                ("vial", h("Bottle Number")),
                ("id", h("SampleID")),
                ("commento", h("Conditions")),
                ("metodo_file", metodo_file.as_deref().and_then(text)),
            ]),
        ),
        (
            "strumento",
            obj(vec![
                ("produttore", Some(V::s("Waters"))),
                ("modello", model.as_deref().and_then(text)),
                ("serie", series.as_deref().and_then(text)),
            ]),
        ),
        ("operatore", obj(vec![("utente", h("User Name"))])),
        ("ms", ms(l)),
        ("lc", lc(l)),
        (
            "campi",
            Some(V::Arr(fields(l).iter().filter_map(Field::json).collect())),
        ),
        (
            "avvisi",
            Some(V::Arr(
                l.warnings
                    .iter()
                    .filter_map(|(k, d)| obj(vec![("chiave", text(k)), ("dove", text(d))]))
                    .collect(),
            )),
        ),
    ]);
    doc.map(|d| d.json()).ok_or_else(|| errore("empty method"))
}

/// The header of the mzML: the instrument, the data files with their SHA-1 and the document.
pub(super) fn file_header(l: &mut WatersReader) -> Result<Header> {
    let doc = document(l)?;
    let sha = impronte(l).to_vec();
    let (model, series) = strumento_di(l);
    let name = l.testata.get("Acquired Name").map_or_else(
        || l.c.name.trim_end_matches(".raw").to_string(),
        str::to_string,
    );
    let mut h = Header {
        run: name,
        software: ("mzLab".into(), env!("CARGO_PKG_VERSION").into()),
        instrument: model.map(|m| ("MS:1000031".to_string(), m)),
        acquired: acquisito(l),
        serial: series,
        source_cv: vec![
            ("MS:1000769".into(), "Waters nativeID format".into()),
            ("MS:1000526".into(), "Waters raw format".into()),
        ],
        ..Header::default()
    };
    for (m, s) in l.c.fs.membri.iter().zip(sha) {
        let n = base(&m.nome);
        if n.len() == 12
            && n.to_ascii_uppercase().starts_with("_FUNC")
            && n.to_ascii_uppercase().ends_with(".DAT")
        {
            h.sources.push(n.to_string());
            h.source_sha1.push(s.unwrap_or_default());
        }
    }
    h.run_params.push((metodo::USERPARAM.to_string(), doc));
    Ok(h)
}
