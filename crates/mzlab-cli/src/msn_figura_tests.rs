//! The figures of the fragmentation graph: the golden SVGs of a synthetic graph (`tests/msn_figura/`, `MZLAB_UPDATE_FIGURA=1` rewrites them), the HTML with
//! nothing outside it, and the same bytes from the command, `msn-graph --figura`, `msn-graph figure` and the step `msn.figure@1`.

use std::path::PathBuf;

use crate::msn_figura::{svg, testi, Vista};
use crate::msn_html::html;

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/msn_figura")
}

fn graph() -> String {
    std::fs::read_to_string(fixtures().join("g.json")).unwrap()
}

fn cli(args: &[&str]) -> String {
    let a: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let o = crate::execute(&a);
    assert_eq!(o.code, 0, "{}", o.stderr);
    String::from_utf8(o.stdout).unwrap()
}

fn golden(name: &str, got: &str) {
    let p = fixtures().join(name);
    if std::env::var_os("MZLAB_UPDATE_FIGURA").is_some() {
        std::fs::write(&p, got).unwrap();
    }
    assert_eq!(
        std::fs::read_to_string(&p).unwrap(),
        got,
        "{name} is stale: MZLAB_UPDATE_FIGURA=1 cargo test -p mzlab-cli --lib msn_figura"
    );
}

#[test]
fn the_svg_of_both_views_is_the_golden_one() {
    golden("albero.svg", &svg(&graph(), Vista::Albero).unwrap());
    golden("radiale.svg", &svg(&graph(), Vista::Radiale).unwrap());
    golden(
        "testi-albero.txt",
        &(testi(&graph(), Vista::Albero).unwrap().join("\n") + "\n"),
    );
    golden(
        "testi-radiale.txt",
        &(testi(&graph(), Vista::Radiale).unwrap().join("\n") + "\n"),
    );
    // a graph with ambiguous ions, a loss named by its mass, a rule label, a common ion and the negative polarity
    let g2 = std::fs::read_to_string(fixtures().join("g2.json")).unwrap();
    for (v, n) in [(Vista::Albero, "albero"), (Vista::Radiale, "radiale")] {
        golden(&format!("g2-{n}.svg"), &svg(&g2, v).unwrap());
        golden(
            &format!("g2-testi-{n}.txt"),
            &(testi(&g2, v).unwrap().join("\n") + "\n"),
        );
    }
    let s = svg(&g2, Vista::Albero).unwrap();
    assert!(
        s.contains("+2 more")
            && s.contains("compatible with ")
            && s.contains("R1 (weak), R2 +1")
            && s.contains("×2")
            && s.contains("known loss MSn")
    );
    assert!(s.contains("Several candidates") && s.contains("aria-label=\"m/z 217.1200, levels MS2, 23.4 % of the base peak, 6 candidate formulas\""));
}

