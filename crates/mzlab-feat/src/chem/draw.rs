//! `draw/1`: a structure given as text (SMILES, molfile, a name, an InChIKey) drawn in 2D by the own layout of `mzlab-mol`, as a molfile,
//! as a scene of atoms, bonds and fragments in angstrom, or as a picture (SVG text or PNG bytes, `draw_svg`, `draw_png`). Names and keys resolve only through a `molecules/1` CSV given in the request; an
//! InChI is never parsed here. Every automatic choice goes to the `choices` of the scene. Same bytes in the molkit `.wasm`, the app
//! (`feat/grafo`, `kind: "draw"`) and `mzlab-cli mol draw`.

use mzlab_core::error::{Error, Result};
use mzlab_core::json::{self, Out, Value};
use mzlab_mol::Mol;

use super::draw_spec::classify;
pub use super::draw_spec::kind_of;
use super::{draw_build, draw_hl, draw_png, draw_svg};

/// Above this many atoms the drawing refuses (a drawing, not a library).
const MAX_ATOMS: usize = 2000;

/// What the answer is: the molfile text, the `draw/1` scene, a KET or CDXML text (`draw_file`), or a picture (SVG text, PNG bytes).
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Format {
    Molfile,
    Scene,
    Ket,
    Cdxml,
    Svg,
    Png,
}

impl Format {
    /// The name of the format in a request and on the command line.
    pub fn name(self) -> &'static str {
        match self {
            Format::Molfile => "molfile",
            Format::Scene => "scene",
            Format::Ket => "ket",
            Format::Cdxml => "cdxml",
            Format::Svg => "svg",
            Format::Png => "png",
        }
    }

    /// Every format, in the order of the usage line.
    pub const ALL: [Format; 6] = [
        Format::Molfile,
        Format::Scene,
        Format::Ket,
        Format::Cdxml,
        Format::Svg,
        Format::Png,
    ];
}

/// An automatic choice or a warning: a stable key and the value the choice took.
#[derive(Clone, PartialEq, Debug)]
pub struct Note {
    pub key: &'static str,
    pub value: String,
}

/// A drawn structure: its record (with the coordinates of the drawing), the title line and what was decided on the way.
pub struct Drawn {
    pub kind: &'static str,
    pub title: String,
    pub mol: Mol,
    pub choices: Vec<Note>,
    pub warnings: Vec<Note>,
    /// A KET or CDXML text to carry in the scene (`"file": "ket"|"cdxml"` of a scene request).
    pub file: Option<Format>,
}

struct Request<'a> {
    spec: &'a str,
    format: Format,
    file: Option<Format>,
    keep: bool,
    db: Option<&'a str>,
    name: Option<&'a str>,
    /// Picture settings: pixels per angstrom, CIP marks, highlight lists (as in the request), and the `draw-image/1` wrapper.
    scale: f64,
    cip: bool,
    highlight: Option<&'a Value>,
    wrapped: bool,
}

/// Pixels per angstrom of a picture by default (a bond is 36 pixels), and its limits.
const SCALE: (f64, f64, f64) = (24.0, 8.0, 80.0);

pub(super) fn note(key: &'static str, value: impl ToString) -> Note {
    Note {
        key,
        value: value.to_string(),
    }
}

/// A setting as it was given, for the message that refuses it: text as is, `true`/`false`, a number as written in the picture, else empty.
fn shown(x: &Value) -> String {
    match (x.str(), x.bool(), x.f64()) {
        (Some(t), ..) => t.to_string(),
        (_, Some(b), _) => b.to_string(),
        (.., Some(n)) => draw_svg::num(n),
        _ => String::new(),
    }
}

