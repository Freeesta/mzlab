//! The files of a figure of the graph `msngrafo/1`: SVG, stand-alone HTML and PNG, by name (`svg,html,png`), written next to a stem; the command line
//! (`msn-graph --figura`, `msn-graph figure`) and the step `msn.figure@1` both go through here, so they write the same bytes.

use std::path::{Path, PathBuf};

use mzlab_core::error::{Error, Result};

use crate::msn_figura::Vista;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Formato {
    Svg,
    Html,
    Png,
}

impl Formato {
    pub fn estensione(self) -> &'static str {
        match self {
            Formato::Svg => "svg",
            Formato::Html => "html",
            Formato::Png => "png",
        }
    }

    pub fn da_nome(n: &str) -> Option<Formato> {
        match n.trim() {
            "svg" => Some(Formato::Svg),
            "html" => Some(Formato::Html),
            "png" => Some(Formato::Png),
            _ => None,
        }
    }
}

/// `svg,html,png` → the formats, in the order given, each once.
pub fn formati(lista: &str) -> std::result::Result<Vec<Formato>, String> {
    let mut out = Vec::new();
    for n in lista.split(',') {
        let f = Formato::da_nome(n).ok_or(format!(
            "--figura needs svg, html or png, not \"{}\"",
            n.trim()
        ))?;
        if !out.contains(&f) {
            out.push(f);
        }
    }
    Ok(out)
}

/// The bytes of the figure of `graph` in the format `f`.
pub fn bytes(graph: &str, f: Formato, vista: Vista) -> Result<Vec<u8>> {
    match f {
        Formato::Svg => Ok(crate::msn_figura::svg(graph, vista)?.into_bytes()),
        Formato::Html => Ok(crate::msn_html::html(graph, vista)?.into_bytes()),
        Formato::Png => crate::msn_png::png(&crate::msn_figura::svg(graph, vista)?),
    }
}

/// Writes the figures next to `stem` (`stem.svg`, `stem.html`, `stem.png`); returns the files.
pub fn scrivi(graph: &str, fs: &[Formato], vista: Vista, stem: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    for &f in fs {
        let p = PathBuf::from(format!("{}.{}", stem.display(), f.estensione()));
        std::fs::write(&p, bytes(graph, f, vista)?)
            .map_err(|e| Error::new("err.file.index").with("detail", e))?;
        out.push(p);
    }
    Ok(out)
}

/// `msn-graph figure IN.json [--figura svg,html,png] [--vista albero|radiale] [--out STEM]`: the figures of a graph already made; the stem defaults to the file
/// without its extension. Returns the message for the standard output.
pub fn figure_cmd(mut a: impl Iterator<Item = String>) -> std::result::Result<String, String> {
    let (mut file, mut out, mut fs, mut vista) =
        (None, None, vec![Formato::Svg, Formato::Html], Vista::Albero);
    while let Some(x) = a.next() {
        match x.as_str() {
            "--out" => out = a.next().map(PathBuf::from),
            "--figura" => fs = formati(&a.next().unwrap_or_default())?,
            "--vista" => {
                vista = Vista::da_nome(&a.next().unwrap_or_default())
                    .ok_or("--vista needs albero or radiale")?
            }
            f if !f.starts_with("--") && file.is_none() => file = Some(PathBuf::from(f)),
            f => return Err(format!("unknown argument {f}")),
        }
    }
    let file = file.ok_or("figure needs an msngrafo/1 file")?;
    let json = std::fs::read_to_string(&file).map_err(|e| format!("{}: {e}", file.display()))?;
    let stem = out.unwrap_or_else(|| file.with_extension(""));
    let made = scrivi(&json, &fs, vista, &stem).map_err(|e| e.to_string())?;
    Ok(made.iter().map(|p| format!("{}\n", p.display())).collect())
}
