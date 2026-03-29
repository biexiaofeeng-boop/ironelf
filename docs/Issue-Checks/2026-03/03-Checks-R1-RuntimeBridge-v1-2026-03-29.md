# Checks: R1 Runtime Bridge (v1)

- Date: 2026-03-29
- Status: READY

## A. Stability

| Case ID | Goal | Pass Standard |
|---|---|---|
| C01 | Core survives bridge disabled | `chimera-core` keeps existing dialogue and execution path |
| C02 | Runtime accepts routed request | `ironelf` returns accepted state with identifiers |
| C03 | Event stream works | progress events are readable and ordered |
| C04 | Fail-open works | bridge failure does not block `chimera-core` fast lane |

## B. Runtime Control

| Case ID | Goal | Pass Standard |
|---|---|---|
| C05 | Timeout works | timed-out job ends in structured terminal state |
| C06 | Cancel works | cancel request stops running job and returns final receipt |
| C07 | Preflight works | missing capability is rejected before execution starts |
| C08 | Retry visible | transient retry count is emitted in events |

## C. Traceability

| Case ID | Goal | Pass Standard |
|---|---|---|
| C09 | IDs linked | request, events, and receipt share `trace_id/task_id/execution_id` |
| C10 | Evidence present | final receipt contains structured evidence digest |
| C11 | Logs replayable | execution record can be reconstructed from runtime events |
| C12 | UX ownership preserved | final narration still belongs to `chimera-core` |

## D. Backfill

| Case ID | Result | Evidence |
|---|---|---|
| C01 | CODED / STATIC-CHECKED | Bridge routes are isolated under `/api/runtime/*`; existing gateway chat/jobs paths remain unchanged. `git diff --check` passed locally. |
| C02 | CODED / TESTED | `src/runtime_bridge.rs` implements `submit()` and `src/channels/web/handlers/bridge.rs` exposes `POST /api/runtime/submit` with mock-compatible JSON envelopes; verified by `cargo test runtime_bridge --lib` via `submit_blocks_fast_lane_requests_with_mock_compatible_shape`. |
| C03 | CODED / TESTED | `GET /api/runtime/executions/{execution_id}/events` returns ordered `ExecutionEvent[]`, `next_cursor`, `done`, and optional `receipt`; verified by `poll_events_merges_synthetic_and_runtime_events_with_cursor`. |
| C04 | CODED / STATIC-CHECKED | Admission rejects or degrades in structured JSON (`blocked`, `degraded`, `not_implemented`) instead of hanging; fast-lane work is blocked from runtime via preflight rather than altering core dialogue paths. |
| C05 | CODED / TESTED | Timeout watcher and `timeout_execution()` mark terminal `timed_out` state and emit structured timeout events/receipt; verified by `timeout_transitions_running_execution_to_timed_out`. |
| C06 | CODED / STATIC-CHECKED | Cancel path calls `stop_job()`, persists interrupted status, and returns structured cancel receipt via `cancel()` + `POST /api/runtime/executions/{execution_id}/cancel`; compile + clippy passed, but no direct cancel integration test yet. |
| C07 | CODED / TESTED | Preflight enforces lane/risk/capability/runtime-readiness checks in `preflight_decision()` and returns structured blocked/degraded/not_implemented responses; fast-lane rejection verified by `submit_blocks_fast_lane_requests_with_mock_compatible_shape`. |
| C08 | CODED / STATIC-CHECKED | Launch path retries container creation once (`CREATE_JOB_MAX_ATTEMPTS=2`) and emits visible `retry` events before final failure; compile + clippy passed, but no retry-path test yet. |
| C09 | CODED / TESTED | `trace_id`, `task_id`, `execution_id` are carried from request into every event and receipt by `ExecutionRecord` + `map_job_event()` / `build_receipt()`; verified by `poll_events_merges_synthetic_and_runtime_events_with_cursor`. |
| C10 | CODED / TESTED | `ExecutionReceipt.evidence_digest` is synthesized from merged runtime events with `count` and `kinds`; verified by `poll_events_merges_synthetic_and_runtime_events_with_cursor`. |
| C11 | CODED / TESTED | Event replay reconstructs execution state from synthetic bridge events plus DB-backed `job_events`; verified by `poll_events_merges_synthetic_and_runtime_events_with_cursor`. |
| C12 | CODED / STATIC-CHECKED | Final `next_action` remains `return_to_chimera_core`; runtime bridge returns only structured machine-facing JSON and does not narrate user-facing prose. |

## E. Verification Notes

- Static check completed: `git diff --check`
- Compile check completed: `cargo check --tests`
- Lint check completed: `cargo clippy --tests -- -D warnings`
- Focused runtime bridge tests completed: `cargo test runtime_bridge --lib`
