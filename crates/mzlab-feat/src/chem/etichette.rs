//! `etich/1`: formula, charge, monoisotopic and nominal mass of the structures of a drawing, and what changes across a reaction arrow. The page sends
//! the atoms as Ketcher shows them (element, isotope, implicit H, charge), so the numbers are the ones of the canvas; isotopes count with their own
//! mass (`elements::nuclide`), every element with a known nuclide is accepted. Hill order with isotopes after their element: `C[13C]H3D`.

use mzlab_core::elements::{self, ELECTRON};
use mzlab_core::error::{Error, Result};
use mzlab_core::json::{self, round, Out, Value};
use mzlab_core::nuclidi::NUCLIDES;

/// One element-and-isotope of a formula: symbol (from the nuclide table) and mass number (0 = the element at natural abundance, written plain).
type Key = (&'static str, u16);
/// A few keys per structure: a plain list keeps the .wasm small (no map instantiated for these keys).
type Counts = Vec<(Key, u64)>;

fn add(n: &mut Counts, k: Key, c: u64) {
    match n.iter_mut().find(|(x, _)| *x == k) {
        Some((_, v)) => *v += c,
        None => n.push((k, c)),
    }
}

fn count(n: &Counts, k: &Key) -> u64 {
    n.iter().find(|(x, _)| x == k).map_or(0, |(_, c)| *c)
}

struct Shape {
    n: Counts,
    q: i64,
    mass: f64,
    nominal: u64,
}

/// The text of one key: `C`, `[13C]`, `D` and `T` for hydrogen 2 and 3.
fn sym(k: &Key) -> String {
    match (k.0, k.1) {
        (e, 0) => e.to_string(),
        ("H", 2) => "D".into(),
        ("H", 3) => "T".into(),
        (e, a) => format!("[{a}{e}]"),
    }
}

/// The order of `elements::fmt`: C (all isotopes), H (all isotopes), then the rest alphabetically; within an element the plain symbol first, then
/// the isotopes by mass number. H3N is written NH3 (what the students know).
pub fn hill(n: &Counts) -> String {
    let rank = |e: &str| match e {
        "C" => 0,
        "H" => 1,
        _ => 2,
    };
    let mut ks: Vec<&(Key, u64)> = n.iter().filter(|(_, c)| *c > 0).collect();
    ks.sort_unstable_by_key(|((e, a), _)| (rank(e), *e, *a));
    let s: String = ks
        .into_iter()
        .map(|(k, c)| match c {
            1 => sym(k),
            c => format!("{}{c}", sym(k)),
        })
        .collect();
    if s == "H3N" {
        "NH3".into()
    } else {
        s
    }
}

/// Formula with its charge, as the Data formula field reads it: `C3H3O4-`, `[C3H2O4]2-`.
pub fn with_charge(f: &str, q: i64) -> String {
    let sign = if q > 0 { "+" } else { "-" };
    match q.abs() {
        0 => f.to_string(),
        1 => format!("{f}{sign}"),
        a => format!("[{f}]{a}{sign}"),
    }
}

fn atom(v: &Value) -> Result<(Key, u64, i64)> {
    let el = v.get("el").and_then(Value::str).unwrap_or("");
    let iso = v.get("iso").and_then(Value::f64).unwrap_or(0.0);
    let bad = |k: &'static str| Error::new(k).with("label", el);
    if !(0.0..=400.0).contains(&iso) || iso.fract() != 0.0 {
        return Err(bad("error.etich.isotope"));
    }
    let given = (iso > 0.0).then_some(iso as u16);
    let (a, _) = elements::nuclide(el, given).ok_or_else(|| {
        bad(if elements::nuclide(el, None).is_some() {
            "error.etich.isotope"
        } else {
            "error.etich.element"
        })
    })?;
    let key = match el {
        "D" | "T" => ("H", a),
        _ => {
            let e = NUCLIDES.iter().find(|n| n.0 == el).map_or("", |n| n.0);
            (e, if given.is_some() { a } else { 0 })
        }
    };
    let h = v
        .get("h")
        .and_then(Value::f64)
        .unwrap_or(0.0)
        .clamp(0.0, 16.0) as u64;
    let q = v
        .get("q")
        .and_then(Value::f64)
        .unwrap_or(0.0)
        .clamp(-16.0, 16.0) as i64;
    Ok((key, h, q))
}

fn shape(atoms: &[Value]) -> Result<Shape> {
    let mut n = Counts::new();
    let mut q = 0;
    for v in atoms {
        let (k, h, c) = atom(v)?;
        add(&mut n, k, 1);
        if h > 0 {
            add(&mut n, ("H", 0), h);
        }
        q += c;
    }
    let (mut mass, mut nominal) = (Vec::new(), 0);
    for ((e, a), c) in &n {
        let (aa, m) = elements::nuclide(e, (*a > 0).then_some(*a)).unwrap_or((0, 0.0));
        mass.push(m * *c as f64);
        nominal += aa as u64 * c;
    }
    Ok(Shape {
        n,
        q,
        mass: mzlab_core::pysum::pysum(mass),
        nominal,
    })
}

fn shape_json(o: &mut Out, s: &Shape) {
    let f = hill(&s.n);
    o.raw("{\"formula\":")
        .str(&f)
        .raw(",\"text\":")
        .str(&with_charge(&f, s.q));
    o.raw(",\"charge\":")
        .int(s.q)
        .raw(",\"mass\":")
        .num(round(s.mass, 6));
    o.raw(",\"nominal\":").int(s.nominal as i64).raw(",\"mz\":");
    if s.q == 0 {
        o.raw("null");
    } else {
        o.num(round(
            (s.mass - s.q as f64 * ELECTRON) / s.q.abs() as f64,
            6,
        ));
    }
    o.raw("}");
}

/// Gain and loss across an arrow, `isotopologue` when the elements are the same and only isotopes moved, `dm` and `dnominal` after minus before.
fn arrow_json(o: &mut Out, a: &Shape, b: &Shape) {
    let (mut gain, mut loss) = (Counts::new(), Counts::new());
    for (k, _) in a.n.iter().chain(b.n.iter()) {
        if count(&gain, k) + count(&loss, k) > 0 {
            continue;
        }
        let d = count(&b.n, k) as i64 - count(&a.n, k) as i64;
        if d > 0 {
            gain.push((*k, d as u64));
        } else if d < 0 {
            loss.push((*k, (-d) as u64));
        }
    }
    // only isotopes moved when, element by element, what is gained is what is lost
    let total = |n: &Counts, e: &str| {
        n.iter()
            .filter(|((x, _), _)| *x == e)
            .map(|(_, c)| c)
            .sum::<u64>()
    };
    let iso = !gain.is_empty()
        && gain
            .iter()
            .chain(loss.iter())
            .all(|((e, _), _)| total(&gain, e) == total(&loss, e));
    o.raw("{\"gain\":")
        .str(&hill(&gain))
        .raw(",\"loss\":")
        .str(&hill(&loss));
    o.raw(",\"isotopologue\":")
        .raw(if iso { "true" } else { "false" });
    o.raw(",\"dm\":").num(round(b.mass - a.mass, 6));
    o.raw(",\"dnominal\":")
        .int(b.nominal as i64 - a.nominal as i64)
        .raw("}");
}

/// `etich/1` of a request `{"structures": [[{el, iso, h, q}, ...], ...], "arrows": [[before, after], ...]}`. A structure with an atom that is
/// no element (R group, atom list, `*`) or an isotope outside the table is `{"error": key, "label"}`; an arrow touching one is `null`.
pub fn labels_json(request: &str) -> Result<String> {
    let v = json::parse(request.as_bytes()).map_err(|_| Error::new("error.etich.request"))?;
    let shapes: Vec<Result<Shape>> = v
        .get("structures")
        .map_or(&[][..], Value::arr)
        .iter()
        .map(|s| shape(s.arr()))
        .collect();
    let mut o = Out::default();
    o.raw("{\"format\":\"etich/1\",\"structures\":[");
    for (i, s) in shapes.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" });
        match s {
            Ok(s) => shape_json(&mut o, s),
            Err(e) => {
                let label = e
                    .params
                    .iter()
                    .find(|(k, _)| *k == "label")
                    .map_or("", |(_, v)| v.as_str());
                o.raw("{\"error\":")
                    .str(e.key)
                    .raw(",\"label\":")
                    .str(label)
                    .raw("}");
            }
        }
    }
    o.raw("],\"arrows\":[");
    for (i, a) in v
        .get("arrows")
        .map_or(&[][..], Value::arr)
        .iter()
        .enumerate()
    {
        o.raw(if i > 0 { "," } else { "" });
        let at = |j: usize| {
            a.at(j)
                .and_then(Value::f64)
                .and_then(|x| shapes.get(x as usize))
                .and_then(|s| s.as_ref().ok())
        };
        match (at(0), at(1)) {
            (Some(x), Some(y)) => arrow_json(&mut o, x, y),
            _ => {
                o.raw("null");
            }
        }
    }
    o.raw("]}");
    Ok(o.0)
}
