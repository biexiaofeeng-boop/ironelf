# Task Package: R1 ChimeraBridge Runtime Plane (v1)

- Date: 2026-03-29
- Status: READY
- Suggested branch: `codex/r1-runtime-bridge-v1`
- Goal: make `ironelf` the safe runtime executor for `chimera-core` without replacing `chimera-core` as the collaboration and control plane.

## 0. Conclusion First

1. This is a boundary-first bridge package, not a full migration.
2. `chimera-core` remains the collaboration plane and business-state owner.
3. `ironelf` becomes the runtime plane for high-risk, long-running, concurrent, isolated execution.
4. The first phase must remain fail-open and reversible.

## 1. Plane Split

## 1.1 `chimera-core` keeps

1. Dialogue entry, soul, prompt system, and user-facing response quality.
2. Intent split, task confirmation, task board, multi-agent collaboration logic.
3. Human-readable progress, final reports, and current task ownership model.
4. Primary task view and current operational data model.

## 1.2 `ironelf` takes

1. `ExecutionRequest` intake and runtime admission checks.
2. Job state machine, timeout, retry, cancel, and concurrency control.
3. Safety policy, plugin capability gating, and sandbox lifecycle.
4. Execution event stream, structured receipts, and runtime audit trail.

## 2. First-Phase Scope

1. Add a local executor API in `ironelf`.
2. Accept structured execution requests from `chimera-core`.
3. Return structured events and final receipts.
4. Support at least one high-value lane: long-running tool execution or isolated subagent execution.
5. Keep all low-risk fast-turn tasks inside `chimera-core`.

## 3. Out of Scope

1. No full data-plane migration from `chimera-core` to `ironelf`.
2. No replacement of `chimera-core` task board, soul, or conversation flow.
3. No deep FFI embedding between Python and Rust.
4. No forced routing of every task through `ironelf`.

## 4. Architecture Principle

1. `chimera-core` is above `ironelf`, not before it.
2. `chimera-core` decides whether to execute.
3. `ironelf` decides how execution runs safely and predictably.
4. The bridge contract is the only integration boundary.

## 5. Integration Mode

1. Local service bridge first: `HTTP + JSON` or `Unix socket + JSON`.
2. Fail-open fallback: if `ironelf` is unavailable, `chimera-core` stays on existing path.
3. Event-driven feedback: execution progress is streamed back as structured events, not free-form text.
4. Explicit lane routing: `fast lane` stays local, `runtime lane` goes to `ironelf`.

## 6. Risks and Controls

1. Boundary drift risk:
- Control: keep request/event schemas versioned and repo-visible.

2. Over-routing risk:
- Control: only high-risk and long-running execution enters runtime lane.

3. UX regression risk:
- Control: `chimera-core` remains the only user-facing narrator.

4. Coupling risk:
- Control: no direct DB coupling in phase 1, only trace-linked bridge events.

## 7. Acceptance Gate

1. `chimera-core` still works with `ironelf` disabled.
2. `ironelf` can execute a routed job and stream events back.
3. Bridge failure does not break core dialogue path.
4. Routed jobs have traceable `trace_id`, `task_id`, and `execution_id`.
5. Rust executor can be developed independently from Python UX logic.
