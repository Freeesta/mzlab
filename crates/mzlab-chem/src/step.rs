//! The `step/1` descriptor of `structure.reactivity` (package `tabui`) and the whole Tabui family: the steps of `mzlab-feat` plus this one, which that
//! crate cannot see. The defaults are read from `Opts::parse("")`; nothing here is called by the `.wasm` `chem`.

use mzlab_core::step::{
    Algorithm, Applicability, DataType, Param, Port, Reference, RunsIn, Status, Step,
};

use crate::reatt::Opts;

/// The `.wasm` of the chemistry engine and its fixed ceiling in bytes (`limit_chem` of `tools/build_feat_wasm.sh`).
const CHEM_WASM: &str = "chem";
const CHEM_CEILING: u64 = 569_344;

/// Every step of the Tabui family, in the order of the chain: the ones of `mzlab-feat` first, then the reactivity of the parent.
pub fn tabui() -> Vec<Step> {
    let mut all = mzlab_feat::steps::tabui();
    all.push(reactivity());
    all
}

/// `structure.reactivity`: condensed Fukui functions of the parent (`reactivity_json`, `reatt/1`), shown as a prior of the sites, never as a decision.
pub fn reactivity() -> Step {
    let d = Opts::parse("").expect("empty options are the defaults");
    let algorithm = |id, title, doc, applicability| Algorithm {
        id,
        version: 1,
        title,
        doc,
        references: Vec::new(),
        applicability,
        params: Vec::new(),
        status: Status::Available,
    };
    let gfn2 = Algorithm {
        references: vec![
            Reference {
                doi: "10.1021/acs.jctc.8b01176",
                citation_key: "bannwarth2019gfn",
                note: "the semiempirical method of the neutral molecule, the cation and the anion",
            },
            Reference {
                doi: "10.1021/acs.jctc.1c00471",
                citation_key: "ehlert2021robust",
                note: "ALPB implicit water",
            },
            Reference {
                doi: "10.1021/ja00279a008",
                citation_key: "yang1986use",
                note: "condensed Fukui functions from the charges of N-1, N and N+1 electrons",
            },
        ],
        ..algorithm(
            "gfn2_xtb",
            "GFN2-xTB with Fukui functions",
            "ETKDG geometry, GFN2-xTB of the neutral molecule, the cation and the anion at the same geometry; f(+), f(-), f(0) and the dual from the Mulliken charges, f(-) also on the lowest conformers for its spread; C-H sites by symmetry class (reactivity_json in crates/mzlab-chem/src/reatt.rs).",
            Applicability::default(),
        )
    };
    let huckel = Algorithm {
        references: vec![Reference {
            doi: "10.1063/1.1734456",
            citation_key: "hoffmann1963extended",
            note: "the extended Hückel method",
        }],
        ..algorithm(
            "extended_huckel",
            "Extended Hückel fallback",
            "One diagonalisation, no SCC and no ions; used by the engine only when an SCC of GFN2-xTB does not converge, and the result says so in its method (eht.rs).",
            Applicability {
                note: "Fallback: the engine picks it, the user does not",
                ..Applicability::default()
            },
        )
    };
    Step {
        package: "tabui",
        ins: vec![Port::new("parent", DataType::Structure, "Parent compound (SMILES)")],
        outs: vec![Port::new("reactivity", DataType::Annot, "Fukui functions per atom and C-H sites with their rank")],
        params: vec![
            Param::choice("solvent", "Solvent", &["water", "none"]).default_str(if d.water { "water" } else { "none" }),
            Param::integer("conformers", "Conformers")
                .min(1.0)
                .max(16.0)
                .default_num(d.conformers as f64)
                .doc("ETKDG conformers of the parent, lowest energy first; f(-) is also computed on the lowest ones to give its spread"),
            Param::choice("protomer", "Form of the parent", &["none", "ph7"])
                .default_str(if d.ph7 { "ph7" } else { "none" })
                .doc("none: the SMILES as written; ph7: the most probable charge form at pH 7 from a table of typical pKa"),
            Param::integer("seed", "Seed")
                .min(0.0)
                .default_num(d.seed as f64)
                .advanced(),
        ],
        runs_in: RunsIn {
            wasm: Some(CHEM_WASM),
            wasm_ceiling_bytes: Some(CHEM_CEILING),
            native: true,
        },
        errors: vec!["error.chem.smiles", "error.chem.opts", "error.chem.embed", "error.chem.scc"],
        algorithms: vec![gfn2, huckel],
        default_algorithm: Some("gfn2_xtb"),
        references: vec![Reference {
            doi: "10.1021/acs.jcim.5b00654",
            citation_key: "riniker2015better",
            note: "ETKDG, the conformer generator",
        }],
        ..Step::core(
            "structure.reactivity",
            "Reactivity of the parent",
            "Computes where the parent is most reactive toward an electrophilic radical (condensed Fukui function f(-) per atom and per class of C-H sites). The result is a prior shown beside the sites of tp.position and never decides one: the experiment does.",
        )
    }
}
