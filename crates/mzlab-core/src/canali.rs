//! The channels of a file (`docs/agenti/canali-dati.md`): every file is a list of channels, each a trace (time → intensity) and, when it has them, a
//! slice of the scans. Built once from the scan table and the chromatogram headers; the tools ask "channel + file", never "kind".
//! Used by `canali::rotte` (`api/canali`, `canale=`) and `mzlab-cli canali`.

use std::collections::HashMap;

use crate::idxread::Scan;
use crate::iteminfo::Facts;
use crate::json::{fmt_g, round, Out};
use crate::mzmlhead::Chrom;

mod rotte;
pub(crate) use rotte::Stato;

/// More distinct MSn keys than this at one level: one `dd:<level>` channel (the same cap as `infusione`).
pub const MAX_CHIAVI: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Famiglia {
    Tic,
    Bpc,
    Ms,
    Dd,
    Srm,
    Sim,
    /// Filled by the UV/analog work; no channel of these families is built here.
    Uv,
    Analog,
}

impl Famiglia {
    pub fn nome(self) -> &'static str {
        match self {
            Famiglia::Tic => "tic",
            Famiglia::Bpc => "bpc",
            Famiglia::Ms => "ms",
            Famiglia::Dd => "dd",
            Famiglia::Srm => "srm",
            Famiglia::Sim => "sim",
            Famiglia::Uv => "uv",
            Famiglia::Analog => "an",
        }
    }
}

/// Where the trace of a channel lives.
#[derive(Debug, Clone, PartialEq)]
pub enum Sorgente {
    /// A slice `da..a` of `Canali::scan_ids`.
    Scan { da: u32, a: u32 },
    /// The chromatogram of that index in the mzML.
    Cromatogramma(i64),
    /// The sum of the SRM transitions (a file with no TIC of its own).
    Somma,
    Dad,
    DadNm(f64, f64),
}

#[derive(Debug, Clone)]
pub struct Canale {
    pub id: String,
    pub famiglia: Famiglia,
    /// Index of the parent channel (`tic`, or the channel of the precursor).
    pub genitore: Option<u32>,
    /// 0 for UV and analog channels.
    pub livello: u8,
    /// +1, -1 or 0 (both, or none).
    pub pol: i8,
    pub sigla: Option<&'static str>,
    pub etichetta: String,
    /// Same value in two files = the same channel (level, nominal masses of the path, activation, collision energy rounded; q1/q3 to 0.1).
    pub omologo: String,
    pub n: u32,
    pub rt: (f64, f64),
    /// The channel has scans with a spectrum (`ms`, `dd`, `uv`).
    pub spettri: bool,
    pub sorgente: Sorgente,
}

/// A proposal for the default tools, never a verdict: the user's own value (CANALI-F) wins.
#[derive(Debug, Clone, PartialEq)]
pub struct Tipo {
    /// `lc-full` | `lc-ms2-mirato` | `lc-dda` | `lc-mrm` | `infusione-msn` | `misto` | `solo-uv`
    pub proposto: &'static str,
    pub motivo: &'static str,
}

#[derive(Debug, Clone)]
pub struct Canali {
    pub v: Vec<Canale>,
    /// The indices of the scans (in `idx.scans`) partitioned by leaf channel, file order inside each channel: 4 B per scan.
    pub scan_ids: Vec<u32>,
    pub tipo: Tipo,
}

/// A group of scans while the channels are being counted.
struct Gruppo {
    id: String,
    level: u8,
    n: u32,
    rt: (f64, f64),
    pol: i8,
    /// Nominal masses of the path (the precursors, one per stage).
    percorso: Vec<i64>,
    sigla: Option<&'static str>,
    etichetta: String,
    omologo: String,
}

fn nominale(m: f64) -> i64 {
    m.round_ties_even() as i64
}

/// `MS²`, `MS³`, ...: the level as a superscript.
fn apice(level: u8) -> String {
    const SUP: [char; 10] = ['⁰', '¹', '²', '³', '⁴', '⁵', '⁶', '⁷', '⁸', '⁹'];
    let cifre: String = level.to_string().chars().filter_map(|c| c.to_digit(10)).map(|d| SUP[d as usize]).collect();
    format!("MS{cifre}")
}

