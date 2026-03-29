# Mock Bridge Compatibility Samples: R1 (v1)

- Date: 2026-03-29
- Status: READY

## Goal

Keep the Rust-side runtime API compatible with the Python-side mock bridge and bridge contract before real integration is complete.

## Health Response

```json
{
  "schema_version": "v1",
  "ok": true,
  "status": "ready",
  "mode": "ok"
}
```

## Accepted Submit Response

```json
{
  "schema_version": "v1",
  "accepted": true,
  "execution_id": "exec-abc123",
  "status": "running"
}
```

## Blocked Submit Response

```json
{
  "schema_version": "v1",
  "accepted": true,
  "execution_id": "exec-abc123",
  "status": "blocked"
}
```

## Error Submit Response

```json
{
  "schema_version": "v1",
  "error": {
    "code": "runtime_submit_failed",
    "message": "Mock bridge submit failure"
  }
}
```

## Event Poll Response

```json
{
  "schema_version": "v1",
  "execution_id": "exec-abc123",
  "events": [
    {
      "schema_version": "v1",
      "trace_id": "tg-20260329-001",
      "task_id": "task-abc123",
      "execution_id": "exec-abc123",
      "event_type": "tool_started",
      "status": "running",
      "timestamp": "2026-03-29T12:00:01Z",
      "payload": {
        "tool": "mock_browser",
        "message": "mock tool running"
      },
      "evidence_refs": []
    }
  ],
  "next_cursor": 2,
  "done": false,
  "receipt": null
}
```

## Receipt Response

```json
{
  "schema_version": "v1",
  "trace_id": "tg-20260329-001",
  "task_id": "task-abc123",
  "execution_id": "exec-abc123",
  "terminal_state": "DONE",
  "execution_state": "executed",
  "summary": "Mock runtime execution completed.",
  "evidence_digest": {
    "count": 2,
    "kinds": ["response", "log"]
  },
  "next_action": "return_to_chimera_core"
}
```

## Compatibility Rule

1. Match field names and top-level structure first.
2. Unknown fields must not break parsing.
3. Error cases must stay structured.
4. `receipt_missing` and `event_drop` remain valid degraded states that `chimera-core` must survive.
