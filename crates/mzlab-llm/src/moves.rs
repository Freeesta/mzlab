//! The moves of the agent machine: what the transport feeds (`Input`) and what it must do (`Output`). Used by `agent`.

use mzlab_core::json::Value;

use crate::provider::Request;

pub enum Input {
    User(String),
    Event(crate::event::Event),
    ToolResult {
        id: String,
        ok: bool,
        content: Value,
    },
    Confirm {
        id: String,
        yes: bool,
    },
    Stop,
}

pub enum Output {
    Send(Request),
    CallTool {
        id: String,
        name: String,
        args: Value,
    },
    AskConfirm {
        id: String,
        summary: String,
    },
    /// A piece of the model's text for the user.
    Show(String),
    /// A complete `llmlog/1` line.
    Log(String),
    Done,
}
