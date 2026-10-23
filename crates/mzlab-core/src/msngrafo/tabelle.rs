//! The tables and the library text of a fragmentation graph (`msngrafo/1`) for the Flow steps `msn.table_export` and `msn.library_export`: the CSV of the nodes, of the
//! modifications (neutral losses between ions) or of the peaks, and the MSP / MassBank text of the tree. Used by the CLI and by the `.wasm` of the page (same bytes).

use super::lib_export::{export_text, Formato};
use crate::error::Result;
use crate::json::{Out, Value};

/// The library text of the graph `g` (written as `json`): the entry is wrapped as a one-entry `msnlib/1` and given to the writer of `msn-lib export`.
pub fn library_text(
    g: &Value,
    json: &str,
    (given, format, levels, clean, annotate): (&str, &str, &str, bool, bool),
) -> Result<String> {
    let ion = g
        .get("ione")
        .and_then(|i| i.get("formula")?.str())
        .unwrap_or("");
    let id = match (given, ion) {
        ("", "") => "MSN",
        ("", f) => f,
        (n, _) => n,
    };
    let mut o = Out::default();
    o.raw("{\"voci\":[{\"id\":").str(id);
    o.raw(",\"ione\":{\"formula\":").str(ion);
    o.raw("},\"grafo\":").raw(json.trim()).raw("}]}");
    let formato = Formato::parse(format)?;
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
    let name = f.and_then(|f| f.get("scelta")).and_then(Value::str);
    let ppm = f.and_then(|f| {
        f.get("candidati")?
            .arr()
            .iter()
            .find(|c| c.get("formula").and_then(Value::str) == name)?
            .get("ppm")
    });
    (cell(name.unwrap_or("")), num(ppm, 2))
}

/// The bond-break hypotheses of the node `id` in the key `rotture`: how many, and the formula of the best one.
fn breaks_of(g: &Value, id: Option<&str>) -> (String, String) {
    let hit = g
        .get("rotture")
        .and_then(|r| r.get("nodi"))
        .map_or(&[][..], |x| x.arr())
        .iter()
        .find(|n| n.get("nodo").and_then(Value::str) == id);
    let best = hit
        .and_then(|n| n.get("ipotesi"))
        .and_then(|i| i.arr().first())
        .and_then(|h| h.get("formula"));
    (num(hit.and_then(|n| n.get("n")), 0), text(best))
}

fn nodes_csv(g: &Value) -> String {
    let has_breaks = g.get("rotture").is_some();
    let mut t = String::from("node,level,mz,dev_ppm,observations,rel_max,formula,formula_ppm,formula_source,candidates,isolated,tree");
    t += if has_breaks {
        ",bond_break_hypotheses,bond_break_best_formula\n"
    } else {
        "\n"
    };
    for n in g.get("nodi").map_or(&[][..], |x| x.arr()) {
        let (formula, ppm) = formula_of(Some(n));
        let f = n.get("formula");
        let candidates = f
            .and_then(|f| f.get("candidati"))
            .map_or(0, |c| c.arr().len());
        let tree: Vec<&str> = n
            .get("albero")
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
            num(n.get("livello"), 0),
            num(n.get("mz"), 5),
            num(n.get("dev_ppm"), 2),
            num(n.get("osservazioni"), 0),
            num(n.get("rel_max"), 2),
            text(f.and_then(|f| f.get("fonte"))),
            n.get("isolato").and_then(Value::bool).unwrap_or(false),
            cell(&tree.join("|")),
        );
    }
    t
}

/// Extra cells of an edge: the named loss and its confidence (`interpretazione`), the bond-break flag (`rotture`).
fn edge_extras(g: &Value, a: &Value) -> String {
    let same = |x: &&Value| {
        x.get("da").and_then(Value::str) == a.get("da").and_then(Value::str)
            && x.get("a").and_then(Value::str) == a.get("a").and_then(Value::str)
    };
    let arc_of = |key: &str| {
        g.get(key)
            .and_then(|k| k.get("archi"))
            .and_then(|l| l.arr().iter().find(same))
    };
    let mut s = String::new();
    if g.get("interpretazione").is_some() {
        let l = arc_of("interpretazione").and_then(|x| x.get("perdita"));
        s += &format!(
            ",{},{}",
            text(l.and_then(|p| p.get("nome"))),
            text(l.and_then(|p| p.get("confidenza")))
        );
    }
    if g.get("rotture").is_some() {
        let e = arc_of("rotture")
            .and_then(|x| x.get("spiegato"))
            .and_then(Value::bool);
        s += &format!(",{}", e.unwrap_or(false));
    }
    s
}

