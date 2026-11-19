//! The tables and the library text of a fragmentation graph (`msn-graph/1`) for the Flow steps `msn.table_export` and `msn.library_export`: the CSV of the nodes, of the
//! modifications (neutral losses between ions) or of the peaks, and the MSP / MassBank text of the tree. Used by the CLI and by the `.wasm` of the page (same bytes).

use super::lib_export::{export_text, Format};
use crate::error::Result;
use crate::json::{Out, Value};

/// The library text of the graph `g` (written as `json`): the entry is wrapped as a one-entry `msn-lib/1` and given to the writer of `msn lib export`.
pub fn library_text(
    g: &Value,
    json: &str,
    (given, format, levels, clean, annotate): (&str, &str, &str, bool, bool),
) -> Result<String> {
    let ion = g
        .get("ion")
        .and_then(|i| i.get("formula")?.str())
        .unwrap_or("");
    let id = match (given, ion) {
        ("", "") => "MSN",
        ("", f) => f,
        (n, _) => n,
    };
    let mut o = Out::default();
    o.raw("{\"entries\":[{\"id\":").str(id);
    o.raw(",\"ion\":{\"formula\":").str(ion);
    o.raw("},\"graph\":").raw(json.trim()).raw("}]}");
    let formato = Format::parse(format)?;
    let all = levels != "ms2";
    export_text(&o.0, formato, (clean, all, annotate))
}

fn cell(s: &str) -> String {
    if s.contains([',', '"', '\n']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn num(v: Option<&Value>, places: usize) -> String {
    v.and_then(Value::f64)
        .map_or(String::new(), |x| format!("{x:.places$}"))
}

fn text(v: Option<&Value>) -> String {
    cell(v.and_then(Value::str).unwrap_or(""))
}

fn find<'a>(g: &'a Value, list: &str, id: Option<&str>) -> Option<&'a Value> {
    g.get(list)?
        .arr()
        .iter()
        .find(|n| n.get("id").and_then(Value::str) == id)
}

/// The chosen sub-formula of a node and its error in ppm.
fn formula_of(n: Option<&Value>) -> (String, String) {
    let f = n.and_then(|n| n.get("formula"));
    let name = f.and_then(|f| f.get("choice")).and_then(Value::str);
    let ppm = f.and_then(|f| {
        f.get("candidates")?
            .arr()
            .iter()
            .find(|c| c.get("formula").and_then(Value::str) == name)?
            .get("ppm")
    });
    (cell(name.unwrap_or("")), num(ppm, 2))
}

/// The bond-break hypotheses of the node `id` in the key `breaks`: how many, and the formula of the best one.
fn breaks_of(g: &Value, id: Option<&str>) -> (String, String) {
    let hit = g
        .get("breaks")
        .and_then(|r| r.get("nodes"))
        .map_or(&[][..], |x| x.arr())
        .iter()
        .find(|n| n.get("node").and_then(Value::str) == id);
    let best = hit
        .and_then(|n| n.get("hypotheses"))
        .and_then(|i| i.arr().first())
        .and_then(|h| h.get("formula"));
    (num(hit.and_then(|n| n.get("n")), 0), text(best))
}

fn nodes_csv(g: &Value) -> String {
    let has_breaks = g.get("breaks").is_some();
    let mut t = String::from("node,level,mz,dev_ppm,observations,rel_max,formula,formula_ppm,formula_source,candidates,isolated,tree");
    t += if has_breaks {
        ",bond_break_hypotheses,bond_break_best_formula\n"
    } else {
        "\n"
    };
    for n in g.get("nodes").map_or(&[][..], |x| x.arr()) {
        let (formula, ppm) = formula_of(Some(n));
        let f = n.get("formula");
        let candidates = f
            .and_then(|f| f.get("candidates"))
            .map_or(0, |c| c.arr().len());
        let tree: Vec<&str> = n
            .get("tree")
            .map_or(&[][..], |a| a.arr())
            .iter()
            .filter_map(Value::str)
            .collect();
        let extra = if has_breaks {
            let (k, best) = breaks_of(g, n.get("id").and_then(Value::str));
            format!(",{},{best}", if k.is_empty() { "0" } else { &k })
        } else {
            String::new()
        };
        t += &format!(
            "{},{},{},{},{},{},{formula},{ppm},{},{candidates},{},{}{extra}\n",
            text(n.get("id")),
            num(n.get("level"), 0),
            num(n.get("mz"), 5),
            num(n.get("dev_ppm"), 2),
            num(n.get("observations"), 0),
            num(n.get("rel_max"), 2),
            text(f.and_then(|f| f.get("source"))),
            n.get("isolated").and_then(Value::bool).unwrap_or(false),
            cell(&tree.join("|")),
        );
    }
    t
}

