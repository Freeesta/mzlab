//! Grouping of the scans into channels: ids, labels, polarity split, DDA merge, ranking and the counting-sort partition.
//! Used only by `Canali::da`.

use std::collections::HashMap;

use super::MAX_CHIAVI;
use crate::idxread::Scan;

/// A group of scans while the channels are being counted.
pub(super) struct Gruppo {
    pub(super) id: String,
    pub(super) level: u8,
    pub(super) n: u32,
    pub(super) rt: (f64, f64),
    pub(super) pol: i8,
    /// Nominal masses of the path (the precursors, one per stage).
    pub(super) percorso: Vec<i64>,
    pub(super) sigla: Option<&'static str>,
    pub(super) etichetta: String,
    pub(super) omologo: String,
}

pub(super) fn nominale(m: f64) -> i64 {
    m.round_ties_even() as i64
}

/// `MS²`, `MS³`, ...: the level as a superscript.
pub(super) fn apice(level: u8) -> String {
    const SUP: [char; 10] = ['⁰', '¹', '²', '³', '⁴', '⁵', '⁶', '⁷', '⁸', '⁹'];
    let cifre: String = level
        .to_string()
        .chars()
        .filter_map(|c| c.to_digit(10))
        .map(|d| SUP[d as usize])
        .collect();
    format!("MS{cifre}")
}

/// Nominal masses of the path of an MSn scan: the stages of the filter, else the precursor alone.
pub(super) fn percorso_di(s: &Scan) -> Vec<i64> {
    match s.path.as_deref() {
        Some(p) if !p.is_empty() => p.iter().map(|x| nominale(x.0)).collect(),
        _ => {
            let p = s.prec(f64::NAN);
            if p.is_nan() {
                Vec::new()
            } else {
                vec![nominale(p)]
            }
        }
    }
}

/// Activation (lower case) and collision energy of the last stage of an MSn scan.
pub(super) fn attivazione_di(s: &Scan) -> (String, Option<f64>) {
    let last = s.path.as_ref().and_then(|p| p.last());
    let act = s
        .act
        .clone()
        .filter(|a| !a.is_empty())
        .or_else(|| last.map(|l| l.1.clone()))
        .unwrap_or_default()
        .to_lowercase();
    let ce = last
        .map(|l| l.2)
        .or(s.collision_energy)
        .filter(|c| c.is_finite() && *c > 0.0);
    (act, ce)
}

/// The id of the channel a scan belongs to. For an MS2 scan the key of the scan does not name the precursor, the channel does.
pub(super) fn id_ms(s: &Scan, percorso: &[i64]) -> String {
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

pub(super) fn sigla_di(s: &Scan) -> Option<&'static str> {
    if s.level <= 1 && s.filter.to_ascii_lowercase().contains(" sim ") {
        return Some("SIM");
    }
    s.kind_name()
}

pub(super) fn pol_label(pol: i8) -> &'static str {
    match pol {
        1 => "+",
        -1 => "-",
        _ => "",
    }
}

/// The group a first scan opens.
pub(super) fn nuovo_gruppo(id: String, s: &Scan, percorso: Vec<i64>) -> Gruppo {
    let (act, ce) = attivazione_di(s);
    let ce_int = ce.map(|c| c.round() as i64);
    let sigla = sigla_di(s);
    let masse = percorso
        .iter()
        .map(i64::to_string)
        .collect::<Vec<_>>()
        .join(">");
    let mut etichetta = apice(s.level.max(1));
    if s.level >= 2 && !percorso.is_empty() {
        etichetta += &format!(
            " {}",
            percorso
                .iter()
                .map(i64::to_string)
                .collect::<Vec<_>>()
                .join("→")
        );
    }
    if let Some(sg) = sigla.filter(|_| s.level <= 1) {
        etichetta += &format!(" {sg}");
    }
    if s.level >= 2 && !act.is_empty() {
        etichetta += &format!(
            " @{}{}",
            act.to_uppercase(),
            ce_int.map(|c| c.to_string()).unwrap_or_default()
        );
    }
    let omologo = format!(
        "ms|{}|{masse}|{act}|{}|{}|{}",
        s.level,
        ce_int.map(|c| c.to_string()).unwrap_or_default(),
        sigla.unwrap_or(""),
        pol_label(s.polarity)
    );
    Gruppo {
        id,
        level: s.level,
        n: 0,
        rt: (f64::INFINITY, f64::NEG_INFINITY),
        pol: s.polarity,
        percorso,
        sigla,
        etichetta,
        omologo,
    }
}

