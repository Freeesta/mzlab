//! The identities of an untargeted feature table as mzFinder gives them (`TP_Mine/py/tpmine/hr/identity.py`: `build` and `polarity_pairs`):
//! the graph of co-eluting, correlated, mass-explained features with the evidence of every edge, and the neutral-mass hypotheses of every
//! connected component. Stateless: the table comes in the payload, the JSON is the one of `api.identity_graph` and `api.polarity_pairs`.

use crate::sortc::SortC;
use mzlab_core::json::{self, Out, Value};

use super::edge::{self, dimer_of, mean, relations, std, EdgeIn, Iimn, Spec};
use super::Res;

mod values;
use values::*;

mod hyp;
use hyp::*;

mod pairs;
pub use pairs::*;

struct Dsu(Vec<usize>);

impl Dsu {
    fn find(&mut self, mut i: usize) -> usize {
        while self.0[i] != i {
            self.0[i] = self.0[self.0[i]];
            i = self.0[i];
        }
        i
    }
    fn union(&mut self, a: usize, b: usize) {
        let (a, b) = (self.find(a), self.find(b));
        if a != b {
            self.0[a.max(b)] = a.min(b);
        }
    }
}

/// One edge: the JSON written so far (without the evidence and the note) and its two features.
struct EdgeOut {
    a: usize,
    b: usize,
    head: String,
    tail: String,
    note: String,
}

fn edge_head(fs: &[Feat], a: usize, b: usize, kind: &str, label: &str, source: &str) -> Out {
    let mut o = Out::default();
    o.raw("{\"a\":");
    write_value(fs[a].v.get("k").unwrap_or(&Value::Null), &mut o);
    o.raw(",\"b\":");
    write_value(fs[b].v.get("k").unwrap_or(&Value::Null), &mut o);
    o.raw(",\"kind\":").str(kind).raw(",\"label\":").str(label);
    o.raw(",\"source\":")
        .str(source)
        .raw(",\"dmz\":")
        .num(fs[b].mz - fs[a].mz);
    o.raw(",\"drt\":").num(fs[b].rt - fs[a].rt);
    o
}

const IIN_KIND: &str = "mzMine IIN";

/// The state of `build` while it adds the edges.
struct Graph {
    dsu: Dsu,
    edges: Vec<EdgeOut>,
    seen: std::collections::HashSet<(usize, usize)>,
    iin_of: Vec<Option<String>>,
    n_iin: usize,
}

fn pair(i: usize, j: usize) -> (usize, usize) {
    (i.min(j), i.max(j))
}

/// (a) the ion identities that the table already carries: the groups of mzMine and its non-cosine edges.
fn iin_edges(fs: &[Feat], payload: &Value, g: &mut Graph) {
    iin_groups(fs, g);
    mzmine_edges(fs, payload, g);
}

/// The edges inside each mzMine ion identity: the lightest ion of the group with each of the others.
fn iin_groups(fs: &[Feat], g: &mut Graph) {
    let mut groups: Vec<(String, Vec<usize>)> = Vec::new();
    for (i, f) in fs.iter().enumerate() {
        if let Some(gid) = f.iin.and_then(|g| g.get("id")).map(py_str) {
            match groups.iter_mut().find(|x| x.0 == gid) {
                Some(x) => x.1.push(i),
                None => groups.push((gid.clone(), vec![i])),
            }
            g.iin_of[i] = Some(gid);
        }
    }
    let ions = |i: usize| {
        fs[i]
            .iin
            .and_then(|x| x.get("ions"))
            .and_then(Value::str)
            .filter(|s| !s.is_empty())
    };
    for (gid, ms) in &groups {
        if ms.len() < 2 {
            continue;
        }
        let mut ms = ms.clone();
        ms.sortc_by(|&a, &b| fs[a].mz.total_cmp(&fs[b].mz));
        let i = ms[0];
        for &j in &ms[1..] {
            let text: Vec<&str> = [ions(i), ions(j)].into_iter().flatten().collect();
            let label = if text.is_empty() {
                format!("mzMine group {gid}")
            } else {
                text.join(" ~ ")
            };
            let mut o = edge_head(fs, i, j, IIN_KIND, &label, "mzMine table");
            o.raw(",\"group\":").str(gid);
            let nj = fs[j]
                .iin
                .and_then(|x| x.get("neutral"))
                .filter(|v| !v.is_null() && v.str() != Some(""));
            if let Some(n) = nj.or_else(|| {
                fs[i]
                    .iin
                    .and_then(|x| x.get("neutral"))
                    .filter(|v| !v.is_null())
            }) {
                o.raw(",\"neutral\":");
                write_value(n, &mut o);
            }
            g.push(i, j, o.0, String::new());
            g.n_iin += 1;
        }
    }
}