/// Extra cells of an edge: the named loss and its confidence (`interpretation`), the bond-break flag (`breaks`).
fn edge_extras(g: &Value, a: &Value) -> String {
    let same = |x: &&Value| {
        x.get("from").and_then(Value::str) == a.get("from").and_then(Value::str)
            && x.get("to").and_then(Value::str) == a.get("to").and_then(Value::str)
    };
    let arc_of = |key: &str| {
        g.get(key)
            .and_then(|k| k.get("edges"))
            .and_then(|l| l.arr().iter().find(same))
    };
    let mut s = String::new();
    if g.get("interpretation").is_some() {
        let l = arc_of("interpretation").and_then(|x| x.get("loss"));
        s += &format!(
            ",{},{}",
            text(l.and_then(|p| p.get("name"))),
            text(l.and_then(|p| p.get("confidence")))
        );
    }
    if g.get("breaks").is_some() {
        let e = arc_of("breaks")
            .and_then(|x| x.get("explained"))
            .and_then(Value::bool);
        s += &format!(",{}", e.unwrap_or(false));
    }
    s
}

fn modifications_csv(g: &Value) -> String {
    let mut t = String::from("from,from_mz,from_formula,to,to_mz,to_formula,loss_mass,loss_formula,loss_ppm,isolation_step,tree,level,activation,ce,scans,rel");
    if g.get("interpretation").is_some() {
        t += ",loss_name,loss_confidence";
    }
    if g.get("breaks").is_some() {
        t += ",bond_break_explained";
    }
    t.push('\n');
    for a in g.get("edges").map_or(&[][..], |x| x.arr()) {
        let (from, to) = (
            find(g, "nodes", a.get("from").and_then(Value::str)),
            find(g, "nodes", a.get("to").and_then(Value::str)),
        );
        let loss = a.get("loss");
        let head = format!(
            "{},{},{},{},{},{},{},{},{},{}",
            text(a.get("from")),
            num(from.and_then(|n| n.get("mz")), 5),
            formula_of(from).0,
            text(a.get("to")),
            num(to.and_then(|n| n.get("mz")), 5),
            formula_of(to).0,
            num(loss.and_then(|l| l.get("da_mass")), 5),
            text(loss.and_then(|l| l.get("formula"))),
            num(loss.and_then(|l| l.get("ppm")), 2),
            a.get("isolation").and_then(Value::bool).unwrap_or(false),
        );
        let extra = edge_extras(g, a);
        let measures = a.get("measurements").map_or(&[][..], |m| m.arr());
        if measures.is_empty() {
            t += &format!("{head},,,,,,{extra}\n");
        }
        for m in measures {
            t += &format!(
                "{head},{},{},{},{},{},{}{extra}\n",
                text(m.get("tree")),
                num(m.get("level"), 0),
                text(m.get("activation")),
                num(m.get("ce"), 1),
                num(m.get("n_scans"), 0),
                num(m.get("rel"), 2),
            );
        }
    }
    t
}

