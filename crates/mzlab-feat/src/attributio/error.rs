// Ported from Attributio campaign.py.
//! The errors of the campaign readers: the message is the one of Attributio (`CampaignError`, `FileNotFoundError`, `ValueError`).

use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// `CampaignError`: the campaign file or the data it points to are wrong.
    Campaign,
    /// `FileNotFoundError`.
    FileNotFound,
    /// `ValueError`: a table has a value that the format does not allow.
    Value,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CampaignError {
    pub kind: Kind,
    pub msg: String,
}

pub type Result<T> = std::result::Result<T, CampaignError>;

impl CampaignError {
    pub fn campaign(msg: impl Into<String>) -> Self {
        Self {
            kind: Kind::Campaign,
            msg: msg.into(),
        }
    }
    pub fn not_found(msg: impl Into<String>) -> Self {
        Self {
            kind: Kind::FileNotFound,
            msg: msg.into(),
        }
    }
    pub fn value(msg: impl Into<String>) -> Self {
        Self {
            kind: Kind::Value,
            msg: msg.into(),
        }
    }
}

impl fmt::Display for CampaignError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.msg)
    }
}

impl std::error::Error for CampaignError {}

/// A list of words as Python writes it in a message: `['a', 'b']`.
pub fn py_list<S: AsRef<str>>(v: &[S]) -> String {
    let items: Vec<String> = v.iter().map(|s| format!("'{}'", s.as_ref())).collect();
    format!("[{}]", items.join(", "))
}
