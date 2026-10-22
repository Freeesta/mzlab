//! The native executor of `msn.figure@1`: the figure of the fragmentation graph `msngrafo/1` of `spectra.msn_graph` as an SVG, a stand-alone HTML or a PNG,
//! the same bytes as `mzlab-cli msn-graph figure`. It is a step of its own, after the graph: the figure needs only the graph (the raw file is not read again),
//! and a Flow output holds one file, so each format is one run of the step (`--set figure.format=html`).

use mzlab_core::error::{Error, Result};
use mzlab_core::flow::{Artifact, Job};
use mzlab_core::json::Value;
use mzlab_core::step::{DataType, Param, Port, RunsIn, Step};

use crate::flow::{read_one, Exec, FileRunner};
use crate::msn_figura::Vista;
use crate::msn_uscite::{bytes, Formato};

/// The executor of the step, read by the runner after its own list.
pub(crate) const EXECUTORS: &[(&str, Exec)] = &[("msn.figure", Exec::File("figure", figure_job))];

/// `msn.figure@1`: one figure of the graph.
pub fn figure_step() -> Step {
    Step {
        ins: vec![Port::new("graph", DataType::Report, "Fragmentation graph (msngrafo/1)")],
        outs: vec![Port::new("figure", DataType::Report, "Figure of the graph (SVG, HTML or PNG)")],
        params: vec![
            Param::choice("format", "Format", &["svg", "html", "png"])
                .default_str("svg")
                .doc("svg: the static figure, colours written in the file; html: one file with no external resource, two views, data table and downloads; png: the SVG rasterised (only in a build with the cargo feature `png`)"),
            Param::choice("view", "View", &["tree", "radial"])
                .default_str("tree")
                .doc("tree: columns by MS level; radial: the precursor in the centre, one ring per step of the best path (the HTML has both and opens on this one)"),
        ],
        runs_in: RunsIn {
            native: true,
            ..RunsIn::default()
        },
        errors: vec!["err.msnfigura.bad", "err.msnfigura.empty", "err.msnfigura.png", "err.msnfigura.png.render"],
        ..Step::core(
            "msn.figure",
            "MSn graph as a figure",
            "Draws the fragmentation graph as an SVG, a stand-alone HTML or a PNG: ions with m/z, relative intensity, MS levels and the proposed formulas, arrows for the neutral losses. The formulas are candidates of the graph, not identifications.",
        )
    }
}

fn param<'a>(job: &'a Job, name: &str) -> Option<&'a str> {
    job.resolved
        .params
        .iter()
        .find(|(k, _)| k == name)
        .and_then(|(_, v): &(String, Value)| v.str())
}

fn figure_job(r: &FileRunner, job: &Job) -> Result<Artifact> {
    let text = String::from_utf8(read_one(r, job, "graph")?.1)
        .map_err(|_| Error::new("err.msnfigura.bad"))?;
    let format = Formato::da_nome(param(job, "format").unwrap_or("svg"))
        .ok_or_else(|| Error::new("err.flow.param.type").with("param", "format"))?;
    let view = Vista::da_nome(param(job, "view").unwrap_or("tree"))
        .ok_or_else(|| Error::new("err.flow.param.type").with("param", "view"))?;
    let out = r
        .work
        .join(format!("{}.{}", job.instance.id, format.estensione()));
    std::fs::write(&out, bytes(&text, format, view)?).map_err(crate::flow::io)?;
    r.keep(out, DataType::Report)
}
