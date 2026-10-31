//! Reading out a built `Canali`: the scans of a channel, its memory and its JSON (`api/canali`, `mzlab-cli canali`).

use super::*;

impl Canali {
    /// The scans of channel `i` in file order (`tic` and `bpc`: every scan).
    pub fn scansioni(&self, i: usize) -> Vec<u32> {
        match self.v.get(i).map(|c| &c.sorgente) {
            Some(Sorgente::Scan { da, a })
                if matches!(self.v[i].famiglia, Famiglia::Tic | Famiglia::Bpc) =>
            {
                let mut t = self.scan_ids[*da as usize..*a as usize].to_vec();
                crate::sortc::slice_ord(&mut t);
                t
            }
            Some(Sorgente::Scan { da, a }) => self.scan_ids[*da as usize..*a as usize].to_vec(),
            _ => Vec::new(),
        }
    }

    /// Bytes this structure holds beyond the scan table.
    pub fn byte(&self) -> usize {
        self.scan_ids.capacity() * 4
            + self
                .v
                .iter()
                .map(|c| {
                    std::mem::size_of::<Canale>() + c.id.len() + c.etichetta.len() + c.omologo.len()
                })
                .sum::<usize>()
    }

    /// `{"v":"canali/1","tipo":{...},"canali":[...]}`.
    pub fn json(&self) -> String {
        let mut o = Out::default();
        o.raw("{\"v\":\"canali/1\",\"tipo\":{\"proposto\":")
            .str(self.tipo.proposto)
            .raw(",\"motivo\":")
            .str(self.tipo.motivo)
            .raw("},\"canali\":[");
        for (i, c) in self.v.iter().enumerate() {
            o.raw(if i > 0 { ",{\"id\":" } else { "{\"id\":" })
                .str(&c.id)
                .raw(",\"fam\":")
                .str(c.famiglia.nome())
                .raw(",\"gen\":");
            match c.genitore {
                Some(g) => o.int(g as i64),
                None => o.raw("null"),
            };
            o.raw(",\"liv\":")
                .int(c.livello as i64)
                .raw(",\"pol\":")
                .int(c.pol as i64)
                .raw(",\"sigla\":");
            match c.sigla {
                Some(s) => o.str(s),
                None => o.raw("null"),
            };
            o.raw(",\"et\":")
                .str(&c.etichetta)
                .raw(",\"omo\":")
                .str(&c.omologo)
                .raw(",\"n\":")
                .int(c.n as i64);
            let (a, b) = if c.rt.0.is_finite() { c.rt } else { (0.0, 0.0) };
            o.raw(",\"rt0\":")
                .num(round(a, 4))
                .raw(",\"rt1\":")
                .num(round(b, 4))
                .raw(",\"spettri\":")
                .raw(if c.spettri { "true" } else { "false" })
                .raw("}");
        }
        o.raw("]}");
        o.0
    }
}
