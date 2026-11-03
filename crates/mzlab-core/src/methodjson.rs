//! The JSON of a method as `mzlab.reader.methodinfo.read_methods` gives it.

use crate::json::Out;
use crate::methodinfo::{Kind, Lc, Method, Param};

fn opt(o: &mut Out, v: Option<f64>) {
    o.opt(v);
}

fn params(o: &mut Out, v: &[Param]) {
    o.raw("[");
    for (i, p) in v.iter().enumerate() {
        o.raw(if i > 0 { ",{\"id\":" } else { "{\"id\":" }).str(&p.id).raw(",\"label\":").str(p.label).raw(",\"value\":").num(p.value).raw(",\"unit\":").str(p.unit).raw("}");
    }
    o.raw("]");
}

impl Method {
    /// The method as Python's `read_methods` gives it, as JSON (the `lab` rows of `/api/method` add `check`).
    pub fn write_json(&self, o: &mut Out) {
        o.raw("{\"source\":");
        params(o, &self.source);
        o.raw(",\"compound\":");
        params(o, &self.compound);
        o.raw(",\"file\":{");
        for (i, (k, v)) in self.file.iter().enumerate() {
            o.raw(if i > 0 { "," } else { "" }).str(k).raw(":").str(v);
        }
        o.raw("},\"experiments\":[");
        for (i, e) in self.experiments.iter().enumerate() {
            o.raw(if i > 0 { ",{\"index\":" } else { "{\"index\":" }).int(e.index as i64);
            match &e.kind {
                Kind::Mrm(tr) => {
                    o.raw(",\"kind\":\"mrm\",\"transitions\":[");
                    for (j, t) in tr.iter().enumerate() {
                        o.raw(if j > 0 { ",{\"q1\":" } else { "{\"q1\":" }).num(t.q1).raw(",\"q3\":").num(t.q3).raw(",\"dwell\":").num(t.dwell).raw(",\"name\":").str(&t.name).raw(",\"ce\":");
                        opt(o, t.ce);
                        o.raw("}");
                    }
                    o.raw("]");
                }
                Kind::Scan { range, ce } => {
                    o.raw(",\"kind\":\"scan\",\"range\":");
                    match range {
                        Some(r) => o.raw("[").num(r[0]).raw(",").num(r[1]).raw("]"),
                        None => o.raw("null"),
                    };
                    o.raw(",\"ce\":").floats(ce);
                }
            }
            o.raw("}");
        }
        o.raw("]");
        if let Some(l) = &self.lc {
            o.raw(",\"lc\":");
            l.write_json(o);
        }
        o.raw(",\"name\":").str(&self.name).raw(",\"sample\":");
        match self.sample {
            Some(n) => o.int(n),
            None => o.raw("null"),
        };
        o.raw("}");
    }
}

impl Lc {
    fn write_json(&self, o: &mut Out) {
        o.raw("{\"gradient\":[");
        for (i, g) in self.gradient.iter().enumerate() {
            o.raw(if i > 0 { ",{\"t\":" } else { "{\"t\":" }).num(g.t).raw(",\"b\":").opt(g.b).raw(",\"flow\":").opt(g.flow).raw("}");
        }
        o.raw("],\"flow\":").opt(self.flow).raw(",\"b0\":").opt(self.b0).raw(",\"run_time\":").opt(self.run_time).raw(",\"pda\":{\"start\":").opt(self.pda.start);
        o.raw(",\"stop\":").opt(self.pda.stop).raw(",\"step\":").opt(self.pda.step).raw(",\"freq\":").opt(self.pda.freq).raw(",\"lamp\":").str(&self.pda.lamp);
        o.raw(",\"cell\":").opt(self.pda.cell).raw(",\"slit\":").opt(self.pda.slit).raw("},\"pda_channels\":[");
        for (i, (w, bw)) in self.channels.iter().enumerate() {
            o.raw(if i > 0 { ",{\"wl\":" } else { "{\"wl\":" }).num(*w).raw(",\"bw\":").opt(*bw).raw("}");
        }
        o.raw("],\"oven\":").opt(self.oven).raw("}");
    }
}