fn peaks_csv(g: &Value, clean: bool) -> String {
    let mut t =
        String::from("tree,level,activation,ce,precursor_mz,mz,rel,node,formula,formula_ppm\n");
    for s in g.get("spectra").map_or(&[][..], |x| x.arr()) {
        let pre = find(g, "nodes", s.get("precursor").and_then(Value::str));
        for p in s.get("peaks").map_or(&[][..], |x| x.arr()) {
            let node = p.get("node").and_then(Value::str);
            if clean && node.is_none() {
                continue;
            }
            let (formula, ppm) = formula_of(find(g, "nodes", node));
            t += &format!(
                "{},{},{},{},{},{},{},{},{formula},{ppm}\n",
                text(s.get("tree")),
                num(s.get("level"), 0),
                text(s.get("activation")),
                num(s.get("ce"), 1),
                num(pre.and_then(|n| n.get("mz")), 5),
                num(p.get("mz"), 5),
                num(p.get("rel"), 2),
                cell(node.unwrap_or("")),
            );
        }
    }
    t
}

/// The CSV text of the table `view` of the graph `g`.
pub fn table_text(g: &Value, view: &str, clean: bool) -> String {
    match view {
        "nodes" => nodes_csv(g),
        "peaks" => peaks_csv(g, clean),
        _ => modifications_csv(g),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json::parse;

    const GRAPH: &str = r#"{"ion":{"formula":"C8H11N4O2+"},"nodes":[{"id":"n0","level":1,"mz":195.0877,"formula":{"choice":"C8H11N4O2+","candidates":[{"formula":"C8H11N4O2+","ppm":0.5}]}},{"id":"n1","level":2,"mz":138.0662,"formula":null}],"edges":[{"from":"n0","to":"n1","loss":{"da_mass":57.0215,"formula":"CH3NCO","ppm":1.2},"measurements":[]}],"spectra":[{"tree":"195@cid35","level":2,"activation":"cid","ce":35,"precursor":"n0","peaks":[{"mz":138.0662,"rel":100,"node":"n1"},{"mz":110.07,"rel":30,"node":null}]}]}"#;

    #[test]
    fn the_tables_and_the_library_come_from_one_graph_text() {
        let g = parse(GRAPH.as_bytes()).unwrap();
        let nodes = table_text(&g, "nodes", false);
        assert!(nodes.starts_with("node,level,mz,"));
        assert!(nodes.contains("n0,1,195.08770,"));
        let mods = table_text(&g, "modifications", false);
        assert!(mods.contains("n0,195.08770,C8H11N4O2+,n1,138.06620,,57.02150,CH3NCO,1.20,"));
        assert_eq!(
            table_text(&g, "peaks", true).lines().count(),
            2,
            "the clean table keeps the explained peak only"
        );
        let msp = library_text(&g, GRAPH, ("", "msp", "all", false, true)).unwrap();
        assert!(msp.contains("Name: C8H11N4O2+") && msp.contains("138.0662"));
        assert!(
            library_text(&g, GRAPH, ("STD", "massbank", "ms2", true, false))
                .unwrap()
                .contains("STD")
        );
        assert!(library_text(&g, GRAPH, ("", "pdf", "all", false, true)).is_err());
    }

    #[test]
    fn the_optional_keys_add_columns_only_when_present() {
        let with = GRAPH.replacen(
            r#"],"edges""#,
            r#"],"breaks":{"nodes":[{"node":"n1","n":2,"hypotheses":[{"formula":"C7H8N3+"}]}],"edges":[{"from":"n0","to":"n1","explained":true}]},"interpretation":{"edges":[{"from":"n0","to":"n1","loss":{"name":"methyl isocyanate","confidence":"high"}}]},"edges""#,
            1,
        );
        let g = parse(with.as_bytes()).unwrap();
        let nodes = table_text(&g, "nodes", false);
        assert!(nodes
            .lines()
            .next()
            .unwrap()
            .ends_with("bond_break_best_formula"));
        assert!(nodes.contains(",2,C7H8N3+"));
        let mods = table_text(&g, "modifications", false);
        assert!(mods
            .lines()
            .next()
            .unwrap()
            .ends_with("loss_name,loss_confidence,bond_break_explained"));
        assert!(mods.contains(",,,,,,methyl isocyanate,high,true"));
    }
}
