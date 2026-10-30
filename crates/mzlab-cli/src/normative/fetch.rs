//! The `Fetch` trait of the builder (the network), its disk cache and its HTTP implementation. With a warm cache a build makes no request; the clock is read only
//! when a file is first stored, and the date goes into the sidecar `DIR/<sha256 of URL>.json` next to the bytes `DIR/<sha256 of URL>`.

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use mzlab_core::budget::sha256_hex;
use mzlab_core::json::{self, Out, Value};
use sha2::{Digest, Sha256};

use super::NormError;

impl Write for Counter<'_> {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        self.n += b.len() as u64;
        if self.n > self.limit {
            return Err(std::io::Error::other("limit"));
        }
        self.hash.update(b);
        self.inner.write_all(b)?;
        Ok(b.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

impl<F: Fetch> Cache<F> {
    pub fn new(dir: &Path, net: F, now: fn() -> String, limit: u64) -> Cache<F> {
        Cache {
            dir: dir.into(),
            net,
            now,
            limit,
        }
    }

    fn names(&self, url: &str) -> (PathBuf, PathBuf) {
        let h = sha256_hex(url.as_bytes());
        (self.dir.join(&h), self.dir.join(format!("{h}.json")))
    }

    fn sidecar(&self, url: &str) -> Option<Cached> {
        let (data, side) = self.names(url);
        let v = json::parse(&std::fs::read(side).ok()?).ok()?;
        let s = |k: &str| v.get(k).and_then(Value::str).map(String::from);
        if !data.exists() || s("url")? != url {
            return None;
        }
        Some(Cached {
            url: url.into(),
            path: data,
            sha256: s("sha256")?,
            retrieved: s("retrieved")?,
            bytes: v.get("bytes").and_then(Value::f64)? as u64,
        })
    }

    /// The file of `url`: from the cache, or fetched and stored.
    pub fn ensure(&self, url: &str) -> Result<Cached, NormError> {
        if let Some(c) = self.sidecar(url) {
            return Ok(c);
        }
        std::fs::create_dir_all(&self.dir).map_err(|e| NormError::io(&self.dir, &e))?;
        let (data, side) = self.names(url);
        let part = data.with_extension("part");
        let file = File::create(&part).map_err(|e| NormError::io(&part, &e))?;
        let mut w = Counter {
            inner: file,
            hash: Sha256::new(),
            n: 0,
            limit: self.limit,
            url,
        };
        let r = self.net.get(url, self.limit, &mut w);
        if let Err(e) = r {
            let _ = std::fs::remove_file(&part);
            return Err(if w.n > w.limit {
                NormError::large(url, self.limit)
            } else {
                e
            });
        }
        let _ = w.url;
        let sha: String = w
            .hash
            .clone()
            .finalize()
            .iter()
            .map(|x| format!("{x:02x}"))
            .collect();
        let bytes = w.n;
        drop(w);
        std::fs::rename(&part, &data).map_err(|e| NormError::io(&data, &e))?;
        let c = Cached {
            url: url.into(),
            path: data,
            sha256: sha,
            retrieved: (self.now)(),
            bytes,
        };
        let mut o = Out::default();
        o.raw("{\"url\":")
            .str(url)
            .raw(",\"retrieved\":")
            .str(&c.retrieved)
            .raw(",\"sha256\":")
            .str(&c.sha256);
        o.raw(",\"bytes\":").int(bytes as i64).raw("}\n");
        std::fs::write(&side, o.0).map_err(|e| NormError::io(&side, &e))?;
        Ok(c)
    }

    pub fn bytes(&self, c: &Cached) -> Result<Vec<u8>, NormError> {
        std::fs::read(&c.path).map_err(|e| NormError::io(&c.path, &e))
    }

    /// Opens the stored file, undoing gzip when the bytes start with its magic number.
    pub fn open(&self, c: &Cached) -> Result<Box<dyn Read>, NormError> {
        open_file(&c.path)
    }
}

impl Throttle {
    pub fn new(per_second: u32) -> Throttle {
        Throttle {
            last: Mutex::new(HashMap::new()),
            gap: Duration::from_millis(1000 / u64::from(per_second.max(1))),
        }
    }

    /// How long to wait before a request to `host` made at `now`; the slot is taken.
    pub fn reserve(&self, host: &str, now: Instant) -> Duration {
        let mut m = self.last.lock().expect("throttle");
        let start = match m.get(host) {
            Some(prev) if *prev + self.gap > now => *prev + self.gap,
            _ => now,
        };
        m.insert(host.into(), start);
        start - now
    }
}

impl Default for Http {
    fn default() -> Http {
        let mut b = ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(30))
            .timeout_read(Duration::from_secs(300))
            .user_agent("mzlab-normative/1 (+https://github.com/Freeesta/mzlab)");
        if let Some(cfg) = tls_with_extra_roots() {
            b = b.tls_config(cfg);
        }
        Http {
            agent: b.build(),
            throttle: Throttle::new(5),
        }
    }
}

