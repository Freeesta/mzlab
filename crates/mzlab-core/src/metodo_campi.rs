//! The fields of `metodo/1.campi` (`{origine, chiave, valore, unita?, mappato?, identita?}`, spec: `docs/agenti/metodo.md`) and the ident of the acquisition that marks them
//! `identita`: shared by the readers of Thermo (`metodo_raw::campi`) and Agilent, so they sit outside the Thermo feature.

use crate::metodo::{obj, text, V};

/// What identifies a computer or a place of the acquisition in the texts of the file (the user is a fixed field: a user name is often a word that
/// the texts hold for other reasons, as «Fusion» in the model): fields whose value names them are marked `identita`.
#[derive(Default)]
pub struct Ident {
    pub computer: String,
    pub paths: Vec<String>,
    /// Other values that vanish from the texts (the serial number of an instrument).
    pub names: Vec<String>,
}

impl Ident {
    fn needles(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.computer.as_str())
            .chain(self.paths.iter().map(String::as_str))
            .chain(self.names.iter().map(String::as_str))
            .filter(|n| n.len() >= 4)
    }

    /// Whether `t` names the computer, the user or a path of the acquisition.
    pub fn names(&self, t: &str) -> bool {
        let t = t.to_ascii_lowercase();
        self.needles().any(|n| t.contains(&n.to_ascii_lowercase()))
    }
}

pub struct Campo {
    pub origine: String,
    pub chiave: String,
    pub valore: String,
    pub unita: Option<String>,
    pub mappato: Option<&'static str>,
    pub identita: bool,
}

impl Campo {
    pub fn new(origine: &str, chiave: &str, valore: impl ToString) -> Campo {
        Campo {
            origine: origine.into(),
            chiave: chiave.into(),
            valore: valore.to_string(),
            unita: None,
            mappato: None,
            identita: false,
        }
    }
    pub fn unit(mut self, u: &str) -> Campo {
        self.unita = text(u).map(|_| u.to_string());
        self
    }
    pub fn map(mut self, to: &'static str) -> Campo {
        self.mappato = Some(to);
        self
    }
    #[cfg(feature = "thermo")]
    pub(crate) fn map_opt(mut self, to: Option<&'static str>) -> Campo {
        self.mappato = to;
        self
    }
    pub fn id(mut self) -> Campo {
        self.identita = true;
        self
    }
    pub fn json(&self) -> Option<V> {
        obj(vec![
            ("origine", text(&self.origine)),
            ("chiave", text(&self.chiave)),
            ("valore", Some(V::s(self.valore.as_str()))),
            ("unita", self.unita.as_deref().and_then(text)),
            ("mappato", self.mappato.map(V::s)),
            ("identita", self.identita.then_some(V::Bool(true))),
        ])
    }
}
