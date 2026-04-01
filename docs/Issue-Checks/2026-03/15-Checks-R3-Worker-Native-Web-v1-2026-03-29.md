# Checks: R3 Worker-Native Web (v1)

- Date: 2026-03-29
- Status: DONE

## A. Boundary

| Case ID | Goal | Pass Standard |
|---|---|---|
| C01 | Migration boundary is explicit | docs clearly separate tool migration from orchestration retention |
| C02 | Browser remains deferred | no accidental claim that Worker now owns full browser session |

## B. `web_fetch`

| Case ID | Goal | Pass Standard |
|---|---|---|
| C03 | Fetch path works | Worker executes one fetch request and returns structured output |
| C04 | Extraction metadata is preserved | output includes extractor/evidence metadata and truncation state |
| C05 | Safety controls work | timeout/size/allowlist violations are blocked or failed cleanly |

## C. `web_search`

| Case ID | Goal | Pass Standard |
|---|---|---|
| C06 | Search path works | Worker executes one search request and returns normalized rows |
| C07 | Provider metadata works | output includes provider_used and fallback_used semantics |
| C08 | Provider failure degrades cleanly | missing key / auth / rate-limit become structured failures |

## D. Bridge Routing

| Case ID | Goal | Pass Standard |
|---|---|---|
| C09 | `web_fetch` routes to Worker | tool-hint routing selects Worker after native support lands |
| C10 | `web_search` routes to Worker | tool-hint routing selects Worker after native support lands |
| C11 | `browser` stays on existing lane | current browser-class tasks do not regress |

## E. Backfill

| Case ID | Result | Evidence |
|---|---|---|
| C01 | PASS | Boundary kept in tool/runtime layer only: native `web_fetch` / `web_search` landed in `src/tools/builtin/web.rs`; no `chimera-core` orchestration logic was migrated into `ironelf`. |
| C02 | PASS | `browser` / `mock_browser` still route to `JobMode::ClaudeCode` in `src/runtime_bridge.rs`; Worker only received `web_fetch` / `web_search`. |
| C03 | PASS | `WebFetchTool` added in `src/tools/builtin/web.rs` and registered into Worker via `src/tools/registry.rs`; verified by `cargo test --lib web_fetch_returns_ -- --nocapture`. |
| C04 | PASS | `web_fetch` output now includes structured `extractor`, `extractMode` / `extract_mode`, `truncated`, `maxChars` / `max_chars`, `status_code`, `content_type`, and nested `evidence`. Covered by `web_fetch_returns_structured_markdown_with_evidence`. |
| C05 | PASS | Fetch path reuses HTTPS validation, DNS resolution, pinned client, redirect cap, body-size cap, timeout, and policy-block mapping; verified by `web_fetch_returns_blocked_json_for_disallowed_target` and `cargo clippy --tests -- -D warnings`. |
| C06 | PASS | `WebSearchTool` added in `src/tools/builtin/web.rs` with normalized `results[{title,url,description}]`; verified by `cargo test --lib web_search_ -- --nocapture`. |
| C07 | PASS | Search output includes `provider_requested`, `provider_used`, `fallback_used`, and structured `evidence.attempts`; covered by `web_search_uses_fallback_provider_and_reports_metadata`. |
| C08 | PASS | Missing provider key returns structured blocked result with `error.kind = auth_missing`; provider/tool failures are normalized into structured error payloads. Covered by `web_search_returns_blocked_when_no_provider_key_exists`. |
| C09 | PASS | `select_job_mode()` now routes `web_fetch` to `JobMode::Worker`; verified by `cargo test --lib select_job_mode_routes_native_web_hints_to_worker_and_browser_to_claude -- --nocapture`. |
| C10 | PASS | `select_job_mode()` now routes `web_search` to `JobMode::Worker`; verified by `cargo test --lib select_job_mode_routes_native_web_hints_to_worker_and_browser_to_claude -- --nocapture`. |
| C11 | PASS | Routing regression test asserts `browser` and mixed `mock_browser + web_fetch` still resolve to `JobMode::ClaudeCode`, preserving browser deferral. |

## F. Verification Notes

- Validate `web_fetch` before touching routing.
- Validate `web_search` provider semantics before enabling fallback in bridge traffic.
- Do not mark the package complete if Worker still routes these tool hints but lacks the actual tool implementation.

## G. Commands Run

```bash
cargo fmt --all
cargo check --tests
cargo test --lib web_fetch_returns_ -- --nocapture
cargo test --lib web_search_ -- --nocapture
cargo test --lib select_job_mode_routes_native_web_hints_to_worker_and_browser_to_claude -- --nocapture
cargo clippy --tests -- -D warnings
```

## H. Known Limits

1. `browser` / `mock_browser` remain deferred and still use the existing lane.
2. `web_search` currently supports `tavily` and `brave` providers via explicit env/config surface; broader provider matrix is still future work.
3. URL allowlisting by business policy is still not implemented; current protection is transport/runtime safety oriented (HTTPS-only, SSRF/private-IP blocking, redirect cap, timeout, body-size cap).
