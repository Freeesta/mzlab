//! One scan of an mzXML: the attributes of `<scan>`, of its `<precursorMz>` and `<peaks>`, the `<nameValue>` pairs, and the peaks themselves (base64, big-endian, 32 or 64 bit, zlib or not,
//! `m/z-int` pairs interleaved). Written from the ISB schema (mzXML 2.1 and 3.2); a number read from the file is checked against `Limiti` before it allocates.

use std::io::Read;

use base64::Engine;
use mzlab_core::error::{Error, Result};
use mzlab_core::lettori::Limiti;

use super::mzml_testata::{tag_di, unescape};

pub(super) type Attributi = Vec<(String, String)>;

/// A `<precursorMz>`: its attributes and its value.
pub(super) struct Precursore {
    pub(super) attr: Attributi,
    pub(super) mz: Option<f64>,
}

/// The start of a scan (everything before `<peaks`).
#[derive(Default)]
pub(super) struct Testa {
    pub(super) scan: Attributi,
    pub(super) precursori: Vec<Precursore>,
    pub(super) valori: Attributi,
    pub(super) picchi: Attributi,
    /// Whether the scan has a `<peaks>` element.
    pub(super) ha_picchi: bool,
}

pub(super) fn cerca<'a>(a: &'a [(String, String)], n: &str) -> &'a str {
    a.iter()
        .find(|(k, _)| k == n)
        .map_or("", |(_, v)| v.as_str())
}

/// A duration `PT1M30.5S` (or a plain number of seconds) in minutes.
pub(super) fn minuti(t: &str) -> Option<f64> {
    let t = t.trim();
    let Some(r) = t.strip_prefix("PT").or_else(|| t.strip_prefix("pt")) else {
        return t.parse::<f64>().ok().map(|s| s / 60.0);
    };
    let (mut sec, mut num) = (0.0, String::new());
    for c in r.chars() {
        let f = match c.to_ascii_uppercase() {
            'H' => 3600.0,
            'M' => 60.0,
            'S' => 1.0,
            _ => {
                num.push(c);
                continue;
            }
        };
        sec += num.parse::<f64>().ok()? * f;
        num.clear();
    }
    num.is_empty().then_some(sec / 60.0)
}

/// The `<nome ...>text</nome>` elements of `s` (at most `tetto`): their attributes and their text.
fn elementi(s: &str, nome: &str, tetto: usize) -> Vec<(Attributi, String)> {
    let apri = format!("<{nome}");
    let mut out = Vec::new();
    let mut resto = s;
    while let Some(p) = resto.find(&apri) {
        let da = &resto[p..];
        let Some(g) = da.find('>') else { break };
        let attr = tag_di(&da[..=g]).into_iter().next().map(|t| t.attr);
        let corpo = &da[g + 1..];
        let fine = corpo.find('<').unwrap_or(corpo.len());
        if da[..g].ends_with('/') {
            out.push((attr.unwrap_or_default(), String::new()));
        } else {
            out.push((attr.unwrap_or_default(), unescape(corpo[..fine].trim())));
        }
        if out.len() >= tetto {
            break;
        }
        resto = corpo;
    }
    out
}

/// The `<scan>`, `<precursorMz>`, `<nameValue>` and `<peaks>` tags of the bytes of a scan (at most `tetto` of each kind).
pub(super) fn testa(byte: &[u8], tetto: usize) -> Result<(Testa, String)> {
    let t = String::from_utf8_lossy(byte);
    let p = t.find("<peaks");
    let head = &t[..p.unwrap_or(t.len())];
    let scan = tag_di(head)
        .into_iter()
        .find(|g| !g.chiusura && g.nome == "scan")
        .ok_or_else(|| Error::new("err.file.xml").with("detail", "no <scan> in its bytes"))?
        .attr;
    let precursori = elementi(head, "precursorMz", tetto)
        .into_iter()
        .map(|(attr, v)| Precursore {
            mz: v.parse().ok(),
            attr,
        })
        .collect();
    // the `nameValue` pairs follow the peaks in the schema
    let coda = match p {
        Some(p) => t[p..].find("</peaks>").map_or("", |f| &t[p + f..]),
        None => head,
    };
    let valori = tag_di(coda)
        .into_iter()
        .filter(|g| !g.chiusura && g.nome == "nameValue")
        .take(tetto)
        .map(|g| (g.get("name").to_string(), g.get("value").to_string()))
        .collect();
    let mut r = Testa {
        scan,
        precursori,
        valori,
        ..Testa::default()
    };
    let mut testo = String::new();
    if let Some(p) = p {
        r.ha_picchi = true;
        let (attr, corpo) = elementi(&t[p..], "peaks", 1).pop().unwrap_or_default();
        r.picchi = attr;
        testo = corpo;
    }
    Ok((r, testo))
}

