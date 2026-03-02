VERSION := 0.4.1

.PHONY: publish test-unit test-integration test

test-unit:
	@cargo test --workspace --lib

test-e2e:
	@cargo test -p perplexity-web-api --test integration -- --ignored --test-threads=1

test: test-unit test-integration

publish:
	@perl -i \
		-pe 's/version = "\d+\.\d+\.\d+"/version = "${VERSION}"/g' \
		crates/perplexity-web-api-mcp/Cargo.toml
	@cargo update -p perplexity-web-api-mcp
	@git add \
		Makefile \
		Cargo.lock \
		crates/perplexity-web-api-mcp/Cargo.toml
	@git commit -m "chore: release ${VERSION} 🔥"
	@git tag "v${VERSION}"
	@git-cliff -o CHANGELOG.md
	@git tag -d "v${VERSION}"
	@git add CHANGELOG.md
	@git commit --amend --no-edit
	@git tag -a "v${VERSION}" -m "release v${VERSION}"
	@git push
	@git push --tags
