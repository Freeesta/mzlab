//! The routes of the m/z offset (`mzcal`) on an open file: `mzoff=DA|auto:MZ[@RT]` on `/api/xic` (the windows move by the offset for that request, the
//! trace keeps the m/z asked) and `/api/mzcal?mz&rt` (the evidence of the estimate). Site and app answer through here.

use crate::error::{Error, Result};
use crate::idxread::ReadAt;
use crate::mzcal::Spec;
use crate::routes::{Query, Routes};

impl<R: ReadAt> Routes<R> {
    /// `/api/xic`, with the offset of `mzoff` (if any) in force while the request is answered.
    pub(crate) fn xic_route(&mut self, q: &Query) -> Result<Option<String>> {
        let Some(t) = q.some("mzoff") else {
            return self.xic_route_in(q, None);
        };
        let off = Spec::parse(t)?.resolve(&mut self.idx, self.pol)?;
        let before = std::mem::replace(&mut self.mz_off, off.da);
        let out = self.xic_route_in(q, Some(&off));
        self.mz_off = before;
        out
    }

    /// `/api/mzcal?mz=M[&rt=T]`: the offset measured on the ion at M (near T min), as `{"mz_offset_da":D,"mz_offset":{...}}`.
    pub(crate) fn mzcal_route(&mut self, q: &Query) -> Result<Option<String>> {
        let Some(mz) = q.num("mz")?.filter(|m| *m > 0.0) else {
            return Err(Error::new("err.mz.offset.spec").with("text", "mz"));
        };
        let off = crate::mzcal::estimate(&mut self.idx, self.pol, mz, q.num("rt")?)?;
        Ok(Some(format!("{{{}}}", off.members())))
    }
}
