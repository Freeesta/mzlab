//! `structure.render` and `structure.draw`: how the modification of a product is drawn and the editor where the student draws a structure. The first runs
//! in the `.wasm` `interp` (`interp::disegno`), the second is the page only (Ketcher in a `sandbox` iframe, `draw.js`): it has no engine and no ceiling.

use mzlab_core::step::{Algorithm, DataType, Port, RunsIn, Status, Step};

use super::tabui::{algorithm, tabui_step, INTERP_CEILING, INTERP_WASM};

/// `structure.render`: ONE form per product row that tells the truth about the position (`interp::disegno::drawing`, `drawing` of `interp/1`).
pub(super) fn render() -> Step {
    Step {
        ins: vec![
            Port::new("sites", DataType::Transform, "Classes of sites with their support and the declared confidence"),
            Port::new("parent", DataType::Structure, "Parent compound (SMILES)"),
        ],
        outs: vec![Port::new("drawing", DataType::Structure, "Drawn form: mode, atoms, bracket or ring, faint atoms, highlighted zone")],
        algorithms: vec![simplest_form()],
        default_algorithm: Some("simplest_form"),
        per_item: true,
        ..tabui_step(
            "structure.render",
            "Draw the modification",
            "Chooses the simplest form that tells the truth about a product: the structure itself only when the position is a site, a multicenter bond when every candidate lies in one aromatic ring, otherwise a bracket around a zone or the whole parent. Never one structure per site. The page draws it over the coordinates of the parent that Ketcher gives; without them the structure becomes a bracket.",
            INTERP_WASM,
            INTERP_CEILING,
        )
    }
}

fn simplest_form() -> Algorithm {
    algorithm(
        "simplest_form",
        "Structure, multicenter or bracket",
        "The mode follows the position report: structure for a site, multicenter for candidates in one ring, bracket otherwise; atoms a loss removes are drawn faint and dashed (drawing in crates/mzlab-feat/src/interp/disegno.rs).",
        Status::Available,
    )
}

/// `structure.draw`: the editor of the Drawing view. It is the only step whose output is made by a person, so the same inputs do not give the same bytes.
pub(super) fn draw() -> Step {
    Step {
        ins: vec![Port {
            optional: true,
            ..Port::new("parent", DataType::Structure, "Structure to start from")
        }],
        outs: vec![Port::new("structure", DataType::Structure, "SMILES, with the Δ or the multicenter attachment of a partial structure (parz/1) when one is drawn")],
        package: "tabui",
        runs_in: RunsIn::default(),
        deterministic: false,
        algorithms: vec![ketcher()],
        default_algorithm: Some("ketcher"),
        ..Step::core(
            "structure.draw",
            "Draw a structure",
            "Opens the structure editor (Ketcher, in a sandboxed frame inside the page) so the student draws the parent or a product. A partial structure carries a Δ in a bracket or an attachment on any atom of a ring, which makes it a level 3 candidate and never an identification.",
        )
    }
}

fn ketcher() -> Algorithm {
    algorithm(
        "ketcher",
        "Ketcher editor",
        "Interactive: the output is the drawing at the moment the step is read (TPDraw.smiles and TPDraw.partial in mzlab/web/draw.js). No engine runs and no .wasm is loaded.",
        Status::Available,
    )
}