/// The peaks of a scan: m/z, intensities, and whether the file stores 64-bit values.
pub(super) struct Picchi {
    pub(super) mz: Vec<f64>,
    pub(super) intensita: Vec<f64>,
    pub(super) a_64_bit: bool,
}

fn xml(why: &str) -> Error {
    Error::new("err.file.xml").with("detail", why)
}

/// The bytes of the `<peaks>` text: base64, then zlib when the file says so (at most `limite` bytes).
fn grezzi(testo: &str, zlib: bool, limite: usize) -> Result<Vec<u8>> {
    let pulito: Vec<u8> = testo.bytes().filter(|c| !c.is_ascii_whitespace()).collect();
    let raw = base64::engine::general_purpose::STANDARD
        .decode(&pulito)
        .map_err(|e| Error::new("err.file.base64").with("detail", e))?;
    if !zlib {
        return Ok(raw);
    }
    let mut out = Vec::new();
    flate2::read::ZlibDecoder::new(&raw[..])
        .take(limite as u64 + 1)
        .read_to_end(&mut out)
        .map_err(|e| Error::new("err.file.zlib").with("detail", e))?;
    if out.len() > limite {
        return Err(Error::memory_budget());
    }
    Ok(out)
}

/// Decodes the `<peaks>` of a scan with the attributes `attr`: `precision`, `byteOrder`, `contentType`/`pairOrder`, `compressionType`; `conteggio` is the `peaksCount` of the scan.
pub(super) fn picchi(
    attr: &[(String, String)],
    testo: &str,
    conteggio: Option<usize>,
    limiti: &Limiti,
) -> Result<Picchi> {
    let larghezza = match cerca(attr, "precision") {
        "" | "32" => 4usize,
        "64" => 8,
        _ => return Err(xml("precision")),
    };
    if !matches!(cerca(attr, "byteOrder"), "" | "network" | "big") {
        return Err(xml("byteOrder"));
    }
    let ordine = [cerca(attr, "contentType"), cerca(attr, "pairOrder")];
    if ordine.iter().any(|o| !matches!(*o, "" | "m/z-int")) {
        return Err(xml("contentType"));
    }
    let zlib = match cerca(attr, "compressionType") {
        "" | "none" => false,
        "zlib" => true,
        _ => return Err(xml("compressionType")),
    };
    let tetto = match conteggio {
        Some(n) => limiti.voci_ok(n as u64)?,
        None => limiti.voci,
    };
    let limite = usize::try_from(limiti.byte)
        .unwrap_or(usize::MAX)
        .min(tetto.saturating_mul(2 * larghezza));
    let b = grezzi(testo, zlib, limite)?;
    let n = (b.len() / (2 * larghezza)).min(conteggio.unwrap_or(usize::MAX));
    let (mut mz, mut intensita) = (Vec::new(), Vec::new());
    mz.try_reserve_exact(n)
        .map_err(|_| Error::memory_budget())?;
    intensita
        .try_reserve_exact(n)
        .map_err(|_| Error::memory_budget())?;
    for c in b.chunks_exact(2 * larghezza).take(n) {
        let (x, y) = c.split_at(larghezza);
        mz.push(grande(x));
        intensita.push(grande(y));
    }
    Ok(Picchi {
        mz,
        intensita,
        a_64_bit: larghezza == 8,
    })
}

/// A big-endian float of 4 or 8 bytes.
fn grande(b: &[u8]) -> f64 {
    match <[u8; 4]>::try_from(b) {
        Ok(q) => f64::from(f32::from_be_bytes(q)),
        Err(_) => <[u8; 8]>::try_from(b).map_or(0.0, f64::from_be_bytes),
    }
}
