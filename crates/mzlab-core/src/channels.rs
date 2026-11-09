//! The channels of a file (`docs/agenti/canali-dati.md`): every file is a list of channels, each a trace (time → intensity) and, when it has them, a
//! slice of the scans. Built once from the scan table and the chromatogram headers; the tools ask "channel + file", never "kind".
//! Used by `channels::routes` (`api/canali`, `canale=`) and `mzlab-cli canali`.

use crate::idxread::Scan;
use crate::iteminfo::Facts;
use crate::json::{fmt_g, round, Out};
use crate::mzmlhead::Chrom;
use groups::*;

mod groups;
mod output;
mod routes;
mod wiff;
pub(crate) use routes::State;

/// More distinct MSn keys than this at one level: one `dd:<level>` channel (the same cap as `infusione`).
pub const MAX_KEYS: usize = 64;

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
    pub fn name(self) -> &'static str {
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
pub enum Source {
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
pub struct Channel {
    pub id: String,
    pub famiglia: Famiglia,
    /// Index of the parent channel (`tic`, or the channel of the precursor).
    pub parent: Option<u32>,
    /// 0 for UV and analog channels.
    pub livello: u8,
    /// +1, -1 or 0 (both, or none).
    pub pol: i8,
    pub sigla: Option<&'static str>,
    pub label: String,
    /// Same value in two files = the same channel (level, nominal masses of the path, activation, collision energy rounded; q1/q3 to 0.1).
    pub omologo: String,
    pub n: u32,
    pub rt: (f64, f64),
    /// The channel has scans with a spectrum (`ms`, `dd`, `uv`).
    pub spettri: bool,
    pub source: Source,
}

/// A proposal for the default tools, never a verdict: the user's own value (CANALI-F) wins.
#[derive(Debug, Clone, PartialEq)]
pub struct Tipo {
    /// `lc-full` | `lc-ms2-mirato` | `lc-dda` | `lc-mrm` | `infusione-msn` | `misto` | `solo-uv`
    pub proposed: &'static str,
    pub reason: &'static str,
}

#[derive(Debug, Clone)]
pub struct Channels {
    pub v: Vec<Channel>,
    /// The indices of the scans (in `idx.scans`) partitioned by leaf channel, file order inside each channel: 4 B per scan.
    pub scan_ids: Vec<u32>,
    pub tipo: Tipo,
}

/// `tic` (and `bpc` when there are scans): from the scan table, else from the TIC chromatogram, else the sum of the MRM traces.
fn tic_channels(scans: &[Scan], chroms: &[Chrom], facts: &Facts) -> Vec<Channel> {
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
        (true, _) => Source::Scan {
            da: 0,
            a: scans.len() as u32,
        },
        (false, Some(c)) => Source::Cromatogramma(c.index as i64),
        (false, None) => Source::Somma,
    };
    let tic = Channel {
        id: "tic".into(),
        famiglia: Famiglia::Tic,
        parent: None,
        livello: 0,
        pol: 0,
        sigla: None,
        label: "TIC".into(),
        omologo: "tic".into(),
        n,
        rt,
        spettri: false,
        source: sorgente,
    };
    let mut v = vec![tic];
    if ha_scan {
        v.push(Channel {
            id: "bpc".into(),
            famiglia: Famiglia::Bpc,
            parent: Some(0),
            label: "BPC".into(),
            omologo: "bpc".into(),
            ..v[0].clone()
        });
    }
    v
}

