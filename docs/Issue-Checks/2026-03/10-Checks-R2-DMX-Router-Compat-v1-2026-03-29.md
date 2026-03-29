# Checks: R2 DMX Router Compatibility (v1)

- Date: 2026-03-29
- Status: BACKFILLED

## A. Config Validity

| Case ID | Goal | Pass Standard |
|---|---|---|
| C01 | Canonical env works | `LLM_BACKEND=openai_compatible` + full `/v1` base URL boots `ironelf` |
| C02 | Endpoint matrix is explicit | supported `dmx` base URL variants are documented with exact samples |
| C03 | Choice is justified | docs-only vs alias/profile vs compatibility patch decision is recorded |

## B. Router Compatibility

| Case ID | Goal | Pass Standard |
|---|---|---|
| C04 | `/models` success path works | model listing returns usable IDs when endpoint supports it |
| C05 | `/models` failure degrades safely | empty or failed model listing does not block runtime execution |
| C06 | Completion path works | one `dmx`-backed completion request succeeds through current provider stack |
| C07 | Streaming behavior is known | streaming works or the exact gap is documented and bounded |
| C08 | Error mapping is structured | auth, rate-limit, and server errors are surfaced cleanly |

## C. Bridge Acceptance

| Case ID | Goal | Pass Standard |
|---|---|---|
| C09 | S9 bridge remains usable | `chimera-core` can route a job through `ironelf(dmx)` or degrade cleanly |
| C10 | Fail-open still holds | `chimera-core` remains operational if `dmx` or `ironelf` is unavailable |
| C11 | Traceability preserved | request, runtime events, and receipt still link by IDs |

## D. Backfill

| Case ID | Result | Evidence |
|---|---|---|
| C01 | PASS | Live operator run on 2026-03-29 booted `ironclaw v0.22.0` with `llm_backend=openai_compatible`, main model `gemini-3.1-pro-preview`, cheap model `kimi-k2.5`, and gateway `http://127.0.0.1:3000/?token=dev-token`. |
| C02 | PASS | DMX endpoint matrix and `/v1` rule are documented in `11-DMX-Config-Samples-R2-v1-2026-03-29.md`, plus operator-facing docs were added to `docs/LLM_PROVIDERS.md` and `README.md`. |
| C03 | PASS | Phase 1 stays on `openai_compatible`; no built-in `dmx` backend was added. Decision is justified by live DMX success and documented in the R2 package/docs. |
| C04 | PASS | Live `curl -H 'Authorization: Bearer dev-token' http://127.0.0.1:3000/v1/models` returned `gemini-3.1-pro-preview` on 2026-03-29. |
| C05 | PASS | Mock-backed isolated instances showed empty `/v1/models` and failed `/v1/models` both degraded safely to the active model list, while the live DMX-backed instance still executed completions successfully with an explicit `LLM_MODEL`. |
| C06 | PASS | Live `POST /v1/chat/completions` through the DMX-backed instance returned `{\"ok\":true,\"provider\":\"dmx\"}` with HTTP 200 on 2026-03-29. |
| C07 | PASS | Live `POST /v1/chat/completions` with `stream=true` returned standard SSE `data:` chunks and `[DONE]`. Bound: gateway streaming is simulated chunking (`x-ironclaw-streaming: simulated`), not a guaranteed byte-for-byte upstream stream passthrough. |
| C08 | PARTIAL | Using a corrected HTTP/1.1 mock upstream, upstream `401`, `429`, and `500` all surfaced as structured JSON and never hung, but the gateway normalized them to `500/internal_error` while embedding the original upstream status/body inside `error.message`. Fine-grained status-code passthrough still needs follow-up if callers depend on exact HTTP semantics. |
| C09 | PARTIAL | The `ironelf` side of the S9 surface was live-verified: `GET /api/runtime/health`, `POST /api/runtime/submit`, `GET /api/runtime/executions/{id}/events`, and `POST /api/runtime/executions/{id}/cancel` all worked against the DMX-backed runtime. Full `chimera-core -> S9 -> ironelf(dmx)` cross-repo smoke was not run in this repository. |
| C10 | PARTIAL | Runtime bridge fail-open behavior is implemented and documented (`blocked` / `degraded` / `not_implemented` states), but a full cross-repo outage drill with `chimera-core` was not executed in this pass. |
| C11 | PASS | Live runtime bridge evidence preserved `trace_id`, `task_id`, and `execution_id` across submit, polled events, cancel, timeout, and terminal receipts on 2026-03-29. |

## E. Verification Notes

- Prefer minimal-change verification first: env sample, provider resolution, model fetch behavior, one request smoke.
- If real `dmx` credentials are not available in dev, use mock-backed checks plus one operator runbook for later live verification.
- Any compatibility patch must be backed by a concrete failing case, not guesswork.
- Live DMX-backed checks executed on 2026-03-29 against a running local instance on `127.0.0.1:3000` with gateway token `dev-token`.
- Live bridge checks exercised:
  `GET /api/runtime/health`,
  `POST /api/runtime/submit`,
  `GET /api/runtime/executions/exec-r2-smoke-1774770154/events?cursor=0`,
  `GET /api/runtime/executions/exec-r2-smoke-1774770154/events?cursor=4`,
  `POST /api/runtime/executions/exec-r2-cancel-1774770240/cancel`,
  and `GET /api/runtime/executions/exec-r2-cancel-1774770240/events?cursor=0`.
- Live bridge outcomes observed:
  one execution reached `TIMED_OUT` with a structured receipt after emitting `tool_started` and `tool_result`,
  and a second execution reached `CANCELLED` with a structured cancel event and receipt.
- Isolated mock-backed instances used separate `IRONCLAW_BASE_DIR`, libsql paths, gateway ports, and orchestrator ports to avoid interfering with the user's main process.
- Corrected mock-backed error checks executed on 2026-03-29 through `127.0.0.1:3013` against a local upstream on `127.0.0.1:18083`; observed mapping was:
  upstream `401 -> gateway 500/internal_error`,
  upstream `429 -> gateway 500/internal_error`,
  upstream `500 -> gateway 500/internal_error`,
  with the original upstream status/body preserved in `error.message`.
