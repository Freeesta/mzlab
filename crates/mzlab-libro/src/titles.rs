//! Retitled sections: what the Teoria editor allows when the headings of a chapter change. Only the TEXT of h2/h3 titles may change, and every section keeps its
//! anchor (an explicit `{#id}`, or the one it has now written out), so links to it and the rows of the search index (rebuilt by `index.rs`) stay right.
use crate::index::heading_anchor;
use crate::inline::{render, Ctx};
use crate::scan::{explicit_id, head};
use crate::splitlines;

/// The headings of the source; lines inside a raw ```html block are HTML, not headings (the converter skips them the same way).
fn headings(md: &str) -> Vec<(usize, String, String)> {
    let lines = splitlines(md);
    let mut out = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let l = lines[i];
        if l.trim_end() == "```html" {
            if let Some(j) = (i + 1..lines.len()).find(|&k| lines[k].trim_end() == "```") {
                i = j + 1;
                continue;
            }
        }
        if let Some((level, text)) = head(l) {
            out.push((level, text, l.trim_end().to_string()));
        }
        i += 1;
    }
    out
}

/// Whether the headings differ in the text of some h2/h3 title (the search index then has to be rebuilt), or why the change is not allowed.
pub fn retitled(old: &str, new: &str, name: &str) -> Result<bool, String> {
    let (a, b) = (headings(old), headings(new));
    if a.len() != b.len() {
        return Err("A section was added, removed or moved: the search index only follows retitled sections, so make that change with a normal pull request.".into());
    }
    let mut changed = false;
    for (o, n) in a.iter().zip(&b) {
        if o.2 == n.2 {
            continue;
        }
        if o.0 != n.0 {
            return Err(
                "A heading changed level: make that change with a normal pull request.".into(),
            );
        }
        if o.0 == 1 {
            return Err("The title of the chapter cannot be changed here: make that change with a normal pull request.".into());
        }
        check_anchor(o, n, name)?;
        changed = true;
    }
    if changed && name.starts_with("21-") {
        return Err("The glossary has rows of its own in the search index: its titles cannot be changed here.".into());
    }
    Ok(changed)
}

fn check_anchor(
    o: &(usize, String, String),
    n: &(usize, String, String),
    name: &str,
) -> Result<(), String> {
    let (old_id, new_id) = (explicit_id(&o.1), explicit_id(&n.1));
    if let Some((_, id)) = &old_id {
        return match &new_id {
            Some((_, nid)) if nid == id => Ok(()),
            _ => Err(format!(
                "The anchor {{#{id}}} of a section must stay: links point to it."
            )),
        };
    }
    // no fixed anchor yet: it is made from the old text at run time, so the new line has to write it out
    let text = &o.1;
    let html = render(text, name, 0, &mut Ctx::default()).map_err(|e| e.to_string())?;
    let anchor = heading_anchor(&html)?;
    match new_id {
        Some((_, nid)) if nid == anchor => Ok(()),
        _ => Err(format!("This section has no fixed anchor: end its title with {{#{anchor}}} so links to it keep working.")),
    }
}