impl Http {
    fn once(&self, url: &str, limit: u64, out: &mut dyn Write) -> Result<(), (bool, NormError)> {
        let mut req = self.agent.get(url).set("Accept-Encoding", "gzip");
        if host_of(url).ends_with("europa.eu") && !url.contains("api.datalake") {
            req = req
                .set("Accept", "application/xhtml+xml")
                .set("Accept-Language", "en");
        }
        let resp = match req.call() {
            Ok(r) => r,
            Err(ureq::Error::Status(c, _)) => {
                return Err((
                    c == 429 || c >= 500,
                    NormError::fetch(url, &format!("HTTP {c}")),
                ));
            }
            Err(e) => return Err((true, NormError::fetch(url, &e.to_string()))),
        };
        if resp.status() == 202 {
            return Err((true, NormError::fetch(url, "HTTP 202 (not ready)")));
        }
        if let Some(n) = resp
            .header("Content-Length")
            .and_then(|v| v.parse::<u64>().ok())
        {
            if n > limit && resp.header("Content-Encoding").is_none() {
                return Err((false, NormError::large(url, limit)));
            }
        }
        let gz = resp
            .header("Content-Encoding")
            .is_some_and(|v| v.eq_ignore_ascii_case("gzip"));
        let mut r: Box<dyn Read> = if gz {
            Box::new(flate2::read::MultiGzDecoder::new(resp.into_reader()))
        } else {
            Box::new(resp.into_reader())
        };
        std::io::copy(&mut r, out).map_err(|e| (false, NormError::fetch(url, &e.to_string())))?;
        Ok(())
    }
}

impl Fetch for Http {
    fn get(&self, url: &str, limit: u64, out: &mut dyn Write) -> Result<(), NormError> {
        // `file:PATH` is a download the user made by hand (ECHA and OEHHA files); it is cached like any other
        if let Some(path) = url
            .strip_prefix("file://")
            .or_else(|| url.strip_prefix("file:"))
        {
            let size = std::fs::metadata(path)
                .map_err(|e| NormError::fetch(url, &e.to_string()))?
                .len();
            if size > limit {
                return Err(NormError::large(url, limit));
            }
            let mut f = File::open(path).map_err(|e| NormError::fetch(url, &e.to_string()))?;
            std::io::copy(&mut f, out).map_err(|e| NormError::fetch(url, &e.to_string()))?;
            return Ok(());
        }
        let mut last = NormError::fetch(url, "no attempt");
        #[allow(clippy::needless_range_loop)]
        for attempt in 0..=RETRY_WAITS.len() {
            let wait = self.throttle.reserve(host_of(url), Instant::now());
            std::thread::sleep(wait);
            match self.once(url, limit, out) {
                Ok(()) => return Ok(()),
                Err((retry, e)) => {
                    last = e;
                    if !retry || attempt == RETRY_WAITS.len() {
                        break;
                    }
                    std::thread::sleep(RETRY_WAITS[attempt]);
                }
            }
        }
        Err(last)
    }
}