fn read_request(v: &Value) -> Result<Request<'_>> {
    let text = |k: &str| v.get(k).and_then(Value::str);
    let spec = text("spec").filter(|s| !s.trim().is_empty());
    let bad = || Error::new("err.mol.draw.request");
    // a setting with two values: true when it is the second
    let setting = |name: &'static str, ok: [&str; 2], default: &str| -> Result<bool> {
        let want = text(name).unwrap_or(default);
        match ok.iter().position(|o| *o == want) {
            Some(i) => Ok(i == 1),
            None => Err(Error::new("err.mol.draw.set")
                .with("name", name)
                .with("value", want)),
        }
    };
    let cip = match v.get("cip") {
        None => true,
        Some(x) => match (x.bool(), x.str()) {
            (Some(b), _) => b,
            (_, Some("on")) => true,
            (_, Some("off")) => false,
            _ => {
                return Err(Error::new("err.mol.draw.set")
                    .with("name", "cip")
                    .with("value", shown(x)))
            }
        },
    };
    let scale = match v.get("scale") {
        None => SCALE.0,
        Some(x) => {
            let n = x.f64().or_else(|| x.str().and_then(draw_hl::decimal));
            n.filter(|n| (SCALE.1..=SCALE.2).contains(n))
                .ok_or_else(|| {
                    Error::new("err.mol.draw.set")
                        .with("name", "scale")
                        .with("value", shown(x))
                })?
        }
    };
    let word = |name: &'static str, value: &str| {
        Format::ALL
            .into_iter()
            .find(|f| f.name() == value)
            .ok_or_else(|| {
                Error::new("err.mol.draw.set")
                    .with("name", name)
                    .with("value", value)
            })
    };
    let format = word("format", text("format").unwrap_or("molfile"))?;
    // a scene may carry the KET or CDXML text of the same drawing
    let file = match text("file").map(|f| (f, word("file", f).ok())) {
        None => None,
        Some((_, Some(f @ (Format::Ket | Format::Cdxml)))) if format == Format::Scene => Some(f),
        Some((f, _)) => {
            return Err(Error::new("err.mol.draw.set")
                .with("name", "file")
                .with("value", f))
        }
    };
    Ok(Request {
        spec: spec.ok_or_else(bad)?,
        format,
        file,
        keep: !setting("layout", ["keep", "new"], "keep")?,
        db: text("db"),
        name: text("name"),
        scale,
        cip,
        highlight: v.get("highlight"),
        wrapped: v.get("json").and_then(Value::bool) == Some(true),
    })
}

/// The answer to a request `{"spec", "format": "molfile"|"scene"|"ket"|"cdxml"|"svg"|"png", "file": "ket"|"cdxml" (scene only), "layout": "keep"|"new",
/// "db": CSV text, "name": title, "scale": pixels per angstrom, "cip": on|off, "highlight": [{"atoms", "bonds", "color"}], "json": true}` as text. A PNG is
/// bytes: `draw_bytes`, or `"json": true`.
pub fn draw_json(request: &str) -> Result<String> {
    let a = answer(request)?;
    String::from_utf8(a.bytes).map_err(|_| Error::new("err.mol.draw.binary"))
}

/// `draw_json` as bytes: a PNG is the file itself.
pub fn draw_bytes(request: &str) -> Result<Vec<u8>> {
    Ok(answer(request)?.bytes)
}

/// The bytes of an answer, the format they are in and the drawn structure (its `choices` and `warnings` for the caller).
pub struct Answer {
    pub bytes: Vec<u8>,
    pub format: Format,
    /// The bytes are the `draw-image/1` wrapper, not the bare picture.
    pub wrapped: bool,
    pub drawn: Drawn,
}

pub fn answer(request: &str) -> Result<Answer> {
    let v = parse(request)?;
    let (mut d, r) = prepare(&v)?;
    let bytes = match r.format {
        Format::Molfile => d.mol.to_molfile(&d.title).into_bytes(),
        Format::Scene => scene(&d)?.into_bytes(),
        Format::Ket => super::draw_file::ket(&d)?.into_bytes(),
        Format::Cdxml => super::draw_file::cdxml(&d)?.into_bytes(),
        Format::Svg | Format::Png => {
            let hl = r
                .highlight
                .map_or(Ok(Vec::new()), |h| draw_hl::highlights(h, &d.mol))?;
            let o = draw_svg::Options {
                scale: r.scale,
                cip: r.cip,
                hl,
            };
            let (fig, reduced) = draw_build::figure(&d, &o);
            if reduced {
                d.choices.push(note(
                    "scale_reduced",
                    format!(
                        "{} px per angstrom (the picture would pass 4096 pixels)",
                        draw_svg::num(fig.scale)
                    ),
                ));
            }
            let png = r.format == Format::Png;
            let body = if png {
                draw_png::png(&fig)
            } else {
                draw_svg::svg(&fig, &d.title).into_bytes()
            };
            if r.wrapped {
                draw_svg::wrapped(&d, &fig, png, &body).into_bytes()
            } else {
                body
            }
        }
    };
    Ok(Answer {
        bytes,
        format: r.format,
        wrapped: r.wrapped,
        drawn: d,
    })
}

