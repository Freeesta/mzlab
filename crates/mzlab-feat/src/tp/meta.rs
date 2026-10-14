//! The meta of the block `tp/1` in the `.mzfeat`: the thresholds used and the counts (with the roles, the rows each reference took out), and back.

use mzlab_core::json::{Out, Value};

use super::*;

impl TpBlock {
    /// The object `tp` of the meta of the `.mzfeat`: schema, thresholds used, id of the parent and the rows of every status.
    pub fn meta_json(&self) -> String {
        let p = &self.params;
        let mut j = Out::default();
        j.raw("{\"schema\":")
            .str(SCHEMA)
            .raw(",\"progenitore\":")
            .int(i64::from(self.parent_row) + 1);
        j.raw(",\"parametri\":{\"rapporto\":")
            .num(p.ratio)
            .raw(",\"isf_cv\":")
            .num(p.isf_cv)
            .raw(",\"isf_rt\":")
            .num(p.isf_rt);
        j.raw(",\"isf_punti\":")
            .int(p.isf_points as i64)
            .raw(",\"ppm\":")
            .num(p.ppm)
            .raw(",\"purezza_min\":")
            .num(p.purity_min);
        j.raw(",\"saturo\":")
            .raw(if p.saturated { "true" } else { "false" })
            .raw("},\"conteggi\":{\"tp\":");
        j.int(self.count(ST_TP) as i64)
            .raw(",\"controllo\":")
            .int(self.count(ST_CONTROL) as i64);
        j.raw(",\"isf\":")
            .int(self.count(ST_ISF) as i64)
            .raw(",\"non_spiegate\":")
            .int(self.count(ST_UNEXPLAINED) as i64);
        if let Some(c) = &self.roles {
            roles_json(&mut j, c);
        }
        j.raw("}}");
        j.0
    }

    /// The thresholds and the parent from the object `tp` of a meta (the columns are read by the caller).
    pub fn from_meta(v: &Value) -> TpBlock {
        let num = |o: &Value, k: &str, d: f64| o.get(k).and_then(Value::f64).unwrap_or(d);
        let d = TpParams::default();
        let o = v.get("parametri");
        let get = |k: &str, dv: f64| o.map_or(dv, |o| num(o, k, dv));
        TpBlock {
            parent_row: (num(v, "progenitore", 0.0) as u32).wrapping_sub(1),
            params: TpParams {
                ratio: get("rapporto", d.ratio),
                isf_cv: get("isf_cv", d.isf_cv),
                isf_rt: get("isf_rt", d.isf_rt),
                isf_points: get("isf_punti", d.isf_points as f64) as usize,
                ppm: get("ppm", d.ppm),
                purity_min: get("purezza_min", d.purity_min),
                saturated: o
                    .and_then(|o| o.get("saturo"))
                    .and_then(Value::bool)
                    .unwrap_or(false),
            },
            roles: v.get("ruoli").map(|o| roles_from(v, o)),
            ..TpBlock::default()
        }
    }
}

/// The counts of gate 1 by role (inside `conteggi`) and the object `ruoli`: no t0, parent in the dark over the t0, kinds of the blanks.
fn roles_json(j: &mut Out, c: &RoleCounts) {
    j.raw(",\"scartate_bianco\":").int(c.by_blank as i64);
    j.raw(",\"scartate_t0\":").int(c.by_t0 as i64);
    j.raw(",\"scartate_buio\":").int(c.by_dark as i64);
    j.raw("},\"ruoli\":{\"senza_t0\":")
        .raw(if c.no_t0 { "true" } else { "false" });
    j.raw(",\"buio_su_t0\":")
        .opt(c.dark_over_t0)
        .raw(",\"bianchi\":[");
    for (i, k) in c.blank_kinds.iter().enumerate() {
        j.raw(if i > 0 { "," } else { "" }).str(k);
    }
    j.raw("]");
}

fn roles_from(v: &Value, o: &Value) -> RoleCounts {
    let n = |k: &str| {
        v.get("conteggi")
            .and_then(|c| c.get(k))
            .and_then(Value::f64)
            .unwrap_or(0.0) as usize
    };
    RoleCounts {
        by_blank: n("scartate_bianco"),
        by_t0: n("scartate_t0"),
        by_dark: n("scartate_buio"),
        no_t0: o.get("senza_t0").and_then(Value::bool).unwrap_or(false),
        dark_over_t0: o.get("buio_su_t0").and_then(Value::f64),
        blank_kinds: o
            .get("bianchi")
            .map(|a| a.arr())
            .unwrap_or_default()
            .iter()
            .map(|x| x.str().unwrap_or("").to_string())
            .collect(),
    }
}
