//! What a Shimadzu file says besides its scans: the property streams (`prop.rs`), the instrument names found in its configuration streams, and the list of what is not decoded. It becomes
//! the `metodo/1` document and the header of the mzML we write (the operator and the paths are in the document of the reader, not in the one of the mzML).

use mzlab_core::metodo::{self, num, obj, text, Source, SrcFile, V};
use mzlab_core::metodo_campi::Campo;
use mzlab_core::mzmlwrite::Header;
use mzlab_core::ole::Ole;

use super::carica::{Dati, Fam};
use super::prop;

/// The most transitions listed in the document.
const TRANSIZIONI_MAX: usize = 5000;
/// The most printable strings kept from one configuration stream.
const TESTI_MAX: usize = 64;

#[derive(Default)]
pub(super) struct Meta {
    pub modello: Option<String>,
    pub software: Option<String>,
    pub creato: Option<String>,
    pub operatore: Option<String>,
    pub campione: Option<String>,
    pub id: Option<String>,
    pub vial: Option<String>,
    pub iniezione_ul: Option<f64>,
    pub metodo_file: Option<String>,
    pub campi: Vec<Campo>,
    /// The parts of the file that are not decoded (`meta.nonDecodificato`).
    pub avvisi: Vec<String>,
}

fn ultimo(p: &str) -> &str {
    p.rsplit(['/', '\\']).next().unwrap_or(p)
}

/// Whether a key of the properties names a person, a computer or a path.
fn e_identita(k: &str, v: &str) -> bool {
    matches!(k, "generatedBy" | "modifiedBy" | "operator")
        || k.ends_with("GeneratedBy")
        || k.ends_with("ModifiedBy")
        || k.ends_with("File")
        || v.contains(['\\', ':'])
}

fn corse(it: impl Iterator<Item = Option<char>>, out: &mut Vec<String>) {
    let mut cur = String::new();
    for c in it.chain(std::iter::once(None)) {
        match c {
            Some(c) => cur.push(c),
            None => {
                if cur.trim().len() >= 4
                    && out.len() < TESTI_MAX
                    && !out.iter().any(|o| *o == cur.trim())
                {
                    out.push(cur.trim().to_string());
                }
                cur.clear();
            }
        }
    }
}

/// The runs of at least 4 printable ASCII characters of a stream (as bytes, and as UTF-16 at both alignments), at most `TESTI_MAX`.
fn testi(b: &[u8]) -> Vec<String> {
    let ascii = |c: u8| (0x20..0x7f).contains(&c).then_some(c as char);
    let mut out = Vec::new();
    corse(b.iter().map(|&c| ascii(c)), &mut out);
    for da in [0, 1] {
        let pari = b.get(da..).unwrap_or(&[]).chunks_exact(2);
        corse(
            pari.map(|p| if p[1] == 0 { ascii(p[0]) } else { None }),
            &mut out,
        );
    }
    out
}

/// A model name like `GCMS-TQ8040` or `LCMS-8060NX` in `t`: `prefix`, a dash and letters and digits.
fn nome_modello(t: &str, prefix: &str) -> Option<String> {
    let p = t.find(prefix)? + prefix.len();
    let r: String = t[p..]
        .chars()
        .take_while(char::is_ascii_alphanumeric)
        .collect();
    (r.len() >= 3).then(|| format!("{prefix}{r}"))
}

fn campo(m: &mut Meta, origine: &str, k: &str, v: &str) {
    if v.is_empty() {
        return;
    }
    let c = Campo::new(origine, k, v);
    m.campi.push(if e_identita(k, v) { c.id() } else { c });
}

/// The metadata of `ole`.
pub(super) fn raccogli(ole: &Ole, fam: Fam) -> Meta {
    let mut m = Meta::default();
    if fam == Fam::Gc {
        gc(ole, &mut m);
    } else {
        lc(ole, &mut m);
    }
    for g in [
        "GC-2010 Instrument Parameters",
        "GC17A Instrument Parameters",
        "GCMS Configuration",
        "LSS Raw Data",
        "PDA 3D Raw Data",
        "GUMM_Information",
    ] {
        let n: u64 = ole
            .paths_list()
            .filter(|p| p.starts_with(g))
            .filter_map(|p| Some(ole.read(p)?.len() as u64))
            .sum();
        if n > 0 {
            m.avvisi.push(format!("{g} ({n} bytes)"));
        }
    }
    let g = fam.storage();
    for s in [
        "Status",
        "MS Status Data",
        "Mass Parameters",
        "Status Curve",
    ] {
        if let Some(b) = ole.read(&format!("{g}/{s}")).filter(|b| !b.is_empty()) {
            m.avvisi.push(format!("{g}/{s} ({} bytes)", b.len()));
        }
    }
    m
}