/// The edges of the mzMine edge table (cosine ones left out); a pair already joined only adds its type to the note.
fn mzmine_edges(fs: &[Feat], payload: &Value, g: &mut Graph) {
    let ids: Vec<Option<&str>> = fs
        .iter()
        .map(|f| f.v.get("id").and_then(Value::str))
        .collect();
    let find = |v: Option<&Value>| {
        v.map(py_str)
            .and_then(|s| ids.iter().rposition(|x| *x == Some(s.as_str())))
    };
    for e in payload.get("edges").map(Value::arr).unwrap_or(&[]) {
        let t = e
            .get("type")
            .filter(|v| !v.is_null())
            .map(py_str)
            .unwrap_or_default();
        if t.is_empty() || t.to_lowercase().contains("cosine") {
            continue;
        }
        let (Some(a), Some(b)) = (find(e.get("a")), find(e.get("b"))) else {
            continue;
        };
        if a == b {
            continue;
        }
        let ann = e
            .get("annotation")
            .filter(|v| !v.is_null() && v.str() != Some(""))
            .map(py_str);
        if g.seen.contains(&pair(a, b)) {
            for ed in g.edges.iter_mut().filter(|x| pair(x.a, x.b) == pair(a, b)) {
                ed.note += &match &ann {
                    Some(x) => format!(" · {t}: {x}"),
                    None => format!(" · {t}"),
                };
            }
            continue;
        }
        let mut o = edge_head(
            fs,
            a,
            b,
            IIN_KIND,
            ann.as_deref().unwrap_or(&t),
            "mzMine edges",
        );
        if let Some(s) = e.get("score").filter(|v| !v.is_null()) {
            o.raw(",\"score\":");
            write_value(s, &mut o);
        }
        g.push(a, b, o.0, String::new());
        g.n_iin += 1;
    }
}

impl Graph {
    fn push(&mut self, a: usize, b: usize, head: String, tail: String) {
        self.seen.insert(pair(a, b));
        self.dsu.union(a, b);
        self.edges.push(EdgeOut {
            a,
            b,
            head,
            tail,
            note: String::new(),
        });
    }
}

/// The context of the computed edges.
struct Ctx<'a> {
    t: &'a Iimn,
    forms: &'a [Form],
    pol: i32,
    o: &'a Opt,
    samples: &'a [Value],
}

/// (b) the steps mzMine did not group: co-eluting, correlated, explained. Returns the pairs tested.
fn computed_edges(c: &Ctx, fs: &[Feat], g: &mut Graph) -> usize {
    let n = fs.len();
    let mut order: Vec<usize> = (0..n).collect();
    order.sortc_by(|&a, &b| fs[a].rt.total_cmp(&fs[b].rt));
    let rts: Vec<f64> = order.iter().map(|&i| fs[i].rt).collect();
    let apex_min = c.o.apex_scans * c.o.cycle_s / 60.0;
    let sh = shifts(c.t, c.forms, c.pol);
    let (pol, p) = (c.pol as f64, c.t.proton);
    let mut n_pairs = 0;
    for pi in 0..n {
        let i = order[pi];
        let lim = rts[pi] + apex_min * (1.0 + 1e-9);
        let hi = rts.partition_point(|&r| r <= lim);
        for &j in order.get(pi + 1..hi.max(pi + 1)).unwrap_or(&[]) {
            let (mi, mj) = (fs[i].mz, fs[j].mz);
            let tol = (mj * c.o.ppm * 1e-6).max(c.o.abs_da);
            let d = mj - mi;
            let near = sh
                .iter()
                .map(|s| (d - s).abs())
                .fold(f64::INFINITY, f64::min)
                <= tol;
            let dim = (mj - (2.0 * (mi - pol * p) + pol * p)).abs() <= tol
                || (mi - (2.0 * (mj - pol * p) + pol * p)).abs() <= tol;
            if !(near || dim) {
                continue;
            }
            n_pairs += 1;
            if g.seen.contains(&pair(i, j)) || (g.iin_of[i].is_some() && g.iin_of[i] == g.iin_of[j])
            {
                continue;
            }
            let (lo, up) = if mi <= mj { (i, j) } else { (j, i) };
            if let Some(tail) = computed_edge(c, fs, lo, up, apex_min) {
                let mut head = edge_head(fs, lo, up, tail.0, &tail.1, "computed");
                head.raw(&tail.2);
                g.push(lo, up, head.0, String::new());
            }
        }
    }
    n_pairs
}

