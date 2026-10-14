//! The graph of the proofs on a series of `.mzidx` files: analysis, families, alignment per family, hypotheses of the parent, kinetics, `evid/1`.
//!
//!     cargo run --release -p mzlab-feat --example prove -- <file.mzidx>... --parent-formula FORMULA [--truth truth.json] [--out FILE.mzfeat] [--twice]
//!
//! The file names give the times (`tNNNmin` = treated, `dark` = control). With a truth file (`parent {mz, rt_obs}`, `tps [{id, mz, rt_obs[]}]`) it
//! prints, for the candidates (rows that confirm a hypothesis), the rank of the true products ordered by the largest area and ordered by the graph, and
//! how many candidates are needed to recover 90 % of those found. `--twice` builds the `.mzfeat` a second time and compares the bytes.

use std::path::{Path, PathBuf};

use mzlab_core::idxread::Index;
use mzlab_feat::evid::producers::{isotope_score, network_share, neutral_of, FeatIndex, Peaks};
use mzlab_feat::evid::*;
use mzlab_feat::family::{analyse_families, FamParams, Families};
use mzlab_feat::hypo::{generate, HypoParams, Inputs as HypoInputs, Registry, Rules};
use mzlab_feat::kinetic::{parent_kinetics, ChainBank, FormBank, Setup, K_PARENT_MIN};
use mzlab_feat::ms2::Ms2Links;
use mzlab_feat::pipeline::analyse;
use mzlab_feat::recipe::Recipe;
use mzlab_feat::series::{align_and_fill, source_sha256};
use mzlab_feat::store::{read_evid, write_table_with_evid, Origin};
use mzlab_feat::table::{Aligned, FileFeatures, CELL_DETECTED};
use mzlab_lib::build::{build_in_memory, BuildOpts};
use mzlab_lib::parse::{parse_all, Format};
use mzlab_lib::read::{cache_budget, Library};
use mzlab_lib::search::{identity, IdentityOpts, Query, Tol};
use serde_json::Value;

type Idx = Index<std::fs::File>;

fn open(path: &str) -> Idx {
    let f = std::fs::File::open(path).expect("open");
    let size = f.metadata().expect("size").len();
    Index::open(f, size).unwrap_or_else(|e| panic!("{path}: {e}"))
}

/// Minutes of `...t030min...` or `..._t0.mzidx`; `None` for a control.
fn minutes(name: &str) -> Option<f64> {
    let k = name.rfind("_t")? + 2;
    let d: String = name[k..].chars().take_while(char::is_ascii_digit).collect();
    d.parse().ok()
}

struct Series {
    names: Vec<String>,
    feats: Vec<FileFeatures>,
    fams: Vec<Families>,
    links: Vec<Ms2Links>,
    origin: Origin,
}

fn analyse_all(files: &[String], r: &Recipe) -> Series {
    let mut s = Series {
        names: vec![],
        feats: vec![],
        fams: vec![],
        links: vec![],
        origin: Origin::default(),
    };
    for path in files {
        let mut ix = open(path);
        let name = path.rsplit('/').next().unwrap_or(path).to_string();
        s.origin.inputs.push((name.clone(), source_sha256(&ix)));
        let (mut f, links) = analyse(&mut ix, &r.detect, &r.steps).expect("analyse");
        let fam = analyse_families(&mut ix, &mut f, &links, &r.detect, &FamParams::default())
            .expect("families");
        s.names.push(name);
        s.feats.push(f);
        s.fams.push(fam);
        s.links.push(links);
    }
    s
}

fn row_near(t: &Aligned, mz: f64, rt: f64, rt_tol: f64) -> Option<usize> {
    let tol = mz * 5e-6;
    (0..t.len())
        .filter(|&r| (t.mz[r] - mz).abs() <= tol && (t.rt[r] - rt).abs() <= rt_tol)
        .max_by(|&a, &b| area_of(t, a).total_cmp(&area_of(t, b)))
}

fn area_of(t: &Aligned, r: usize) -> f64 {
    t.area[r * t.n_files..(r + 1) * t.n_files]
        .iter()
        .copied()
        .filter(|a| a.is_finite())
        .fold(0.0, f64::max)
}

struct Truth {
    parent: (f64, f64),
    tps: Vec<(String, f64, Vec<f64>)>,
}

fn load_truth(path: &str) -> Truth {
    let v: Value = serde_json::from_slice(&std::fs::read(path).expect("truth")).expect("json");
    let rts = |x: &Value| -> Vec<f64> {
        match &x["rt_obs"] {
            Value::Array(a) => a.iter().filter_map(Value::as_f64).collect(),
            n => n.as_f64().into_iter().collect(),
        }
    };
    Truth {
        parent: (v["parent"]["mz"].as_f64().unwrap(), rts(&v["parent"])[0]),
        tps: v["tps"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| {
                (
                    x["id"].as_str().unwrap().to_string(),
                    x["mz"].as_f64().unwrap(),
                    rts(x),
                )
            })
            .collect(),
    }
}

