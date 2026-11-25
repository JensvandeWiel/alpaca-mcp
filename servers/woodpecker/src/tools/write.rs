//! Tools that change something. They are only registered when `read_only` is off.

use std::collections::BTreeMap;

use reqwest::{Method, StatusCode};
use rmcp::{handler::server::wrapper::Parameters, tool, tool_router};
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{PipelineParams, ToolResult, failed, json};
use crate::{WoodpeckerServer, client::Response, model, view};

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct TriggerParams {
    /// Repository as `owner/name` or numeric id.
    pub repo: String,
    /// Branch to run. Defaults to the repository's default branch.
    pub branch: Option<String>,
    /// Extra environment variables for the pipeline.
    pub variables: Option<BTreeMap<String, String>>,
    /// Note shown with the pipeline.
    pub message: Option<String>,
}

#[derive(Serialize)]
struct PipelineOptions {
    branch: String,
    variables: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

fn started(response: &Response) -> ToolResult {
    let pipeline: model::Pipeline = response.json().map_err(failed)?;
    json(&view::Pipeline::summary(pipeline))
}

impl WoodpeckerServer {
    /// `POST repos/{repo}/pipelines/{number}[/action]`
    async fn pipeline_action(
        &self,
        params: &PipelineParams,
        action: Option<&str>,
    ) -> Result<Response, String> {
        let repo = self.client.repo_id(&params.repo).await.map_err(failed)?;
        let number = params.number.to_string();
        let mut segments = vec!["repos", &repo, "pipelines", &number];
        segments.extend(action);
        self.client
            .send(self.client.request(Method::POST, &segments))
            .await
            .map_err(failed)
    }
}

#[tool_router(router = write_router, vis = "pub(crate)")]
impl WoodpeckerServer {
    #[tool(
        description = "Start a new manual pipeline on a branch. This runs the repository's pipeline, which may build, publish or deploy.",
        annotations(read_only_hint = false, destructive_hint = true)
    )]
    async fn trigger_pipeline(&self, Parameters(params): Parameters<TriggerParams>) -> ToolResult {
        let repo = self.client.repo_id(&params.repo).await.map_err(failed)?;
        let branch = match params.branch.filter(|b| !b.is_empty()) {
            Some(branch) => branch,
            None => {
                let found: model::Repo =
                    self.client.get(&["repos", &repo]).await.map_err(failed)?;
                found.default_branch
            }
        };
        let options = PipelineOptions {
            branch,
            variables: params.variables.unwrap_or_default(),
            message: params.message,
        };
        let request = self
            .client
            .request(Method::POST, &["repos", &repo, "pipelines"])
            .json(&options);
        let response = self.client.send(request).await.map_err(failed)?;
        if response.status == StatusCode::NO_CONTENT {
            return Err(format!(
                "No pipeline was started: no workflow on branch `{}` matches the `manual` event. \
                 The pipeline configuration needs a `when` condition that includes it.",
                options.branch
            ));
        }
        started(&response)
    }

    #[tool(
        description = "Run an existing pipeline again as a new pipeline, with all its workflows.",
        annotations(read_only_hint = false, destructive_hint = true)
    )]
    async fn restart_pipeline(&self, Parameters(params): Parameters<PipelineParams>) -> ToolResult {
        // No query parameters: Woodpecker turns unknown ones into pipeline variables.
        let response = self.pipeline_action(&params, None).await?;
        if response.status == StatusCode::NO_CONTENT {
            return Err("No pipeline was started: no workflow matched.".to_owned());
        }
        started(&response)
    }

    #[tool(
        description = "Cancel a running or pending pipeline.",
        annotations(read_only_hint = false, destructive_hint = true)
    )]
    async fn cancel_pipeline(&self, Parameters(params): Parameters<PipelineParams>) -> ToolResult {
        self.pipeline_action(&params, Some("cancel")).await?;
        json(&json!({ "canceled": params.number }))
    }

    #[tool(
        description = "Approve a blocked pipeline so it runs. For pull requests from forks this executes code written by outsiders: review the changes first and only approve when the user asked for it.",
        annotations(read_only_hint = false, destructive_hint = true)
    )]
    async fn approve_pipeline(&self, Parameters(params): Parameters<PipelineParams>) -> ToolResult {
        let response = self.pipeline_action(&params, Some("approve")).await?;
        started(&response)
    }

    #[tool(
        description = "Decline a blocked pipeline so it never runs.",
        annotations(read_only_hint = false, destructive_hint = true)
    )]
    async fn decline_pipeline(&self, Parameters(params): Parameters<PipelineParams>) -> ToolResult {
        let response = self.pipeline_action(&params, Some("decline")).await?;
        started(&response)
    }
}
