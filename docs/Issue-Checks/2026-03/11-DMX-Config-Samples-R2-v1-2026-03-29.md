# DMX Config Samples: R2 (v1)

- Date: 2026-03-29
- Status: READY

## Goal

Provide operator-ready `dmx` config samples for `ironelf` without requiring a dedicated backend type.

## Canonical Rule

1. Use `LLM_BACKEND=openai_compatible`.
2. `LLM_BASE_URL` must be a full OpenAI-compatible base URL that already includes `/v1`.
3. Keep `LLM_MODEL` pass-through; do not remap model IDs in phase 1 unless testing proves it is required.

## Sample A: Official CN

```env
LLM_BACKEND=openai_compatible
LLM_BASE_URL=https://www.dmxapi.cn/v1
LLM_API_KEY=...
LLM_MODEL=...
```

## Sample B: International

```env
LLM_BACKEND=openai_compatible
LLM_BASE_URL=https://www.dmxapi.com/v1
LLM_API_KEY=...
LLM_MODEL=...
```

## Sample C: Enterprise

```env
LLM_BACKEND=openai_compatible
LLM_BASE_URL=https://ssvip.dmxapi.com/v1
LLM_API_KEY=...
LLM_MODEL=...
```

## Optional P1: Built-In Alias/Profile

Only do this if operator experience or live compatibility proves the generic path is not enough.

Possible shape:

```env
LLM_BACKEND=dmx
DMX_API_KEY=...
DMX_MODEL=...
```

Notes:

1. If implemented, the built-in profile should still use the same OpenAI-compatible protocol underneath.
2. Default base URL should be explicit and documented.
3. This is an ergonomics improvement, not a protocol redesign.

## Smoke Expectations

1. `${LLM_BASE_URL}/models` may succeed, fail, or return a filtered set; runtime execution must still be able to proceed with an explicit `LLM_MODEL`.
2. Completion and streaming behavior must be validated against at least one configured endpoint.
3. `chimera-core` should not need any `dmx`-specific logic for `S9`.
