# Task Package: R2 DMX Router Compatibility (v1)

- Date: 2026-03-29
- Status: READY
- Suggested branch: `codex/r2-dmx-router-compat-v1`
- Goal: make `ironelf` run reliably against `dmx` as the LLM router backend so the `chimera-core -> S9 bridge -> ironelf` chain is usable before `Cherry + ironelf` integration.

## 0. Conclusion First

1. Phase 1 should treat `dmx` as a verified `openai_compatible` router, not a brand new LLM backend.
2. The canonical config contract is `LLM_BACKEND=openai_compatible` plus a full `/v1` base URL.
3. `ironelf` already has the main primitives we need: generic OpenAI-compatible config, setup wizard support, model fetch, and unknown-backend fallback.
4. The real work is compatibility hardening: endpoint samples, smoke validation, error behavior, and bridge acceptance under `S9`.
5. Only add a built-in `dmx`/`dmxapi` provider profile if real testing shows the generic path is too brittle or too awkward.

## 1. Why This Package Exists

1. The current `S9` bridge can only be considered useful if `ironelf` can boot and serve requests with the same router stack we actually use.
2. Right now the likely migration is `vllm/local proxy -> dmx router`, so we need a clean replacement path instead of ad hoc env swapping.
3. `Cherry` already treats `dmxapi` as a first-class provider, which gives us a strong reference for endpoint conventions, but `ironelf` should still prefer minimal-change integration.

## 2. Facts Already Confirmed In Code

1. `ironelf` supports `LLM_BACKEND=openai_compatible` with `LLM_BASE_URL`, `LLM_API_KEY`, and `LLM_MODEL`.
2. The setup wizard already supports generic OpenAI-compatible endpoints.
3. Model discovery for OpenAI-compatible providers calls `${LLM_BASE_URL}/models`, so the configured base URL must already include the `/v1` prefix.
4. Unknown provider IDs already fall back to the generic OpenAI-compatible config path.
5. This means `dmx` can likely be integrated without inventing a new backend type.

## 3. Canonical Phase-1 Contract

## 3.1 Required behavior

1. `ironelf` can start and serve requests with `dmx` through the existing OpenAI-compatible path.
2. `chimera-core` does not need to know whether the downstream router is `vllm`, `litellm`, or `dmx`.
3. Routing failure must fail cleanly and observably, not silently hang the bridge.
4. If `/models` is unavailable or filtered, startup and request execution must still degrade predictably.

## 3.2 Canonical env shape

```env
LLM_BACKEND=openai_compatible
LLM_BASE_URL=https://www.dmxapi.cn/v1
LLM_API_KEY=...
LLM_MODEL=...
```

## 4. Scope

1. Define the exact `dmx` config contract for `ironelf`.
2. Validate `dmx` against model listing, request execution, streaming, and common failure paths.
3. Improve docs and setup guidance so a Rust dev or operator can configure it once and run it.
4. Prove `S9` can run with `dmx` behind `ironelf`.
5. Decide whether a built-in provider alias/profile is necessary.

## 5. Out of Scope

1. No `Cherry + ironelf` integration in this package.
2. No unified data-plane work across `Cherry`, `chimera-core`, and `ironelf`.
3. No new Anthropic-protocol path unless `dmx` testing shows the OpenAI-compatible path is insufficient.
4. No broad provider refactor across all routers.

## 6. Decision Rule

1. If the generic `openai_compatible` path passes real checks, keep the implementation minimal and land docs/tests only.
2. If operator ergonomics are poor but runtime behavior is correct, add a light built-in provider alias/profile.
3. If `dmx` has protocol quirks that break generic behavior, fix only the narrow compatibility surface required by `S9`.

## 7. Risks and Controls

1. Base URL mismatch risk:
- Control: standardize on full `/v1` URLs in all docs, samples, and tests.

2. `/models` instability risk:
- Control: treat model discovery as optional; fallback must not block runtime execution.

3. Router-specific error ambiguity risk:
- Control: explicitly test and surface 401/429/5xx as structured runtime failures.

4. Overengineering risk:
- Control: do not add a dedicated `dmx` backend unless generic compatibility is proven insufficient.

5. Bridge regression risk:
- Control: `S9` acceptance remains mandatory; `chimera-core` must still survive if `ironelf/dmx` is unavailable.

## 8. Acceptance Gate

1. `ironelf` runs with `dmx` using the canonical `openai_compatible` config or a justified light alias.
2. A complete operator sample exists for at least one `dmx` endpoint.
3. `/models` failure or empty results do not block execution.
4. Request execution and streaming behavior are verified or explicitly bounded.
5. `chimera-core -> S9 -> ironelf(dmx)` has a reproducible smoke path.
6. Any extra compatibility code is minimal, documented, and justified.
