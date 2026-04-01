# Task Package: R3 Worker-Native Web (v1)

- Date: 2026-03-29
- Status: READY
- Suggested branch: `codex/r3-worker-native-web-v1`
- Goal: make `ironelf` Worker natively support `web_fetch` and `web_search` so runtime bridge requests do not need to detour into `ClaudeCode` for basic web access.

## 0. Conclusion First

1. This package is the natural follow-up to R1 bridge routing and R2 DMX compatibility.
2. The first native web target is `web_fetch`, then `web_search`.
3. `browser` and `vision/rpa` are explicitly deferred.
4. `chimera-core` remains the orchestration and route-policy owner.
5. `ironelf` only needs to own real tool execution, structured evidence, and runtime safety.

## 1. Why This Package Exists

1. Current bridge routing sends `browser / mock_browser / web_search / web_fetch` to `JobMode::ClaudeCode`.
2. This is acceptable for R1 minimum viability, but not ideal for long-term runtime determinism.
3. Worker already owns shell/file/workspace style execution; adding controlled web tools is the next clean step.
4. `chimera-core` already proves these capabilities are useful, but its implementation is split across tool layer and orchestration layer.

## 2. Boundary Decision

## 2.1 Keep in `chimera-core`

1. Route policy such as `http -> managed -> browser -> vision`.
2. Site policy override logic and domain-specific escalation.
3. User-facing narration, fallback explanation, and task-board semantics.
4. Decision of whether a task should escalate from fetch/search into browser/vision.

## 2.2 Move into `ironelf` Worker

1. Native `web_fetch` tool.
2. Native `web_search` tool.
3. Structured timeout/error/evidence mapping for these two tools.
4. Safe config injection, provider selection, and admission checks.

## 3. Reference Sources

1. Native web tools reference:
   `/Users/sourcefire/X-lab/chimera-core/nanobot/agent/tools/web.py`
2. Route-chain reference only, not for direct migration:
   `/Users/sourcefire/X-lab/chimera-core/nanobot/intel/web_intel_router.py`
3. Browser adapter boundary reference:
   `/Users/sourcefire/X-lab/chimera-core/nanobot/intel/adapters/browser_session.py`
4. Vision/RPA boundary reference:
   `/Users/sourcefire/X-lab/chimera-core/nanobot/intel/adapters/vision_rpa.py`
5. RPA executor reference for future step-based execution shape:
   `/Users/sourcefire/X-lab/chimera-core/nanobot/executors/rpa_adapter.py`

## 4. Phase Scope

1. Implement Worker-native `web_fetch`.
2. Implement Worker-native `web_search` with provider abstraction.
3. Add tool registration, config loading, and runtime admission checks.
4. Emit structured evidence/events compatible with current bridge expectations.
5. Update bridge mode selection so `web_fetch` and `web_search` no longer require `ClaudeCode`.

## 5. Out of Scope

1. No full browser session runtime in Worker.
2. No visual/RPA automation in Worker.
3. No rewrite of `chimera-core` route chain.
4. No attempt to migrate `managed extract` or all existing web-intel semantics.

## 6. Implementation Principle

1. Port capability, not architecture.
2. Reuse `chimera-core` behavior where it is truly tool-level.
3. Do not import `chimera-core` orchestration assumptions into Rust runtime.
4. Prefer explicit provider config over hidden heuristics.
5. Evidence must be machine-readable first, human-readable second.

## 7. Recommended Capability Order

## 7.1 `web_fetch` first

1. Lowest complexity.
2. Clear input/output contract.
3. Easy to secure with URL allowlisting, timeouts, and size caps.

## 7.2 `web_search` second

1. Needs provider abstraction.
2. Needs API key and fallback governance.
3. Needs explicit provider_used/fallback_used evidence.

## 7.3 `browser` later

1. Requires session model, page state, richer evidence, and likely separate runtime lane.
2. Must not be smuggled into this package as “just another tool”.

## 8. Risks and Controls

1. Scope creep risk:
- Control: stop at `web_fetch` + `web_search`.

2. Security risk:
- Control: strict outbound rules, timeouts, response size caps, and explicit provider env mapping.

3. Evidence mismatch risk:
- Control: standardize `evidence_refs` and final summary shape before landing.

4. Routing regression risk:
- Control: update tool-hint routing only after tools are actually available in Worker.

5. Provider drift risk:
- Control: one explicit config surface for search provider, fallback provider, and keys.

## 9. Acceptance Gate

1. Worker can execute `web_fetch` without `ClaudeCode`.
2. Worker can execute `web_search` without `ClaudeCode`.
3. Errors become structured runtime failures, not hangs.
4. `tool_hints` routing sends `web_fetch` and `web_search` to Worker only after capability is live.
5. `browser` remains on `ClaudeCode` or existing lane with no accidental regression.
