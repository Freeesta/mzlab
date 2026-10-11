//! Download of the spectral-library packages into the folder of the libraries (`librerie.rs`), only on a click of the user.
//! Packages of the site (`static/librerie/catalogo.json`): pieces of 4 MiB checked one by one with their SHA-256, resumed from the first piece that is missing
//! or wrong, assembled into `.mzlab/<id>.new`, checked as a whole (size, SHA-256, header of the format) and only then swapped for the old `<id>.mzlib` in one rename
//! (the old one stays until the new one is verified). External sources (the `esterne` list of the catalog, e.g. GNPS): ONLY with the switch «Scarica librerie
//! pubbliche da fonti esterne» on (`Config::esterne`, off by default), only from the hosts of an allow-list over HTTPS, one plain GET with no data of the user.
//! Every error is a key of `lang/` (`lib.dl.err.*`) plus a detail for the log.

use std::fs;
use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::librerie::{Folder, META};
use crate::update::{sha256_hex, Source};

/// The highest `.mzlib` version this app reads (`mzlab_lib::format::VERSION`; a test keeps them equal).
pub const READER: u16 = 1;
pub const CATALOG: &str = "static/librerie/catalogo.json";
const LIBS_DIR: &str = "static/librerie/";
/// Hosts the external downloads may use (and follow redirects to): the sources named in the catalog and where they keep their files.
pub const ALLOWED_HOSTS: [&str; 6] = [
    "external.gnps2.org",
    "github.com",
    "release-assets.githubusercontent.com",
    "objects.githubusercontent.com",
    "zenodo.org",
    "freeesta.github.io",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DlError {
    pub key: &'static str,
    pub detail: String,
}

impl DlError {
    fn new(key: &'static str, detail: impl Into<String>) -> Self {
        DlError {
            key,
            detail: detail.into(),
        }
    }
    fn io(e: impl std::fmt::Display) -> Self {
        DlError::new("lib.dl.err.io", e.to_string())
    }
    /// `{status:"error", error_key, detail}` for the page.
    pub fn to_json(&self) -> String {
        serde_json::json!({"status": "error", "error_key": self.key, "detail": self.detail})
            .to_string()
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Piece {
    pub file: String,
    pub dimensione: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Package {
    pub id: String,
    #[serde(default)]
    pub nome: String,
    pub versione: String,
    pub sha256: String,
    pub dimensione: u64,
    #[serde(default = "one")]
    pub lettore_min: u16,
    #[serde(default)]
    pub spettri: u64,
    #[serde(default)]
    pub picchi: u64,
    #[serde(default)]
    pub polarita: i32,
    pub pezzi: Vec<Piece>,
}

fn one() -> u16 {
    1
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct External {
    pub id: String,
    #[serde(default)]
    pub nome: String,
    pub url: String,
    #[serde(default)]
    pub formato: String,
    #[serde(default)]
    pub sha256: Option<String>,
    #[serde(default)]
    pub dimensione: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Catalog {
    pub formato: String,
    #[serde(default)]
    pub librerie: Vec<Package>,
    #[serde(default)]
    pub esterne: Vec<External>,
}

pub fn parse_catalog(bytes: &[u8]) -> Result<Catalog, DlError> {
    let c: Catalog = serde_json::from_slice(bytes)
        .map_err(|e| DlError::new("lib.dl.err.catalog", e.to_string()))?;
    if c.formato != "librerie/1" {
        return Err(DlError::new("lib.dl.err.catalog", c.formato));
    }
    // names that become file names: plain only
    for id in c
        .librerie
        .iter()
        .map(|p| &p.id)
        .chain(c.esterne.iter().map(|e| &e.id))
    {
        if id.is_empty()
            || !id
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
        {
            return Err(DlError::new("lib.dl.err.catalog", format!("id {id}")));
        }
    }
    for p in &c.librerie {
        for pc in &p.pezzi {
            if !pc.file.starts_with(&format!("{}/", p.id))
                || pc.file.contains("..")
                || pc.file.contains('\\')
            {
                return Err(DlError::new("lib.dl.err.catalog", pc.file.clone()));
            }
        }
        if p.pezzi.iter().map(|x| x.dimensione).sum::<u64>() != p.dimensione {
            return Err(DlError::new(
                "lib.dl.err.catalog",
                format!("size of {}", p.id),
            ));
        }
    }
    Ok(c)
}

/// The catalog of the site (a network call: only when the user asks for the list).
pub fn fetch_catalog(src: &dyn Source) -> Result<Catalog, DlError> {
    fetch_catalog_raw(src).map(|x| x.0)
}

/// The checked catalog and its whole JSON (the page also shows the licence text, the attribution and the source of every package).
pub fn fetch_catalog_raw(src: &dyn Source) -> Result<(Catalog, serde_json::Value), DlError> {
    let bytes = src.get(CATALOG).map_err(|e| {
        DlError::new(
            if e.key == "update.error.offline" {
                "lib.dl.err.offline"
            } else {
                "lib.dl.err.http"
            },
            e.detail,
        )
    })?;
    let cat = parse_catalog(&bytes)?;
    let raw = serde_json::from_slice(&bytes)
        .map_err(|e| DlError::new("lib.dl.err.catalog", e.to_string()))?;
    Ok((cat, raw))
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Progress {
    pub id: String,
    pub piece: usize,
    pub pieces: usize,
    pub bytes: u64,
    pub total: u64,
}

fn piece_rel(id: &str, i: usize) -> String {
    format!("{META}/{id}.{i:03}.piece")
}

fn check_cancel(cancel: &AtomicBool) -> Result<(), DlError> {
    if cancel.load(Ordering::Relaxed) {
        Err(DlError::new("lib.dl.err.cancelled", ""))
    } else {
        Ok(())
    }
}

/// True if the file of the folder is already the piece (size and SHA-256).
fn piece_ok(folder: &Folder, rel: &str, p: &Piece) -> bool {
    match folder.stat(rel) {
        Ok(e) if e.size == p.dimensione => folder
            .read(rel, 0, p.dimensione)
            .map_or(false, |b| sha256_hex(&b) == p.sha256),
        _ => false,
    }
}

/// Downloads and installs one package. Pieces already in the folder and correct are not fetched again; a wrong piece is fetched once more (twice in all) before
/// the download fails with `lib.dl.err.hash`.
pub fn download_package(
    folder: &Folder,
    src: &dyn Source,
    pkg: &Package,
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(Progress),
) -> Result<(), DlError> {
    if pkg.lettore_min > READER {
        return Err(DlError::new(
            "lib.dl.err.version",
            format!("{}>{}", pkg.lettore_min, READER),
        ));
    }
    let n = pkg.pezzi.len();
    let mut done = 0u64;
    for (i, p) in pkg.pezzi.iter().enumerate() {
        check_cancel(cancel)?;
        let rel = piece_rel(&pkg.id, i);
        if !piece_ok(folder, &rel, p) {
            let mut tries = 0;
            loop {
                let data = src.get(&format!("{LIBS_DIR}{}", p.file)).map_err(|e| {
                    DlError::new(
                        if e.key == "update.error.offline" {
                            "lib.dl.err.offline"
                        } else {
                            "lib.dl.err.http"
                        },
                        e.detail,
                    )
                })?;
                if data.len() as u64 == p.dimensione && sha256_hex(&data) == p.sha256 {
                    folder
                        .put(&rel, 0, data.len() as u64, &data)
                        .map_err(|e| DlError::io(e.key))?;
                    break;
                }
                tries += 1;
                if tries >= 2 {
                    return Err(DlError::new("lib.dl.err.hash", p.file.clone()));
                }
                check_cancel(cancel)?;
            }
        }
        done += p.dimensione;
        progress(Progress {
            id: pkg.id.clone(),
            piece: i + 1,
            pieces: n,
            bytes: done,
            total: pkg.dimensione,
        });
    }
    check_cancel(cancel)?;
    // assemble, check as a whole
    let new_rel = format!("{META}/{}.new", pkg.id);
    let new_path = folder.resolve(&new_rel).map_err(|e| DlError::io(e.key))?;
    if let Some(parent) = new_path.parent() {
        fs::create_dir_all(parent).map_err(DlError::io)?;
    }
    let mut out = fs::File::create(&new_path).map_err(DlError::io)?;
    let mut h = Sha256::new();
    let mut size = 0u64;
    let mut head = Vec::new();
    for (i, p) in pkg.pezzi.iter().enumerate() {
        let b = folder
            .read(&piece_rel(&pkg.id, i), 0, p.dimensione)
            .map_err(|e| DlError::io(e.key))?;
        if head.len() < 36 {
            head.extend_from_slice(&b[..b.len().min(36 - head.len())]);
        }
        h.update(&b);
        size += b.len() as u64;
        out.write_all(&b).map_err(DlError::io)?;
    }
    drop(out);
    let bad = |what: &str| {
        let _ = fs::remove_file(&new_path);
        for i in 0..n {
            let _ = folder.remove(&piece_rel(&pkg.id, i)); // the pieces are wrong as a set: start again
        }
        DlError::new("lib.dl.err.hash", format!("{what} of {}", pkg.id))
    };
    if size != pkg.dimensione || format!("{:x}", h.finalize()) != pkg.sha256 {
        return Err(bad("total"));
    }
    if head.len() < 36 || &head[..4] != b"MZLB" {
        return Err(bad("header"));
    }
    if u16::from_le_bytes([head[6], head[7]]) > READER {
        let _ = fs::remove_file(&new_path);
        return Err(DlError::new("lib.dl.err.version", pkg.id.clone()));
    }
    // swap: the old file stays until here
    let file = format!("{}.mzlib", pkg.id);
    folder
        .rename(&new_rel, &file)
        .map_err(|e| DlError::io(e.key))?;
    let st = folder.stat(&file).map_err(|e| DlError::io(e.key))?;
    let (pos, neg) = match pkg.polarita {
        1 => (pkg.spettri, 0),
        -1 => (0, pkg.spettri),
        _ => (0, 0),
    };
    let side = serde_json::json!({
        "id": pkg.id, "versione": pkg.versione, "sha256": pkg.sha256, "size": st.size, "mtime": st.mtime,
        "info": {"n": pkg.spettri, "pos": pos, "neg": neg, "dropped": 0, "peaks": pkg.picchi, "date": st.mtime}
    })
    .to_string();
    folder
        .put(
            &format!("{META}/{file}.pkg.json"),
            0,
            side.len() as u64,
            side.as_bytes(),
        )
        .map_err(|e| DlError::io(e.key))?;
    for i in 0..n {
        let _ = folder.remove(&piece_rel(&pkg.id, i));
    }
    Ok(())
}

// ------------------------------------------------------------------ external sources

#[derive(Debug, Clone)]
pub struct Policy {
    pub hosts: Vec<String>,
    pub https_only: bool,
}

impl Default for Policy {
    fn default() -> Self {
        Policy {
            hosts: ALLOWED_HOSTS.iter().map(|s| s.to_string()).collect(),
            https_only: true,
        }
    }
}

fn host_of(url: &str) -> Option<(&str, &str)> {
    let (scheme, rest) = url.split_once("://")?;
    let host = rest.split(['/', '?', '#']).next()?;
    let host = host.rsplit_once('@').map_or(host, |x| x.1); // no credentials in an address
    Some((scheme, host.split(':').next()?))
}

impl Policy {
    pub fn allows(&self, url: &str) -> bool {
        match host_of(url) {
            Some((scheme, host)) => {
                (scheme == "https" || (!self.https_only && scheme == "http"))
                    && self.hosts.iter().any(|h| h == host)
                    && !url.contains('@')
            }
            None => false,
        }
    }
}

/// Downloads an external file to `<id>.<formato>` of the folder (the page then indexes it like any file of the folder). Redirects are followed by hand, each hop
/// checked against the allow-list; the SHA-256 and the size are checked when the catalog knows them.
pub fn download_external(
    folder: &Folder,
    ext: &External,
    policy: &Policy,
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(Progress),
) -> Result<String, DlError> {
    let name = format!(
        "{}.{}",
        ext.id,
        if ext.formato.is_empty() {
            "dat"
        } else {
            &ext.formato
        }
    );
    let mut url = ext.url.clone();
    let agent = ureq::AgentBuilder::new()
        .redirects(0)
        .timeout_connect(Duration::from_secs(15))
        .timeout_read(Duration::from_secs(60))
        .user_agent("mzlab-desktop")
        .build();
    let mut resp = None;
    for _ in 0..6 {
        if !policy.allows(&url) {
            return Err(DlError::new("lib.dl.err.host", url));
        }
        let r = agent.get(&url).call().map_err(|e| match e {
            ureq::Error::Status(code, _) => DlError::new("lib.dl.err.http", code.to_string()),
            ureq::Error::Transport(t) => DlError::new("lib.dl.err.offline", t.to_string()),
        })?;
        if (300..400).contains(&r.status()) {
            url = r
                .header("location")
                .map(str::to_string)
                .ok_or_else(|| DlError::new("lib.dl.err.http", r.status().to_string()))?;
            continue;
        }
        resp = Some(r);
        break;
    }
    let resp = resp.ok_or_else(|| DlError::new("lib.dl.err.http", "too many redirects"))?;
    if resp.status() != 200 {
        return Err(DlError::new("lib.dl.err.http", resp.status().to_string()));
    }
    let total = resp
        .header("content-length")
        .and_then(|v| v.parse::<u64>().ok())
        .or(ext.dimensione)
        .unwrap_or(0);
    let part_rel = format!("{META}/{name}.part");
    let part = folder.resolve(&part_rel).map_err(|e| DlError::io(e.key))?;
    if let Some(parent) = part.parent() {
        fs::create_dir_all(parent).map_err(DlError::io)?;
    }
    let mut out = fs::File::create(&part).map_err(DlError::io)?;
    let mut rd = resp.into_reader();
    let (mut h, mut got, mut buf) = (Sha256::new(), 0u64, vec![0u8; 1 << 20]);
    let mut last = 0u64;
    loop {
        if cancel.load(Ordering::Relaxed) {
            drop(out);
            let _ = fs::remove_file(&part);
            return Err(DlError::new("lib.dl.err.cancelled", ""));
        }
        let n = rd
            .read(&mut buf)
            .map_err(|e| DlError::new("lib.dl.err.offline", e.to_string()))?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
        out.write_all(&buf[..n]).map_err(DlError::io)?;
        got += n as u64;
        if got - last >= 4 << 20 {
            last = got;
            progress(Progress {
                id: ext.id.clone(),
                piece: 0,
                pieces: 0,
                bytes: got,
                total,
            });
        }
    }
    drop(out);
    let sha = format!("{:x}", h.finalize());
    if ext.sha256.as_ref().map_or(false, |want| *want != sha)
        || ext.dimensione.map_or(false, |d| d != got)
    {
        let _ = fs::remove_file(&part);
        return Err(DlError::new("lib.dl.err.hash", name));
    }
    progress(Progress {
        id: ext.id.clone(),
        piece: 0,
        pieces: 0,
        bytes: got,
        total: got,
    });
    folder
        .rename(&part_rel, &name)
        .map_err(|e| DlError::io(e.key))?;
    Ok(sha)
}
