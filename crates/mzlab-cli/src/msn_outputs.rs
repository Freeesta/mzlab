//! The files of a figure of the graph `msn-graph/1`: SVG, stand-alone HTML and PNG, by name (`svg,html,png`), written next to a stem; the command line
//! (`msn graph --figure`, `msn graph figure`) and the step `msn.figure@1` both go through here, so they write the same bytes.

use std::path::{Path, PathBuf};

use mzlab_core::error::{Error, Result};

use mzlab_feat::msnfigure::View;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Format {
    Svg,
    Html,
    Png,
}

impl Format {
    pub fn extension(self) -> &'static str {
        match self {
            Format::Svg => "svg",
            Format::Html => "html",
            Format::Png => "png",
        }
    }

    pub fn from_name(n: &str) -> Option<Format> {
        match n.trim() {
            "svg" => Some(Format::Svg),
            "html" => Some(Format::Html),
            "png" => Some(Format::Png),
            _ => None,
        }
    }
}

/// `svg,html,png` → the formats, in the order given, each once.
pub fn formats(list: &str) -> std::result::Result<Vec<Format>, String> {
    let mut out = Vec::new();
    for n in list.split(',') {
        let f = Format::from_name(n).ok_or(format!(
            "--figure needs svg, html or png, not \"{}\"",
            n.trim()
        ))?;
        if !out.contains(&f) {
            out.push(f);
        }
    }
    Ok(out)
}

/// The bytes of the figure of `graph` in the format `f`.
pub fn bytes(graph: &str, f: Format, view: View) -> Result<Vec<u8>> {
    match f {
        Format::Svg => Ok(mzlab_feat::msnfigure::svg(graph, view)?.into_bytes()),
        Format::Html => Ok(mzlab_feat::msnfigure::html::html(graph, view)?.into_bytes()),
        Format::Png => crate::msn_png::png(&mzlab_feat::msnfigure::svg(graph, view)?),
    }
}

/// Writes the figures next to `stem` (`stem.svg`, `stem.html`, `stem.png`); returns the files.
pub fn write_outputs(graph: &str, fs: &[Format], view: View, stem: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    for &f in fs {
        let p = PathBuf::from(format!("{}.{}", stem.display(), f.extension()));
        std::fs::write(&p, bytes(graph, f, view)?)
            .map_err(|e| Error::new("err.file.index").with("detail", e))?;
        out.push(p);
    }
    Ok(out)
}

/// The `msn-figures/1` document for the files made: what the commands print on stdout, so it stays one JSON document.
pub fn files_json(made: &[PathBuf]) -> String {
    let mut o = mzlab_core::json::Out::default();
    o.raw("{\"schema\":\"msn-figures/1\",\"files\":[");
    for (i, p) in made.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .str(&p.display().to_string());
    }
    o.raw("]}\n");
    o.0
}

/// `msn graph figure IN.json [--figure svg,html,png] [--view tree|radial] [--out STEM]`: the figures of a graph already made; the stem defaults to the file
/// without its extension. Returns the `msn-figures/1` document for the standard output.
pub fn figure_cmd(mut a: impl Iterator<Item = String>) -> std::result::Result<String, String> {
    let (mut file, mut out, mut fs, mut view) =
        (None, None, vec![Format::Svg, Format::Html], View::Tree);
    while let Some(x) = a.next() {
        match x.as_str() {
            "--out" => out = a.next().map(PathBuf::from),
            "--figure" => fs = formats(&a.next().unwrap_or_default())?,
            "--view" => {
                view = View::da_nome(&a.next().unwrap_or_default())
                    .ok_or("--view needs tree or radial")?
            }
            f if !f.starts_with("--") && file.is_none() => file = Some(PathBuf::from(f)),
            f => return Err(format!("unknown argument {f}")),
        }
    }
    let file = file.ok_or("figure needs an msn-graph/1 file")?;
    let json = std::fs::read_to_string(&file).map_err(|e| format!("{}: {e}", file.display()))?;
    let stem = out.unwrap_or_else(|| file.with_extension(""));
    let made = write_outputs(&json, &fs, view, &stem).map_err(|e| e.to_string())?;
    Ok(files_json(&made))
}
