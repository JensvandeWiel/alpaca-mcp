# woodpecker-mcp

MCP server for [Woodpecker CI](https://woodpecker-ci.org): ask why a build failed, read
logs, and (if you allow it) start, cancel and approve pipelines.

Built for Woodpecker 3.x and tested against 3.20.1.

## Setup

1. In the Woodpecker UI, open your user settings, **CLI & API**, and copy the personal
   access token.
2. Add the server to your MCP client:

```json
{
  "mcpServers": {
    "woodpecker": {
      "command": "npx",
      "args": ["-y", "@alpaca-mcp/woodpecker-mcp"],
      "env": {
        "WOODPECKER_MCP_URL": "https://ci.example.com",
        "WOODPECKER_MCP_TOKEN": "${WOODPECKER_TOKEN}"
      }
    }
  }
}
```

On Windows use `"command": "cmd"` with
`"args": ["/c", "npx", "-y", "@alpaca-mcp/woodpecker-mcp"]`. With Bun, use
`"command": "bunx"` and `"args": ["@alpaca-mcp/woodpecker-mcp"]`. `${...}` expansion works
in Claude Code; other clients may need the value itself. Do not commit your token.

Ask the assistant to call `get_server_info` to check the connection.

To use several Woodpecker instances, add the server once per instance under different names.

## Tools

Repositories are given as `owner/name` or numeric id.

| Tool | Description |
|---|---|
| `get_server_info` | Server version, token user and read-only state |
| `list_repos` | Enabled repositories with their latest pipeline |
| `list_pipelines` | Pipelines, filtered by branch, status and event |
| `get_pipeline` | One pipeline with errors, workflows and step ids |
| `get_step_logs` | Log of a step: tail, search, page backwards |
| `get_pipeline_config` | The YAML a pipeline ran with |
| `diagnose_pipeline` | Failed steps with exit code and last log lines |

Only with `read_only = false`:

| Tool | Description |
|---|---|
| `trigger_pipeline` | Manual run of a branch, with variables |
| `restart_pipeline` | Run a pipeline again |
| `cancel_pipeline` | Cancel a running or pending pipeline |
| `approve_pipeline` | Approve a blocked pipeline |
| `decline_pipeline` | Decline a blocked pipeline |

A manual run only starts workflows whose `when` condition includes the `manual` event.

## Configuration

Later sources win: defaults, TOML file, environment variables, `--set`.

| Key | Environment variable | Default | Description |
|---|---|---|---|
| `url` | `WOODPECKER_MCP_URL` | required | Server address, including a subpath, without `/api` |
| `token` | `WOODPECKER_MCP_TOKEN` | required | Personal access token |
| `read_only` | `WOODPECKER_MCP_READ_ONLY` | `true` | Hide all tools that change something |
| `max_log_lines` | `WOODPECKER_MCP_MAX_LOG_LINES` | `200` | Default log lines per call (1-2000) |
| `timeout_seconds` | `WOODPECKER_MCP_TIMEOUT_SECONDS` | `30` | Timeout per request |

| Option | Environment variable | Description |
|---|---|---|
| `--config PATH` | `WOODPECKER_MCP_CONFIG` | TOML configuration file |
| `--set KEY=VAL` | | Override one key; repeatable |
| `--log FILTER` | `WOODPECKER_MCP_LOG` | Log filter, logs go to stderr |

`woodpecker-mcp check` validates the configuration and exits. It does not contact
Woodpecker.

## Security

- The token has all rights of its user. Read-only mode limits what this server offers, not
  what the token can do.
- Prefer `https`. A warning is logged for plain `http` to another machine.
- TLS uses the certificates your system trusts, so a private CA works once installed.
- Redirects are not followed; set `url` to the final address.
- Logs, commit messages and pipeline files are written by others and may try to instruct the
  assistant. Keep your client's tool approval on when `read_only` is off.
- Approving a pull request from a fork runs outside code on your agents.
- Secrets, registries and administration are deliberately not exposed.
