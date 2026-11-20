//! A source of the catalog `libraries/2` (read by `download.rs`, shown by the library window).
use serde::{Deserialize, Serialize};

/// One source of the catalog `libraries/2` (`tools/librerie_catalogo.json`): `mode` is `site` (pieces of the site), `external` (desktop app only) or `link`.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct CatalogSource {
    pub id: String,
    #[serde(default)]
    pub group: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub download: Option<String>,
    #[serde(default)]
    pub license: Option<String>,
    #[serde(default)]
    pub format: Option<String>,
    #[serde(default)]
    pub size: Option<u64>,
    #[serde(default)]
    pub sha256: Option<String>,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub tech: String,
    #[serde(default)]
    pub pol: i32,
    #[serde(default)]
    pub doi: Option<String>,
    #[serde(default)]
    pub verified: String,
    pub mode: String,
}