fn registry(formula: &str, neg: bool) -> Registry {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let rules =
        Rules::parse(&std::fs::read(root.join("TP_Mine/data/regole_tp.json")).unwrap()).unwrap();
    let inp = HypoInputs {
        parent: formula,
        suspects: None,
        families: &[],
        imported: &[],
        fragments: &[],
        groups: None,
    };
    let p = HypoParams {
        polarity: if neg { -1 } else { 1 },
        ..HypoParams::default()
    };
    generate(&rules, &inp, &p).expect("hypotheses")
}

/// The strongest file of a row and the peak that holds it (position in that file).
fn best_peak(aligned: &Aligned, idx: &[FeatIndex], row: usize) -> Option<(usize, usize)> {
    let n = aligned.n_files;
    let k = (0..n)
        .filter(|&k| {
            aligned.state[row * n + k] == CELL_DETECTED && aligned.area[row * n + k].is_finite()
        })
        .max_by(|&a, &b| {
            aligned.area[row * n + a]
                .total_cmp(&aligned.area[row * n + b])
                .then(b.cmp(&a))
        })?;
    idx[k]
        .find(aligned.mz[row], aligned.rt[row], 0.3)
        .map(|p| (k, p))
}

/// The MS2 of every row (the linked scan with the best purity, in the strongest file) and its purity: one pass over the MS2 section of each file.
fn row_spectra(
    files: &[String],
    s: &Series,
    aligned: &Aligned,
    idx: &[FeatIndex],
    pol: Option<i8>,
) -> Vec<Option<(Peaks, f32)>> {
    let mut out: Vec<Option<(Peaks, f32)>> = vec![None; aligned.len()];
    let mut want: Vec<Vec<(usize, u32, f32)>> = vec![vec![]; files.len()];
    for row in 0..aligned.len() {
        let Some((k, p)) = best_peak(aligned, idx, row) else {
            continue;
        };
        let (scans, pur) = s.links[k].of(p);
        let best = (0..scans.len()).max_by(|&a, &b| pur[a].total_cmp(&pur[b]).then(b.cmp(&a)));
        if let Some(b) = best {
            want[k].push((row, scans[b], pur[b]));
        }
    }
    for (k, path) in files.iter().enumerate() {
        let mut ix = open(path);
        let Some(si) = ix.section_of(2, pol) else {
            continue;
        };
        let ids = ix.arrays(si).expect("arrays").scan_ids.clone();
        let mut slot: Vec<Option<usize>> = vec![None; ids.len()];
        for (j, w) in want[k].iter().enumerate() {
            if let Some(pos) = ids.iter().position(|&g| g == i64::from(w.1)) {
                slot[pos] = Some(j);
            }
        }
        let mut peaks = vec![Peaks::default(); want[k].len()];
        let n = ix.sections[si].n_peaks;
        ix.each_peak(si, 0, n, |pos, mz, it| {
            if let Some(Some(j)) = slot.get(pos) {
                peaks[*j].mz.push(mz);
                peaks[*j].it.push(it);
            }
        })
        .expect("peaks");
        for (w, p) in want[k].iter().zip(peaks) {
            out[w.0] = Some((p, w.2));
        }
    }
    out
}

/// The library of the MSP file, built in memory.
fn load_library(path: &str) -> Library<Vec<u8>> {
    let data = std::fs::read(path).expect("library");
    let (spectra, _) = parse_all(Format::Msp, &data);
    let (bytes, _) = build_in_memory(&spectra, BuildOpts::default()).expect("build");
    Library::open(bytes, cache_budget(false)).expect("open")
}

/// The best library match of the MS2 of every row (precursor within 10 ppm, fragments within 0.01).
fn library_hits(
    lib: &Library<Vec<u8>>,
    aligned: &Aligned,
    spec: &[Option<(Peaks, f32)>],
    pol: i8,
) -> Vec<LibHit> {
    let opts = IdentityOpts {
        prec_tol: Tol::Ppm(10.0),
        frag_da: 0.01,
        pol,
        top: 1,
    };
    let mut out = Vec::new();
    for (row, s) in spec.iter().enumerate() {
        let Some((p, _)) = s else { continue };
        let Some(q) = Query::new(aligned.mz[row], pol, &p.mz, &p.it) else {
            continue;
        };
        if let Some(h) = identity(lib, &q, &opts).expect("search").first() {
            let name = lib.meta(h.index).map(|m| m.name).unwrap_or_default();
            out.push(LibHit {
                row,
                name,
                library: "library".into(),
                entropy: h.entropy,
                shared: h.shared as u32,
                ppm: h.dprec,
            });
        }
    }
    out
}

