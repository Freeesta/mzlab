//! Writer of `mzflow/2`: a `Plan` back to JSON text, the inverse of `parse` up to the order of the parameters. Parameters and facets in canonical form
//! (sorted keys, shortest numbers), one input and one step per line. Used by `mzlab-cli flow run` for the effective plan of a run (`plan.json`).

use super::canon::value_text;
use super::{FileInput, Instance, Plan, Ref};
use crate::json::{Out, Value};

fn reference(r: &Ref) -> String {
    match r {
        Ref::File(name) => format!("file:{name}"),
        Ref::Out { instance, port } => format!("{instance}.{port}"),
    }
}

fn input(o: &mut Out, f: &FileInput) {
    o.raw("{\"name\":").str(&f.name);
    if !f.sha256.is_empty() {
        o.raw(",\"sha256\":").str(&f.sha256);
    }
    if let Some(b) = f.bytes {
        o.raw(",\"bytes\":").num(b as f64);
    }
    if let Some(t) = &f.ty {
        o.raw(",\"type\":").str(t);
    }
    let ob = &f.observed;
    let mut facets: Vec<(String, Value)> = Vec::new();
    let nums = |v: Vec<f64>| Value::Arr(v.into_iter().map(Value::Num).collect());
    if !ob.resolution.is_empty() {
        let v = ob
            .resolution
            .iter()
            .map(|s| Value::Str(s.clone()))
            .collect();
        facets.push(("resolution".into(), Value::Arr(v)));
    }
    if !ob.polarity.is_empty() {
        facets.push((
            "polarity".into(),
            nums(ob.polarity.iter().map(|&p| f64::from(p)).collect()),
        ));
    }
    if !ob.ms_levels.is_empty() {
        facets.push((
            "ms_levels".into(),
            nums(ob.ms_levels.iter().map(|&l| f64::from(l)).collect()),
        ));
    }
    if !facets.is_empty() {
        o.raw(",\"facets\":").raw(&value_text(&Value::Obj(facets)));
    }
    o.raw("}");
}

fn step(o: &mut Out, s: &Instance) {
    o.raw("{\"id\":")
        .str(&s.id)
        .raw(",\"step\":")
        .str(&s.step_ref());
    for (key, v) in [
        ("algorithm", &s.algorithm),
        ("preset", &s.preset),
        ("label", &s.label),
    ] {
        if let Some(v) = v {
            o.raw(",\"").raw(key).raw("\":").str(v);
        }
    }
    if !s.ins.is_empty() {
        o.raw(",\"in\":{");
        for (k, (port, refs)) in s.ins.iter().enumerate() {
            o.raw(if k > 0 { "," } else { "" }).str(port).raw(":");
            if let [one] = refs.as_slice() {
                o.str(&reference(one));
            } else {
                o.raw("[");
                for (i, r) in refs.iter().enumerate() {
                    o.raw(if i > 0 { "," } else { "" }).str(&reference(r));
                }
                o.raw("]");
            }
        }
        o.raw("}");
    }
    if !s.params.is_empty() {
        o.raw(",\"params\":")
            .raw(&value_text(&Value::Obj(s.params.clone())));
    }
    o.raw("}");
}

/// The `mzflow/2` text of `plan`.
pub fn plan_json(plan: &Plan) -> String {
    let mut o = Out::default();
    o.raw("{\"format\":").str(&plan.format);
    let p = &plan.program;
    if !p.name.is_empty() || !p.version.is_empty() || p.commit.is_some() {
        o.raw(",\"program\":{\"name\":")
            .str(&p.name)
            .raw(",\"version\":")
            .str(&p.version);
        if let Some(c) = &p.commit {
            o.raw(",\"commit\":").str(c);
        }
        o.raw("}");
    }
    o.raw(",\n\"inputs\":[");
    for (k, f) in plan.inputs.iter().enumerate() {
        o.raw(if k > 0 { ",\n " } else { "\n " });
        input(&mut o, f);
    }
    o.raw("],\n\"steps\":[");
    for (k, s) in plan.steps.iter().enumerate() {
        o.raw(if k > 0 { ",\n " } else { "\n " });
        step(&mut o, s);
    }
    o.raw("]");
    if let Some(ops) = &plan.operations {
        o.raw(",\n\"operations\":").raw(&value_text(ops));
    }
    o.raw("}\n");
    o.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flow::parse;

    #[test]
    fn writes_back_what_parse_reads() {
        let text = br#"{"format":"mzflow/2","program":{"name":"mzLab","version":"1","commit":"c0"},
          "inputs":[{"name":"a.raw","sha256":"ab","bytes":3,"type":"raw","facets":{"resolution":["hr"],"polarity":[1,-1],"ms_levels":[1,2]}},{"name":"b \"x\".mzML"}],
          "steps":[{"id":"conv","step":"convert.mzml@1","in":{"raw":"file:a.raw"},"params":{"zlib_level":1e0,"numpress":true}},
                   {"id":"idx","step":"index.mzidx@2","label":"Index","in":{"source":["conv.mzml","file:b \"x\".mzML"]},"algorithm":"x","preset":"y"}]}"#;
        let mut plan = parse(text).unwrap();
        let out = plan_json(&plan);
        plan.steps[0].params.reverse();
        assert_eq!(parse(out.as_bytes()).unwrap(), plan);
        assert!(
            out.contains(r#""params":{"numpress":true,"zlib_level":1}"#),
            "{out}"
        );
        assert_eq!(plan_json(&parse(out.as_bytes()).unwrap()), out);
    }
}
