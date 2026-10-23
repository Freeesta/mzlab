//! The candidate filters of `spectra.match`: text filters on the library metadata and the retention-time window (`rt_tol`); used by `flow_match`.

use crate::flow_match::{num, text_param, Cand, Lib};
use mzlab_core::flow::Job;
use mzlab_lib::spec::Meta;
use std::collections::HashMap;

#[derive(Clone, Copy)]
enum Field {
    Name,
    Adduct,
    Instrument,
    Ce,
}

pub(crate) struct Filters {
    /// (field, lower-case text); an empty text keeps everything
    meta: [(Field, String); 4],
    rt_tol: f64,
}

impl Filters {
    pub(crate) fn from_job(job: &Job) -> Filters {
        let t = |k: &str| text_param(job, k).to_lowercase();
        Filters {
            meta: [
                (Field::Name, t("name_contains")),
                (Field::Adduct, t("adduct_contains")),
                (Field::Instrument, t("instrument_contains")),
                (Field::Ce, t("ce_contains")),
            ],
            rt_tol: num(job, "rt_tol"),
        }
    }

    /// Whether the library entry has every filter text in its field (any case).
    fn passes(&self, m: &Meta) -> bool {
        self.meta.iter().all(|(f, t)| {
            let v = match f {
                Field::Name => &m.name,
                Field::Adduct => &m.adduct,
                Field::Instrument => &m.instrument,
                Field::Ce => &m.ce,
            };
            t.is_empty() || v.to_lowercase().contains(t.as_str())
        })
    }

    /// Drops the candidates the filters exclude and returns the RT difference (query minus library, minutes) of those where both are known.
    /// An entry or a query without a retention time cannot be excluded by it.
    pub(crate) fn apply(
        &self,
        lib: &Lib,
        c: &mut Vec<Cand>,
        q_rt: Option<f64>,
    ) -> HashMap<u32, f64> {
        if self.meta.iter().any(|(_, t)| !t.is_empty()) {
            c.retain(|h| lib.meta(h.index).is_ok_and(|m| self.passes(&m)));
        }
        let mut delta = HashMap::new();
        c.retain(|h| {
            let lib_rt = lib.rec(h.index).ok().and_then(|r| r.rt_min());
            let Some(d) = q_rt.zip(lib_rt).map(|(a, b)| a - b) else {
                return true;
            };
            delta.insert(h.index, d);
            !(self.rt_tol > 0.0 && d.abs() > self.rt_tol)
        });
        delta
    }
}