/// The drawn structure of a request and the format asked for.
pub fn draw(request: &str) -> Result<(Drawn, Format)> {
    let v = parse(request)?;
    let (d, r) = prepare(&v)?;
    Ok((d, r.format))
}

fn parse(request: &str) -> Result<Value> {
    json::parse(request.as_bytes()).map_err(|_| Error::new("err.mol.draw.request"))
}

fn prepare(v: &Value) -> Result<(Drawn, Request<'_>)> {
    let r = read_request(v)?;
    let (kind, body) = classify(r.spec);
    let mut d = match kind {
        "smiles" => from_smiles(body)?,
        "molfile" => from_molfile(body, r.keep)?,
        _ => super::draw_db::from_db(kind, body, r.db)?,
    };
    too_big(&d.mol)?;
    if kind != "molfile" {
        lay_out(&mut d);
        d.choices.push(note("layout", "new"));
    }
    d.kind = kind;
    d.file = r.file;
    d.choices.insert(0, note("spec_kind", kind));
    if let Some(f) = r
        .file
        .or(Some(r.format).filter(|f| matches!(f, Format::Ket | Format::Cdxml)))
    {
        let scale = if f == Format::Ket {
            "bond length 1, y up"
        } else {
            "points, 30 per bond, y down"
        };
        d.choices.push(note("file_format", f.name()));
        d.choices.push(note("file_scale", scale));
        if let Some(n) = r.name {
            d.title = n.to_string();
        }
        let dropped = d
            .title
            .lines()
            .skip(1)
            .filter(|l| !l.trim().is_empty())
            .count();
        if dropped > 0 {
            d.choices.push(note(
                "file_title",
                format!("first line only, {dropped} more dropped"),
            ));
        }
    } else if let Some(n) = r.name {
        d.title = n.to_string();
    }
    Ok((d, r))
}

/// The size limit, checked before any layout work.
fn too_big(m: &Mol) -> Result<()> {
    if m.atoms.len() > MAX_ATOMS {
        return Err(Error::new("err.mol.draw.tooBig").with("max", MAX_ATOMS));
    }
    Ok(())
}

/// Lays the drawing out; when the work budget of the layout ran out the cleaning stopped early and the scene says so.
fn lay_out(d: &mut Drawn) {
    let (c, cut) = d.mol.layout_2d_checked();
    d.mol.coords = Some(c);
    if cut {
        d.choices
            .push(note("layout_budget", "cleaning stopped at the work limit"));
    }
}

fn blank(kind: &'static str, mol: Mol) -> Drawn {
    Drawn {
        kind,
        title: String::new(),
        mol,
        choices: Vec::new(),
        warnings: Vec::new(),
        file: None,
    }
}

pub(super) fn from_smiles(text: &str) -> Result<Drawn> {
    let m = Mol::from_smiles(text)
        .map_err(|e| Error::new("err.mol.draw.smiles").with("message", e.to_string()))?;
    Ok(blank("smiles", m))
}

/// Largest coordinate kept from a molfile, in angstrom.
const MAX_COORD: f64 = 1000.0;

/// A molfile: its coordinates are kept when it has 2D ones that are not all at the origin and `keep` is on; else it is laid out again.
fn from_molfile(text: &str, keep: bool) -> Result<Drawn> {
    let m = Mol::from_molfile(text)
        .map_err(|e| Error::new("err.mol.draw.molfile").with("message", e.to_string()))?;
    let mut d = blank(
        "molfile",
        Mol::new(Vec::new(), Vec::new(), Vec::new(), Vec::new()),
    );
    let version = if text.lines().nth(3).is_some_and(|l| l.contains("V3000")) {
        "V3000"
    } else {
        "V2000"
    };
    d.choices.push(note("molfile_version", version));
    d.title = text.lines().next().unwrap_or("").trim().to_string();
    let flat = m.coords.as_ref().is_some_and(|c| c.dim == 2);
    let spread = m
        .coords
        .as_ref()
        .is_some_and(|c| c.xy.iter().any(|r| r.iter().take(2).any(|v| *v != 0.0)));
    // a file can state any number: a coordinate that is not finite or is beyond MAX_COORD angstrom would make an unusable (or huge) picture
    let sane = m.coords.as_ref().is_none_or(|c| {
        c.xy.iter()
            .all(|r| r.iter().all(|v| v.is_finite() && v.abs() <= MAX_COORD))
    });
    let (why, relay) = match (keep, flat, spread) {
        (false, ..) => ("layout=new", true),
        _ if !sane => ("coordinates not finite or beyond 1000 angstrom", true),
        (_, false, true) => ("3D coordinates", true),
        (_, _, false) => ("all atoms at the origin", true),
        _ => ("2D coordinates of the file", false),
    };
    d.choices.push(note(
        "layout",
        format!("{} ({why})", if relay { "new" } else { "kept" }),
    ));
    d.mol = m;
    too_big(&d.mol)?;
    if relay {
        lay_out(&mut d);
    }
    Ok(d)
}