/// The kind, label and the rest of a computed edge, or None when nothing relates the two ions or their shapes differ.
fn computed_edge(
    c: &Ctx,
    fs: &[Feat],
    lo: usize,
    up: usize,
    apex_min: f64,
) -> Option<(&'static str, String, String)> {
    let (ml, mu) = (fs[lo].mz, fs[up].mz);
    let mut rels = pair_relations(c.t, c.forms, mu - ml, ml, c.pol, c.o.ppm, c.o.abs_da);
    if dimer_of(c.t, ml, mu, c.o.ppm, c.o.abs_da, c.pol) {
        let dn = if c.pol > 0 { "[2M+H]+" } else { "[2M-H]-" };
        rels.push(Relation {
            role: "dimer",
            name: dn.into(),
            label: format!("{dn} / {}", base(c.pol)),
            expected: mu - ml,
        });
    }
    if rels.is_empty() {
        return None;
    }
    let (r, ncom, common) = shape_r(&fs[lo].areas, &fs[up].areas, c.o.min_common);
    let r = r.filter(|r| *r >= c.o.r_min)?;
    let label: Vec<&str> = rels.iter().map(|x| x.label.as_str()).collect();
    let mut o = Out::default();
    let drt = fs[up].rt - fs[lo].rt;
    o.raw(",\"dscans\":").num(drt.abs() * 60.0 / c.o.cycle_s);
    o.raw(",\"r\":")
        .num(r)
        .raw(",\"n_common\":")
        .int(ncom as i64)
        .raw(",\"samples\":[");
    for (k, &s) in common.iter().take(12).enumerate() {
        o.raw(if k > 0 { "," } else { "" });
        write_value(c.samples.get(s).unwrap_or(&Value::Null), &mut o);
    }
    o.raw("],\"relations\":[");
    for (k, x) in rels.iter().enumerate() {
        o.raw(if k > 0 { ",{\"role\":" } else { "{\"role\":" })
            .str(x.role);
        o.raw(",\"name\":")
            .str(&x.name)
            .raw(",\"label\":")
            .str(&x.label);
        o.raw(",\"expected\":").num(x.expected);
        o.raw(",\"ppm\":")
            .num((mu - ml - x.expected) / ml * 1e6)
            .raw("}");
    }
    o.raw("]");
    let _ = apex_min;
    Some((rels[0].role, label.join(" | "), o.0))
}

/// `api.identity_graph(payload)`: `{identities, edges, stats}`.
pub fn identity_graph(payload_json: &str) -> Res<String> {
    let p = json::parse(payload_json.as_bytes()).map_err(|e| e.to_string())?;
    let pol = if p.get("pol").and_then(Value::f64).unwrap_or(1.0) >= 0.0 {
        1
    } else {
        -1
    };
    let o = opt_of(p.get("opt"));
    let samples = p.get("samples").map(Value::arr).unwrap_or(&[]);
    let fs = feats_of(
        p.get("features").ok_or("payload without features")?.arr(),
        samples.len(),
    )?;
    let (t, n) = (Iimn::new(), fs.len());
    let forms = forms(&t, pol);
    let c = Ctx {
        t: &t,
        forms: &forms,
        pol,
        o: &o,
        samples,
    };
    let mut g = Graph {
        dsu: Dsu((0..n).collect()),
        edges: Vec::new(),
        seen: Default::default(),
        iin_of: vec![None; n],
        n_iin: 0,
    };
    iin_edges(&fs, &p, &mut g);
    let n_pairs = computed_edges(&c, &fs, &mut g);
    for e in g.edges.iter_mut() {
        e.tail = evidence_of(&t, &fs, e.a, e.b, pol, samples.len());
    }
    let mut out = Out::default();
    out.raw("{\"identities\":");
    let n_ids = identities(&c, &fs, &mut g, &mut out);
    out.raw(",\"edges\":[");
    for (k, e) in g.edges.iter().enumerate() {
        out.raw(if k > 0 { "," } else { "" }).raw(&e.head);
        if !e.note.is_empty() {
            out.raw(",\"note\":").str(&e.note);
        }
        out.raw(",\"evidence\":").raw(&e.tail).raw("}");
    }
    let (ne, ni) = (g.edges.len() as i64, g.n_iin as i64);
    out.raw("],\"stats\":{\"features\":")
        .int(n as i64)
        .raw(",\"identities\":")
        .int(n_ids.0 as i64);
    out.raw(",\"grouped\":")
        .int(n_ids.1 as i64)
        .raw(",\"edges\":")
        .int(ne)
        .raw(",\"edges_mzmine\":")
        .int(ni);
    out.raw(",\"edges_computed\":")
        .int(ne - ni)
        .raw(",\"pairs_tested\":")
        .int(n_pairs as i64);
    out.raw(",\"apex_window_min\":")
        .num(o.apex_scans * o.cycle_s / 60.0)
        .raw("}}");
    Ok(out.0)
}

