//! The assistant in the app: the routes `llm/*`. The key lives in `llm-chiavi.json` (mode 0600) and only this file reads it; `llm/send` opens the
//! request with `ureq` on a thread and `llm/poll` hands the normalized events to the page, like `flow/progress` does for a plan.

use std::collections::HashMap;
use std::io::Read;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use mzlab_core::{Error, Result};
use mzlab_llm::conv::Conversation;
use mzlab_llm::event::Event;
use mzlab_llm::provider::{self, Provider, Request, Stream, ToolMode};
use mzlab_llm::tools::{catalog, Tool, UI_TOOLS};
use serde_json::{json, Value};

use crate::engine::State;
use crate::rpc::{text, Answer};

const KEYS_FILE: &str = "llm-chiavi.json";

impl State {
    fn keys_path(&self) -> Result<PathBuf> {
        Ok(self
            .store
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .data_dir()?
            .join(KEYS_FILE))
    }

    /// `llm/providers`, `llm/request`, `llm/send`, `llm/poll`, `llm/abort`, `llm/key/set`, `llm/key/forget`: see `docs/agenti/llm.md`.
    pub(crate) fn op_llm(&self, op: &str, p: &Value) -> Result<Answer> {
        let s = |k: &str| p[k].as_str().unwrap_or("");
        match op {
            "llm/providers" => {
                let keys = read_keys(&self.keys_path()?);
                let has: HashMap<String, bool> = provider::builtin()
                    .iter()
                    .map(|x| (x.id.clone(), keys.contains_key(&x.id)))
                    .collect();
                text(json!({ "providers": provider::BUILTIN_JSON, "has_key": has }).to_string())
            }
            "llm/request" => text(build(s("conv"))?.2.to_json()),
            "llm/send" => {
                let (prov, conv, req) = build(s("conv"))?;
                let key = read_keys(&self.keys_path()?).remove(&prov.id);
                let prompted = prov
                    .model(&conv.model)
                    .is_some_and(|m| m.tools == ToolMode::Prompted);
                let abort = Arc::new(AtomicBool::new(false));
                let id = with_jobs(|n, m| {
                    *n += 1;
                    m.insert(
                        *n,
                        Job {
                            abort: abort.clone(),
                            ..Job::default()
                        },
                    );
                    *n
                });
                std::thread::spawn(move || run(id, prov, prompted, req, key, abort));
                text(json!({ "id": id.to_string() }).to_string())
            }
            "llm/poll" => {
                let id: u64 = s("id")
                    .parse()
                    .map_err(|_| err("err.llm.conversation", "field", "id"))?;
                let (events, done) = with_jobs(|_, m| match m.get_mut(&id) {
                    Some(j) => {
                        let ev = std::mem::take(&mut j.events);
                        let done = j.done;
                        if done {
                            m.remove(&id);
                        }
                        (ev, done)
                    }
                    None => (Vec::new(), true),
                });
                text(json!({ "events": events, "done": done }).to_string())
            }
            "llm/abort" => {
                if let Ok(id) = s("id").parse::<u64>() {
                    with_jobs(|_, m| {
                        if let Some(j) = m.get(&id) {
                            j.abort.store(true, Ordering::Relaxed);
                        }
                    });
                }
                text("{}")
            }
            "llm/key/set" | "llm/key/forget" => key_route(&self.keys_path()?, op, p),
            _ => Err(Error::new("rpc.unknown")),
        }
    }
}

#[derive(Default)]
struct Job {
    events: Vec<String>,
    done: bool,
    abort: Arc<AtomicBool>,
}

static JOBS: Mutex<Option<(u64, HashMap<u64, Job>)>> = Mutex::new(None);

fn with_jobs<R>(f: impl FnOnce(&mut u64, &mut HashMap<u64, Job>) -> R) -> R {
    let mut g = JOBS.lock().unwrap_or_else(|e| e.into_inner());
    let (n, m) = g.get_or_insert_with(|| (0, HashMap::new()));
    f(n, m)
}

fn err(key: &'static str, name: &'static str, v: &str) -> Error {
    Error::new(key).with(name, v)
}

fn read_keys(path: &PathBuf) -> HashMap<String, String> {
    std::fs::read(path)
        .ok()
        .and_then(|b| serde_json::from_slice::<HashMap<String, String>>(&b).ok())
        .unwrap_or_default()
}