fn notes(o: &mut Out, key: &str, list: &[Note]) {
    o.raw(",").str(key).raw(":[");
    for (i, n) in list.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"key\":")
            .str(n.key)
            .raw(",\"value\":")
            .str(&n.value)
            .raw("}");
    }
    o.raw("]");
}

/// A coordinate already rounded to 4 decimals, written shortest (`1.299`, `0`) by integer arithmetic: the float formatter of `Out::num`
/// costs the `.wasm` about 14 KB.
pub(super) fn coord(v: f64) -> String {
    let s = mzlab_mol::fixed(v, 4);
    let t = s.trim_end_matches('0').trim_end_matches('.');
    // a tiny negative rounds to "-0"; the scene writes 0
    if t == "-0" {
        "0".into()
    } else {
        t.into()
    }
}

/// The `draw/1` scene: atoms with their position in angstrom, bonds as written in the molfile (a wedge belongs to its first atom), the
/// connected pieces and what was chosen on the way.
pub fn scene(d: &Drawn) -> Result<String> {
    let m = &d.mol;
    let xy = |i: usize, k: usize| {
        m.coords
            .as_ref()
            .and_then(|c| c.xy.get(i))
            .and_then(|r| r.get(k))
            .copied()
            .unwrap_or(0.0)
            + 0.0
    };
    let mut o = Out::default();
    o.raw("{\"schema\":\"draw/1\",\"spec_kind\":")
        .str(d.kind)
        .raw(",\"title\":")
        .str(&d.title)
        .raw(",\"atoms\":[");
    for (i, a) in m.atoms.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"i\":")
            .int(i as i64)
            .raw(",\"el\":")
            .str(&a.el);
        o.raw(",\"chg\":")
            .int(i64::from(a.chg))
            .raw(",\"iso\":")
            .int(i64::from(a.iso))
            .raw(",\"h\":")
            .int(i64::from(a.h));
        o.raw(",\"x_a\":")
            .raw(&coord(xy(i, 0)))
            .raw(",\"y_a\":")
            .raw(&coord(xy(i, 1)))
            .raw("}");
    }
    o.raw("],\"bonds\":[");
    for (i, (a, b, order, wedge)) in m.written_bonds().into_iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"a\":")
            .int(i64::from(a))
            .raw(",\"b\":")
            .int(i64::from(b));
        o.raw(",\"o\":")
            .int(i64::from(order))
            .raw(",\"wedge\":")
            .int(i64::from(wedge))
            .raw("}");
    }
    o.raw("],\"fragments\":[");
    for (i, f) in m.layout_pieces().iter().enumerate() {
        o.raw(if i > 0 { ",[" } else { "[" });
        for (k, a) in f.iter().enumerate() {
            o.raw(if k > 0 { "," } else { "" }).int(*a as i64);
        }
        o.raw("]");
    }
    o.raw("]");
    notes(&mut o, "choices", &d.choices);
    notes(&mut o, "warnings", &d.warnings);
    match d.file {
        Some(Format::Ket) => o
            .raw(",\"file_format\":\"ket\",\"file\":")
            .str(&super::draw_file::ket(d)?),
        Some(Format::Cdxml) => o
            .raw(",\"file_format\":\"cdxml\",\"file\":")
            .str(&super::draw_file::cdxml(d)?),
        _ => &mut o,
    };
    o.raw("}");
    Ok(o.0)
}

#[cfg(test)]
#[path = "draw_tests.rs"]
mod tests;