fn evidence_of(t: &Iimn, fs: &[Feat], i: usize, j: usize, pol: i32, s: usize) -> String {
    let (lo, up) = if fs[i].mz <= fs[j].mz { (i, j) } else { (j, i) };
    let (al, au) = (&fs[lo].areas[..s], &fs[up].areas[..s]);
    let e = EdgeIn {
        mz_lo: fs[lo].mz,
        mz_up: fs[up].mz,
        pol,
        ms2_lo: fs[lo].ms2.as_ref(),
        ms2_up: fs[up].ms2.as_ref(),
        areas: Some((al, au)),
        drt_min: Some((fs[up].rt - fs[lo].rt).abs()),
    };
    let mut o = Out::default();
    edge::edge_evidence(t, &e, &mut o);
    o.0
}

/// The connected components as identities; returns (identities, grouped).
fn identities(c: &Ctx, fs: &[Feat], g: &mut Graph, out: &mut Out) -> (usize, usize) {
    let n = fs.len();
    let mut comp: Vec<(usize, Vec<usize>)> = Vec::new();
    for i in 0..n {
        let r = g.dsu.find(i);
        match comp.iter_mut().find(|x| x.0 == r) {
            Some(x) => x.1.push(i),
            None => comp.push((r, vec![i])),
        }
    }
    let mut by_edge: std::collections::HashMap<usize, Vec<usize>> = Default::default();
    for (k, e) in g.edges.iter().enumerate() {
        by_edge.entry(g.dsu.find(e.a)).or_default().push(k);
    }
    let maxa: Vec<f64> = fs
        .iter()
        .map(|f| f.areas.iter().copied().fold(f64::NEG_INFINITY, f64::max))
        .collect();
    let mut comp: Vec<(f64, usize, Vec<usize>)> = comp
        .into_iter()
        .map(|(r, m)| (median(m.iter().map(|&i| fs[i].rt).collect()), r, m))
        .collect();
    comp.sortc_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    out.raw("[");
    let mut grouped = 0;
    for (k, (rt, r, members)) in comp.iter().enumerate() {
        let mut ms = members.clone();
        ms.sortc_by(|&a, &b| fs[a].mz.total_cmp(&fs[b].mz));
        grouped += (ms.len() > 1) as usize;
        let hyp = hypotheses(c, &ms, fs, &maxa);
        out.raw(if k > 0 { ",{\"id\":" } else { "{\"id\":" })
            .str(&format!("ID{}", k + 1))
            .raw(",\"members\":[");
        for (q, &i) in ms.iter().enumerate() {
            out.raw(if q > 0 { "," } else { "" });
            write_value(fs[i].v.get("k").unwrap_or(&Value::Null), out);
        }
        out.raw("],\"rt\":").num(*rt).raw(",\"neutral\":");
        hyp_out(hyp.first(), fs, out);
        out.raw(",\"alternatives\":[");
        for (q, h) in hyp.iter().skip(1).take(4).enumerate() {
            out.raw(if q > 0 { "," } else { "" });
            hyp_out(Some(h), fs, out);
        }
        out.raw("],\"edges\":[");
        for (q, e) in by_edge
            .get(r)
            .map(Vec::as_slice)
            .unwrap_or(&[])
            .iter()
            .enumerate()
        {
            out.raw(if q > 0 { "," } else { "" }).int(*e as i64);
        }
        let mut groups: Vec<&str> = ms.iter().filter_map(|&i| g.iin_of[i].as_deref()).collect();
        groups.sortc();
        groups.dedup();
        out.raw("],\"mzmine_groups\":[");
        for (q, s) in groups.iter().enumerate() {
            out.raw(if q > 0 { "," } else { "" }).str(s);
        }
        out.raw("]}");
    }
    out.raw("]");
    (comp.len(), grouped)
}
