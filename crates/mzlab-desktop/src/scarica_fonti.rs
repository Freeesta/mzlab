//! A source of the catalog `librerie/2` (read by `scarica.rs`, shown by the library window).
use serde::{Deserialize, Serialize};

/// One source of the catalog `librerie/2` (`tools/librerie_catalogo.json`): `modo` is `site` (pieces of the site), `external` (desktop app only) or `link`.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct CatalogSource {
    pub id: String,
    #[serde(default)]
    pub gruppo: String,
    #[serde(default)]
    pub nome: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub download: Option<String>,
    #[serde(default)]
    pub licenza: Option<String>,
    #[serde(default)]
    pub formato: Option<String>,
    #[serde(default)]
    pub dimensione: Option<u64>,
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
    pub verificato: String,
    pub modo: String,
}
