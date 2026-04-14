# Perplexity Web API MCP Server

<p>
    <a href="https://cursor.com/en/install-mcp?name=perplexity-web&config=eyJ0eXBlIjoic3RkaW8iLCJjb21tYW5kIjoibnB4IiwiYXJncyI6WyIteSIsInBlcnBsZXhpdHktd2ViLWFwaS1tY3AiXSwiZW52Ijp7IlBFUlBMRVhJVFlfU0VTU0lPTl9UT0tFTiI6IiIsIlBFUlBMRVhJVFlfQ1NSRl9UT0tFTiI6IiJ9fQ==" target="_blank">
        <img src="https://custom-icon-badges.demolab.com/badge/Install_in_Cursor-000000?style=for-the-badge&logo=cursor-ai-white" alt="Install in Cursor">
    </a>
    <a href="https://vscode.dev/redirect/mcp/install?name=perplexity-web&config=%7B%22type%22%3A%22stdio%22%2C%22command%22%3A%22npx%22%2C%22args%22%3A%5B%22-y%22%2C%22perplexity-web-api-mcp%22%5D%2C%22env%22%3A%7B%22PERPLEXITY_SESSION_TOKEN%22%3A%22%22%2C%22PERPLEXITY_CSRF_TOKEN%22%3A%22%22%7D%7D" target="_blank">
        <img src="https://custom-icon-badges.demolab.com/badge/Install_in_VS_Code-007ACC?style=for-the-badge&logo=vsc&logoColor=white" alt="Install in VS Code">
    </a>
    <a href="https://www.npmjs.com/package/perplexity-web-api-mcp" target="_blank">
        <img
            src="https://img.shields.io/npm/v/perplexity-web-api-mcp?style=for-the-badge&logo=npm&logoColor=white&color=CB3837"
            alt="NPM Version" />
    </a>
</p>

MCP (Model Context Protocol) server that exposes Perplexity AI search, research, and reasoning capabilities as tools.

## No API Key Required

This MCP server uses your Perplexity account session directly — **no API key needed**.

