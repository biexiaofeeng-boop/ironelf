# 联合回测用例说明：S9 Runtime Bridge -> Ironelf

- 日期：2026-03-29
- 目标：供 `ironelf` 开发 agent 修复后复现与验收
- 范围：runtime bridge 真机联调
- 说明：本版本已按 R3 真实分流结果更新，`web_search` 与 `web_fetch` 走 `Worker`，`browser` 走 `ClaudeCode`

## 1. 测试目标

需要覆盖三条路径：

1. `web_fetch` -> `Worker`
2. `web_search` -> `Worker`
3. `browser` -> `ClaudeCode`

同时需要覆盖两层验收：

1. `ironelf` 直连 API 回测
2. `chimera-core -> ironelf` 联合回测

## 2. 前置条件

### 2.1 服务启动

确认本机 `ironelf` 正常运行，例如：

```bash
lsof -nP -iTCP:3000 -sTCP:LISTEN
```

### 2.2 gateway token

本轮联调使用：

```bash
GATEWAY_AUTH_TOKEN=dev-token
```

### 2.3 基础健康检查

```bash
curl -sS -H "Authorization: Bearer dev-token" \
  http://127.0.0.1:3000/api/runtime/health
```

通过标准：

- `ok=true`
- `status=ready`
- `capabilities` 包含 `health submit events cancel`

## 3. 用例 A：web_fetch Worker 路径复测

### 3.1 目的

验证 `web_fetch` 是否正确进入 `Worker`，且不再失败于 502 代理链路。

### 3.2 请求

```bash
cat >/tmp/ironelf-runtime-webfetch.json <<JSON
{
  "schema_version": "v1",
  "trace_id": "trace-r3-webfetch-20260329",
  "task_id": "task-r3-webfetch-20260329",
  "execution_id": "exec-r3-webfetch-20260329",
  "lane": "runtime",
  "risk_level": "medium",
  "objective": "Fetch a target webpage and summarize key evidence.",
  "tool_hints": ["web_fetch"],
  "timeout_s": 60,
  "requires_confirmation": false,
  "context_refs": [
    {"type": "taskops", "id": "r3-webfetch"}
  ],
  "payload": {
    "instruction": "Fetch the target webpage and summarize the key evidence."
  }
}
JSON

curl -sS -X POST \
  -H "Authorization: Bearer dev-token" \
  -H "Content-Type: application/json" \
  --data @/tmp/ironelf-runtime-webfetch.json \
  http://127.0.0.1:3000/api/runtime/submit
```

修复前现象：`job_mode=worker`，但最终摘要为 `Provider proxy request failed ... 502 Bad Gateway`。

修复后通过标准：`job_mode=worker`、`done=true`、receipt 存在、非 502 代理错误。

## 4. 用例 B：web_search Worker 路径复测

### 4.1 目的

验证 `web_search` 是否正确进入 `Worker`，且不再失败于 502 代理链路。

### 4.2 请求

```bash
cat >/tmp/ironelf-runtime-websearch.json <<JSON
{
  "schema_version": "v1",
  "trace_id": "trace-r3-websearch-20260329",
  "task_id": "task-r3-websearch-20260329",
  "execution_id": "exec-r3-websearch-20260329",
  "lane": "runtime",
  "risk_level": "medium",
  "objective": "Search the web for authoritative evidence and summarize it.",
  "tool_hints": ["web_search"],
  "timeout_s": 60,
  "requires_confirmation": false,
  "context_refs": [
    {"type": "taskops", "id": "r3-websearch"}
  ],
  "payload": {
    "instruction": "Search the web for authoritative evidence and summarize it."
  }
}
JSON

curl -sS -X POST \
  -H "Authorization: Bearer dev-token" \
  -H "Content-Type: application/json" \
  --data @/tmp/ironelf-runtime-websearch.json \
  http://127.0.0.1:3000/api/runtime/submit
```

修复前现象：`job_mode=worker`，但最终摘要为 `Provider proxy request failed ... 502 Bad Gateway`。

修复后通过标准：`job_mode=worker`、`done=true`、receipt 存在、非 502 代理错误。

## 5. 用例 C：显式 browser 路径确认

确认修复过程中没有把 `browser` 路径和 `web_search` 或 `web_fetch` 混回去。

```bash
cat >/tmp/ironelf-runtime-browser.json <<JSON
{
  "schema_version": "v1",
  "trace_id": "trace-browser-check-20260329",
  "task_id": "task-browser-check-20260329",
  "execution_id": "exec-browser-check-20260329",
  "lane": "runtime",
  "risk_level": "high",
  "objective": "Open the target page and capture a screenshot.",
  "tool_hints": ["browser"],
  "timeout_s": 60,
  "requires_confirmation": false,
  "context_refs": [
    {"type": "taskops", "id": "browser-check"}
  ],
  "payload": {
    "instruction": "Open the target page and capture a screenshot."
  }
}
JSON
```