/// Nominal masses of the path of an MSn scan: the stages of the filter, else the precursor alone.
fn percorso_di(s: &Scan) -> Vec<i64> {
    match s.path.as_deref() {
        Some(p) if !p.is_empty() => p.iter().map(|x| nominale(x.0)).collect(),
        _ => {
            let p = s.prec(f64::NAN);
            if p.is_nan() { Vec::new() } else { vec![nominale(p)] }
        }
    }
}

/// Activation (lower case) and collision energy of the last stage of an MSn scan.
fn attivazione_di(s: &Scan) -> (String, Option<f64>) {
    let last = s.path.as_ref().and_then(|p| p.last());
    let act = s.act.clone().filter(|a| !a.is_empty()).or_else(|| last.map(|l| l.1.clone())).unwrap_or_default().to_lowercase();
    let ce = last.map(|l| l.2).or(s.collision_energy).filter(|c| c.is_finite() && *c > 0.0);
    (act, ce)
}

/// The id of the channel a scan belongs to. For an MS2 scan the key of the scan does not name the precursor, the channel does.
fn id_ms(s: &Scan, percorso: &[i64]) -> String {
    let key = s.key();
    if s.level != 2 || percorso.is_empty() {
        return format!("ms:{key}");
    }
    let m = percorso[0];
    match key.rfind(" @") {
        Some(p) => format!("ms:{} {m}{}", &key[..p], &key[p..]),
        None => format!("ms:{key} {m}"),
    }
}

fn sigla_di(s: &Scan) -> Option<&'static str> {
    if s.level <= 1 && s.filter.to_ascii_lowercase().contains(" sim ") {
        return Some("SIM");
    }
    s.kind_name()
}

fn pol_label(pol: i8) -> &'static str {
    match pol {
        1 => "+",
        -1 => "-",
        _ => "",
    }
}

/// The group a first scan opens.
fn nuovo_gruppo(id: String, s: &Scan, percorso: Vec<i64>) -> Gruppo {
    let (act, ce) = attivazione_di(s);
    let ce_int = ce.map(|c| c.round() as i64);
    let sigla = sigla_di(s);
    let masse = percorso.iter().map(i64::to_string).collect::<Vec<_>>().join(">");
    let mut etichetta = apice(s.level.max(1));
    if s.level >= 2 && !percorso.is_empty() {
        etichetta += &format!(" {}", percorso.iter().map(i64::to_string).collect::<Vec<_>>().join("→"));
    }
    if let Some(sg) = sigla.filter(|_| s.level <= 1) {
        etichetta += &format!(" {sg}");
    }
    if s.level >= 2 && !act.is_empty() {
        etichetta += &format!(" @{}{}", act.to_uppercase(), ce_int.map(|c| c.to_string()).unwrap_or_default());
    }
    let omologo = format!("ms|{}|{masse}|{act}|{}|{}|{}", s.level, ce_int.map(|c| c.to_string()).unwrap_or_default(), sigla.unwrap_or(""), pol_label(s.polarity));
    Gruppo { id, level: s.level, n: 0, rt: (f64::INFINITY, f64::NEG_INFINITY), pol: s.polarity, percorso, sigla, etichetta, omologo }
}