fn write_keys(path: &PathBuf, keys: &HashMap<String, String>) -> Result<()> {
    let body = serde_json::to_vec(keys).map_err(|_| err("err.llm.network", "reason", "keys"))?;
    let tmp = path.with_extension("tmp");
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&tmp);
        if let Ok(f) = f.as_mut() {
            f.write_all(&body)
                .map_err(|_| err("err.llm.network", "reason", "keys"))?;
        } else {
            return Err(err("err.llm.network", "reason", "keys"));
        }
    }
    #[cfg(not(unix))]
    std::fs::write(&tmp, &body).map_err(|_| err("err.llm.network", "reason", "keys"))?;
    std::fs::rename(&tmp, path).map_err(|_| err("err.llm.network", "reason", "keys"))
}

fn find(id: &str) -> Result<Provider> {
    provider::builtin()
        .into_iter()
        .find(|p| p.id == id)
        .ok_or_else(|| err("err.llm.conversation", "field", "provider"))
}

fn tools() -> Vec<Tool> {
    catalog(
        &mzlab_cli::flow::steps(),
        UI_TOOLS,
        &mzlab_cli::flow::executable,
    )
}

fn build(conv: &str) -> Result<(Provider, Conversation, Request)> {
    let c = Conversation::parse(conv)?;
    let p = find(&c.provider)?;
    let r = provider::request(&p, &c, &tools())?;
    Ok((p, c, r))
}

fn line(e: &Event) -> String {
    e.to_json()
}

fn push_events(id: u64, evs: &[Event]) {
    if evs.is_empty() {
        return;
    }
    with_jobs(|_, m| {
        if let Some(j) = m.get_mut(&id) {
            j.events.extend(evs.iter().map(line));
        }
    });
}

fn finish_job(id: u64) {
    with_jobs(|_, m| {
        if let Some(j) = m.get_mut(&id) {
            j.done = true;
        }
    });
}

/// `llm/key/set` stores the key of a provider, `llm/key/forget` drops it; the file is the only place a key lives.
fn key_route(path: &PathBuf, op: &str, p: &Value) -> Result<Answer> {
    let s = |k: &str| p[k].as_str().unwrap_or("");
    let mut keys = read_keys(path);
    if op == "llm/key/set" {
        let prov = find(s("provider"))?;
        let key = s("key").trim();
        if key.is_empty() {
            return Err(err("err.llm.auth", "reason", "nokey"));
        }
        keys.insert(prov.id, key.to_string());
    } else {
        keys.remove(s("provider"));
    }
    write_keys(path, &keys)?;
    text("{}")
}

/// Runs one request to its end: the bytes of the answer go through the decoder of the provider, whatever the split.
fn run(
    id: u64,
    p: Provider,
    model_prompted: bool,
    req: Request,
    key: Option<String>,
    abort: Arc<AtomicBool>,
) {
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(15))
        .timeout_read(Duration::from_secs(120))
        .build();
    let mut r = agent.post(&req.url);
    for (k, v) in &req.headers {
        r = r.set(k, v);
    }
    if !req.auth.header.is_empty() {
        let Some(k) = key else {
            push_events(
                id,
                &[Event::error(
                    "err.llm.auth",
                    &[("reason", "nokey".to_string())],
                )],
            );
            return finish_job(id);
        };
        r = r.set(&req.auth.header, &format!("{}{}", req.auth.prefix, k));
    }
    let mut stream = Stream::new(p.adapter);
    if model_prompted {
        stream = stream.prompted();
    }
    match r.send_string(&req.body) {
        Ok(resp) => {
            let mut rd = resp.into_reader();
            let mut buf = [0u8; 4096];
            loop {
                if abort.load(Ordering::Relaxed) {
                    push_events(id, &[Event::error("err.llm.aborted", &[])]);
                    return finish_job(id);
                }
                match rd.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => push_events(id, &stream.push(&buf[..n])),
                    Err(_) => {
                        push_events(
                            id,
                            &[Event::error(
                                "err.llm.network",
                                &[("reason", "read".to_string())],
                            )],
                        );
                        return finish_job(id);
                    }
                }
            }
            push_events(id, &stream.finish());
        }
        Err(ureq::Error::Status(code, resp)) => {
            let mut body = String::new();
            let _ = resp.into_reader().take(4096).read_to_string(&mut body);
            push_events(id, &[provider::http_error(code, &body)]);
        }
        Err(_) => push_events(
            id,
            &[Event::error(
                "err.llm.network",
                &[("reason", "connect".to_string())],
            )],
        ),
    }
    finish_job(id);
}

#[cfg(test)]
#[path = "llm_tests.rs"]
mod tests;
