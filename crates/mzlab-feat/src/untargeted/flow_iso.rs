//! The isotope grouping `grouper` of `features.detect`: the isotope grouper of mzmine (`isotopes::group_items`) on the features of one file, in
//! place of the averagine grouping of `crate::isotopes`. The output has the same shape: `iso_group` = position + 1 of the representative of the
//! group (the most intense row, or the lowest m/z), `charge` shared by the members, `FLAG_ISOTOPE` on every member but the representative; no
//! row is removed.

use mzlab_core::error::Result;

use super::isotopes::{group_items, IsoParams, Item, Representative, ISOTOPE_STEP};
use crate::align_mzmine::IsoGrouper;
use crate::table::{FileFeatures, FLAG_ISOTOPE};
use crate::trace::filled;

/// The `IsoParams` of the grouper.
pub fn iso_params(g: &IsoGrouper) -> IsoParams {
    IsoParams {
        tol_da: g.tol_da,
        tol_ppm: g.tol_ppm,
        rt_tol: g.rt_tol,
        max_charge: g.max_charge,
        monotonic: g.monotonic,
        representative: if g.lowest_mz {
            Representative::LowestMz
        } else {
            Representative::MostIntense
        },
        step: ISOTOPE_STEP,
    }
}

/// Groups the features of `f` with the isotope grouper of mzmine.
pub fn group(f: &mut FileFeatures, g: &IsoGrouper) -> Result<()> {
    let n = f.len();
    let items: Vec<Item> = (0..n)
        .map(|i| Item {
            mz: f.mz[i],
            rt: f64::from(f.rt[i]),
            height: f64::from(f.height[i]),
        })
        .collect();
    let groups = group_items(&items, &iso_params(g));
    let mut gid = filled(n, 0u32)?;
    let mut charge = filled(n, 0i8)?;
    for flag in &mut f.flags {
        *flag &= !FLAG_ISOTOPE;
    }
    for grp in &groups.groups {
        for &m in &grp.members {
            gid[m] = grp.representative as u32 + 1;
            charge[m] = grp.charge as i8;
            if m != grp.representative {
                f.flags[m] |= FLAG_ISOTOPE;
            }
        }
    }
    f.iso_group = gid;
    f.charge = charge;
    Ok(())
}