fn gc(ole: &Ole, m: &mut Meta) {
    let o = "shimadzu:File Property";
    for (k, v) in ole
        .read("File Property")
        .map(|b| prop::qgd(&b))
        .unwrap_or_default()
    {
        match k.as_str() {
            "softwareVersion" => m.software = Some(v.clone()),
            "generatedTime" => m.creato = Some(v.clone()),
            "sampleName" => m.campione = Some(v.clone()),
            "sampleId" => m.id = Some(v.clone()),
            "operator" => m.operatore = Some(v.clone()),
            "methodFile" => m.metodo_file = Some(ultimo(&v).to_string()),
            _ => {}
        }
        campo(m, o, &k, &v);
    }
    for s in ["System", "DetectorMS", "GC", "Available Modules"] {
        let n = format!("GCMS Configuration/{s}");
        for (i, t) in ole
            .read(&n)
            .map(|b| testi(&b))
            .unwrap_or_default()
            .into_iter()
            .enumerate()
        {
            if m.modello.is_none() {
                m.modello = nome_modello(&t, "GCMS-");
            }
            // an instrument serial is a string with digits and letters, no more than a name
            let serie = t.len() >= 8 && t.chars().any(|c| c.is_ascii_digit());
            let c = Campo::new(&format!("shimadzu:{n}"), &format!("text{i}"), &t);
            m.campi.push(if serie { c.id() } else { c });
        }
    }
    if m.modello.is_none() {
        let st = ole
            .read("GCMS Raw Data/Status")
            .map(|b| testi(&b))
            .unwrap_or_default();
        // the name is followed by the number of the instrument (GCMS-QP2010 + 1)
        m.modello = st.iter().find_map(|t| nome_modello(t, "GCMS-")).map(|n| {
            if n.len() == 12 && n.ends_with(|c: char| c.is_ascii_digit()) {
                n[..11].to_string()
            } else {
                n
            }
        });
    }
}

fn lc(ole: &Ole, m: &mut Meta) {
    for s in ["File Property", "Method File Property"] {
        let xml = ole.read(s).map(|b| prop::xml(&b)).unwrap_or_default();
        let o = format!("shimadzu:{s}");
        let get = |k: &str| xml.iter().find(|(n, _)| n == k).map(|(_, v)| v.as_str());
        if s == "File Property" {
            m.creato = get("dwLowGeneratedDateTime")
                .zip(get("dwHighGeneratedDateTime"))
                .and_then(|(l, h)| prop::filetime_xml(l, h));
            m.software = xml
                .iter()
                .rev()
                .find(|(n, _)| n == "szVersion")
                .map(|(_, v)| v.clone());
            m.operatore = get("szGeneratedBy").map(str::to_string);
            m.vial = get("vial_num").map(str::to_string);
            m.id = get("smpl_id").map(str::to_string);
            m.iniezione_ul = get("inj_vol").and_then(|v| v.parse().ok());
        }
        for (k, v) in &xml {
            if k.ends_with("DateTime") || k.starts_with("is_amount") {
                continue;
            }
            campo(m, &o, k, v);
        }
        if let Some(t) = get("dwLowGeneratedDateTime").zip(get("dwHighGeneratedDateTime")) {
            if let Some(d) = prop::filetime_xml(t.0, t.1) {
                m.campi.push(Campo::new(&o, "generatedTime", d));
            }
        }
    }
    for p in ole
        .paths_list()
        .filter(|p| p.starts_with("GUMM_Information/Shimadzu") && p.ends_with("/GUM.CONFIG"))
    {
        let t = ole.read(p).map(|b| testi(&b)).unwrap_or_default();
        if let Some(n) = t.iter().find_map(|t| nome_modello(t, "LCMS-")) {
            m.campi
                .push(Campo::new(&format!("shimadzu:{p}"), "model", &n));
            // 2020/2050, 80xx and 90xx are instruments; the name of an older family (3030 for an 8040) is only kept as a field
            if m.modello.is_none()
                && n.chars()
                    .nth(5)
                    .is_some_and(|c| matches!(c, '2' | '8' | '9'))
            {
                m.modello = Some(n);
            }
        }
    }
}

fn tipo_evento(filtro: &str) -> &'static str {
    if filtro.contains("SIM") {
        "SIM"
    } else {
        "Q1 scan"
    }
}

/// The experiments of the `ms` section: one per event seen, then one MRM experiment with the transitions.
fn esperimenti(d: &Dati) -> Vec<Option<V>> {
    let mut esperimenti: Vec<Option<V>> = d
        .eventi
        .iter()
        .enumerate()
        .map(|(i, e)| {
            obj(vec![
                ("indice", num(Some((i + 1) as f64))),
                ("tipo", Some(V::s(tipo_evento(&e.filtro)))),
                ("mz", Some(V::Arr(vec![V::Num(e.mz.0), V::Num(e.mz.1)]))),
            ])
        })
        .collect();
    if !d.transizioni.is_empty() {
        let tr = d
            .transizioni
            .iter()
            .take(TRANSIZIONI_MAX)
            .filter_map(|&(q1, q3, p)| {
                obj(vec![
                    ("q1", num(Some(q1))),
                    ("q3", num(Some(q3))),
                    (
                        "polarita",
                        (p != 0).then(|| V::s(if p < 0 { "negative" } else { "positive" })),
                    ),
                ])
            });
        esperimenti.push(obj(vec![
            ("indice", num(Some((esperimenti.len() + 1) as f64))),
            ("tipo", Some(V::s("MRM"))),
            ("transizioni", Some(V::Arr(tr.collect()))),
        ]));
    }
    esperimenti
}