Perplexity offers a separate [paid API](https://docs.perplexity.ai/guides/pricing) with per-request pricing that is charged independently from your Pro subscription. With this MCP, you don't need to pay for API access — your existing Perplexity subscription (or even a free account) is enough.

You still need to copy two token values manually from a logged-in browser session. This project does **not** extract cookies from browser profiles, automate browsers, or read existing logged-in browser sessions for you.

## Authentication Flow

Authentication is resolved in this order:

1. `PERPLEXITY_SESSION_TOKEN` + `PERPLEXITY_CSRF_TOKEN` environment variables
2. Saved local config in your OS user config directory
3. Interactive first-run setup when both stdin and stdout are attached to a terminal
4. Tokenless mode with a warning

That means `npx -y perplexity-web-api-mcp` can be run once in a normal terminal to save auth locally, and future runs will reuse it automatically.

## Tokenless Mode

The server can run **without any authentication**. In this mode:

- Only `perplexity_search` (links only) and `perplexity_ask` (answer with sources) are available — `perplexity_research` and `perplexity_reason` require authenticated session access.
- Both tools use the `turbo` model; `PERPLEXITY_ASK_MODEL` and `PERPLEXITY_REASON_MODEL` cannot be set (the server will throw an error if they are).
- File attachments (`files` parameter) are unavailable — they require authenticated session access.

To use tokenless mode, omit `PERPLEXITY_SESSION_TOKEN` and `PERPLEXITY_CSRF_TOKEN` and do not save local auth. If the server starts without auth in a non-interactive environment, it will continue in tokenless mode and log a warning.

For full access to all tools and model selection, provide both tokens or complete the first-run setup described in the [Configuration](#configuration) section below.

## Requirements

### Supported Platforms

- macOS (arm64, x86_64)
- Linux (x86_64, aarch64)
- Windows (x86_64)

## Configuration

### Getting Your Tokens

This server requires a Perplexity AI account. You need to copy two authentication tokens manually from your browser cookies:

1. Log in to [perplexity.ai](https://www.perplexity.ai) in your browser
2. Open Developer Tools (F12 or right-click → Inspect)
3. Go to Application → Cookies → `https://www.perplexity.ai`
4. Copy the values of:
   - `__Secure-next-auth.session-token` → use as `PERPLEXITY_SESSION_TOKEN`
   - `next-auth.csrf-token` → use as `PERPLEXITY_CSRF_TOKEN`

### Saved Local Config

If you run `npx -y perplexity-web-api-mcp` in a normal interactive terminal with no auth configured, the binary offers a first-run setup wizard. It prompts for the session token and CSRF token, validates them by initializing the client, and only saves them after validation succeeds.

Saved auth is stored as JSON in your OS user config directory:

- macOS: `~/Library/Application Support/perplexity-web-api-mcp/config.json`
- Linux: `~/.config/perplexity-web-api-mcp/config.json`
- Windows: `%AppData%\perplexity-web-api-mcp\config.json`

Delete that file manually if you want to remove the saved auth cache.

If a previously saved session expires, the server now rejects it during startup instead of silently treating it as authenticated. Remove the cached file and either:

1. rerun `npx -y perplexity-web-api-mcp` in an interactive terminal to save fresh tokens, or
2. provide fresh `PERPLEXITY_SESSION_TOKEN` and `PERPLEXITY_CSRF_TOKEN` values in your MCP client config.

### Environment Variables

Environment variables are still the highest-priority auth source.

- `PERPLEXITY_SESSION_TOKEN` (optional, highest priority): Perplexity session token (`__Secure-next-auth.session-token` cookie from your browser). Required for `perplexity_research`, `perplexity_reason`, and file attachments when you are not using saved local auth.
- `PERPLEXITY_CSRF_TOKEN` (optional, highest priority): Perplexity CSRF token (`next-auth.csrf-token` cookie). Required for `perplexity_research`, `perplexity_reason`, and file attachments when you are not using saved local auth.
- `PERPLEXITY_ASK_MODEL` (optional, requires authenticated session access): Model for `perplexity_ask`.
  Valid values:
    - `turbo` (the implicit tokenless/default `perplexity_search` and `perplexity_ask` model)
    - `pro-auto` (default for authenticated)
    - `sonar`
    - `gpt-5.4`
    - `claude-4.6-sonnet`
    - `nemotron-3-super`
- `PERPLEXITY_REASON_MODEL` (optional, requires authenticated session access): Model for `perplexity_reason`.
  Valid values:
    - `gemini-3.1-pro` (default)
    - `gpt-5.4-thinking`
    - `claude-4.6-sonnet-thinking`
- `PERPLEXITY_INCOGNITO` (optional, default: `true`): Whether requests should use Perplexity's incognito mode.
  Valid values: `true` or `false`
- `MCP_TRANSPORT` (optional, default: `stdio`): Transport mode. Valid values are `stdio` and `streamable-http`. `streamable-http` requires building with the `streamable-http` cargo feature.
- `MCP_HOST` (optional, default: `0.0.0.0`): Host/interface to bind when `MCP_TRANSPORT=streamable-http`.
- `MCP_PORT` (optional, default: `8080`): TCP port to bind when `MCP_TRANSPORT=streamable-http`.

When authentication cookies are provided, client startup validates them immediately by warming up `/api/auth/session`. Invalid or expired cookies fail startup instead of producing partial access later. The configured client timeout is treated as the total budget for that warm-up, including reading the session payload.

In tokenless mode, empty `PERPLEXITY_ASK_MODEL` / `PERPLEXITY_REASON_MODEL` values are ignored, but any non-empty model override is rejected because tokenless mode always uses the built-in free/default behavior.

### First-Run Setup for Non-Interactive MCP Clients

Many MCP clients start MCP servers non-interactively, so they cannot answer the first-run prompts. For those clients, either:

1. set `PERPLEXITY_SESSION_TOKEN` and `PERPLEXITY_CSRF_TOKEN` directly in the client config, or
2. run `npx -y perplexity-web-api-mcp` once in a normal terminal to save local auth first, then configure the client without auth env vars.

### Claude Code

```bash
claude mcp add perplexity --env PERPLEXITY_SESSION_TOKEN="your-session-token" --env PERPLEXITY_CSRF_TOKEN="your-csrf-token" -- npx -y perplexity-web-api-mcp
```

### Cursor, Claude Desktop & Windsurf

I recommend using the one-click install badge at the top of this README for Cursor.

For manual setup, all these clients use the same `mcpServers` format:

| Client | Config File |
|--------|-------------|
| Cursor | `~/.cursor/mcp.json` |
| Claude Desktop | `claude_desktop_config.json` |
| Windsurf | `~/.codeium/windsurf/mcp_config.json` |

```json
{
  "mcpServers": {
    "perplexity": {
      "command": "npx",
      "args": ["-y", "perplexity-web-api-mcp"],
      "env": {
        "PERPLEXITY_SESSION_TOKEN": "your-session-token",
        "PERPLEXITY_CSRF_TOKEN": "your-csrf-token"
      }
    }
  }
}
```

### Zed

Add following following to `context_servers` in your [settings file](https://zed.dev/docs/configuring-zed.html#settings-files):

```json
{
  "context_servers": {
    "perplexity": {
      "command": "npx",
      "args": ["-y", "perplexity-web-api-mcp"],
      "env": {
        "PERPLEXITY_SESSION_TOKEN": "your-session-token",
        "PERPLEXITY_CSRF_TOKEN": "your-csrf-token"
      }
    }
  }
}
```

### VS Code

I recommend using the one-click install badge at the top of this README for VS Code, or for manual setup, add to `.vscode/mcp.json`:

```json
{
  "servers": {
    "perplexity": {
      "type": "stdio",
      "command": "npx",
      "args": ["-y", "perplexity-web-api-mcp"],
      "env": {
        "PERPLEXITY_SESSION_TOKEN": "your-session-token",
        "PERPLEXITY_CSRF_TOKEN": "your-csrf-token"
      }
    }
  }
}
```

### Codex

```bash
codex mcp add perplexity --env PERPLEXITY_SESSION_TOKEN="your-session-token" --env PERPLEXITY_CSRF_TOKEN="your-csrf-token" -- npx -y perplexity-web-api-mcp
```

### Building from Source

Source build instructions, including optional cargo features, are documented in [CONTRIBUTING.md](CONTRIBUTING.md).

### Local Verification

Before opening a PR or cutting a release, run:

```bash
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo build --workspace --all-targets
cargo test --workspace --lib
```

To validate the optional HTTP transport build:

```bash
cargo build -p perplexity-web-api-mcp --all-targets --features streamable-http
```

Live authenticated e2e coverage remains opt-in because it depends on real Perplexity session cookies:

```bash
PERPLEXITY_SESSION_TOKEN="..." \
PERPLEXITY_CSRF_TOKEN="..." \
cargo test -p perplexity-web-api --test integration -- --ignored --test-threads=1
```

### Other MCP Clients

Most clients can be manually configured to use the `mcpServers` wrapper in their configuration file (like Cursor). If your client doesn't work, check its documentation for the correct wrapper format.

## Docker

A pre-built multi-arch image (`linux/amd64`, `linux/arm64`) is available on Docker Hub:

```bash
docker run -d \
  -p 8080:8080 \
  -e PERPLEXITY_SESSION_TOKEN="your-session-token" \
  -e PERPLEXITY_CSRF_TOKEN="your-csrf-token" \
  mishamyrt/perplexity-web-api-mcp
```

The container exposes the MCP server via Streamable HTTP at `http://localhost:8080/mcp`.
The Docker image is built with `--features streamable-http`; local/source builds need the same feature if you want HTTP transport.

Configure your MCP client to connect:

```json
{
  "mcpServers": {
    "perplexity": {
      "url": "http://localhost:8080/mcp"
    }
  }
}
```

### Environment Variables (Docker-specific)

| Variable | Default | Description |
|----------|---------|-------------|
| `MCP_TRANSPORT` | `streamable-http` | Transport mode. `stdio` or `streamable-http` (requires the `streamable-http` cargo feature) |
| `MCP_HOST` | `0.0.0.0` | Host address to bind |
| `MCP_PORT` | `8080` | Port to listen on |

The [auth flow, model variables, and incognito flag](#configuration) described above work the same way in Docker.

## Available Tools

### `perplexity_search`

Quick web search using the `turbo` model. Returns only links, titles, and snippets — no generated answer.

**Best for:** Finding relevant URLs and sources quickly.

**Parameters:**

- `query` (required): The search query or question
- `sources` (optional): Array of sources — `"web"`, `"scholar"`, `"social"`. Defaults to `["web"]`
- `language` (optional): Language code, e.g., `"en-US"`. Defaults to `"en-US"`

> File attachments are not supported by this tool.

### `perplexity_ask`

Ask Perplexity AI a question and get a comprehensive answer with source citations. By default uses the best model (Pro auto mode) when authenticated via environment variables or saved local config, or `turbo` in tokenless mode. Can be configured via `PERPLEXITY_ASK_MODEL`.

**Best for:** Getting detailed answers to questions with web context.

**Parameters:** Same as `perplexity_search`, plus:

- `files` (optional, requires authenticated session access): Array of file attachments for document analysis. See [File Attachments](#file-attachments).

### `perplexity_reason`

Advanced reasoning and problem-solving. By default uses Perplexity's `sonar-reasoning` model, but can be configured via `PERPLEXITY_REASON_MODEL`.

**Best for:** Logical problems, complex analysis, decision-making, and tasks requiring step-by-step reasoning.

**Parameters:** Same as `perplexity_ask`.

### `perplexity_research`

Deep, comprehensive research using Perplexity's sonar-deep-research (`pplx_alpha`) model.

**Best for:** Complex topics requiring detailed investigation, comprehensive reports, and in-depth analysis. Provides thorough analysis with citations.

**Parameters:** Same as `perplexity_ask`.

## File Attachments

`perplexity_ask`, `perplexity_research`, and `perplexity_reason` accept an optional `files` parameter for document analysis. **Requires authenticated session access from environment variables or saved local setup.**

Each entry in the `files` array must have:

- `filename` (required): Filename with extension, e.g. `"report.pdf"` or `"notes.txt"`
- `text` (mutually exclusive with `data`): Plain-text file content. Use for `.txt`, `.md`, `.csv`, `.json`, source code, etc.
- `data` (mutually exclusive with `text`): Base64-encoded binary content. Use for `.pdf`, `.docx`, images, etc.

**Example — plain text:**

```json
{
  "query": "Summarise the key points",
  "files": [
    {
      "filename": "notes.txt",
      "text": "Meeting notes: Q1 revenue up 12%..."
    }
  ]
}
```

**Example — binary file (PDF):**

```json
{
  "query": "What does this contract say about termination?",
  "files": [
    {
      "filename": "contract.pdf",
      "data": "JVBERi0xLjQK..."
    }
  ]
}
```

Multiple files can be passed in a single request — they are uploaded to Perplexity's storage in parallel before the query is sent.

## Response Format

`perplexity_search` returns only web results:

```json
{
  "web_results": [
    {
      "name": "Source name",
      "url": "https://example.com",
      "snippet": "Source snippet"
    }
  ]
}
```

`perplexity_ask`, `perplexity_research`, and `perplexity_reason` return a full response:

```json
{
  "answer": "The generated answer text...",
  "web_results": [
    {
      "name": "Source name",
      "url": "https://example.com",
      "snippet": "Source snippet"
    }
  ],
  "follow_up": {
    "backend_uuid": "uuid-for-follow-up-queries",
    "attachments": []
  }
}
```

## License

MIT
