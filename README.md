# Perplexity AI Web API MCP Server

MCP (Model Context Protocol) server that exposes Perplexity AI search, research, and reasoning capabilities as tools.

## Requirements

This server requires a Perplexity AI account. You need to extract two authentication tokens from your browser cookies:

- `SESSION_TOKEN` - The `next-auth.session-token` cookie value
- `CSRF_TOKEN` - The `next-auth.csrf-token` cookie value

### Getting Your Tokens

1. Log in to [perplexity.ai](https://www.perplexity.ai) in your browser
2. Open Developer Tools (F12 or right-click → Inspect)
3. Go to Application → Cookies → `https://www.perplexity.ai`
4. Copy the values of:
   - `next-auth.session-token` → use as `SESSION_TOKEN`
   - `next-auth.csrf-token` → use as `CSRF_TOKEN`

## Usage

### Running the Server

```bash
SESSION_TOKEN="your-session-token" CSRF_TOKEN="your-csrf-token" perlexity-web-mcp
```

### Testing with MCP Inspector

```bash
SESSION_TOKEN="..." CSRF_TOKEN="..." npx @modelcontextprotocol/inspector cargo run -p perlexity-web-mcp
```

### Claude Desktop Configuration

Add to your Claude Desktop config (`~/Library/Application Support/Claude/claude_desktop_config.json` on macOS):

```json
{
  "mcpServers": {
    "perplexity": {
      "command": "perlexity-web-mcp",
      "env": {
        "SESSION_TOKEN": "your-session-token",
        "CSRF_TOKEN": "your-csrf-token"
      }
    }
  }
}
```

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
  "chunks": [
    // Citation/source chunks from Perplexity
  ],
  "follow_up": {
    "backend_uuid": "uuid-for-follow-up-queries",
    "attachments": []
  }
}
```

## License

MIT
