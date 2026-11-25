//! The parts of Woodpecker's API objects this server uses.
//!
//! Everything is lenient: Go encodes empty lists and maps as `null`, and fields
//! differ between Woodpecker versions.

use serde::{Deserialize, Deserializer};

fn null_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Default + Deserialize<'de>,
{
    Ok(Option::<T>::deserialize(deserializer)?.unwrap_or_default())
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct User {
    pub login: String,
    pub admin: bool,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Version {
    pub version: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Repo {
    pub id: i64,
    pub full_name: String,
    pub default_branch: String,
    pub active: bool,
    pub forge_url: String,
    pub last_pipeline: Option<Pipeline>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Pipeline {
    pub number: i64,
    pub parent: i64,
    pub event: String,
    pub status: String,
    pub created: i64,
    pub started: i64,
    pub finished: i64,
    pub branch: String,
    pub commit: String,
    #[serde(rename = "ref")]
    pub git_ref: String,
    pub title: String,
    pub message: String,
    pub author: String,
    pub forge_url: String,
    pub deploy_to: String,
    pub from_fork: bool,
    #[serde(deserialize_with = "null_default")]
    pub errors: Vec<PipelineError>,
    #[serde(deserialize_with = "null_default")]
    pub workflows: Vec<Workflow>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct PipelineError {
    #[serde(rename = "type")]
    pub kind: String,
    pub message: String,
    pub is_warning: bool,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Workflow {
    pub name: String,
    pub state: String,
    pub error: String,
    #[serde(deserialize_with = "null_default")]
    pub children: Vec<Step>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Step {
    pub id: i64,
    pub name: String,
    pub state: String,
    pub error: String,
    pub exit_code: i64,
    pub started: i64,
    pub finished: i64,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct LogEntry {
    pub line: i64,
    /// Base64, or `null` for an empty line.
    pub data: Option<String>,
    #[serde(rename = "type")]
    pub kind: i64,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct ConfigFile {
    pub name: String,
    /// Base64.
    pub data: Option<String>,
}
