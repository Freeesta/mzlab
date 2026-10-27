//! The channels of a file (`docs/agenti/canali-dati.md`): every file is a list of channels, each a trace (time → intensity) and, when it has them, a
//! slice of the scans. Built once from the scan table and the chromatogram headers; the tools ask "channel + file", never "kind".
//! Used by `canali::rotte` (`api/canali`, `canale=`) and `mzlab-cli canali`.

use crate::idxread::Scan;
use crate::iteminfo::Facts;
use crate::json::{fmt_g, round, Out};
use crate::mzmlhead::Chrom;
use gruppi::*;

mod gruppi;
mod rotte;
mod uscita;
mod wiff;
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
    /// Built by `canali_uv`.
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
    Scan {
        da: u32,
        a: u32,
    },
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

/// `tic` (and `bpc` when there are scans): from the scan table, else from the TIC chromatogram, else the sum of the MRM traces.
fn canali_tic(scans: &[Scan], chroms: &[Chrom], facts: &Facts) -> Vec<Canale> {
    let ha_scan = !scans.is_empty();
    let srm: Vec<&Chrom> = chroms.iter().filter(|c| c.kind == "srm").collect();
    if !ha_scan && srm.is_empty() {
        return Vec::new();
    }
    let (n, rt) = if ha_scan {
        (
            scans.len() as u32,
            scans
                .iter()
                .fold((f64::INFINITY, f64::NEG_INFINITY), |a, s| {
                    (a.0.min(s.rt), a.1.max(s.rt))
                }),
        )
    } else {
        (
            srm.iter()
                .map(|c| c.n.unwrap_or(0).max(0) as u32)
                .max()
                .unwrap_or(0),
            facts.srm_rt.unwrap_or((0.0, 0.0)),
        )
    };
    let sorgente = match (ha_scan, chroms.iter().find(|c| c.kind == "tic")) {
        (true, _) => Sorgente::Scan {
            da: 0,
            a: scans.len() as u32,
        },
        (false, Some(c)) => Sorgente::Cromatogramma(c.index as i64),
        (false, None) => Sorgente::Somma,
    };
    let tic = Canale {
        id: "tic".into(),
        famiglia: Famiglia::Tic,
        genitore: None,
        livello: 0,
        pol: 0,
        sigla: None,
        etichetta: "TIC".into(),
        omologo: "tic".into(),
        n,
        rt,
        spettri: false,
        sorgente,
    };
    let mut v = vec![tic];
    if ha_scan {
        v.push(Canale {
            id: "bpc".into(),
            famiglia: Famiglia::Bpc,
            genitore: Some(0),
            etichetta: "BPC".into(),
            omologo: "bpc".into(),
            ..v[0].clone()
        });
    }
    v
}

/// Parents of the MSn channels (`v[base + k]` is group `rango[k]`): the channel of the precursor, else the DDA aggregate of the level before, else the TIC.
fn collega_genitori(
    v: &mut [Canale],
    base: usize,
    rango: &[u32],
    finali: &[Gruppo],
    radice: Option<u32>,
) {
    for (k, g) in rango.iter().enumerate() {
        let f = &finali[*g as usize];
        v[base + k].genitore = radice;
        if f.level < 2 || f.id.starts_with("dd:") {
            continue;
        }
        let padre = rango
            .iter()
            .enumerate()
            .filter(|(_, h)| {
                finali[**h as usize].level == f.level - 1
                    && !finali[**h as usize].id.starts_with("dd:")
            })
            .filter(|(_, h)| {
                let p = &finali[**h as usize];
                p.percorso == f.percorso[..f.percorso.len().saturating_sub(1)]
                    && (p.pol == f.pol || p.pol == 0 || f.pol == 0)
            });
        let padre = padre
            .max_by_key(|&(j, h)| (finali[*h as usize].n, std::cmp::Reverse(j)))
            .map(|(j, _)| base + j)
            .or_else(|| {
                rango
                    .iter()
                    .position(|h| finali[*h as usize].id == format!("dd:{}", f.level - 1))
                    .map(|j| base + j)
            });
        if let Some(p) = padre {
            v[base + k].genitore = Some(p as u32);
        }
    }
}

