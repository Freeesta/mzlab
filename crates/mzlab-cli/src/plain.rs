//! The subcommands of `mzlab-cli` that take their arguments as a plain list.

#[path = "background_cmd.rs"]
pub(crate) mod background_cmd;
#[path = "dda_window_cmd.rs"]
pub(crate) mod dda_window_cmd;
#[path = "elements_cmd.rs"]
pub(crate) mod elements_cmd;
#[path = "ion_mz_cmd.rs"]
pub(crate) mod ion_mz_cmd;
#[path = "match_mz_cmd.rs"]
pub(crate) mod match_mz_cmd;
#[path = "mol_check_cmd.rs"]
pub(crate) mod mol_check_cmd;
#[path = "mol_cip_cmd.rs"]
pub(crate) mod mol_cip_cmd;
#[path = "mol_draw_cmd.rs"]
pub(crate) mod mol_draw_cmd;
#[path = "mol_export_cmd.rs"]
pub(crate) mod mol_export_cmd;
#[path = "mol_hilite_cmd.rs"]
pub(crate) mod mol_hilite_cmd;
#[path = "mol_link_cmd.rs"]
pub(crate) mod mol_link_cmd;
#[path = "mol_mech_cmd.rs"]
pub(crate) mod mol_mech_cmd;
#[path = "mol_sel_cmd.rs"]
pub(crate) mod mol_sel_cmd;
#[path = "peak_area_cmd.rs"]
pub(crate) mod peak_area_cmd;
#[path = "peak_split_cmd.rs"]
pub(crate) mod peak_split_cmd;
#[path = "peak_window_cmd.rs"]
pub(crate) mod peak_window_cmd;
#[path = "ref_list_cmd.rs"]
pub(crate) mod ref_list_cmd;

#[cfg(test)]
#[path = "plain_tests.rs"]
pub(crate) mod tests;