通过标准：`job_mode=claude_code` 或同义显式浏览器执行态，且不影响 Worker 分流。

## 6. 用例 D：真实 chimera-core 联合回测

在 `chimera-core` S9 worktree 中执行 search 样例：

```bash
cd /tmp/chimera-core-s9-runtime-bridge
python3.11 deploy/it/runtime_bridge_compare.py \
  --base-url http://127.0.0.1:3000 \
  --auth-token dev-token \
  --runtime-timeout-s 25 \
  --task "请执行发布检查，并检索 Rust 官方网站收集证据"
```

再执行 fetch 样例：

```bash
cd /tmp/chimera-core-s9-runtime-bridge
python3.11 deploy/it/runtime_bridge_compare.py \
  --base-url http://127.0.0.1:3000 \
  --auth-token dev-token \
  --runtime-timeout-s 25 \
  --task "请执行发布检查，并 fetch https://www.rust-lang.org/ 并总结页面证据"
```

联合通过标准：

- `after.lane = runtime`
- `after.provider_calls = 0`
- `after.execution_id` 非空
- search 样例映射到 `web_search`
- fetch 样例映射到 `web_fetch`
- 非 `browser` 污染
- 非 `runtime_task` 拒绝
- 若 `ironelf` 已修复，则最终 receipt 应闭环成功

## 7. 回测记录模板

### A. web_fetch / Worker

- 请求：PASS / FAIL
- submit：PASS / FAIL
- `job_mode=worker`：PASS / FAIL
- receipt：PASS / FAIL
- 终态：
- 摘要：

### B. web_search / Worker

- 请求：PASS / FAIL
- submit：PASS / FAIL
- `job_mode=worker`：PASS / FAIL
- receipt：PASS / FAIL
- 终态：
- 摘要：

### C. browser / ClaudeCode

- 请求：PASS / FAIL
- `job_mode=claude_code`：PASS / FAIL
- 终态：
- 摘要：

### D. chimera-core 联合回测

- search 样例：PASS / FAIL
- fetch 样例：PASS / FAIL
- hint 映射：
- lane：
- execution_id：
- 最终摘要：

## 8. 2026-03-31 最新回测结果回填

### 8.1 预检查

- `/api/runtime/health`：PASS
- `/api/gateway/status`：PASS
- `/v1/models`：PASS
- 关键事实：
  - `ok=true`
  - `capabilities=[health, submit, events, cancel]`
  - `supported_tool_hints` 包含 `web_fetch`、`web_search`、`browser`、`shell`、`workspace`
  - model list 包含 `qwen3.5-plus-2026-02-15`

### 8.2 A. web_fetch / Worker

- 请求：PASS
- submit：PASS
- `job_mode=worker`：PASS
- receipt：PASS
- `execution_id=exec-regress-webfetch-002`
- 终态：`terminal_state=DONE` / `execution_state=executed`
- 摘要：成功返回结构化页面证据

### 8.3 B. web_search / Worker

- 请求：PASS
- submit：PASS
- `job_mode=worker`：PASS
- receipt：PASS
- `execution_id=exec-regress-websearch-002`
- 终态：`terminal_state=DONE` / `execution_state=executed`
- 摘要：成功返回结构化检索结论和来源

### 8.4 B+. web_search 来源列表强约束

- 请求：PASS
- submit：PASS
- `job_mode=worker`：PASS
- receipt：PASS
- `execution_id=exec-regress-websearch-003`
- 终态：`terminal_state=DONE` / `execution_state=executed`
- 摘要：成功返回显式来源列表
- 已观测来源：
  - `https://www.iana.org/domains/reserved`
  - `https://www.rfc-editor.org/rfc/rfc2606.html`
  - `https://michael.kjorling.se/internet-reserved-names-and-networks/`
  - `https://www.ietf.org/archive/id/draft-jabley-reserved-domain-names-00.html`

### 8.5 C. browser / ClaudeCode

- 请求：PASS
- `job_mode=claude_code`：PASS
- `execution_id=exec-regress-browser-002`
- 终态：`terminal_state=FAILED` / `execution_state=failed`
- 摘要：`claude binary not found in runtime worker env`
- 判定：PASS
- 说明：这是守护性回归通过，证明 `browser` 没有被错误混回 `worker`；失败为结构化失败而非挂死

### 8.6 D. cancel 回归

- 请求：PASS
- cancel 接口：PASS
- `execution_id=exec-regress-cancel-002`
- 终态：`terminal_state=CANCELLED` / `execution_state=cancelled`
- 判定：PASS

### 8.7 最新结论

1. `web_fetch` 与 `web_search` 已达到当前联合验收要求。
2. `browser` 路径保持独立，未被 Worker 分流污染。
3. 当前剩余非阻塞改进项是 `web_search` receipt 摘要的 provenance 还可以更 search-native。
