//! The MCP tools, split by whether they change anything.

mod read;
mod write;

use serde::{Deserialize, Serialize};

use crate::client::Error;

/// Errors are returned to the model as tool errors it can react to.
type ToolResult = Result<String, String>;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct PipelineParams {
    /// Repository as `owner/name` or numeric id.
    pub repo: String,
    /// Pipeline number, as shown in the Woodpecker UI.
    pub number: i64,
}

fn failed(error: Error) -> String {
    error.to_string()
}

fn json(value: &impl Serialize) -> ToolResult {
    serde_json::to_string(value).map_err(|error| format!("cannot encode the result: {error}"))
}
