# Security

## Binary Checksum Verification

Starting with the npm installer modifications in this repository, downloaded binaries are verified against SHA256 checksums published alongside each GitHub release.

### How it works

1. When the npm postinstall script downloads a precompiled binary (e.g., `perplexity-web-api-mcp-aarch64-apple-darwin.tar.xz`), it also fetches the corresponding `.sha256` sidecar file from the same release.
2. The SHA256 hash is computed during download using Node.js `crypto.createHash('sha256')` — no extra file read is needed.
3. The computed hash is compared against the expected hash from the sidecar file.
4. On mismatch, the install **aborts** with an error showing both hashes and the download URL.
5. If the checksum file is unavailable (network error, 404, unexpected format), a warning is printed to stderr but the install proceeds — this ensures backwards compatibility if the release format changes.

### Checksum file format

cargo-dist publishes checksums in the standard format:

```
<hex-digest> *<filename>
```

Example:
```
5cf078b0ea3773b1641f71edbf5e99ee817eb1dd39551898f2aa027cac8d1033 *perplexity-web-api-mcp-aarch64-apple-darwin.tar.xz
```

### Cookie-based authentication

This MCP server uses browser session cookies for authentication with Perplexity. Users should:

- Never commit cookie values to version control
- Store cookies in environment variables or secure credential stores
- Rotate cookies regularly (sessions expire after ~20 minutes)
- Be aware that cookies grant full account access — treat them as passwords

## Reporting vulnerabilities

If you discover a security vulnerability, please open a GitHub issue or contact the maintainer directly.
