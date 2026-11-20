//! `mol draw --format ket|cdxml`: the drawn structure written as a KET (JSON) or a CDXML (XML) text. Both are open formats, written here from
//! their public documentation (the KET JSON schema, the CDX SDK object pages) and not tested against ChemDraw or Ketcher. One molecule with
//! every atom and bond, wedges from the stereo of the record, the title as a plain text label. Deterministic: integer-arithmetic numbers, no
//! clock, so the bytes are equal in the app, the CLI and the molkit `.wasm`.

use super::draw::{coord, Drawn};
use mzlab_core::error::{Error, Result};
use mzlab_core::json::Out;
use mzlab_mol::{allowed_valences, fixed, Mol};

/// Angstrom of one bond length of the layout (the scale of KET, and of CDXML before its points).
const BOND_A: f64 = 1.5;
/// Points of one bond length in a written CDXML.
const BOND_PT: f64 = 30.0;

/// The molecule's atoms as `(x, y)` in bond lengths and the extent `(x_min, y_min, y_max)` that places the CDXML page.
fn positions(m: &Mol) -> (Vec<(f64, f64)>, (f64, f64, f64)) {
    let at = |i: usize, k: usize| {
        m.coords
            .as_ref()
            .and_then(|c| c.xy.get(i))
            .and_then(|r| r.get(k))
            .copied()
            .unwrap_or(0.0)
            / BOND_A
            + 0.0
    };
    let p: Vec<(f64, f64)> = (0..m.atoms.len()).map(|i| (at(i, 0), at(i, 1))).collect();
    let fold = |f: fn(f64, f64) -> f64, k: usize, init: f64| {
        p.iter()
            .map(|q| if k == 0 { q.0 } else { q.1 })
            .fold(init, f)
    };
    if p.is_empty() {
        return (p, (0.0, 0.0, 0.0));
    }
    let ext = (
        fold(f64::min, 0, f64::MAX),
        fold(f64::min, 1, f64::MAX),
        fold(f64::max, 1, f64::MIN),
    );
    (p, ext)
}

/// The hydrogens the molfile reader would add by itself (lowest allowed valence that fits, less unpaired electrons): a count that differs
/// from it is written, so the reader of the file gets the same atom back.
fn default_h(m: &Mol, sums: &[u32], i: usize) -> u32 {
    let a = &m.atoms[i];
    let fit = allowed_valences(&a.el, i32::from(a.chg))
        .into_iter()
        .find(|&v| v >= sums[i]);
    fit.map_or(0, |v| v - sums[i])
        .saturating_sub(u32::from(a.rad))
}

/// Bond order sum per atom.
fn sums(m: &Mol) -> Vec<u32> {
    let mut s = vec![0u32; m.atoms.len()];
    for b in &m.bonds {
        s[b.a as usize] += u32::from(b.o);
        s[b.b as usize] += u32::from(b.o);
    }
    s
}

/// The molfile codes of a radical, which KET and CDXML share: 2 one unpaired electron, 3 two.
fn radical_code(rad: u8) -> i64 {
    if rad == 0 {
        0
    } else {
        i64::from(rad.min(2)) + 1
    }
}

/// The CDXML name of a radical (an enumeration there: the numbers belong to binary CDX).
fn radical_name(rad: u8) -> &'static str {
    match rad {
        0 => "",
        1 => "Doublet",
        _ => "Triplet",
    }
}

/// The atomic number of every atom. An atom that is no element is refused in both formats (KET could carry its label, CDXML cannot), so
/// the two agree and an unknown label is never written as carbon.
fn numbers(m: &Mol) -> Result<Vec<u16>> {
    (m.atoms.iter())
        .map(|a| {
            mzlab_mol::atomic_number(&a.el)
                .ok_or_else(|| Error::new("err.mol.draw.element").with("symbol", a.el.as_str()))
        })
        .collect()
}

