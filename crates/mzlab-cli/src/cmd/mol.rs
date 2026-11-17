//! The `mol-*` commands: drawn structures.

use super::Entry;

pub(crate) static TABLE: &[Entry] = &[
    Entry { name: "mol-label", aliases: &[], help: "mzlab-cli mol-label REQUEST.json|- [--out PATH]   formula, charge and masses of drawn structures and arrows as JSON (mol-label --help); mol-anal: average mass, mass % and theoretical isotope pattern of one structure (mol-anal --help)", run: crate::mol_label_cmd::run },
    Entry { name: "mol-anal", aliases: &[], help: "", run: crate::mol_label_cmd::run_anal },
    Entry { name: "mol-check", aliases: &[], help: "mzlab-cli mol-check REQUEST.json|- [--out PATH]   what is wrong in a drawing (valence, charge, isotope, wedges) as JSON (mol-check --help); mol-cip: its R/S and E/Z (mol-cip --help); mol-hilite: the atoms an evidence colours (mol-hilite --help); mol-sel REQUEST.json|-: masses, heavy atoms and rings of a selection (mol-sel --help); mol-link REQUEST.json|-: the PubChem, ChEBI and BioTransformer URLs of a structure (mol-link --help); mol-export MOLFILE|-: the drawing as .mol or .sdf (mol-export --help)", run: crate::plain::mol_check_cmd::run },
    Entry { name: "mol-cip", aliases: &[], help: "", run: crate::plain::mol_cip_cmd::run },
    Entry { name: "mol-hilite", aliases: &[], help: "", run: crate::plain::mol_hilite_cmd::run },
    Entry { name: "mol-sel", aliases: &[], help: "", run: crate::plain::mol_sel_cmd::run },
    Entry { name: "mol-link", aliases: &[], help: "", run: crate::plain::mol_link_cmd::run },
    Entry { name: "mol-export", aliases: &[], help: "", run: crate::plain::mol_export_cmd::run },
    Entry { name: "mol-cut", aliases: &[], help: "mzlab-cli mol-cut MOLFILE|- --set bonds=A-B[,...] [--set NAME=VALUE...]   the pieces of a homolytic, heterolytic or +H cut as JSON (mol-cut --help); mol-mech REQUEST.json|-: whether the curved arrows of a drawn mechanism are well formed (mol-mech --help); mol-search --sub QUERY TARGET...: where a substructure occurs in structures (mol-search --help)", run: crate::mol_cut_cmd::run },
    Entry { name: "mol-mech", aliases: &[], help: "", run: crate::plain::mol_mech_cmd::run },
    Entry { name: "mol-search", aliases: &[], help: "", run: crate::mol_search_cmd::run },
];