/// Parents of the MSn channels (`v[base + k]` is group `rango[k]`): the channel of the precursor, else the DDA aggregate of the level before, else the TIC.
fn link_parents(v: &mut [Channel], base: usize, rank: &[u32], finali: &[Group], root: Option<u32>) {
    for (k, g) in rank.iter().enumerate() {
        let f = &finali[*g as usize];
        v[base + k].parent = root;
        if f.level < 2 || f.id.starts_with("dd:") {
            continue;
        }
        let parent = rank
            .iter()
            .enumerate()
            .filter(|(_, h)| {
                finali[**h as usize].level == f.level - 1
                    && !finali[**h as usize].id.starts_with("dd:")
            })
            .filter(|(_, h)| {
                let p = &finali[**h as usize];
                p.path == f.path[..f.path.len().saturating_sub(1)]
                    && (p.pol == f.pol || p.pol == 0 || f.pol == 0)
            });
        let padre = parent
            .max_by_key(|&(j, h)| (finali[*h as usize].n, std::cmp::Reverse(j)))
            .map(|(j, _)| base + j)
            .or_else(|| {
                rank.iter()
                    .position(|h| finali[*h as usize].id == format!("dd:{}", f.level - 1))
                    .map(|j| base + j)
            });
        if let Some(p) = padre {
            v[base + k].parent = Some(p as u32);
        }
    }
}

/// The MRM transitions of the file, one channel each.
fn srm_channels(v: &mut Vec<Channel>, chroms: &[Chrom], facts: &Facts, root: Option<u32>) {
    for c in chroms.iter().filter(|c| c.kind == "srm") {
        if c.q3.is_none() && c.id.starts_with("SIM SIC ") {
            // the same ion in every sample: m/z and polarity, not the sample of the id
            let pol =
                c.id.split(' ')
                    .find_map(|w| w.strip_prefix("polarity="))
                    .unwrap_or("");
            // a reader that gives no Q1 leaves the ion told by its place in the experiment
            let ion = c.q1.map_or_else(
                || format!("#{}", id_field(&c.id, "transition=").unwrap_or("?")),
                |q| format!("{q:.1}"),
            );
            let omologo = format!("sim|1|{ion}|{pol}");
            v.push(sim_channel(c, facts, root, omologo));
            continue;
        }
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
        v.push(Channel {
            id,
            famiglia: Famiglia::Srm,
            parent: root,
            livello: 2,
            pol: 0,
            sigla: Some("MRM"),
            label: etichetta,
            omologo,
            n: c.n.unwrap_or(0).max(0) as u32,
            rt,
            spettri: false,
            source: Source::Cromatogramma(c.index as i64),
        });
    }
}

/// The value of `key=` among the space-separated words of a chromatogram id (none when empty).
fn id_field<'a>(id: &'a str, key: &str) -> Option<&'a str> {
    id.split(' ')
        .find_map(|w| w.strip_prefix(key))
        .filter(|v| !v.is_empty())
}

/// The label of a SIM channel: `SIM <m/z> (+|−) <dwell> ms «name»` for the msconvert ids (`SIM SIC Q1=… dwell=… polarity=… name=…`), `SIM <id>` otherwise.
fn sim_label(id: &str) -> String {
    let Some(rest) = id.strip_prefix("SIM SIC ") else {
        return format!("SIM {id}");
    };
    let field = |k: &str| id_field(id, k);
    let mut out = String::from("SIM");
    match field("Q1=") {
        Some(q1) => out += &format!(" {q1}"),
        None => match field("transition=").and_then(|t| t.parse::<u32>().ok()) {
            Some(t) => out += &format!(" #{}", t + 1),
            None => out += " #?",
        },
    }
    match field("polarity=") {
        Some("positive") => out += " (+)",
        Some("negative") => out += " (−)",
        _ => {}
    }
    if let Some(d) = field("dwell=") {
        out += &format!(" {d} ms");
    }
    if let Some(n) = rest
        .split_once(" name=")
        .map(|x| x.1)
        .filter(|n| !n.is_empty())
    {
        out += &format!(" «{n}»");
    }
    out
}

/// The SIM / SIC chromatograms of the file, one channel each.
fn sim_channels(v: &mut Vec<Channel>, chroms: &[Chrom], facts: &Facts, root: Option<u32>) {
    for c in chroms.iter().filter(|c| {
        c.kind != "srm"
            && c.id
                .get(..3)
                .is_some_and(|p| p.eq_ignore_ascii_case("sic") || p.eq_ignore_ascii_case("sim"))
    }) {
        v.push(sim_channel(c, facts, root, format!("sim|1|{}", c.id)));
    }
}