fn modifications_csv(g: &Value) -> String {
    let mut t = String::from("from,from_mz,from_formula,to,to_mz,to_formula,loss_mass,loss_formula,loss_ppm,isolation_step,tree,level,activation,ce,scans,rel");
    if g.get("interpretazione").is_some() {
        t += ",loss_name,loss_confidence";
    }
    if g.get("rotture").is_some() {
        t += ",bond_break_explained";
    }
    t.push('\n');
    for a in g.get("archi").map_or(&[][..], |x| x.arr()) {
        let (from, to) = (
            find(g, "nodi", a.get("da").and_then(Value::str)),
            find(g, "nodi", a.get("a").and_then(Value::str)),
        );
        let loss = a.get("perdita");
        let head = format!(
            "{},{},{},{},{},{},{},{},{},{}",
            text(a.get("da")),
            num(from.and_then(|n| n.get("mz")), 5),
            formula_of(from).0,
            text(a.get("a")),
            num(to.and_then(|n| n.get("mz")), 5),
            formula_of(to).0,
            num(loss.and_then(|l| l.get("da_massa")), 5),
            text(loss.and_then(|l| l.get("formula"))),
            num(loss.and_then(|l| l.get("ppm")), 2),
            a.get("isolamento").and_then(Value::bool).unwrap_or(false),
        );
        let extra = edge_extras(g, a);
        let measures = a.get("misure").map_or(&[][..], |m| m.arr());
        if measures.is_empty() {
            t += &format!("{head},,,,,,{extra}\n");
        }
        for m in measures {
            t += &format!(
                "{head},{},{},{},{},{},{}{extra}\n",
                text(m.get("albero")),
                num(m.get("livello"), 0),
                text(m.get("att")),
                num(m.get("ce"), 1),
                num(m.get("n_scansioni"), 0),
                num(m.get("rel"), 2),
            );
        }
    }
    t
}

fn peaks_csv(g: &Value, clean: bool) -> String {
    let mut t =
        String::from("tree,level,activation,ce,precursor_mz,mz,rel,node,formula,formula_ppm\n");
    for s in g.get("spettri").map_or(&[][..], |x| x.arr()) {
        let pre = find(g, "nodi", s.get("precursore").and_then(Value::str));
        for p in s.get("picchi").map_or(&[][..], |x| x.arr()) {
            let node = p.get("nodo").and_then(Value::str);
            if clean && node.is_none() {
                continue;
            }
            let (formula, ppm) = formula_of(find(g, "nodi", node));
            t += &format!(
                "{},{},{},{},{},{},{},{},{formula},{ppm}\n",
                text(s.get("albero")),
                num(s.get("livello"), 0),
                text(s.get("att")),
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

    const GRAPH: &str = r#"{"ione":{"formula":"C8H11N4O2+"},"nodi":[{"id":"n0","livello":1,"mz":195.0877,"formula":{"scelta":"C8H11N4O2+","candidati":[{"formula":"C8H11N4O2+","ppm":0.5}]}},{"id":"n1","livello":2,"mz":138.0662,"formula":null}],"archi":[{"da":"n0","a":"n1","perdita":{"da_massa":57.0215,"formula":"CH3NCO","ppm":1.2},"misure":[]}],"spettri":[{"albero":"195@cid35","livello":2,"att":"cid","ce":35,"precursore":"n0","picchi":[{"mz":138.0662,"rel":100,"nodo":"n1"},{"mz":110.07,"rel":30,"nodo":null}]}]}"#;

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
            r#"],"archi""#,
            r#"],"rotture":{"nodi":[{"nodo":"n1","n":2,"ipotesi":[{"formula":"C7H8N3+"}]}],"archi":[{"da":"n0","a":"n1","spiegato":true}]},"interpretazione":{"archi":[{"da":"n0","a":"n1","perdita":{"nome":"methyl isocyanate","confidenza":"high"}}]},"archi""#,
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