/// The text label of a drawing: its title, a plain text line.
fn label_of(d: &Drawn) -> Option<&str> {
    let t = d.title.lines().next().unwrap_or("").trim();
    (!t.is_empty()).then_some(t)
}

/// KET: `{"root":{"nodes":[{"$ref":"mol0"},...]},"mol0":{"type":"molecule","atoms":[...],"bonds":[...]}}`, bond length 1, y up. A wedge
/// starts at its narrow end (`stereo` 1 up, 6 down, 4 or 3 either); the title is one text node under the structure.
pub fn ket(d: &Drawn) -> Result<String> {
    let m = &d.mol;
    numbers(m)?;
    let (p, (x0, y0, _)) = positions(m);
    let (sum, mut o) = (sums(m), Out::default());
    let num = |o: &mut Out, key: &str, v: i64| {
        o.raw(",").str(key).raw(":").int(v);
    };
    o.raw("{\"root\":{\"nodes\":[");
    if !m.atoms.is_empty() {
        o.raw("{\"$ref\":\"mol0\"}");
    }
    if let Some(t) = label_of(d) {
        o.raw(if m.atoms.is_empty() { "" } else { "," });
        o.raw("{\"type\":\"text\",\"data\":{\"content\":");
        let mut b = Out::default();
        b.raw("{\"blocks\":[{\"text\":").str(t).raw(",\"type\":\"unstyled\",\"depth\":0,\"inlineStyleRanges\":[],\"entityRanges\":[],\"data\":{}}],\"entityMap\":{}}");
        o.str(&b.0).raw(",\"position\":{\"x\":");
        o.raw(&coord(x0)).raw(",\"y\":").raw(&coord(y0 - 1.0));
        o.raw(",\"z\":0}}}");
    }
    o.raw("]}");
    if !m.atoms.is_empty() {
        o.raw(",\"mol0\":{\"type\":\"molecule\",\"atoms\":[");
        for (i, a) in m.atoms.iter().enumerate() {
            o.raw(if i > 0 { ",{\"label\":" } else { "{\"label\":" })
                .str(&a.el);
            // the alias is KET's own place for a shown text that is not the element (the molfile's alias line)
            if let Some(l) = d.label_at(i) {
                o.raw(",\"alias\":").str(l);
            }
            for (key, v) in [
                ("charge", i64::from(a.chg)),
                ("isotope", i64::from(a.iso)),
                ("radical", radical_code(a.rad)),
            ] {
                if v != 0 {
                    num(&mut o, key, v);
                }
            }
            let h = u32::from(a.h);
            if h != default_h(m, &sum, i) {
                num(&mut o, "implicitHCount", i64::from(h));
            }
            o.raw(",\"location\":[")
                .raw(&coord(p[i].0))
                .raw(",")
                .raw(&coord(p[i].1))
                .raw(",0]}");
        }
        o.raw("],\"bonds\":[");
        for (i, (a, b, order, wedge)) in m.written_bonds().into_iter().enumerate() {
            o.raw(if i > 0 { "," } else { "" });
            o.raw("{\"type\":").int(i64::from(order));
            o.raw(",\"atoms\":[").int(i64::from(a));
            o.raw(",").int(i64::from(b)).raw("]");
            match (wedge, order) {
                (1, _) => o.raw(",\"stereo\":1"),
                (-1, _) => o.raw(",\"stereo\":6"),
                (2, 1) => o.raw(",\"stereo\":4"),
                (2, _) => o.raw(",\"stereo\":3"),
                _ => &mut o,
            };
            o.raw("}");
        }
        o.raw("]}");
    }
    o.raw("}\n");
    Ok(o.0)
}

/// The text of an XML attribute or element: the five markup characters escaped, control characters and U+FFFE/U+FFFF (XML 1.0 forbids them) dropped.
fn xml_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            c if c.is_control() || c == '\u{fffe}' || c == '\u{ffff}' => {}
            c => out.push(c),
        }
    }
    out
}