/// Groups the scans by channel id and polarity; `di[i]` is the group of scan `i`.
pub(super) fn raggruppa(scans: &[Scan]) -> (Vec<Gruppo>, Vec<u32>) {
    let mut gruppi: Vec<Gruppo> = Vec::new();
    let mut per_id: HashMap<String, u32> = HashMap::new();
    let mut di: Vec<u32> = Vec::with_capacity(scans.len());
    for s in scans {
        let percorso = if s.level >= 2 {
            percorso_di(s)
        } else {
            Vec::new()
        };
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
    (gruppi, di)
}

/// The same key in both polarities is two channels: the sign tells them apart.
pub(super) fn distingui_polarita(gruppi: &mut [Gruppo]) {
    let mut doppi: HashMap<String, u32> = HashMap::new();
    for g in gruppi.iter() {
        *doppi.entry(g.id.clone()).or_default() += 1;
    }
    for g in gruppi.iter_mut() {
        if doppi[&g.id] > 1 {
            g.id += if g.pol < 0 { "#-" } else { "#+" };
        }
    }
}

/// Too many keys at a level: one aggregate (DDA). Returns the final groups and `verso[k]`, the final group of group `k`.
pub(super) fn fondi_dd(gruppi: Vec<Gruppo>) -> (Vec<Gruppo>, Vec<u32>) {
    let mut chiavi: HashMap<u8, usize> = HashMap::new();
    for g in gruppi.iter().filter(|g| g.level >= 2) {
        *chiavi.entry(g.level).or_default() += 1;
    }
    let mut dd_livelli: Vec<u8> = chiavi
        .iter()
        .filter(|(_, n)| **n > MAX_CHIAVI)
        .map(|(l, _)| *l)
        .collect();
    crate::sortc::slice_ord(&mut dd_livelli);
    let mut finali: Vec<Gruppo> = Vec::new();
    let mut verso: Vec<u32> = vec![0; gruppi.len()];
    let mut dd_pos: HashMap<u8, u32> = HashMap::new();
    for (k, g) in gruppi.into_iter().enumerate() {
        if !dd_livelli.contains(&g.level) {
            finali.push(g);
            verso[k] = (finali.len() - 1) as u32;
            continue;
        }
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
    }
    (finali, verso)
}

/// Order: level, then the most populated, then the id. Returns `rango` (groups in order) and `pos_di[g]`, the rank of group `g`.
pub(super) fn ordina(finali: &[Gruppo]) -> (Vec<u32>, Vec<u32>) {
    let mut rango: Vec<u32> = (0..finali.len() as u32).collect();
    crate::sortc::slice_by(&mut rango, |a, b| {
        let (x, y) = (&finali[*a as usize], &finali[*b as usize]);
        x.level
            .cmp(&y.level)
            .then(y.n.cmp(&x.n))
            .then_with(|| x.id.cmp(&y.id))
    });
    let mut pos_di = vec![0u32; finali.len()];
    for (r, g) in rango.iter().enumerate() {
        pos_di[*g as usize] = r as u32;
    }
    (rango, pos_di)
}

/// The partition of the scans: a counting sort by rank, file order inside each channel. Returns the slice starts and the scan indices.
pub(super) fn partiziona(
    n_scan: usize,
    finali: &[Gruppo],
    rango: &[u32],
    pos_di: &[u32],
    di: &[u32],
    verso: &[u32],
) -> (Vec<u32>, Vec<u32>) {
    let mut inizio = vec![0u32; finali.len() + 1];
    for (r, g) in rango.iter().enumerate() {
        inizio[r + 1] = inizio[r] + finali[*g as usize].n;
    }
    let mut riempiti = inizio.clone();
    let mut scan_ids = vec![0u32; n_scan];
    for (i, g) in di.iter().enumerate() {
        let r = pos_di[verso[*g as usize] as usize] as usize;
        scan_ids[riempiti[r] as usize] = i as u32;
        riempiti[r] += 1;
    }
    (inizio, scan_ids)
}
