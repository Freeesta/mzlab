//! The fragments of a product against the supposed tree of the parent (`fragtree.compare_lr`, `_tally`): a fragment of the product within 0.5 Da of
//! a node of the parent is "same", within 0.5 Da of the node plus the mass change of the product "shifted".

use super::tree::LR_TOL;
use mzlab_core::json::Value;
use mzlab_core::jsonb::{int, text, write, Obj};

/// `compare_lr(tree, tp_ms2, dm)`: `tree` is the JSON of `lr_tree`, `tp_mz` the m/z of the fragments of the product, `dm` its mass change.
/// JSON text.
pub fn compare_lr(tree: &Value, tp_mz: &[f64], dm: Option<f64>) -> String {
    if tp_mz.is_empty() {
        return r#"{"ok":false,"note":"no MS2 of the product","marks":{}}"#.into();
    }
    let nodes = tree.get("nodes").map_or(&[][..], |v| v.arr());
    let mut marks = Obj::new();
    let (mut same, mut shifted) = (0, 0);
    for n in nodes {
        let (Some("frag"), Some(id), Some(mz)) = (
            n.get("kind").and_then(|v| v.str()),
            n.get("id").and_then(|v| v.str()),
            n.get("mz").and_then(|v| v.f64()),
        ) else {
            continue;
        };
        if tp_mz.iter().any(|f| (f - mz).abs() <= LR_TOL) {
            marks = marks.put(id, Obj::new().put("state", text("same")).done());
            same += 1;
        } else if let Some(d) = dm.filter(|d| tp_mz.iter().any(|f| (f - (mz + d)).abs() <= LR_TOL))
        {
            marks = marks.put(
                id,
                Obj::new()
                    .put("state", text("shifted"))
                    .put("delta", text(&format!("{d:+.2}")))
                    .done(),
            );
            shifted += 1;
        }
    }
    let n_nodes = nodes
        .iter()
        .filter(|n| {
            n.get("kind").and_then(|v| v.str()) == Some("frag")
                || n.get("id").and_then(|v| v.str()) != Some("MS1")
        })
        .count();
    let o = Obj::new()
        .b("ok", true)
        .put("marks", marks.done())
        .put("n_same", int(same))
        .put("n_shifted", int(shifted))
        .put("n_nodes", int(n_nodes as i64))
        .put("note", text(""));
    write(&o.done())
}
