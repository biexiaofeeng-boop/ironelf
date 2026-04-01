# 联合定位修复报告：S9 Runtime Bridge -> Ironelf

- 日期：2026-03-29
- 来源：`chimera-core` 与 `ironelf` 联调
- 范围：`ironelf` R3 native web 能力并入后的 runtime bridge 真机联调
- 优先级：P0

## 1. 问题摘要

当前联合链路已经确认三件事：

1. `ironelf` runtime bridge API 正常。
2. R3 分流已经落地，`web_search` 与 `web_fetch` 已进入 `Worker`。
3. 当前新的主 blocker 在 `Worker` 执行平面，终态统一收口为 `Provider proxy request failed ... 502 Bad Gateway`。

因此本轮结论已经更新为：

- 旧 blocker：`web_search` 与 `web_fetch` 被错误送到 `ClaudeCode`，已不成立。
- 新 blocker：`ironelf` Worker 平面的 LLM 或 orchestrator 代理链路异常。
- 并行待收口项：`chimera-core` 侧仍需把 runtime hint 从泛 web 对齐为明确的 `web_search`、`web_fetch`、`browser`。

## 2. 已确认的事实

### 2.1 健康检查正常

本机 `ironelf` 正在 `127.0.0.1:3000` 监听，健康检查返回：

```bash
curl -H "Authorization: Bearer dev-token" \
  http://127.0.0.1:3000/api/runtime/health
```

关键结果：

- `ok=true`
- `status=ready`
- `capabilities=[health, submit, events, cancel]`

### 2.2 当前 live 分流逻辑已包含 R3 调整

`/Users/sourcefire/X-lab/ironelf/src/runtime_bridge.rs:1483` 当前逻辑为：

- `browser | mock_browser` => `JobMode::ClaudeCode`
- 其他 hint，包括 `web_search`、`web_fetch`、`shell`、`http`、`file`、`workspace` => `JobMode::Worker`

这说明 R3 的关键目标已经成立：`web_search` 与 `web_fetch` 不再绑定浏览器路径。

### 2.3 web_fetch 真实联调结果

提交参数：`tool_hints=["web_fetch"]`

执行号：`exec-r3-webfetch-20260329`

观察到：

- `submit` 成功
- events 出现 `accepted` 与 `running`
- `job_mode=worker`
- receipt 最终失败

终态摘要：

```text
Execution failed: LLM error: Provider proxy request failed: LLM tool complete: orchestrator returned 502 Bad Gateway:
```

### 2.4 web_search 真实联调结果

提交参数：`tool_hints=["web_search"]`

执行号：`exec-r3-websearch-20260329`

观察到：

- `submit` 成功
- events 出现 `accepted` 与 `running`
- `job_mode=worker`
- receipt 最终失败

终态摘要：

```text
Execution failed: LLM error: Provider proxy request failed: LLM tool complete: orchestrator returned 502 Bad Gateway:
```

### 2.5 browser 仍然是 ClaudeCode 路径

这点需要保持：

- `browser` 仍按设计进入 `ClaudeCode`
- 这不是本轮 R3 的回归问题
- `web_search` 与 `web_fetch` 和 `browser` 已经是两条不同路径

修复时不要把 `web_search` 与 `web_fetch` 再次并回 `browser`。

### 2.6 当前 joint blocker 已拆成两段

A. `ironelf` 侧 blocker

- `web_search` 与 `web_fetch` 已正确进入 `Worker`
- 但 Worker 内部请求 LLM 或 orchestrator 时得到 `502 Bad Gateway`

B. `chimera-core` 侧待对齐项

在修复前，`chimera-core` runtime hint 存在两个偏差：

1. 泛 `web` 词会误触发 `browser`
2. 某些检索或抓取语句会落成 `runtime_task`，而不是 `web_search` 或 `web_fetch`

## 3. 根因分析

### 3.1 当前 P0 根因：Worker 执行平面的代理链路异常

证据链：

1. health、submit、events 正常。
2. `web_search` 与 `web_fetch` 均进入 `job_mode=worker`。
3. 终态不是超时无回执，而是明确失败。
4. 失败摘要一致收敛到 `Provider proxy request failed` 与 `orchestrator returned 502 Bad Gateway`。

优先判断：

- Worker 内部调用 LLM provider 的代理链路有异常。
- 或 Worker 到 orchestrator 的转发层在 R3 tool complete 阶段返回 502。

### 3.2 已降级为 P1 的旧问题：ClaudeCode PTY 启动问题

此前我们见过：

```text
failed to spawn claude with PTY: No such file or directory (os error 2)
```

但基于本轮 R3 复测，至少对 `web_search` 与 `web_fetch` 来说，这已经不是当前 blocker。

## 4. 建议修复顺序

