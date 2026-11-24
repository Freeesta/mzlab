//! Chemistry from a structure. Not used by the engine of the proofs: it lives in a `.wasm` of its own (`crates/mzlab-feat-grafo-wasm`), loaded when
//! the user gives the SMILES of the parent.

pub mod analysis;
mod aromatic_h;
mod atom_block;
pub mod breaks;
#[cfg(feature = "check")]
pub mod check;
#[cfg(feature = "check")]
pub mod cip;
#[cfg(feature = "check")]
mod cip_rank;
pub mod cut;
#[cfg(feature = "check")]
pub mod descriptors;
#[cfg(feature = "check")]
pub mod draw;
#[cfg(feature = "check")]
mod draw_build;
#[cfg(feature = "check")]
mod draw_cip;
#[cfg(feature = "check")]
mod draw_db;
#[cfg(feature = "check")]
pub mod draw_file;
#[cfg(feature = "check")]
mod draw_font;
#[cfg(feature = "check")]
mod draw_hl;
#[cfg(feature = "check")]
mod draw_label;
#[cfg(feature = "check")]
mod draw_png;
#[cfg(feature = "check")]
mod draw_spec;
#[cfg(feature = "check")]
mod draw_svg;
#[cfg(feature = "check")]
mod edit;
#[cfg(feature = "check")]
mod edit_end;
#[cfg(feature = "check")]
mod edit_group;
#[cfg(feature = "check")]
mod edit_op;
#[cfg(feature = "check")]
mod edit_set;
pub mod fragmenter;
pub mod fragments;
pub mod graph;
#[cfg(feature = "record")]
mod graph_record;
mod groups;
#[cfg(all(feature = "check", feature = "sub"))]
pub mod hilite;
pub mod labels;
pub mod link;
pub mod mech;
#[cfg(feature = "sub")]
pub mod moiety;
mod molfile_read;
pub mod positions;
pub mod rules;
mod rules_site;
#[cfg(feature = "check")]
pub mod sdf;
#[cfg(feature = "sel")]
pub mod sel;
mod shuffle;
mod smiles;
#[cfg(feature = "check")]
pub mod structure_file;
pub mod structures;
#[cfg(feature = "sub")]
pub mod substructure;
