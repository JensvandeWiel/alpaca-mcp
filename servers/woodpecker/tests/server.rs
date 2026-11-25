//! Drives the server through a real MCP client against a mocked Woodpecker API.

use rmcp::{RoleClient, ServiceExt, model::CallToolRequestParams, service::RunningService};
use serde_json::{Value, json};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{body_json, header, method, path, query_param, query_param_is_missing},
};
use woodpecker_mcp::{Config, WoodpeckerServer};

type Client = RunningService<RoleClient, ()>;

async fn connect(api: &MockServer, read_only: bool) -> anyhow::Result<Client> {
    let server = WoodpeckerServer::new(Config {
        url: Some(api.uri()),
        token: Some("test-token".to_owned()),
        read_only,
        ..Config::default()
    })?;
    let (server_side, client_side) = tokio::io::duplex(1 << 20);
    tokio::spawn(async move {
        server.serve(server_side).await?.waiting().await?;
        anyhow::Ok(())
    });
    Ok(().serve(client_side).await?)
}

/// Returns whether the call failed, and the text of the result.
async fn call(client: &Client, tool: &'static str, arguments: Value) -> (bool, String) {
    let arguments = arguments.as_object().cloned().unwrap_or_default();
    let result = client
        .call_tool(CallToolRequestParams::new(tool).with_arguments(arguments))
        .await
        .expect("tool call failed at protocol level");
    let text = result.content[0]
        .as_text()
        .map(|t| t.text.clone())
        .unwrap_or_default();
    (result.is_error.unwrap_or(false), text)
}

async fn get(api: &MockServer, route: &str, body: Value) {
    Mock::given(method("GET"))
        .and(path(route))
        .and(header("authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(api)
        .await;
}

async fn post(api: &MockServer, route: &str, response: ResponseTemplate) {
    Mock::given(method("POST"))
        .and(path(route))
        .and(header("authorization", "Bearer test-token"))
        .respond_with(response)
        .expect(1)
        .mount(api)
        .await;
}

async fn tool_names(client: &Client) -> Vec<String> {
    let mut names: Vec<String> = client
        .list_all_tools()
        .await
        .expect("cannot list tools")
        .into_iter()
        .map(|tool| tool.name.into_owned())
        .collect();
    names.sort();
    names
}

const READ_TOOLS: [&str; 7] = [
    "diagnose_pipeline",
    "get_pipeline",
    "get_pipeline_config",
    "get_server_info",
    "get_step_logs",
    "list_pipelines",
    "list_repos",
];

#[tokio::test]
async fn read_only_mode_has_no_write_tools() -> anyhow::Result<()> {
    let api = MockServer::start().await;

    let client = connect(&api, true).await?;
    assert_eq!(tool_names(&client).await, READ_TOOLS);

    // Calling a hidden tool anyway must not reach Woodpecker.
    let arguments = json!({ "repo": "1", "number": 1 });
    let result = client
        .call_tool(
            CallToolRequestParams::new("restart_pipeline")
                .with_arguments(arguments.as_object().cloned().unwrap_or_default()),
        )
        .await;
    assert!(result.is_err());
    assert!(api.received_requests().await.unwrap_or_default().is_empty());
    Ok(())
}

#[tokio::test]
async fn write_mode_adds_annotated_tools() -> anyhow::Result<()> {
    let api = MockServer::start().await;
    let client = connect(&api, false).await?;

    let tools = client.list_all_tools().await?;
    assert_eq!(tools.len(), READ_TOOLS.len() + 5);
    for tool in tools {
        let annotations = tool.annotations.expect("every tool is annotated");
        let read_only = READ_TOOLS.contains(&tool.name.as_ref());
        assert_eq!(annotations.read_only_hint, Some(read_only), "{}", tool.name);
        if !read_only {
            assert_eq!(annotations.destructive_hint, Some(true), "{}", tool.name);
        }
    }
    Ok(())
}

#[tokio::test]
async fn lists_pipelines_by_repository_name() -> anyhow::Result<()> {
    let api = MockServer::start().await;
    get(&api, "/api/repos/lookup/octo/app", json!({ "id": 7 })).await;
    Mock::given(method("GET"))
        .and(path("/api/repos/7/pipelines"))
        .and(query_param("status", "failure"))
        .and(query_param("event", "pull_request"))
        .and(query_param("perPage", "50"))
        .and(query_param("page", "1"))
        .and(query_param_is_missing("branch"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{
            "number": 3, "status": "failure", "event": "pull_request", "branch": "main",
            "commit": "0123456789abcdef", "message": "subject\n\nbody", "created": 1_700_000_000,
            "started": 100, "finished": 160, "errors": null, "workflows": null
        }])))
        .expect(1)
        .mount(&api)
        .await;
    let client = connect(&api, true).await?;

    let (is_error, text) = call(
        &client,
        "list_pipelines",
        json!({ "repo": "octo/app", "status": "failure", "event": "pull_request", "per_page": 500 }),
    )
    .await;

    assert!(!is_error, "{text}");
    let pipeline = &serde_json::from_str::<Value>(&text)?["pipelines"][0];
    assert_eq!(pipeline["number"], 3);
    assert_eq!(pipeline["commit"], "0123456789ab");
    assert_eq!(pipeline["message"], "subject");
    assert_eq!(pipeline["duration_seconds"], 60);
    assert_eq!(pipeline["created"], "2023-11-14T22:13:20Z");
    Ok(())
}