/// The measured facts of every row: isotope pattern against the formula of the hypothesis, purity, share of the MS2 explained by the parent.
fn facts_of(
    files: &[String],
    s: &Series,
    aligned: &Aligned,
    reg: &Registry,
    parent: Option<(f64, f64)>,
    r: &Recipe,
) -> (Vec<Facts>, Vec<Option<(Peaks, f32)>>) {
    let pol = r.detect.polarity;
    let sign = pol.unwrap_or(1);
    let idx: Vec<FeatIndex> = s.feats.iter().map(FeatIndex::new).collect();
    let spec = row_spectra(files, s, aligned, &idx, pol);
    let parent_row = parent.and_then(|(mz, rt)| row_near(aligned, mz, rt, 0.3));
    let parent_spec = parent_row.and_then(|r| spec[r].as_ref().map(|x| x.0.clone()));
    let parent_neutral = parent.map(|p| neutral_of(p.0, sign));
    let facts = (0..aligned.len())
        .map(|row| {
            let mz = aligned.mz[row];
            let hyp = reg.hyps.iter().find(|h| (h.mz - mz).abs() <= mz * 5e-6);
            let isotope =
                hyp.and_then(|h| isotope_score(&idx, mz, aligned.rt[row], &h.formula, sign));
            let network = match (hyp, &parent_spec, &spec[row], parent_neutral) {
                (Some(h), Some(ps), Some((cs, _)), Some(pn)) if parent_row != Some(row) => {
                    network_share(cs, ps, h.neutral - pn, 0.01)
                }
                _ => None,
            };
            Facts {
                isotope,
                fragments: None,
                purity: spec[row].as_ref().map(|x| f64::from(x.1)),
                network,
            }
        })
        .collect();
    (facts, spec)
}

/// Treated files (with a time), controls and the times in minutes, from the names.
fn groups(names: &[String]) -> (Vec<usize>, Vec<usize>, Vec<f64>) {
    let treated: Vec<usize> = (0..names.len())
        .filter(|&k| minutes(&names[k]).is_some())
        .collect();
    let controls = (0..names.len())
        .filter(|&k| minutes(&names[k]).is_none())
        .collect();
    let times = treated.iter().filter_map(|&k| minutes(&names[k])).collect();
    (treated, controls, times)
}

/// Areas of the parent in the treated files (ones when the parent is not known).
fn parent_series(
    aligned: &Aligned,
    treated: &[usize],
    parent: Option<(f64, f64)>,
    n: usize,
) -> Vec<f64> {
    parent
        .and_then(|(mz, rt)| row_near(aligned, mz, rt, 0.3))
        .map(|row| {
            treated
                .iter()
                .map(|&k| aligned.area[row * aligned.n_files + k])
                .collect()
        })
        .unwrap_or_else(|| vec![1.0; n])
}

/// Builds the graph of the series; returns the table, the identities and the `.mzfeat` bytes.
fn build(
    files: &[String],
    r: &Recipe,
    formula: &str,
    truth: Option<&Truth>,
    lib: Option<&Library<Vec<u8>>>,
) -> (Aligned, Vec<Identity>, Vec<u8>) {
    let s = analyse_all(files, r);
    let (aligned, _) = align_and_fill(&s.feats, r, |k| Ok(open(&files[k]))).expect("align");
    let arcs = arcs_from_families(
        &aligned,
        &s.fams
            .iter()
            .zip(&s.feats)
            .map(|(fam, feats)| FileFamilies { feats, fam })
            .collect::<Vec<_>>(),
    );
    let reg = registry(formula, r.detect.polarity == Some(-1));
    let table = PointsTable::bundled().unwrap();
    let (facts, spec) = facts_of(files, &s, &aligned, &reg, truth.map(|t| t.parent), r);
    let libs = lib.map_or(vec![], |l| {
        library_hits(l, &aligned, &spec, r.detect.polarity.unwrap_or(1))
    });
    let (treated, controls, times) = groups(&s.names);
    let (forms, chain) = (FormBank::new(&times), ChainBank::new(&times, K_PARENT_MIN));
    let parent_at = truth.map(|t| t.parent);
    let y = parent_series(&aligned, &treated, parent_at, times.len());
    let setup = Setup {
        times: &times,
        forms: &forms,
        chain: &chain,
        parent: parent_kinetics(&forms, &times, &y, None),
        pseudo: 2e4 * 3.0,
    };
    let cnt = |f: fn(&Facts) -> bool| facts.iter().filter(|x| f(x)).count();
    println!(
        "facts: isotope {}, network {}, purity {}",
        cnt(|x| x.isotope.is_some()),
        cnt(|x| x.network.is_some()),
        cnt(|x| x.purity.is_some())
    );
    let inp = Inputs {
        table: &table,
        aligned: &aligned,
        facts: &facts,
        arcs: &arcs,
        registry: &reg,
        libs: &libs,
        external: &[],
        kinetics: Some(Kinetics {
            setup: &setup,
            treated: &treated,
            controls: &controls,
        }),
        parent: parent_at,
    };
    let ids = build_graph(&inp);
    let json = graph_json(&ids, &table);
    println!(
        "{} rows, {} hypotheses, {} identities, {} arcs, evid/1 {} bytes",
        aligned.len(),
        reg.len(),
        ids.len(),
        arcs.len(),
        json.len()
    );
    let bytes = write_table_with_evid(&aligned, r, &s.origin, json.as_bytes());
    (aligned, ids, bytes)
}