impl Canali {
    /// The channels of a file from its scan table and the headers of its chromatograms (`chroms` may be empty); `nome` is the file name (the infusion hint).
    pub fn da(scans: &[Scan], chroms: &[Chrom], facts: &Facts, nome: &str) -> Canali {
        let mut gruppi: Vec<Gruppo> = Vec::new();
        let mut per_id: HashMap<String, u32> = HashMap::new();
        let mut di: Vec<u32> = Vec::with_capacity(scans.len());
        for s in scans {
            let percorso = if s.level >= 2 { percorso_di(s) } else { Vec::new() };
            let id = id_ms(s, &percorso);
            let chiave = format!("{id}\u{1}{}", s.polarity);
            let g = *per_id.entry(chiave).or_insert_with(|| {
                gruppi.push(nuovo_gruppo(id, s, percorso));
                (gruppi.len() - 1) as u32
            });
            let x = &mut gruppi[g as usize];
            x.n += 1;
            x.rt = (x.rt.0.min(s.rt), x.rt.1.max(s.rt));
            di.push(g);
        }
        drop(per_id);
        // the same key in both polarities is two channels: the sign tells them apart
        let mut doppi: HashMap<&str, u32> = HashMap::new();
        for g in &gruppi {
            *doppi.entry(g.id.as_str()).or_default() += 1;
        }
        let ripetuti: Vec<bool> = gruppi.iter().map(|g| doppi[g.id.as_str()] > 1).collect();
        drop(doppi);
        for (g, r) in gruppi.iter_mut().zip(&ripetuti) {
            if *r {
                g.id += if g.pol < 0 { "#-" } else { "#+" };
            }
        }
        // too many keys at a level: one aggregate (DDA)
        let mut chiavi: HashMap<u8, usize> = HashMap::new();
        for g in gruppi.iter().filter(|g| g.level >= 2) {
            *chiavi.entry(g.level).or_default() += 1;
        }
        let mut dd_livelli: Vec<u8> = chiavi.iter().filter(|(_, n)| **n > MAX_CHIAVI).map(|(l, _)| *l).collect();
        crate::sortc::slice_ord(&mut dd_livelli);
        let mut finali: Vec<Gruppo> = Vec::new();
        let mut verso: Vec<u32> = vec![0; gruppi.len()];
        let mut dd_pos: HashMap<u8, u32> = HashMap::new();
        for (k, g) in gruppi.into_iter().enumerate() {
            if dd_livelli.contains(&g.level) {
                let p = *dd_pos.entry(g.level).or_insert_with(|| {
                    finali.push(Gruppo {
                        id: format!("dd:{}", g.level),
                        level: g.level,
                        n: 0,
                        rt: (f64::INFINITY, f64::NEG_INFINITY),
                        pol: g.pol,
                        percorso: Vec::new(),
                        sigla: Some("DDA"),
                        etichetta: format!("{} DDA", apice(g.level)),
                        omologo: format!("dd|{}", g.level),
                    });
                    (finali.len() - 1) as u32
                });
                let d = &mut finali[p as usize];
                d.n += g.n;
                d.rt = (d.rt.0.min(g.rt.0), d.rt.1.max(g.rt.1));
                if d.pol != g.pol {
                    d.pol = 0;
                }
                verso[k] = p;
            } else {
                finali.push(g);
                verso[k] = (finali.len() - 1) as u32;
            }
        }
        // order: level, then the most populated, then the id; `ordine[g]` = rank of group g
        let mut rango: Vec<u32> = (0..finali.len() as u32).collect();
        crate::sortc::slice_by(&mut rango, |a, b| {
            let (x, y) = (&finali[*a as usize], &finali[*b as usize]);
            x.level.cmp(&y.level).then(y.n.cmp(&x.n)).then_with(|| x.id.cmp(&y.id))
        });
        let mut pos_di = vec![0u32; finali.len()];
        for (r, g) in rango.iter().enumerate() {
            pos_di[*g as usize] = r as u32;
        }
        // the partition of the scans: a counting sort by rank, file order inside each channel
        let mut inizio = vec![0u32; finali.len() + 1];
        for (r, g) in rango.iter().enumerate() {
            inizio[r + 1] = inizio[r] + finali[*g as usize].n;
        }
        let mut riempiti = inizio.clone();
        let mut scan_ids = vec![0u32; scans.len()];
        for (i, g) in di.iter().enumerate() {
            let r = pos_di[verso[*g as usize] as usize] as usize;
            scan_ids[riempiti[r] as usize] = i as u32;
            riempiti[r] += 1;
        }
        drop(di);
        let mut v: Vec<Canale> = Vec::new();
        let ha_scan = !scans.is_empty();
        let srm: Vec<&Chrom> = chroms.iter().filter(|c| c.kind == "srm").collect();
        let tic_cromatogramma = chroms.iter().find(|c| c.kind == "tic");
        if ha_scan || !srm.is_empty() {
            let (n, rt) = if ha_scan {
                (scans.len() as u32, scans.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |a, s| (a.0.min(s.rt), a.1.max(s.rt))))
            } else {
                let rt = facts.srm_rt.unwrap_or((0.0, 0.0));
                (srm.iter().map(|c| c.n.unwrap_or(0).max(0) as u32).max().unwrap_or(0), rt)
            };
            let sorgente = match (ha_scan, tic_cromatogramma) {
                (true, _) => Sorgente::Scan { da: 0, a: scans.len() as u32 },
                (false, Some(c)) => Sorgente::Cromatogramma(c.index as i64),
                (false, None) => Sorgente::Somma,
            };
            v.push(Canale { id: "tic".into(), famiglia: Famiglia::Tic, genitore: None, livello: 0, pol: 0, sigla: None, etichetta: "TIC".into(), omologo: "tic".into(), n, rt, spettri: false, sorgente: sorgente.clone() });
            if ha_scan {
                v.push(Canale { id: "bpc".into(), famiglia: Famiglia::Bpc, genitore: Some(0), etichetta: "BPC".into(), omologo: "bpc".into(), ..v[0].clone() });
            }
        }
        let base = v.len();
        for g in &rango {
            let f = &finali[*g as usize];
            let r = pos_di[*g as usize] as usize;
            v.push(Canale {
                id: f.id.clone(),
                famiglia: if f.id.starts_with("dd:") { Famiglia::Dd } else { Famiglia::Ms },
                genitore: Some(0),
                livello: f.level,
                pol: f.pol,
                sigla: f.sigla,
                etichetta: f.etichetta.clone(),
                omologo: f.omologo.clone(),
                n: f.n,
                rt: f.rt,
                spettri: true,
                sorgente: Sorgente::Scan { da: inizio[r], a: inizio[r + 1] },
            });
        }
        // parents of the MSn channels: the channel of the precursor, else the DDA aggregate of the level before, else the TIC
        let radice = if v.is_empty() { None } else { Some(0u32) };
        for (k, g) in rango.iter().enumerate() {
            let f = &finali[*g as usize];
            v[base + k].genitore = radice;
            if f.level < 2 || f.id.starts_with("dd:") {
                continue;
            }
            let padre = rango.iter().enumerate().filter(|(_, h)| finali[**h as usize].level == f.level - 1 && !finali[**h as usize].id.starts_with("dd:")).filter(|(_, h)| {
                let p = &finali[**h as usize];
                p.percorso == f.percorso[..f.percorso.len().saturating_sub(1)] && (p.pol == f.pol || p.pol == 0 || f.pol == 0)
            });
            let padre = padre.max_by_key(|&(j, h)| (finali[*h as usize].n, std::cmp::Reverse(j))).map(|(j, _)| base + j).or_else(|| {
                rango.iter().position(|h| finali[*h as usize].id == format!("dd:{}", f.level - 1)).map(|j| base + j)
            });
            if let Some(p) = padre {
                v[base + k].genitore = Some(p as u32);
            }
        }
        for c in srm {
            let ce = c.ce.filter(|x| x.is_finite());
            let (q1, q3) = (c.q1.unwrap_or(0.0), c.q3.unwrap_or(0.0));
            let mut id = format!("srm:{}>{}", fmt_g(q1), fmt_g(q3));
            if let Some(x) = ce {
                id += &format!("@{}", fmt_g(x));
            }
            if v.iter().any(|x| x.id == id) {
                id += &format!("#{}", c.index);
            }
            let mut etichetta = format!("MRM {}→{}", fmt_g(q1), fmt_g(q3));
            if let Some(x) = ce {
                etichetta += &format!(" @{}", fmt_g(x));
            }
            if !c.name.is_empty() {
                etichetta += &format!(" «{}»", c.name);
            }
            let rt = facts.srm_rt.unwrap_or((0.0, 0.0));
            let omologo = format!("srm|2|{:.1}>{:.1}||{}||", q1, q3, ce.map(|x| (x.round() as i64).to_string()).unwrap_or_default());
            v.push(Canale { id, famiglia: Famiglia::Srm, genitore: radice, livello: 2, pol: 0, sigla: Some("MRM"), etichetta, omologo, n: c.n.unwrap_or(0).max(0) as u32, rt, spettri: false, sorgente: Sorgente::Cromatogramma(c.index as i64) });
        }
        for c in chroms.iter().filter(|c| c.kind != "srm" && c.id.get(..3).is_some_and(|p| p.eq_ignore_ascii_case("sic") || p.eq_ignore_ascii_case("sim"))) {
            let rt = facts.srm_rt.unwrap_or((0.0, 0.0));
            v.push(Canale { id: format!("sim:{}", c.id), famiglia: Famiglia::Sim, genitore: radice, livello: 1, pol: 0, sigla: Some("SIM"), etichetta: format!("SIM {}", c.id), omologo: format!("sim|1|{}", c.id), n: c.n.unwrap_or(0).max(0) as u32, rt, spettri: false, sorgente: Sorgente::Cromatogramma(c.index as i64) });
        }
        let tipo = tipo_di(scans, &v, facts, nome);
        Canali { v, scan_ids, tipo }
    }

    /// Index of the channel `id`.
    pub fn trova(&self, id: &str) -> Option<usize> {
        self.v.iter().position(|c| c.id == id)
    }

    /// The scans of channel `i` in file order (`tic` and `bpc`: every scan).
    pub fn scansioni(&self, i: usize) -> Vec<u32> {
        match self.v.get(i).map(|c| &c.sorgente) {
            Some(Sorgente::Scan { da, a }) if matches!(self.v[i].famiglia, Famiglia::Tic | Famiglia::Bpc) => {
                let mut t = self.scan_ids[*da as usize..*a as usize].to_vec();
                crate::sortc::slice_ord(&mut t);
                t
            }
            Some(Sorgente::Scan { da, a }) => self.scan_ids[*da as usize..*a as usize].to_vec(),
            _ => Vec::new(),
        }
    }

    /// Bytes this structure holds beyond the scan table.
    pub fn byte(&self) -> usize {
        self.scan_ids.capacity() * 4
            + self.v.iter().map(|c| std::mem::size_of::<Canale>() + c.id.len() + c.etichetta.len() + c.omologo.len()).sum::<usize>()
    }

    /// `{"v":"canali/1","tipo":{...},"canali":[...]}`.
    pub fn json(&self) -> String {
        let mut o = Out::default();
        o.raw("{\"v\":\"canali/1\",\"tipo\":{\"proposto\":").str(self.tipo.proposto).raw(",\"motivo\":").str(self.tipo.motivo).raw("},\"canali\":[");
        for (i, c) in self.v.iter().enumerate() {
            o.raw(if i > 0 { ",{\"id\":" } else { "{\"id\":" }).str(&c.id).raw(",\"fam\":").str(c.famiglia.nome()).raw(",\"gen\":");
            match c.genitore {
                Some(g) => o.int(g as i64),
                None => o.raw("null"),
            };
            o.raw(",\"liv\":").int(c.livello as i64).raw(",\"pol\":").int(c.pol as i64).raw(",\"sigla\":");
            match c.sigla {
                Some(s) => o.str(s),
                None => o.raw("null"),
            };
            o.raw(",\"et\":").str(&c.etichetta).raw(",\"omo\":").str(&c.omologo).raw(",\"n\":").int(c.n as i64);
            let (a, b) = if c.rt.0.is_finite() { c.rt } else { (0.0, 0.0) };
            o.raw(",\"rt0\":").num(round(a, 4)).raw(",\"rt1\":").num(round(b, 4)).raw(",\"spettri\":").raw(if c.spettri { "true" } else { "false" }).raw("}");
        }
        o.raw("]}");
        o.0
    }
}

