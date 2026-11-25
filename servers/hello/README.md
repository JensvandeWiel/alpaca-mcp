# hello-mcp

Example MCP server that greets people

## Run

```sh
npx -y @alpaca-mcp/hello-mcp
bunx @alpaca-mcp/hello-mcp
```

## MCP client configuration

```json
{
  "mcpServers": {
    "hello": {
      "command": "npx",
      "args": ["-y", "@alpaca-mcp/hello-mcp"],
      "env": { "HELLO_MCP_GREETING": "Hi" }
    }
  }
}
```

On Windows use `"command": "cmd"` with `"args": ["/c", "npx", "-y", "@alpaca-mcp/hello-mcp"]`.

## Tools

| Tool    | Description          |
|---------|----------------------|
| `greet` | Greet someone by name |

## Configuration

Later sources win: defaults, TOML file, environment variables, `--set`.

| Key        | Environment variable | Default   | Description          |
|------------|----------------------|-----------|----------------------|
| `greeting` | `HELLO_MCP_GREETING`   | `"Hello"` | Word used to greet   |

| Option           | Environment variable | Description                       |
|------------------|----------------------|-----------------------------------|
| `--config PATH`  | `HELLO_MCP_CONFIG`     | TOML configuration file           |
| `--set KEY=VAL`  |                      | Override one key; repeatable      |
| `--log FILTER`   | `HELLO_MCP_LOG`        | Log filter, logs go to stderr     |

`hello-mcp check` validates the configuration and exits.
