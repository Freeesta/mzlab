// Ported from Attributio ecotox/ghs.py.
//! Aquatic hazard categories from toxicity values and H-statements (CLP Regulation (EC) 1272/2008, Annex I, Table 4.1.0; Acute 2 and 3 are UN GHS bands).
//! One place for the thresholds; used by the hazard record and by the registry checks.

pub const ACUTE: [&str; 3] = ["Acute 1", "Acute 2", "Acute 3"];
pub const CHRONIC: [&str; 4] = ["Chronic 1", "Chronic 2", "Chronic 3", "Chronic 4"];
pub const NOT_CLASSIFIED: &str = "Not classified";
pub const TAXA: [&str; 3] = ["fish", "crustacean", "algae"];

fn rank(cat: Option<&str>, order: &[&str]) -> usize {
    cat.and_then(|c| order.iter().position(|o| *o == c))
        .unwrap_or(order.len())
}

/// The most severe of several categories (`None` and «Not classified» are the least severe); the first one wins a tie.
pub fn most_severe(cats: &[Option<&str>], order: &[&str]) -> Option<String> {
    let mut best: Option<&str> = None;
    for c in cats.iter().flatten().filter(|c| !c.is_empty()) {
        if best.is_none() || rank(Some(c), order) < rank(best, order) {
            best = Some(c);
        }
    }
    best.map(str::to_string)
}

fn finite(v: Option<f64>) -> Option<f64> {
    v.filter(|x| x.is_finite())
}

/// GHS acute category of the lowest L(E)C50 (mg/L).
pub fn acute_from_value(lc50: Option<f64>) -> Option<&'static str> {
    let v = finite(lc50)?;
    Some(if v <= 1.0 {
        "Acute 1"
    } else if v <= 10.0 {
        "Acute 2"
    } else if v <= 100.0 {
        "Acute 3"
    } else {
        NOT_CLASSIFIED
    })
}

/// CLP chronic category from the lowest chronic NOEC/ECx (mg/L), adequate chronic data.
pub fn chronic_from_noec(noec: Option<f64>, rapidly_degradable: bool) -> Option<&'static str> {
    let v = finite(noec)?;
    let (c1, c2, c3) = if rapidly_degradable {
        (0.01, 0.1, 1.0)
    } else {
        (0.1, 1.0, f64::NEG_INFINITY)
    };
    Some(if v <= c1 {
        "Chronic 1"
    } else if v <= c2 {
        "Chronic 2"
    } else if v <= c3 {
        "Chronic 3"
    } else {
        NOT_CLASSIFIED
    })
}

/// CLP chronic category without adequate chronic data, from the acute L(E)C50: only if the substance is not rapidly degradable and/or BCF >= 500 (without
/// BCF: log Kow >= 4).
pub fn chronic_from_acute(
    lc50: Option<f64>,
    rapidly_degradable: bool,
    log_kow: Option<f64>,
    bcf: Option<f64>,
) -> Option<&'static str> {
    let v = finite(lc50)?;
    let bioacc = match (bcf, log_kow) {
        (Some(b), _) => b >= 500.0,
        (None, Some(k)) => k >= 4.0,
        _ => false,
    };
    if rapidly_degradable && !bioacc {
        return Some(NOT_CLASSIFIED);
    }
    Some(if v <= 1.0 {
        "Chronic 1"
    } else if v <= 10.0 {
        "Chronic 2"
    } else if v <= 100.0 {
        "Chronic 3"
    } else {
        NOT_CLASSIFIED
    })
}

/// Result of `classify`: the categories and which route decided the chronic one.
#[derive(Clone, Debug, PartialEq)]
pub struct Classified {
    pub acute: Option<String>,
    pub chronic: Option<String>,
    pub chronic_route: Option<String>,
}

/// Categories from per-taxon values (mg/L, missing taxa absent; `acute` and `chronic` keep their taxon names). The chronic route is «chronic data», «acute
/// data» or «both (acute data of …  more stringent)» (CLP 4.1.3.3).
pub fn classify(
    acute: &[(&str, f64)],
    chronic: &[(&str, f64)],
    rapidly_degradable: bool,
    log_kow: Option<f64>,
    bcf: Option<f64>,
) -> Classified {
    let min = |v: &[(&str, f64)]| v.iter().map(|(_, x)| *x).fold(f64::INFINITY, f64::min);
    let a_cat = (!acute.is_empty())
        .then(|| acute_from_value(Some(min(acute))))
        .flatten()
        .map(str::to_string);
    let (mut c_cat, mut route) = (None, None);
    if !chronic.is_empty() {
        let base = chronic_from_noec(Some(min(chronic)), rapidly_degradable);
        let mut cat = base.map(str::to_string);
        route = Some("chronic data".to_string());
        let missing: Vec<&str> = TAXA
            .iter()
            .copied()
            .filter(|t| !chronic.iter().any(|(k, _)| k == t) && acute.iter().any(|(k, _)| k == t))
            .collect();
        if !missing.is_empty() {
            let lowest = missing
                .iter()
                .map(|t| {
                    acute
                        .iter()
                        .find(|(k, _)| k == t)
                        .map_or(f64::INFINITY, |(_, x)| *x)
                })
                .fold(f64::INFINITY, f64::min);
            let alt = chronic_from_acute(Some(lowest), rapidly_degradable, log_kow, bcf);
            let mut order: Vec<&str> = CHRONIC.to_vec();
            order.push(NOT_CLASSIFIED);
            let new = most_severe(&[base, alt], &order);
            if new != cat {
                route = Some(format!(
                    "both (acute data of {} more stringent)",
                    missing.join(", ")
                ));
            }
            cat = new;
        }
        c_cat = cat;
    } else if !acute.is_empty() {
        c_cat = chronic_from_acute(Some(min(acute)), rapidly_degradable, log_kow, bcf)
            .map(str::to_string);
        route = Some("acute data".to_string());
    }
    Classified {
        acute: a_cat,
        chronic: c_cat,
        chronic_route: route,
    }
}

/// Categories stated by H-statements (H400-H413): the most severe acute and chronic ones.
pub fn from_h_codes(codes: &[&str]) -> (Option<String>, Option<String>) {
    let acute: Vec<Option<&str>> = codes
        .iter()
        .map(|c| match *c {
            "H400" => Some("Acute 1"),
            "H401" => Some("Acute 2"),
            "H402" => Some("Acute 3"),
            _ => None,
        })
        .collect();
    let chronic: Vec<Option<&str>> = codes
        .iter()
        .map(|c| match *c {
            "H410" => Some("Chronic 1"),
            "H411" => Some("Chronic 2"),
            "H412" => Some("Chronic 3"),
            "H413" => Some("Chronic 4"),
            _ => None,
        })
        .collect();
    (most_severe(&acute, &ACUTE), most_severe(&chronic, &CHRONIC))
}