/// The proposed type of experiment from the structure of the channels (`infusione` decides the infusions).
fn tipo_di(scans: &[Scan], v: &[Canale], facts: &Facts, nome: &str) -> Tipo {
    let srm = v.iter().any(|c| c.famiglia == Famiglia::Srm);
    let ms: Vec<&Canale> = v.iter().filter(|c| matches!(c.famiglia, Famiglia::Ms | Famiglia::Dd)).collect();
    let t = |proposto, motivo| Tipo { proposto, motivo };
    if scans.is_empty() {
        return match (srm, facts.pda) {
            (true, _) => t("lc-mrm", "srm_only"),
            (false, true) => t("solo-uv", "uv_only"),
            _ => t("misto", "empty"),
        };
    }
    if srm {
        return t("misto", "srm_and_scans");
    }
    if crate::infusione::con_msn(scans, facts.lc, nome) {
        let motivo = match crate::infusione::infusione(scans, facts.lc, nome).map(|i| i.motivo) {
            Some("blocchi") => "blocks",
            Some("metodo_senza_lc") => "method_without_lc",
            _ => "name",
        };
        return t("infusione-msn", motivo);
    }
    if ms.iter().any(|c| c.famiglia == Famiglia::Dd) {
        return t("lc-dda", "many_precursors");
    }
    if ms.iter().any(|c| c.livello >= 2) {
        return t("lc-ms2-mirato", "few_precursors");
    }
    t("lc-full", "ms1_only")
}

#[cfg(test)]
mod tests;
