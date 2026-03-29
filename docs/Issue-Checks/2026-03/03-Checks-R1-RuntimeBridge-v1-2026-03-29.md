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
| C01 | TODO | |
| C02 | TODO | |
| C03 | TODO | |
| C04 | TODO | |
| C05 | TODO | |
| C06 | TODO | |
| C07 | TODO | |
| C08 | TODO | |
| C09 | TODO | |
| C10 | TODO | |
| C11 | TODO | |
| C12 | TODO | |
