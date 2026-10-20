//! Editing from the preview: the page is built with `data-src` on the blocks that can be edited (a simple paragraph, a one-line list item, a table cell);
//! an edited block comes back as inline HTML, goes back into the source at its lines and is accepted only if the converter then makes the same HTML from it.
//! Used by the Teoria editor through the `.wasm` (`crates/mzlab-libro-wasm`).
use crate::frag::{canon, parse, plain, to_md};
use crate::page::{convert, Options};
use crate::scan::item;
use crate::splitlines;

const CANT: &str = "This text cannot be written in the source here without changing its meaning: edit it in the source box.";

/// The page for the preview: like the real one, plus `data-src` on the editable blocks.
pub fn preview(text: &str, name: &str, opt: &Options) -> Result<String, String> {
    convert(
        text,
        name,
        &Options {
            annotate: true,
            ..*opt
        },
    )
    .map_err(|e| e.to_string())
}

/// The source with the block `src` (`a-b` lines of a paragraph, `n` a list item, `n:c` cell c of the table row on line n) replaced by `new_html`.
pub fn edit_block(
    text: &str,
    name: &str,
    opt: &Options,
    src: &str,
    new_html: &str,
) -> Result<String, String> {
    let nodes = parse(new_html)?;
    let mut md = to_md(&nodes)?;
    if plain(&nodes).trim().is_empty() {
        return Err("A block cannot be left empty.".into());
    }
    if md.contains('\n') || md.starts_with("{.") {
        return Err(CANT.into());
    }
    let lines = splitlines(text);
    if !text.contains('\u{a0}') {
        md = md.replace('\u{a0}', " ");
    }
    let (new_lines, key) = replace(&lines, src, &md)?;
    let mut new_text = new_lines.join("\n");
    if text.ends_with('\n') {
        new_text.push('\n');
    }
    let page = preview(&new_text, name, opt).map_err(|_| CANT.to_string())?;
    let inner = find_inner(&page, &key).ok_or_else(|| CANT.to_string())?;
    if canon(&parse(&inner)?) != canon(&nodes) {
        return Err(CANT.into());
    }
    Ok(new_text)
}

fn num(s: &str) -> Result<usize, String> {
    s.parse::<usize>()
        .ok()
        .filter(|&n| n >= 1)
        .ok_or_else(|| "Unknown block.".to_string())
}

/// The lines with the block replaced, and the `data-src` key the block has afterwards.
fn replace(lines: &[&str], src: &str, md: &str) -> Result<(Vec<String>, String), String> {
    let mut out: Vec<String> = lines.iter().map(|l| l.to_string()).collect();
    let at = |n: usize| {
        lines
            .get(n - 1)
            .copied()
            .ok_or_else(|| "Unknown block.".to_string())
    };
    if let Some((l, c)) = src.split_once(':') {
        let (l, c) = (
            num(l)?,
            c.parse::<usize>()
                .map_err(|_| "Unknown block.".to_string())?,
        );
        if md.contains('|') {
            return Err("A table cell cannot contain the | character.".into());
        }
        out[l - 1] = cell_line(at(l)?, c, md)?;
        return Ok((out, src.to_string()));
    }
    if let Some((a, b)) = src.split_once('-') {
        let (a, b) = (num(a)?, num(b)?);
        let first = at(a)?;
        let prefix = ["{.lead} ", "{.note} "]
            .into_iter()
            .find(|p| first.starts_with(p))
            .unwrap_or("");
        out.splice(a - 1..b.min(lines.len()), [format!("{prefix}{md}")]);
        return Ok((out, format!("{a}-{a}")));
    }
    let l = num(src)?;
    let line = at(l)?;
    let (_, rest) = item(line).ok_or_else(|| "Unknown block.".to_string())?;
    out[l - 1] = format!("{}{md}", &line[..line.len() - rest.len()]);
    Ok((out, src.to_string()))
}

/// The row with the text of cell `c` replaced, the padding of the cell kept.
fn cell_line(line: &str, c: usize, md: &str) -> Result<String, String> {
    let pipes: Vec<usize> = line.match_indices('|').map(|p| p.0).collect();
    let (Some(&a), Some(&b)) = (pipes.get(c), pipes.get(c + 1)) else {
        return Err("Unknown block.".into());
    };
    let old = &line[a + 1..b];
    let lead = &old[..old.len() - old.trim_start().len()];
    let trail = &old[old.trim_end().len()..];
    Ok(format!("{}{lead}{md}{trail}{}", &line[..a + 1], &line[b..]))
}

/// The inner HTML of the element that carries `data-src="key"`.
fn find_inner(page: &str, key: &str) -> Option<String> {
    let mark = format!("data-src=\"{key}\"");
    let at = page.find(&mark)?;
    let open = page[..at].rfind('<')?;
    let tag: String = page[open + 1..]
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric())
        .collect();
    let start = at + page[at..].find('>')? + 1;
    let end = start + page[start..].find(&format!("</{tag}>"))?;
    Some(page[start..end].to_string())
}
