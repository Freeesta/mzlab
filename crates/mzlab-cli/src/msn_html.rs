//! The stand-alone HTML of a fragmentation graph `msngrafo/1`: one file, no external resource. It holds the graph (JSON), the drawer `msngrafo-vista.js` of the
//! page (the same code, `include_str!`), the page script `msn_html_pagina.js`, and both static figures (the SVG of the CLI, for the downloads). Native only.

use mzlab_core::error::{Error, Result};

use crate::msn_figura::{molecola_svg, svg, Vista};

const MODELLO: &str = include_str!("msn_html.html");
const VISTA_JS: &str = include_str!("../../../mzlab/web/msngrafo-vista.js");
const PAGINA_JS: &str = include_str!("msn_html_pagina.js");

/// The drawer as classic script text: the `export` keywords go (the page is not a module), the rest is the file as it is.
fn vista_js() -> String {
    VISTA_JS
        .lines()
        .map(|l| l.strip_prefix("export ").unwrap_or(l))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Text that can sit inside a `<script>` element: no closing tag and no comment opener can end it early. The `\/` and `\u0021` are the same character in JSON
/// and in a JavaScript string; the SVG and the molecule never hold either of the two (their markup has no script tag, no comment).
fn in_script(t: &str) -> String {
    t.replace("</script", "<\\/script")
        .replace("</SCRIPT", "<\\/SCRIPT")
        .replace("<!--", "<\\u0021--")
}

/// The HTML of the graph `graph` (the key `vista` is added when missing), opened on the view `vista`.
pub fn html(graph: &str, vista: Vista) -> Result<String> {
    let done =
        mzlab_core::msngrafo::completa(graph).map_err(|_| Error::new("err.msnfigura.bad"))?;
    let (albero, radiale) = (svg(&done, Vista::Albero)?, svg(&done, Vista::Radiale)?);
    let root = molecola_svg(&done)?.unwrap_or_default();
    let apri = if vista == Vista::Radiale {
        "radial"
    } else {
        "tree"
    };
    let page = format!("{PAGINA_JS}\nsetMode(\"{apri}\");");
    // the pieces go in one by one with the bytes of the template between them: a marker inside a piece is not looked for again
    let mut out = String::new();
    let mut rest = MODELLO;
    for (marker, text) in [
        ("/*DATA*/", in_script(done.trim_end())),
        ("/*ROOT*/", in_script(&root)),
        ("/*SVG_TREE*/", in_script(&albero)),
        ("/*SVG_RADIAL*/", in_script(&radiale)),
        ("/*VISTA_JS*/", in_script(&vista_js())),
        ("/*PAGE_JS*/", in_script(&page)),
    ] {
        let (a, b) = rest
            .split_once(marker)
            .ok_or_else(|| Error::new("err.msnfigura.bad"))?;
        out += a;
        out += &text;
        rest = b;
    }
    out += rest;
    Ok(out)
}