/// The MRM transitions of the file, one channel each.
fn canali_srm(v: &mut Vec<Canale>, chroms: &[Chrom], facts: &Facts, radice: Option<u32>) {
    for c in chroms.iter().filter(|c| c.kind == "srm") {
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
        let omologo = format!(
            "srm|2|{:.1}>{:.1}||{}||",
            q1,
            q3,
            ce.map(|x| (x.round() as i64).to_string())
                .unwrap_or_default()
        );
        v.push(Canale {
            id,
            famiglia: Famiglia::Srm,
            genitore: radice,
            livello: 2,
            pol: 0,
            sigla: Some("MRM"),
            etichetta,
            omologo,
            n: c.n.unwrap_or(0).max(0) as u32,
            rt,
            spettri: false,
            sorgente: Sorgente::Cromatogramma(c.index as i64),
        });
    }
}

/// The SIM / SIC chromatograms of the file, one channel each.
fn canali_sim(v: &mut Vec<Canale>, chroms: &[Chrom], facts: &Facts, radice: Option<u32>) {
    for c in chroms.iter().filter(|c| {
        c.kind != "srm"
            && c.id
                .get(..3)
                .is_some_and(|p| p.eq_ignore_ascii_case("sic") || p.eq_ignore_ascii_case("sim"))
    }) {
        let rt = facts.srm_rt.unwrap_or((0.0, 0.0));
        v.push(Canale {
            id: format!("sim:{}", c.id),
            famiglia: Famiglia::Sim,
            genitore: radice,
            livello: 1,
            pol: 0,
            sigla: Some("SIM"),
            etichetta: format!("SIM {}", c.id),
            omologo: format!("sim|1|{}", c.id),
            n: c.n.unwrap_or(0).max(0) as u32,
            rt,
            spettri: false,
            sorgente: Sorgente::Cromatogramma(c.index as i64),
        });
    }
}

impl Canali {
    /// The channels of a file from its scan table and the headers of its chromatograms (`chroms` may be empty); `nome` is the file name (the infusion hint).
    pub fn da(scans: &[Scan], chroms: &[Chrom], facts: &Facts, nome: &str) -> Canali {
        let (mut gruppi, di) = raggruppa(scans);
        distingui_polarita(&mut gruppi);
        let (finali, verso) = fondi_dd(gruppi);
        let (rango, pos_di) = ordina(&finali);
        let (inizio, scan_ids) = partiziona(scans.len(), &finali, &rango, &pos_di, &di, &verso);
        drop(di);
        let mut v = canali_tic(scans, chroms, facts);
        let base = v.len();
        for g in &rango {
            let f = &finali[*g as usize];
            let r = pos_di[*g as usize] as usize;
            v.push(Canale {
                id: f.id.clone(),
                famiglia: if f.id.starts_with("dd:") {
                    Famiglia::Dd
                } else {
                    Famiglia::Ms
                },
                genitore: Some(0),
                livello: f.level,
                pol: f.pol,
                sigla: f.sigla,
                etichetta: f.etichetta.clone(),
                omologo: f.omologo.clone(),
                n: f.n,
                rt: f.rt,
                spettri: true,
                sorgente: Sorgente::Scan {
                    da: inizio[r],
                    a: inizio[r + 1],
                },
            });
        }
        let radice = if v.is_empty() { None } else { Some(0u32) };
        collega_genitori(&mut v, base, &rango, &finali, radice);
        canali_srm(&mut v, chroms, facts, radice);
        canali_sim(&mut v, chroms, facts, radice);
        crate::canali_uv::aggiungi(&mut v, chroms, facts);
        let tipo = tipo_di(scans, &v, facts, nome);
        Canali { v, scan_ids, tipo }
    }

    /// Index of the channel `id`.
    pub fn trova(&self, id: &str) -> Option<usize> {
        self.v.iter().position(|c| c.id == id)
    }
}

/// The proposed type of experiment from the structure of the channels (`infusione` decides the infusions).
fn tipo_di(scans: &[Scan], v: &[Canale], facts: &Facts, nome: &str) -> Tipo {
    let srm = v.iter().any(|c| c.famiglia == Famiglia::Srm);
    let ms: Vec<&Canale> = v
        .iter()
        .filter(|c| matches!(c.famiglia, Famiglia::Ms | Famiglia::Dd))
        .collect();
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
#[cfg(test)]
mod tests_rotte;