### 4.1 P0：先修 Worker 侧 502 闭环

建议先核查：

1. Worker 内 LLM provider、proxy、orchestrator 的调用链路。
2. `tool complete` 回传阶段为何得到 `502 Bad Gateway`。
3. 是否是上游地址配置错误、token 或 headers 丢失、代理超时重试、R3 新增 tool schema 与 orchestrator 侧不一致。
4. 增加结构化日志：execution_id、job_id、tool_hints、job_mode、provider target、HTTP status、response body 摘要。

### 4.2 P1：保留 ClaudeCode 环境检查，但不作为本轮主 blocker

建议保留下面检查项，但顺序后置：

1. `claude` binary 可见性
2. PTY spawn 环境变量
3. PATH 与 cwd
4. 显式 `CLAUDE_BIN` 配置

## 5. 期望修复完成定义

P0 通过标准：

1. `tool_hints=["web_fetch"]` 时：`job_mode=worker`、`done=true`、receipt 存在、且不再出现 502。
2. `tool_hints=["web_search"]` 时：`job_mode=worker`、`done=true`、receipt 存在、且不再出现 502。
3. 若失败，必须给出可诊断的结构化失败原因，而不是笼统 502。

联合通过标准：

1. `chimera-core` 发来的 search 或 fetch 任务能稳定映射成 `web_search` 或 `web_fetch`。
2. 不再被误映射成 `browser`。
3. joint retest 可证明 `after.lane=runtime`、`job_mode=worker`、非 `runtime_task` 拒绝。

## 6. 联合结论

当前可以明确下结论：

- `chimera-core` 与 `ironelf` 的 runtime API 兼容层已打通。
- R3 的 `web_search` 与 `web_fetch` 分流已落地。
- 本轮新的核心 blocker 在 `ironelf` Worker 平面的 502。
- `chimera-core` 侧需要并行做 hint 对齐，避免把 search 或 fetch 再污染成 `browser` 或 `runtime_task`。

## 7. 2026-03-31 最新联合判定更新

基于 2026-03-31 的再次联调，前述 blocker 已经发生变化，需要更新单一事实源：

### 7.1 已确认修复的项

1. `GET /api/runtime/health` 正常：
   - `ok=true`
   - `capabilities=[health, submit, events, cancel]`
   - `supported_tool_hints` 包含 `web_fetch`、`web_search`、`browser`、`shell`、`workspace`
2. `GET /api/gateway/status` 正常。
3. `GET /v1/models` 正常：
   - 返回 `200 OK`
   - 模型列表包含 `qwen3.5-plus-2026-02-15`
4. `web_fetch` 已可稳定进入 `Worker` 并成功收口：
   - `execution_id=exec-regress-webfetch-002`
   - `job_mode=worker`
   - `done=true`
   - `terminal_state=DONE`
   - `execution_state=executed`
5. `web_search` 已可稳定进入 `Worker` 并成功收口：
   - `execution_id=exec-regress-websearch-002`
   - `job_mode=worker`
   - `done=true`
   - `terminal_state=DONE`
   - `execution_state=executed`
6. 增补的“必须返回来源列表”强约束回测已通过：
   - `execution_id=exec-regress-websearch-003`
   - `job_mode=worker`
   - `done=true`
   - `terminal_state=DONE`
   - `execution_state=executed`
   - 返回了结构化 `Sources` 列表，至少包含 IANA / RFC 2606 等来源 URL

### 7.2 仍需保留的设计事实

1. `browser` 仍按设计走非 Worker 路径：
   - `execution_id=exec-regress-browser-002`
   - `job_mode=claude_code`
   - 未被错误送入 `worker`
2. 当前 `browser` 在缺少 `claude` binary 的环境下会结构化失败：
   - `done=true`
   - `terminal_state=FAILED`
   - `execution_state=failed`
   - 摘要明确指出 `claude binary not found in runtime worker env`
3. 这说明 `browser` 路径的守护没有回归，但其运行环境是否完备，仍是独立的运维课题。

### 7.3 结论修正

此前文档里的主 blocker：

- `Provider proxy request failed ... 502 Bad Gateway`

在 2026-03-31 最新回测中，对 `web_fetch` / `web_search` 主路径已不再成立。当前联合结论应更新为：

1. `ironelf` R3 Worker 主路径对 `web_fetch` / `web_search` 已达到可用状态。
2. `chimera-core` 与 `ironelf` 在 `web_fetch` / `web_search` 的能力名和分流结果上，已基本对齐。
3. 当前剩余关注点不是 `Worker 502`，而是：
   - `browser` 路径运行环境是否需要补齐 `claude` binary
   - `web_search` 的 receipt 摘要 provenance 仍偏 `fetch` 风格，可继续优化但不阻塞上线前联调