/// The channel of one SIM / SIC chromatogram.
fn sim_channel(c: &Chrom, facts: &Facts, root: Option<u32>, omologo: String) -> Channel {
    Channel {
        id: format!("sim:{}", c.id),
        famiglia: Famiglia::Sim,
        parent: root,
        livello: 1,
        pol: 0,
        sigla: Some("SIM"),
        label: sim_label(&c.id),
        omologo,
        n: c.n.unwrap_or(0).max(0) as u32,
        rt: facts.srm_rt.unwrap_or((0.0, 0.0)),
        spettri: false,
        source: Source::Cromatogramma(c.index as i64),
    }
}

impl Channels {
    /// The channels of a file from its scan table and the headers of its chromatograms (`chroms` may be empty); `nome` is the file name (the infusion hint).
    pub fn da(scans: &[Scan], chroms: &[Chrom], facts: &Facts, name: &str) -> Channels {
        let (mut groups, di) = group(scans);
        distingui_polarita(&mut groups);
        let (finali, direction) = fondi_dd(groups);
        let (rank, pos_di) = sort(&finali);
        let (start, scan_ids) = partiziona(scans.len(), &finali, &rank, &pos_di, &di, &direction);
        drop(di);
        let mut v = tic_channels(scans, chroms, facts);
        let base = v.len();
        for g in &rank {
            let f = &finali[*g as usize];
            let r = pos_di[*g as usize] as usize;
            v.push(Channel {
                id: f.id.clone(),
                famiglia: if f.id.starts_with("dd:") {
                    Famiglia::Dd
                } else {
                    Famiglia::Ms
                },
                parent: Some(0),
                livello: f.level,
                pol: f.pol,
                sigla: f.tag,
                label: f.etichetta.clone(),
                omologo: f.omologo.clone(),
                n: f.n,
                rt: f.rt,
                spettri: true,
                source: Source::Scan {
                    da: start[r],
                    a: start[r + 1],
                },
            });
        }
        let root = if v.is_empty() { None } else { Some(0u32) };
        link_parents(&mut v, base, &rank, &finali, root);
        srm_channels(&mut v, chroms, facts, root);
        sim_channels(&mut v, chroms, facts, root);
        crate::channels_uv::add(&mut v, chroms, facts);
        let tipo = tipo_di(scans, &v, facts, name);
        Channels { v, scan_ids, tipo }
    }

    /// Index of the channel `id`.
    pub fn find(&self, id: &str) -> Option<usize> {
        self.v.iter().position(|c| c.id == id)
    }
}

/// The proposed type of experiment from the structure of the channels (`infusione` decides the infusions).
fn tipo_di(scans: &[Scan], v: &[Channel], facts: &Facts, name: &str) -> Tipo {
    let srm = v.iter().any(|c| c.famiglia == Famiglia::Srm);
    let sim = v.iter().any(|c| c.famiglia == Famiglia::Sim);
    let ms: Vec<&Channel> = v
        .iter()
        .filter(|c| matches!(c.famiglia, Famiglia::Ms | Famiglia::Dd))
        .collect();
    let t = |proposto, motivo| Tipo {
        proposed: proposto,
        reason: motivo,
    };
    if scans.is_empty() {
        // SIM channels are MS data: a SIM file is never "UV only" or "empty"
        return match (srm, sim, facts.pda) {
            (true, _, _) => t("lc-mrm", "srm_only"),
            (false, true, _) => t("lc-sim", "sim_only"),
            (false, false, true) => t("solo-uv", "uv_only"),
            _ => t("misto", "empty"),
        };
    }
    if srm {
        return t("misto", "srm_and_scans");
    }
    if crate::infusion::with_msn(scans, facts.lc, name) {
        let motivo = match crate::infusion::infusione(scans, facts.lc, name).map(|i| i.reason) {
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
mod tests_routes;
