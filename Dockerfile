FROM rust:1-bookworm AS builder

RUN apt-get update && apt-get install -y cmake clang && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . .

RUN cargo build --profile dist --bin perplexity-web-api-mcp --features streamable-http

FROM gcr.io/distroless/cc-debian12

COPY --from=builder /app/target/dist/perplexity-web-api-mcp /perplexity-web-api-mcp

EXPOSE 8080

ENV MCP_TRANSPORT=streamable-http
ENV MCP_HOST=0.0.0.0
ENV MCP_PORT=8080

ENTRYPOINT ["/perplexity-web-api-mcp"]