/// The network: writes the body of `url` into `out`. `limit` is the most bytes accepted; more is an `err.normative.large`.
pub trait Fetch {
    fn get(&self, url: &str, limit: u64, out: &mut dyn Write) -> Result<(), NormError>;
}

/// A file in the cache.
#[derive(Clone, Debug)]
pub struct Cached {
    pub url: String,
    pub path: PathBuf,
    pub sha256: String,
    /// Date (UTC, `YYYY-MM-DD`) of the first retrieval.
    pub retrieved: String,
    pub bytes: u64,
}

/// The default limit of one download (bigger files need `--allow-large`).
pub const LARGE: u64 = 90 * 1_000_000;

pub struct Cache<F: Fetch> {
    dir: PathBuf,
    net: F,
    now: fn() -> String,
    limit: u64,
}

struct Counter<'a> {
    inner: File,
    hash: Sha256,
    n: u64,
    limit: u64,
    url: &'a str,
}

/// A file as a reader, gunzipped when it is gzip.
pub fn open_file(path: &Path) -> Result<Box<dyn Read>, NormError> {
    let mut f = File::open(path).map_err(|e| NormError::io(path, &e))?;
    let mut magic = [0u8; 2];
    let n = f.read(&mut magic).map_err(|e| NormError::io(path, &e))?;
    let f = File::open(path).map_err(|e| NormError::io(path, &e))?;
    Ok(if n == 2 && magic == [0x1f, 0x8b] {
        Box::new(BufReader::new(flate2::read::MultiGzDecoder::new(
            BufReader::new(f),
        )))
    } else {
        Box::new(BufReader::new(f))
    })
}

/// Today (UTC) as `YYYY-MM-DD`.
pub fn today() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let z = (secs / 86400) as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

/// At most 5 requests a second to one host.
pub struct Throttle {
    last: Mutex<HashMap<String, Instant>>,
    gap: Duration,
}

/// The waits between the attempts of one request (exponential).
pub const RETRY_WAITS: [Duration; 3] = [
    Duration::from_secs(1),
    Duration::from_secs(2),
    Duration::from_secs(4),
];

/// The real network (native only).
pub struct Http {
    agent: ureq::Agent,
    throttle: Throttle,
}

/// The DER bodies of the certificates of a PEM file.
fn pem_certs(pem: &str) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    let mut body = String::new();
    let mut inside = false;
    for l in pem.lines() {
        if l.starts_with("-----BEGIN CERTIFICATE") {
            inside = true;
            body.clear();
        } else if l.starts_with("-----END CERTIFICATE") {
            inside = false;
            out.push(unbase64(&body));
        } else if inside {
            body.push_str(l.trim());
        }
    }
    out
}

fn unbase64(s: &str) -> Vec<u8> {
    let (mut out, mut acc, mut bits) = (Vec::new(), 0u32, 0);
    for c in s.bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => continue,
        };
        acc = (acc << 6) | u32::from(v);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    out
}

/// The built-in roots plus the certificates of the file named by `SSL_CERT_FILE` (a proxy that re-signs HTTPS needs its own authority trusted).
fn tls_with_extra_roots() -> Option<std::sync::Arc<rustls::ClientConfig>> {
    let pem = std::fs::read_to_string(std::env::var("SSL_CERT_FILE").ok()?).ok()?;
    let mut roots = rustls::RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    for der in pem_certs(&pem) {
        let _ = roots.add(rustls::pki_types::CertificateDer::from(der));
    }
    let provider = std::sync::Arc::new(rustls::crypto::ring::default_provider());
    let cfg = rustls::ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .ok()?
        .with_root_certificates(roots)
        .with_no_client_auth();
    Some(std::sync::Arc::new(cfg))
}

fn host_of(url: &str) -> &str {
    url.split("//")
        .nth(1)
        .unwrap_or(url)
        .split('/')
        .next()
        .unwrap_or("")
}
