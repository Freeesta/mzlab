//! The `ion` group (masses, isotope patterns, what a number matches, the periodic table) and the `mol` group (drawn structures).

use super::Entry;

pub(crate) static TABLE: &[Entry] = &[
    Entry::new("ion mz", "mzlab-cli ion mz TEXT [--adduct A]   the m/z of an ion with a signed shift", crate::plain::ion_mz_cmd::run).examples(&["mzlab-cli ion mz \"C6H12O6\" --adduct +H"]),
    Entry::new("ion pattern", "mzlab-cli ion pattern FORMULA ADDUCT [--out PATH]   the unit-resolution isotope pattern of an ion as JSON", crate::ion_pattern_cmd::run),
    Entry::new("ion match", "mzlab-cli ion match X --tables FILE.json [--set hr=1 ppm=N]   what a number is compatible with: losses, adduct shifts, known contaminants, as candidates", crate::plain::match_mz_cmd::run),
    Entry::new("ion elements", "mzlab-cli ion elements [--json]   the periodic table (elements/1)", crate::plain::elements_cmd::run),
    Entry::new("mol label", "mzlab-cli mol label REQUEST.json|- [--out PATH]   formula, charge and masses of drawn structures and arrows as JSON", crate::mol_label_cmd::run),
    Entry::new("mol anal", "mzlab-cli mol anal REQUEST.json|-   average mass, mass % and theoretical isotope pattern of one structure", crate::mol_label_cmd::run_anal),
    Entry::new("mol check", "mzlab-cli mol check REQUEST.json|- [--out PATH]   what is wrong in a drawing (valence, charge, isotope, wedges) as JSON", crate::plain::mol_check_cmd::run),
    Entry::new("mol cip", "mzlab-cli mol cip REQUEST.json|-   the R/S and E/Z of a drawing", crate::plain::mol_cip_cmd::run),
    Entry::new("mol hilite", "mzlab-cli mol hilite REQUEST.json|-   the atoms an evidence colours", crate::plain::mol_hilite_cmd::run),
    Entry::new("mol sel", "mzlab-cli mol sel REQUEST.json|-   masses, heavy atoms and rings of a selection", crate::plain::mol_sel_cmd::run),
    Entry::new("mol link", "mzlab-cli mol link REQUEST.json|-   the PubChem, ChEBI and BioTransformer URLs of a structure", crate::plain::mol_link_cmd::run),
    Entry::new("mol export", "mzlab-cli mol export MOLFILE|-   the drawing as .mol or .sdf", crate::plain::mol_export_cmd::run),
    Entry::new("mol cut", "mzlab-cli mol cut MOLFILE|- --set bonds=A-B[,...] [--set NAME=VALUE...]   the pieces of a homolytic, heterolytic or +H cut as JSON", crate::mol_cut_cmd::run),
    Entry::new("mol mech", "mzlab-cli mol mech REQUEST.json|-   whether the curved arrows of a drawn mechanism are well formed", crate::plain::mol_mech_cmd::run),
    Entry::new("mol search", "mzlab-cli mol search --sub QUERY TARGET...   where a substructure occurs in structures", crate::mol_search_cmd::run),
];