/// One run of text in the default style (font 1, 10 pt).
fn run(s: &str) -> String {
    format!("<s font=\"1\" size=\"10\" face=\"0\">{}</s>", xml_text(s))
}

/// CDXML: one `fragment` with a node per atom (`Element`, `NumHydrogens`, `Charge`, `Isotope`, `Radical`; a label for every atom but a
/// bare carbon) and a bond per bond (`Order`; `Display` `WedgeBegin`, `WedgedHashBegin` or `Wavy` from the narrow end), in points, y
/// down, 30 to the bond, one bond of margin; the title is a text object under the structure.
pub fn cdxml(d: &Drawn) -> Result<String> {
    let m = &d.mol;
    let zs = numbers(m)?;
    let (p, (x0, _, y1)) = positions(m);
    let sum = sums(m);
    let pt = |x: f64, y: f64| {
        let (u, v) = ((x - x0 + 1.0) * BOND_PT, (y1 - y + 1.0) * BOND_PT);
        format!("{} {}", fixed(u + 0.0, 2), fixed(v + 0.0, 2))
    };
    let mut out = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\" ?>\n<!DOCTYPE CDXML SYSTEM \"http://www.camsoft.com/xml/cdxml.dtd\">\n<CDXML CreationProgram=\"mzLab\" BondLength=\"30\">\n<fonttable><font id=\"1\" charset=\"iso-8859-1\" name=\"Arial\"/></fonttable>\n<page id=\"2\">\n");
    if !m.atoms.is_empty() {
        out += "<fragment id=\"3\">\n";
    }
    // font 1, page 2, fragment 3: atom `i` is object `4 + i`, and the bonds and the text follow, so no id is used twice
    for (i, a) in m.atoms.iter().enumerate() {
        out += &format!("<n id=\"{}\" p=\"{}\"", 4 + i, pt(p[i].0, p[i].1));
        let z = zs[i];
        let h = u32::from(a.h);
        if z != 6 {
            out += &format!(" Element=\"{z}\"");
        }
        if z != 6 || h != default_h(m, &sum, i) {
            out += &format!(" NumHydrogens=\"{h}\"");
        }
        for (name, v) in [("Charge", i64::from(a.chg)), ("Isotope", i64::from(a.iso))] {
            if v != 0 {
                out += &format!(" {name}=\"{v}\"");
            }
        }
        if a.rad > 0 {
            out += &format!(" Radical=\"{}\"", radical_name(a.rad));
        }
        let shown = d.label_at(i);
        if z == 6 && shown.is_none() {
            out += "/>\n";
        } else {
            let hs = match h {
                0 => String::new(),
                1 => "H".into(),
                n => format!("H{n}"),
            };
            let text = shown.map_or_else(|| format!("{}{hs}", a.el), str::to_string);
            out += &format!("><t>{}</t></n>\n", run(&text));
        }
    }
    let mut id = 4 + m.atoms.len();
    for (a, b, order, wedge) in m.written_bonds() {
        let more = match (wedge, order) {
            (1, _) => " Display=\"WedgeBegin\"".to_string(),
            (-1, _) => " Display=\"WedgedHashBegin\"".to_string(),
            (2, _) => " Display=\"Wavy\"".to_string(),
            (_, 1) => String::new(),
            (_, n) => format!(" Order=\"{n}\""),
        };
        out += &format!("<b id=\"{id}\" B=\"{}\" E=\"{}\"{more}/>\n", 4 + a, 4 + b);
        id += 1;
    }
    if !m.atoms.is_empty() {
        out += "</fragment>\n";
    }
    if let Some(t) = label_of(d) {
        let (x, y) = (x0, p.iter().map(|q| q.1).fold(f64::MAX, f64::min) - 1.0);
        out += &format!("<t id=\"{id}\" p=\"{}\">{}</t>\n", pt(x, y), run(t));
    }
    Ok(out + "</page>\n</CDXML>\n")
}

#[cfg(test)]
#[path = "draw_file_tests.rs"]
mod tests;