#[test]
fn the_svg_lives_alone_and_says_what_it_is() {
    let s = svg(&graph(), Vista::Albero).unwrap();
    // colours written in the file, no style sheet, no class, nothing from outside
    assert!(
        s.starts_with("<svg xmlns=\"http://www.w3.org/2000/svg\"")
            && s.contains("<title id=\"t\">")
            && s.contains("<desc id=\"d\">")
    );
    assert!(
        s.contains("#0072B2")
            && !s.contains("class=")
            && !s.contains("<style")
            && !s.contains("href")
    );
    // subscripts and the charge as tspans, the plain form in the accessible name
    assert!(s.contains("baseline-shift=\"sub\"") && s.contains("baseline-shift=\"super\""));
    assert!(
        s.contains("aria-label=\"m/z 305.1000, levels MS1, formula C14H15N3O5+\""),
        "{s}"
    );
    // the root draws its molecule from the molfile of the graph
    assert!(s.contains("<line "));
    // a graph with a SMILES but no molfile says so; one with neither draws nothing
    let nomol = graph()
        .replace(r#""molfile":"#, r#""molfile_":"#)
        .replace(r#""smiles":null"#, r#""smiles":"CCO""#);
    assert!(svg(&nomol, Vista::Albero)
        .unwrap()
        .contains("no 2D coordinates"));
    assert!(!svg(
        &graph().replace(r#""molfile":"#, r#""molfile_":"#),
        Vista::Albero
    )
    .unwrap()
    .contains("no 2D coordinates"));
}

#[test]
fn the_html_is_one_file_with_nothing_outside() {
    let h = html(&graph(), Vista::Radiale).unwrap();
    assert!(h.len() < 2_000_000);
    for bad in ["src=", "<link", "@import", "http://www.w3.org/1999/xlink"] {
        assert!(!h.contains(bad), "{bad}");
    }
    // no `url(` but the markers of the figure (`url(#h-b)`)
    assert!(h
        .match_indices("url(")
        .all(|(i, _)| h[i + 4..].starts_with('#')));
    // the only addresses are the XML namespaces of the SVG
    let urls: Vec<&str> = h
        .match_indices("http")
        .map(|(i, _)| &h[i..(i + 30).min(h.len())])
        .collect();
    assert!(
        urls.iter()
            .all(|u| u.starts_with("http://www.w3.org/2000/svg")),
        "{urls:?}"
    );
    assert!(
        h.contains("setMode(\"radial\")")
            && h.contains("function drawVista")
            && !h.contains("\nexport ")
    );
    // the graph is inside, and the text of a script cannot end its element early
    assert!(h.contains("\"formato\":\"msngrafo/1\""));
    let end = html(&graph().replace("msn.mzML", "a</script><b>"), Vista::Albero).unwrap();
    // the five script elements of the page, no more
    assert_eq!(end.matches("</script>").count(), 5);
}

#[test]
fn a_graph_without_vista_is_completed_first() {
    let j = graph();
    let at = j.rfind(",\"vista\":").unwrap();
    let old = format!("{}}}", &j[..at]);
    assert_eq!(
        svg(&old, Vista::Radiale).unwrap(),
        svg(&j, Vista::Radiale).unwrap()
    );
    assert!(svg("{}", Vista::Albero).is_err());
}

fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("mzlab_msnfig_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn the_command_and_figure_write_the_same_bytes() {
    let d = dir("cmd");
    let msn = fixtures().join("../golden/ref/msntree_rotta/msn.mzML");
    let (g, mol) = (d.join("g.json"), fixtures().join("molecola.mol"));
    let made = cli(&[
        "msn-graph",
        msn.to_str().unwrap(),
        "--formula",
        "C14H15N3O5+",
        "--molfile",
        mol.to_str().unwrap(),
        "--out",
        g.to_str().unwrap(),
        "--figura",
        "svg,html",
        "--vista",
        "radiale",
    ]);
    assert_eq!(made, format!("{0}/g.svg\n{0}/g.html\n", d.display()));
    // `figure` on the graph the command wrote
    let other = d.join("o");
    cli(&[
        "msn-graph",
        "figure",
        g.to_str().unwrap(),
        "--vista",
        "radiale",
        "--out",
        other.to_str().unwrap(),
    ]);
    for e in ["svg", "html"] {
        assert_eq!(
            std::fs::read(d.join(format!("g.{e}"))).unwrap(),
            std::fs::read(d.join(format!("o.{e}"))).unwrap()
        );
    }
    let text = std::fs::read_to_string(&g).unwrap();
    assert_eq!(
        std::fs::read_to_string(d.join("g.svg")).unwrap(),
        svg(&text, Vista::Radiale).unwrap()
    );
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_png_needs_its_feature() {
    let d = dir("png");
    let r = crate::execute(&[
        "msn-graph".to_string(),
        "figure".to_string(),
        fixtures().join("g.json").display().to_string(),
        "--figura".to_string(),
        "png".to_string(),
        "--out".to_string(),
        d.join("x").display().to_string(),
    ]);
    if cfg!(feature = "png") {
        assert_eq!(r.code, 0, "{}", r.stderr);
        assert_eq!(
            &std::fs::read(d.join("x.png")).unwrap()[..8],
            b"\x89PNG\r\n\x1a\n"
        );
    } else {
        assert_eq!(r.code, 2);
        assert!(
            r.stderr.contains("err.msnfigura.png") && r.stderr.contains("--features png"),
            "{}",
            r.stderr
        );
    }
}

#[test]
fn the_arguments_are_checked() {
    for a in [
        &["msn-graph", "f.mzML", "--figura", "svg"][..],
        &["msn-graph", "f.mzML", "--figura", "gif", "--out", "x"],
        &["msn-graph", "f.mzML", "--vista", "cubo"],
    ] {
        let o = crate::execute(&a.iter().map(|s| s.to_string()).collect::<Vec<_>>());
        assert_eq!(o.code, 2, "{a:?}");
    }
}

/// The step of the Flow writes the bytes of `msn-graph figure`.
#[test]
fn the_step_writes_the_bytes_of_the_command() {
    let d = dir("step");
    std::fs::copy(
        fixtures().join("../golden/ref/msntree_rotta/msn.mzML"),
        d.join("msn.mzML"),
    )
    .unwrap();
    let mol = std::fs::read_to_string(fixtures().join("molecola.mol")).unwrap();
    let run = |format: &str, view: &str| {
        let plan = serde_json::json!({"format":"mzflow/2","inputs":[{"name":"msn.mzML","type":"mzml"}],"steps":[
            {"id":"g","step":"spectra.msn_graph@1","in":{"source":"file:msn.mzML"},"params":{"formula":"C14H15N3O5+","molfile":mol}},
            {"id":"f","step":"msn.figure@1","in":{"graph":"g.graph"},"params":{"format":format,"view":view}}]});
        let _ = std::fs::remove_dir_all(d.join("work"));
        let (r, prov) =
            crate::flow::run_plan(plan.to_string().as_bytes(), &d, &d.join("work"), 1, None)
                .unwrap();
        assert!(
            r.skipped.is_empty() && !prov.contains("\"error\""),
            "{prov}"
        );
        (
            std::fs::read(d.join("work/g.json")).unwrap(),
            std::fs::read(d.join(format!("work/f.{format}"))).unwrap(),
        )
    };
    for (format, view) in [("svg", "tree"), ("svg", "radial"), ("html", "tree")] {
        let (g, got) = run(format, view);
        let want = crate::msn_uscite::bytes(
            std::str::from_utf8(&g).unwrap(),
            crate::msn_uscite::Formato::da_nome(format).unwrap(),
            Vista::da_nome(view).unwrap(),
        )
        .unwrap();
        assert_eq!(got, want, "{format} {view}");
    }
    // the graph of the step has the molecule, so its figure draws it
    assert!(String::from_utf8(run("svg", "tree").1)
        .unwrap()
        .contains("<line "));
    let _ = std::fs::remove_dir_all(&d);
}
