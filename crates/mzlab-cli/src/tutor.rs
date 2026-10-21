//! `mzlab-cli tutor eval CASES --blocchi F (--replay DIR | --solo-recupero | --provider ID --key-file F ...)`: scores the tutor on the eval cases. With
//! `--replay` the model answers come from recordings `DIR/<case key>.jsonl` (CI, no network); the live mode is by hand only and calls the provider
//! through `curl`. The logic is `mzlab_tutor::eval`; this file reads files and prints lines.

use std::path::{Path, PathBuf};

use mzlab_core::error::Error;
use mzlab_llm::event::Event;
use mzlab_llm::fake::Replay;
use mzlab_llm::provider::{builtin, Request};
use mzlab_tutor::answer::Level;
use mzlab_tutor::eval::{estimate_micros, load_cases, run_case, Corpus, Setup};
use mzlab_tutor::quota::{Prices, DOLLAR};

use crate::Outcome;

const HELP: &str = "mzlab-cli tutor eval CASES --blocchi F (--replay DIR | --solo-recupero | --provider ID --key-file F --pick M --answer M --effort E [--yes])
  CASES: a folder of case files (*.json) or one file; F: the teoria-blocchi/1 file (teoria-blocchi DIR OUT). --prices FILE: the PRICES variable (default: list prices).
  --replay DIR: the model turns come from DIR/<case key>.jsonl (llmreg/1: turn 0 the choice, turn 1 the answer); a case with no recording is skipped.
  --solo-recupero: only the search recall@5, no model. The mode with a key calls the provider for real: it prints the estimated cost and needs --yes.
  Output: one JSON line per scored case, then a summary line; exit 0 when every scored case passes, 1 otherwise, 2 for a usage error.";

struct Opts {
    cases: String,
    blocchi: String,
    replay: Option<String>,
    recall_only: bool,
    provider: Option<String>,
    key_file: Option<String>,
    pick: String,
    answer: String,
    effort: String,
    prices: Option<String>,
    yes: bool,
}

fn parse(args: Vec<String>) -> Result<Opts, String> {
    let mut o = Opts {
        cases: String::new(),
        blocchi: String::new(),
        replay: None,
        recall_only: false,
        provider: None,
        key_file: None,
        pick: "claude-haiku-5-5".into(),
        answer: "claude-sonnet-5-5".into(),
        effort: "low".into(),
        prices: None,
        yes: false,
    };
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        let mut value = |name: &str| it.next().ok_or(format!("{name} needs a value"));
        match a.as_str() {
            "--blocchi" => o.blocchi = value("--blocchi")?,
            "--replay" => o.replay = Some(value("--replay")?),
            "--solo-recupero" => o.recall_only = true,
            "--provider" => o.provider = Some(value("--provider")?),
            "--key-file" => o.key_file = Some(value("--key-file")?),
            "--pick" => o.pick = value("--pick")?,
            "--answer" => o.answer = value("--answer")?,
            "--effort" => o.effort = value("--effort")?,
            "--prices" => o.prices = Some(value("--prices")?),
            "--yes" => o.yes = true,
            x if !x.starts_with('-') && o.cases.is_empty() => o.cases = x.to_string(),
            x => return Err(format!("unknown option {x}")),
        }
    }
    let modes = usize::from(o.replay.is_some())
        + usize::from(o.recall_only)
        + usize::from(o.provider.is_some());
    if o.cases.is_empty() || o.blocchi.is_empty() || modes != 1 {
        return Err(format!("usage: {HELP}"));
    }
    if o.provider.is_some() && o.key_file.is_none() {
        return Err("--provider needs --key-file".into());
    }
    Ok(o)
}

fn read(p: &Path) -> Result<String, String> {
    std::fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()))
}

fn case_files(p: &str) -> Result<Vec<PathBuf>, String> {
    let path = Path::new(p);
    if path.is_file() {
        return Ok(vec![path.to_path_buf()]);
    }
    let mut v: Vec<PathBuf> = std::fs::read_dir(path)
        .map_err(|e| format!("{p}: {e}"))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|f| f.extension().is_some_and(|x| x == "json"))
        .collect();
    v.sort();
    Ok(v)
}

pub(crate) fn run(args: Vec<String>) -> Outcome {
    let mut it = args.into_iter();
    match it.next().as_deref() {
        Some("eval") => {}
        Some("--help" | "-h") => return Outcome::ok(format!("{HELP}\n").into_bytes()),
        _ => return Outcome::fail(&format!("usage: {HELP}")),
    }
    let rest: Vec<String> = it.collect();
    if rest.iter().any(|a| a == "--help" || a == "-h") {
        return Outcome::ok(format!("{HELP}\n").into_bytes());
    }
    match parse(rest).and_then(|o| eval(&o)) {
        Ok(out) => out,
        Err(e) => Outcome::fail(&e),
    }
}

