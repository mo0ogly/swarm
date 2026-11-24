//! Narrow wire representation of provider-declared actions. No provider IDs,
//! inputs or free-form tool output cross this boundary.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Family {
    Read,
    Write,
    Command,
    Search,
    Tool,
}
impl Family {
    /// Map existing stream names to a closed public vocabulary.
    pub(crate) fn from_tool(name: &str) -> Self {
        match name {
            "Read" | "Glob" | "Grep" => Self::Read,
            "Write" | "Edit" | "MultiEdit" | "file_change" => Self::Write,
            "Bash" | "command_execution" => Self::Command,
            "WebSearch" | "web_search" => Self::Search,
            _ => Self::Tool,
        }
    }
}

#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Outcome {
    Pending,
    Succeeded,
    Failed,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Action {
    pub(crate) number: i64,
    pub(crate) family: Family,
    pub(crate) outcome: Outcome,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ActionReceipt {
    pub(crate) actions: Vec<Action>,
    pub(crate) partial: bool,
}
