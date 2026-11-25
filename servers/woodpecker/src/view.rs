//! What tools return: smaller and easier to read than Woodpecker's raw objects.

use chrono::{DateTime, SecondsFormat};
use serde::Serialize;

use crate::model;

const MAX_MESSAGE_CHARS: usize = 200;

#[derive(Serialize)]
pub struct Repo {
    pub id: i64,
    pub full_name: String,
    pub default_branch: String,
    pub active: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_pipeline: Option<Pipeline>,
}

#[derive(Serialize)]
pub struct Pipeline {
    pub number: i64,
    pub status: String,
    pub event: String,
    pub branch: String,
    pub commit: String,
    #[serde(rename = "ref", skip_serializing_if = "String::is_empty")]
    pub git_ref: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub message: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub author: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_seconds: Option<i64>,
    /// Number of the pipeline this one restarted.
    #[serde(skip_serializing_if = "is_zero")]
    pub restart_of: i64,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub deploy_to: String,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub from_fork: bool,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub forge_url: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<PipelineError>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub workflows: Vec<Workflow>,
}

#[derive(Serialize)]
pub struct PipelineError {
    #[serde(rename = "type")]
    pub kind: String,
    pub message: String,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub warning: bool,
}

#[derive(Serialize)]
pub struct Workflow {
    pub name: String,
    pub state: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub error: String,
    pub steps: Vec<Step>,
}

#[derive(Serialize)]
pub struct Step {
    pub step_id: i64,
    pub name: String,
    pub state: String,
    pub exit_code: i64,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_seconds: Option<i64>,
}

fn is_zero(value: &i64) -> bool {
    *value == 0
}

fn timestamp(seconds: i64) -> Option<String> {
    (seconds > 0)
        .then(|| DateTime::from_timestamp(seconds, 0))
        .flatten()
        .map(|time| time.to_rfc3339_opts(SecondsFormat::Secs, true))
}

fn duration(started: i64, finished: i64) -> Option<i64> {
    (started > 0 && finished >= started).then(|| finished - started)
}

fn first_line(text: &str) -> String {
    let line = text.lines().next().unwrap_or_default();
    line.chars().take(MAX_MESSAGE_CHARS).collect()
}

impl From<model::Repo> for Repo {
    fn from(repo: model::Repo) -> Self {
        Self {
            id: repo.id,
            full_name: repo.full_name,
            default_branch: repo.default_branch,
            active: repo.active,
            last_pipeline: repo.last_pipeline.map(Pipeline::summary),
        }
    }
}

impl Pipeline {
    /// For lists: without errors, workflows and links.
    pub fn summary(pipeline: model::Pipeline) -> Self {
        let mut view = Self::from(pipeline);
        view.errors = Vec::new();
        view.workflows = Vec::new();
        view.forge_url = String::new();
        view
    }
}

impl From<model::Pipeline> for Pipeline {
    fn from(p: model::Pipeline) -> Self {
        let message = if p.title.is_empty() {
            &p.message
        } else {
            &p.title
        };
        Self {
            number: p.number,
            message: first_line(message),
            status: p.status,
            event: p.event,
            branch: p.branch,
            commit: p.commit.chars().take(12).collect(),
            git_ref: p.git_ref,
            author: p.author,
            created: timestamp(p.created),
            duration_seconds: duration(p.started, p.finished),
            restart_of: p.parent,
            deploy_to: p.deploy_to,
            from_fork: p.from_fork,
            forge_url: p.forge_url,
            errors: p.errors.into_iter().map(PipelineError::from).collect(),
            workflows: p.workflows.into_iter().map(Workflow::from).collect(),
        }
    }
}

impl From<model::PipelineError> for PipelineError {
    fn from(error: model::PipelineError) -> Self {
        Self {
            kind: error.kind,
            message: error.message,
            warning: error.is_warning,
        }
    }
}

impl From<model::Workflow> for Workflow {
    fn from(workflow: model::Workflow) -> Self {
        Self {
            name: workflow.name,
            state: workflow.state,
            error: workflow.error,
            steps: workflow.children.into_iter().map(Step::from).collect(),
        }
    }
}

impl From<model::Step> for Step {
    fn from(step: model::Step) -> Self {
        Self {
            step_id: step.id,
            name: step.name,
            state: step.state,
            exit_code: step.exit_code,
            error: step.error,
            duration_seconds: duration(step.started, step.finished),
        }
    }
}