fn eval(o: &Opts) -> Result<Outcome, String> {
    let texts = case_files(&o.cases)?
        .iter()
        .map(|p| read(p))
        .collect::<Result<Vec<_>, _>>()?;
    let cases = load_cases(&texts).map_err(|e| e.to_string())?;
    let corpus = Corpus::parse(&read(Path::new(&o.blocchi))?).map_err(|e| e.to_string())?;
    let mut out = String::new();
    if o.recall_only {
        for lang in ["it", "en"] {
            let (r, n) = corpus.recall(&cases, Some(lang), 5);
            out.push_str(&format!(
                "{{\"cases\":{n},\"lang\":\"{lang}\",\"recall_at_5\":{r:.3}}}\n"
            ));
        }
        return Ok(Outcome::ok(out.into_bytes()));
    }
    let prices = match &o.prices {
        Some(p) => Prices::parse(&read(Path::new(p))?),
        None => Ok(Prices::list()),
    }
    .map_err(|e| e.to_string())?;
    let provider_id = o.provider.as_deref().unwrap_or("anthropic");
    let provider = builtin()
        .into_iter()
        .find(|p| p.id == provider_id)
        .ok_or(format!("unknown provider {provider_id}"))?;
    let setup = Setup {
        corpus: &corpus,
        provider: &provider,
        model_pick: &o.pick,
        model_answer: &o.answer,
        effort_pick: "low",
        effort_answer: &o.effort,
        level: Level::Base,
        prices: &prices,
    };
    let mut live = None;
    if o.provider.is_some() {
        let micros = estimate_micros(cases.len() as u64, &o.pick, &o.answer, &prices)
            .map_err(|e| e.to_string())?;
        eprintln!(
            "estimated cost of {} questions: about {:.2} USD",
            cases.len(),
            micros as f64 / DOLLAR as f64
        );
        if !o.yes {
            return Err("this calls the provider for real: add --yes to go on".into());
        }
        let key = read(Path::new(o.key_file.as_deref().unwrap_or("")))?;
        live = Some(key.trim().to_string());
    }
    let (mut scored, mut passed, mut skipped, mut cost) = (0u32, 0u32, 0u32, 0u64);
    for c in &cases {
        let mut recorded = None;
        if let Some(dir) = &o.replay {
            let f = Path::new(dir).join(format!("{}.jsonl", c.key()));
            if !f.is_file() {
                skipped += 1;
                continue;
            }
            recorded = Some(Replay::parse(&read(&f)?, false).map_err(|e| e.to_string())?);
        }
        scored += 1;
        let result = match (&recorded, &live) {
            (Some(r), _) => run_case(&setup, c, &mut |turn, req| r.at(turn, req)),
            (None, Some(key)) => run_case(&setup, c, &mut |_, req| live_send(&provider, req, key)),
            (None, None) => unreachable!("a mode is always chosen"),
        };
        match result {
            Ok(r) => {
                passed += u32::from(r.score.passed());
                cost += r.cost_micros;
                out.push_str(&r.to_json());
            }
            Err(e) => out.push_str(&format!(
                "{{\"case\":\"{}\",\"error\":\"{e}\",\"pass\":false}}",
                c.key()
            )),
        }
        out.push('\n');
    }
    out.push_str(&format!(
        "{{\"cost_micros\":{cost},\"failed\":{},\"passed\":{passed},\"scored\":{scored},\"skipped\":{skipped}}}\n",
        scored - passed
    ));
    let mut outcome = Outcome::ok(out.into_bytes());
    if scored == 0 || passed < scored {
        outcome.code = 1;
    }
    Ok(outcome)
}

/// One real call through `curl` (the key and the headers travel on its standard input, never in the arguments). By hand only: nothing in the tests or
/// in CI reaches it.
#[cfg(not(target_family = "wasm"))]
fn live_send(
    provider: &mzlab_llm::provider::Provider,
    req: &Request,
    key: &str,
) -> Result<Vec<Event>, Error> {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let net = |why: &str| Error::new("err.llm.network").with("reason", why);
    let quote = |s: &str| format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""));
    let body = std::env::temp_dir().join(format!("mzlab-tutor-{}.json", std::process::id()));
    std::fs::write(&body, &req.body).map_err(|_| net("temp"))?;
    let mut cfg = format!(
        "silent\nshow-error\nmax-time = 180\nrequest = \"POST\"\nurl = {}\ndata-binary = {}\nwrite-out = \"\\n%{{http_code}}\"\n",
        quote(&req.url),
        quote(&format!("@{}", body.display()))
    );
    for (k, v) in &req.headers {
        cfg.push_str(&format!("header = {}\n", quote(&format!("{k}: {v}"))));
    }
    if !req.auth.header.is_empty() {
        cfg.push_str(&format!(
            "header = {}\n",
            quote(&format!("{}: {}{}", req.auth.header, req.auth.prefix, key))
        ));
    }
    let child = Command::new("curl")
        .args(["-K", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn();
    let result = child.map_err(|_| net("curl")).and_then(|mut c| {
        c.stdin
            .take()
            .ok_or_else(|| net("stdin"))?
            .write_all(cfg.as_bytes())
            .map_err(|_| net("stdin"))?;
        c.wait_with_output().map_err(|_| net("curl"))
    });
    let _ = std::fs::remove_file(&body);
    let out = result?.stdout;
    let cut = out
        .iter()
        .rposition(|&b| b == b'\n')
        .ok_or_else(|| net("empty"))?;
    let status: u16 = String::from_utf8_lossy(&out[cut + 1..])
        .trim()
        .parse()
        .unwrap_or(0);
    if status != 200 {
        let text = String::from_utf8_lossy(&out[..cut]);
        return Ok(vec![mzlab_llm::provider::http_error(status, &text)]);
    }
    let mut stream = mzlab_llm::provider::Stream::new(provider.adapter);
    let mut events = stream.push(&out[..cut]);
    events.extend(stream.finish());
    Ok(events)
}

#[cfg(target_family = "wasm")]
fn live_send(_: &mzlab_llm::provider::Provider, _: &Request, _: &str) -> Result<Vec<Event>, Error> {
    Err(Error::new("err.llm.network").with("reason", "wasm"))
}