/// The `strumento` section: maker, model and the software that wrote the file.
fn strumento(fam: Fam, m: &Meta) -> Option<V> {
    let s = |t: &Option<String>| t.as_deref().and_then(text);
    obj(vec![
        ("produttore", Some(V::s("Shimadzu"))),
        ("modello", s(&m.modello)),
        (
            "software",
            obj(vec![
                ("nome", Some(V::s(software(fam)))),
                ("versione", s(&m.software)),
            ])
            .map(|v| V::Arr(vec![v])),
        ),
    ])
}

/// The `metodo/1` document. `con_identita` false leaves out the operator and the fields that name a person, a computer, a serial or a path (the one inside an mzML).
pub(super) fn documento(
    (nome, dim, sha1): (&str, u64, &str),
    fam: Fam,
    d: &Dati,
    m: &Meta,
    con_identita: bool,
) -> String {
    let src = Source {
        format: "shimadzu",
        files: vec![SrcFile {
            name: nome.to_string(),
            sha1: Some(sha1.to_string()),
            bytes: dim,
        }],
        sample: None,
    };
    let s = |t: &Option<String>| t.as_deref().and_then(text);
    let esperimenti = esperimenti(d);
    let avvisi = m.avvisi.iter().filter_map(|dove| {
        obj(vec![
            ("chiave", Some(V::s("meta.nonDecodificato"))),
            ("dove", text(dove)),
        ])
    });
    let doc = obj(vec![
        ("schema", Some(V::s(metodo::SCHEMA))),
        ("origine", metodo::origin(&src, m.creato.as_deref())),
        ("strumento", strumento(fam, m)),
        (
            "campione",
            obj(vec![
                ("nome", s(&m.campione)),
                ("id", s(&m.id)),
                ("vial", s(&m.vial)),
                ("iniezione_uL", num(m.iniezione_ul)),
                ("metodo_file", s(&m.metodo_file)),
            ]),
        ),
        (
            "operatore",
            if con_identita {
                obj(vec![("utente", s(&m.operatore))])
            } else {
                None
            },
        ),
        (
            "ms",
            obj(vec![
                (
                    "sorgente",
                    (fam == Fam::Gc).then(|| V::Obj(vec![("tipo", V::s("EI"))])),
                ),
                (
                    "periodi",
                    obj(vec![(
                        "esperimenti",
                        Some(V::Arr(esperimenti.into_iter().flatten().collect())),
                    )])
                    .map(|p| V::Arr(vec![p])),
                ),
            ]),
        ),
        (
            "campi",
            Some(V::Arr(
                m.campi
                    .iter()
                    .filter(|c| con_identita || !c.identita)
                    .filter_map(Campo::json)
                    .collect(),
            )),
        ),
        ("avvisi", Some(V::Arr(avvisi.collect()))),
    ]);
    doc.map_or_else(String::new, |d| d.json())
}

fn software(fam: Fam) -> &'static str {
    if fam == Fam::Gc {
        "GCMSsolution"
    } else {
        "LabSolutions"
    }
}

/// The header of the mzML: instrument, source file and its hash, the program, and the document without identities in the `<run>`.
pub(super) fn header(id: (&str, u64, &str), fam: Fam, d: &Dati, m: &Meta) -> Header {
    let nome = id.0;
    let mut h = Header {
        run: nome.rsplit_once('.').map_or(nome, |(s, _)| s).to_string(),
        software: ("mzLab".into(), env!("CARGO_PKG_VERSION").into()),
        instrument: Some((
            "MS:1000031".to_string(),
            m.modello.clone().unwrap_or_else(|| "Shimadzu".to_string()),
        )),
        acquired: m.creato.clone(),
        ..Header::default()
    };
    if fam == Fam::Gc {
        h.components.push((
            "source".into(),
            "MS:1000389".into(),
            "electron ionization".into(),
        ));
    }
    h.components
        .push(("analyzer".into(), "MS:1000081".into(), "quadrupole".into()));
    h.sources.push(nome.to_string());
    h.source_sha1.push(id.2.to_string());
    h.extra_software.push((
        "MS:1000531".to_string(),
        software(fam).to_string(),
        m.software.clone().unwrap_or_default(),
    ));
    h.sample = m.campione.clone().map(|n| ("1".to_string(), n));
    h.run_params.push((
        metodo::USERPARAM.to_string(),
        documento(id, fam, d, m, false),
    ));
    h
}
