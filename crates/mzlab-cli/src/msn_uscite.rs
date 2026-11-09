//! The files of a figure of the graph `msngrafo/1`: SVG, stand-alone HTML and PNG, by name (`svg,html,png`), written next to a stem; the command line
//! (`msn-graph --figura`, `msn-graph figure`) and the step `msn.figure@1` both go through here, so they write the same bytes.

use std::path::{Path, PathBuf};

use mzlab_core::error::{Error, Result};

use mzlab_feat::msnfigura::Vista;

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
pub fn bytes(graph: &str, f: Format, view: Vista) -> Result<Vec<u8>> {
    match f {
        Format::Svg => Ok(mzlab_feat::msnfigura::svg(graph, view)?.into_bytes()),
        Format::Html => Ok(mzlab_feat::msnfigura::html::html(graph, view)?.into_bytes()),
        Format::Png => crate::msn_png::png(&mzlab_feat::msnfigura::svg(graph, view)?),
    }
}

/// Writes the figures next to `stem` (`stem.svg`, `stem.html`, `stem.png`); returns the files.
pub fn scrivi(graph: &str, fs: &[Format], view: Vista, stem: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    for &f in fs {
        let p = PathBuf::from(format!("{}.{}", stem.display(), f.extension()));
        std::fs::write(&p, bytes(graph, f, view)?)
            .map_err(|e| Error::new("err.file.index").with("detail", e))?;
        out.push(p);
    }
    Ok(out)
}

/// Old flag names of `msn-graph figure` (old, new).
const ALIASES: &[(&str, &str)] = &[("--figura", "--figure"), ("--vista", "--view")];

/// The current name of the flag `x`: `aliases` pairs (old, new); any other text is returned as it is.
pub fn canonical(aliases: &[(&str, &str)], x: String) -> String {
    match aliases.iter().find(|(old, _)| *old == x) {
        Some((_, new)) => (*new).to_string(),
        None => x,
    }
}

/// `msn-graph figure IN.json [--figure svg,html,png] [--view tree|radial] [--out STEM]`: the figures of a graph already made; the stem defaults to the file
/// without its extension. Returns the message for the standard output.
pub fn figure_cmd(mut a: impl Iterator<Item = String>) -> std::result::Result<String, String> {
    let (mut file, mut out, mut fs, mut view) =
        (None, None, vec![Format::Svg, Format::Html], Vista::Albero);
    while let Some(x) = a.next() {
        let x = canonical(ALIASES, x);
        match x.as_str() {
            "--out" => out = a.next().map(PathBuf::from),
            "--figure" => fs = formats(&a.next().unwrap_or_default())?,
            "--view" => {
                view = Vista::da_nome(&a.next().unwrap_or_default())
                    .ok_or("--view needs tree or radial")?
            }
            f if !f.starts_with("--") && file.is_none() => file = Some(PathBuf::from(f)),
            f => return Err(format!("unknown argument {f}")),
        }
    }
    let file = file.ok_or("figure needs an msngrafo/1 file")?;
    let json = std::fs::read_to_string(&file).map_err(|e| format!("{}: {e}", file.display()))?;
    let stem = out.unwrap_or_else(|| file.with_extension(""));
    let made = scrivi(&json, &fs, view, &stem).map_err(|e| e.to_string())?;
    Ok(made.iter().map(|p| format!("{}\n", p.display())).collect())
}