fn report(aligned: &Aligned, ids: &[Identity], truth: &Truth) {
    let cands: Vec<&Identity> = ids.iter().filter(|i| i.hypothesis.is_some()).collect();
    let mut by_area: Vec<u32> = cands.iter().map(|i| i.id).collect();
    by_area.sort_by(|&a, &b| area_of(aligned, b as usize).total_cmp(&area_of(aligned, a as usize)));
    let by_graph: Vec<u32> = cands.iter().map(|i| i.id).collect();
    let pos =
        |list: &[u32], row: usize| list.iter().position(|&x| x as usize == row).map(|p| p + 1);
    let (mut found, mut ra, mut rg) = (0, vec![], vec![]);
    for (id, mz, rts) in &truth.tps {
        let row = rts.iter().find_map(|&rt| row_near(aligned, *mz, rt, 0.3));
        let (a, g) = row.map_or((None, None), |r| (pos(&by_area, r), pos(&by_graph, r)));
        let why = row
            .and_then(|r| ids.iter().find(|i| i.id as usize == r))
            .map(|i| {
                let ag: Vec<_> = i
                    .breakdown
                    .rows
                    .iter()
                    .filter(|e| e.sign == Sign::Against)
                    .map(|e| e.kind.name())
                    .collect();
                format!("total {} against {ag:?}", i.breakdown.total)
            });
        println!(
            "{id:>8}: row {row:?} rank by area {a:?}, by graph {g:?} {}",
            why.unwrap_or_default()
        );
        found += usize::from(row.is_some());
        ra.extend(a);
        rg.extend(g);
    }
    let stat = |mut v: Vec<usize>| {
        v.sort_unstable();
        let n90 = v
            .get(((v.len() * 9).div_ceil(10)).saturating_sub(1))
            .copied();
        (v.get(v.len() / 2).copied(), n90)
    };
    println!(
        "truth: {} products, {found} in the table, {} among {} candidates",
        truth.tps.len(),
        ra.len(),
        cands.len()
    );
    println!(
        "median rank / candidates for 90 %: by area {:?}, by graph {:?}",
        stat(ra),
        stat(rg)
    );
}

fn main() {
    let (mut files, mut out, mut formula, mut truth_path) = (
        Vec::<String>::new(),
        None::<PathBuf>,
        String::new(),
        None::<String>,
    );
    let (mut twice, mut r) = (false, Recipe::default());
    let (mut json_out, mut lib_path) = (None::<PathBuf>, None::<String>);
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--neg" => r.detect.polarity = Some(-1),
            "--parent-formula" => formula = args.next().expect("formula"),
            "--truth" => truth_path = args.next(),
            "--out" => out = args.next().map(PathBuf::from),
            "--twice" => twice = true,
            "--library" => lib_path = args.next(),
            "--json" => json_out = args.next().map(PathBuf::from),
            _ => files.push(a),
        }
    }
    let truth = truth_path.as_deref().map(load_truth);
    let lib = lib_path.as_deref().map(load_library);
    let (aligned, ids, bytes) = build(&files, &r, &formula, truth.as_ref(), lib.as_ref());
    assert_eq!(read_evid(&bytes).expect("evid").is_empty(), false);
    println!("{} bytes of .mzfeat", bytes.len());
    if let Some(t) = &truth {
        report(&aligned, &ids, t);
    }
    if twice {
        let again = build(&files, &r, &formula, truth.as_ref(), lib.as_ref()).2;
        println!(
            "second run: {}",
            if again == bytes {
                "identical bytes"
            } else {
                "DIFFERENT"
            }
        );
    }
    if let Some(p) = json_out {
        std::fs::write(p, read_evid(&bytes).expect("evid")).expect("write");
    }
    if let Some(p) = out {
        std::fs::write(p, &bytes).expect("write");
    }
}