#[tokio::test]
async fn diagnoses_the_newest_pipeline() -> anyhow::Result<()> {
    let api = MockServer::start().await;
    get(&api, "/api/repos/7/pipelines", json!([{ "number": 4 }])).await;
    get(
        &api,
        "/api/repos/7/pipelines/4",
        json!({
            "number": 4, "status": "failure", "event": "manual", "branch": "main",
            "workflows": [{ "name": "test", "state": "failure", "children": [
                { "id": 10, "name": "clone", "state": "success" },
                { "id": 11, "name": "unit", "state": "failure", "exit_code": 101 },
                { "id": 12, "name": "gone", "state": "killed" }
            ]}]
        }),
    )
    .await;
    // "test result: FAILED"
    get(
        &api,
        "/api/repos/7/logs/4/11",
        json!([{ "line": 0, "type": 0, "data": "dGVzdCByZXN1bHQ6IEZBSUxFRA==" }]),
    )
    .await;
    let client = connect(&api, true).await?;

    let (is_error, report) = call(&client, "diagnose_pipeline", json!({ "repo": "7" })).await;

    assert!(!is_error, "{report}");
    assert!(report.contains("test / unit (step_id 11): failure, exit code 101"));
    assert!(report.contains("1: test result: FAILED"));
    assert!(report.contains("Log unavailable"), "{report}");
    assert!(!report.contains("clone"), "{report}");
    Ok(())
}

#[tokio::test]
async fn api_errors_become_tool_errors() -> anyhow::Result<()> {
    let api = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(401).set_body_string("User not authorized"))
        .mount(&api)
        .await;
    let client = connect(&api, true).await?;

    let (is_error, text) = call(&client, "get_server_info", json!({})).await;
    assert!(is_error);
    assert!(
        text.contains("401") && text.contains("access token"),
        "{text}"
    );
    assert!(!text.contains("test-token"), "{text}");

    let (is_error, text) = call(&client, "list_pipelines", json!({ "repo": "../admin" })).await;
    assert!(is_error);
    assert!(text.contains("Invalid repository"), "{text}");
    Ok(())
}

#[tokio::test]
async fn does_not_follow_redirects() -> anyhow::Result<()> {
    let api = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(302).insert_header("location", "https://elsewhere.example"),
        )
        .expect(1)
        .mount(&api)
        .await;
    let client = connect(&api, true).await?;

    let (is_error, text) = call(&client, "get_server_info", json!({})).await;
    assert!(is_error);
    assert!(text.contains("https://elsewhere.example"), "{text}");
    Ok(())
}

#[tokio::test]
async fn trigger_uses_the_default_branch() -> anyhow::Result<()> {
    let api = MockServer::start().await;
    get(
        &api,
        "/api/repos/7",
        json!({ "id": 7, "default_branch": "trunk" }),
    )
    .await;
    Mock::given(method("POST"))
        .and(path("/api/repos/7/pipelines"))
        .and(body_json(
            json!({ "branch": "trunk", "variables": { "A": "1" } }),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "number": 9 })))
        .expect(1)
        .mount(&api)
        .await;
    let client = connect(&api, false).await?;

    let (is_error, text) = call(
        &client,
        "trigger_pipeline",
        json!({ "repo": "7", "variables": { "A": "1" } }),
    )
    .await;

    assert!(!is_error, "{text}");
    assert_eq!(serde_json::from_str::<Value>(&text)?["number"], 9);
    Ok(())
}

#[tokio::test]
async fn reports_when_no_workflow_matched() -> anyhow::Result<()> {
    let api = MockServer::start().await;
    post(&api, "/api/repos/7/pipelines", ResponseTemplate::new(204)).await;
    let client = connect(&api, false).await?;

    let (is_error, text) = call(
        &client,
        "trigger_pipeline",
        json!({ "repo": "7", "branch": "main" }),
    )
    .await;

    assert!(is_error);
    assert!(text.contains("No pipeline was started"), "{text}");
    Ok(())
}

#[tokio::test]
async fn pipeline_actions_post_to_the_right_routes() -> anyhow::Result<()> {
    let api = MockServer::start().await;
    let pipeline = ResponseTemplate::new(200).set_body_json(json!({ "number": 5 }));
    post(&api, "/api/repos/7/pipelines/4", pipeline.clone()).await;
    post(&api, "/api/repos/7/pipelines/4/approve", pipeline.clone()).await;
    post(&api, "/api/repos/7/pipelines/4/decline", pipeline).await;
    post(
        &api,
        "/api/repos/7/pipelines/4/cancel",
        ResponseTemplate::new(204),
    )
    .await;
    let client = connect(&api, false).await?;

    for tool in [
        "restart_pipeline",
        "approve_pipeline",
        "decline_pipeline",
        "cancel_pipeline",
    ] {
        let (is_error, text) = call(&client, tool, json!({ "repo": "7", "number": 4 })).await;
        assert!(!is_error, "{tool}: {text}");
    }

    // Woodpecker would turn any query parameter of a restart into a variable.
    let requests = api.received_requests().await.unwrap_or_default();
    assert!(requests.iter().all(|r| r.url.query().is_none()));
    Ok(())
}
