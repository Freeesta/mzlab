//! The permission policy of the assistant: what each autonomy level may do without asking, what needs a confirmation and what is never allowed.
//! The page does not decide; it only shows the confirmation the policy asks for. Pure: no clock, no I/O.

use mzlab_core::json::{parse, Value};

use crate::schema::validate;
use crate::tools::{Class, Tool};

/// L0 advisor, L1 executor with confirmation, L2 autonomous with log.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    L0,
    L1,
    L2,
}

impl Level {
    pub fn name(self) -> &'static str {
        match self {
            Level::L0 => "L0",
            Level::L1 => "L1",
            Level::L2 => "L2",
        }
    }

    pub fn parse(s: &str) -> Option<Level> {
        [Level::L0, Level::L1, Level::L2]
            .into_iter()
            .find(|l| l.name() == s)
    }
}

/// `Course` is a didactic setting, not a security barrier: the assistant does not exist there.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Profile {
    Course,
    Research,
}

/// What a user turn has used so far.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counters {
    pub calls: u32,
    pub runs: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Decision {
    Allow,
    Confirm,
    Deny { key: &'static str },
}

/// L2 limits per user turn: tool calls, and runs before a confirmation is asked.
pub const MAX_CALLS: u32 = 20;
pub const MAX_RUNS: u32 = 3;

/// The decision for one call of `tool` with `args`: the profile first, then the level, then the arguments, then the L2 limits.
pub fn decide(
    tool: &Tool,
    args: &Value,
    level: Level,
    counters: &Counters,
    profile: Profile,
) -> Decision {
    if profile == Profile::Course {
        return Decision::Deny {
            key: "err.llm.profile.course",
        };
    }
    let by_level = match (level, tool.class) {
        (_, Class::Read | Class::Propose) => Decision::Allow,
        (Level::L0, _) => {
            return Decision::Deny {
                key: "err.llm.level.denied",
            }
        }
        (Level::L1, _) => Decision::Confirm,
        (Level::L2, Class::Run) if counters.runs >= MAX_RUNS => Decision::Confirm,
        (Level::L2, _) => Decision::Allow,
    };
    let args_ok = parse(tool.input_schema.as_bytes())
        .ok()
        .is_some_and(|s| validate(&s, args).is_ok());
    if !args_ok {
        return Decision::Deny {
            key: "err.llm.tool.args",
        };
    }
    if level == Level::L2 && counters.calls >= MAX_CALLS {
        return Decision::Deny {
            key: "err.llm.limit.calls",
        };
    }
    by_level
}
