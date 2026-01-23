# Perplexity Web API MCP Server

<p>
<a href="https://npmjs.com/package/perplexity-web-api-mcp">
    <a href="https://cursor.com/en/install-mcp?name=perplexity-web&config=eyJ0eXBlIjoic3RkaW8iLCJjb21tYW5kIjoibnB4IiwiYXJncyI6WyIteSIsInBlcnBsZXhpdHktd2ViLWFwaS1tY3AiXSwiZW52Ijp7IlBFUlBMRVhJVFlfU0VTU0lPTl9UT0tFTiI6IiIsIlBFUlBMRVhJVFlfQ1NSRl9UT0tFTiI6IiJ9fQ==">
        <img src="https://custom-icon-badges.demolab.com/badge/Install_in_Cursor-000000?style=for-the-badge&logo=cursor-ai-white" alt="Install in Cursor">
    </a>
    <a href="https://vscode.dev/redirect/mcp/install?name=perplexity-web&config=%7B%22type%22%3A%22stdio%22%2C%22command%22%3A%22npx%22%2C%22args%22%3A%5B%22-y%22%2C%22perplexity-web-api-mcp%22%5D%2C%22env%22%3A%7B%22PERPLEXITY_SESSION_TOKEN%22%3A%22%22%2C%22PERPLEXITY_CSRF_TOKEN%22%3A%22%22%7D%7D">
        <img src="https://custom-icon-badges.demolab.com/badge/Install_in_VS_Code-007ACC?style=for-the-badge&logo=vsc&logoColor=white" alt="Install in VS Code">
    </a>
    <img
        src="https://img.shields.io/npm/v/perplexity-web-api-mcp?style=for-the-badge&logo=npm&logoColor=white&color=CB3837"
        alt="NPM Version" />
    </a>
</p>

MCP (Model Context Protocol) server that exposes Perplexity AI search, research, and reasoning capabilities as tools.

## Requirements

### Supported Platforms

- macOS
- Linux
- ~~Windows~~ (not supported yet)

### Supported Architectures

- x86_64
- aarch64

## Configuration

### User Tokens

This server requires a Perplexity AI account. You need to extract two authentication tokens from your browser cookies:

- `PERPLEXITY_SESSION_TOKEN` - The `next-auth.session-token` cookie value
- `PERPLEXITY_CSRF_TOKEN` - The `next-auth.csrf-token` cookie value

#### Getting Your Tokens

1. Log in to [perplexity.ai](https://www.perplexity.ai) in your browser
2. Open Developer Tools (F12 or right-click → Inspect)
3. Go to Application → Cookies → `https://www.perplexity.ai`
4. Copy the values of:
   - `next-auth.session-token` → use as `PERPLEXITY_SESSION_TOKEN`
   - `next-auth.csrf-token` → use as `PERPLEXITY_CSRF_TOKEN`

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

### Other MCP Clients

Most clients can be manually configured to use the `mcpServers` wrapper in their configuration file (like Cursor). If your client doesn't work, check its documentation for the correct wrapper format.

## Available Tools

### `perplexity_search`

Quick web search using Perplexity's turbo model.

**Best for:** Quick questions, everyday searches, and conversational queries that benefit from web context.

**Parameters:**

- `query` (required): The search query or question
- `sources` (optional): Array of sources - `"web"`, `"scholar"`, `"social"`. Defaults to `["web"]`
- `language` (optional): Language code, e.g., `"en-US"`. Defaults to `"en-US"`

### `perplexity_research`

Deep, comprehensive research using Perplexity's sonar-deep-research (`pplx_alpha`) model.

**Best for:** Complex topics requiring detailed investigation, comprehensive reports, and in-depth analysis. Provides thorough analysis with citations.

**Parameters:** Same as `perplexity_search`

### `perplexity_reason`

Advanced reasoning and problem-solving using Perplexity's sonar-reasoning-pro (`pplx_reasoning`) model.

**Best for:** Logical problems, complex analysis, decision-making, and tasks requiring step-by-step reasoning.

**Parameters:** Same as `perplexity_search`

## Response Format

All tools return a JSON response with:

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
