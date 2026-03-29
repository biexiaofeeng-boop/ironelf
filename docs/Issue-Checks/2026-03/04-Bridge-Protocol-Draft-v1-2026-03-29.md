# Bridge Protocol Draft (v1)

- Date: 2026-03-29
- Status: READY

## 1. `ExecutionRequest`

```json
{
  "schema_version": "v1",
  "trace_id": "tg-20260329-001",
  "task_id": "task-abc123",
  "execution_id": "exec-abc123",
  "lane": "runtime",
  "risk_level": "high",
  "objective": "Run isolated web intelligence collection with evidence",
  "tool_hints": ["browser", "web_search"],
  "timeout_s": 600,
  "requires_confirmation": false,
  "context_refs": [
    {
      "type": "taskops",
      "id": "task-abc123"
    }
  ],
  "payload": {
    "instruction": "Collect and summarize target page evidence"
  }
}
```

## 2. `ExecutionEvent`

```json
{
  "schema_version": "v1",
  "trace_id": "tg-20260329-001",
  "task_id": "task-abc123",
  "execution_id": "exec-abc123",
  "event_type": "tool_started",
  "status": "running",
  "timestamp": "2026-03-29T12:00:00Z",
  "payload": {
    "tool": "browser",
    "message": "browser session created"
  },
  "evidence_refs": []
}
```

## 3. `ExecutionReceipt`

```json
{
  "schema_version": "v1",
  "trace_id": "tg-20260329-001",
  "task_id": "task-abc123",
  "execution_id": "exec-abc123",
  "terminal_state": "DONE",
  "execution_state": "executed",
  "summary": "Collected target evidence and produced structured notes.",
  "evidence_digest": {
    "count": 3,
    "kinds": ["snapshot", "response", "log"]
  },
  "next_action": "return_to_chimera_core"
}
```

## 4. Routing Rule

1. `lane=fast` stays in `chimera-core`.
2. `lane=runtime` is sent to `ironelf`.
3. Phase 1 only allows `runtime` for high-risk, long-running, concurrent, or isolated jobs.

## 5. Compatibility Rule

1. Additive fields only within `v1`.
2. Unknown fields must be ignored by the receiver.
3. Bridge rejection must always return structured error payload, never plain text only.
