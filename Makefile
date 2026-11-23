.PHONY: build web dev test smoke clean

# Build the web UI, then the single release binary that embeds it.
build: web
	cargo build --release
	@ls -lh target/release/tuffie

web:
	cd web && npm ci && npm run build

# Backend on :7777 and Vite dev server on :5173 (proxies /api to the backend).
dev:
	cargo run -- serve --no-open & cd web && npm run dev

test:
	cargo test
	cd web && npm run check

# End-to-end check against a mock terraform binary.
smoke:
	cargo build
	./scripts/smoke.sh

clean:
	cargo clean
	rm -rf web/build web/.svelte-kit
