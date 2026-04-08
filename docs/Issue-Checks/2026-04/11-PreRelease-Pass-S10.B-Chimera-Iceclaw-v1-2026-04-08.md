# 11-PreRelease-Pass-S10.B-Chimera-Iceclaw-v1-2026-04-08

## 目标

记录 `chimera-iceclaw` 作为生产基线候选目录的首轮预发布通过证据。

## 检查基线

- 目录：
  - `/Users/sourcefire/X-lab/chimera-iceclaw`
- 分支：
  - `codex/s10b-rename-base-alignment-v1`
- 提交：
  - `a6e3f5ad4f5943d20c47632d2b5062f25adc9f01`

## 执行命令

### 1. 预发布检查脚本

```bash
cd /Users/sourcefire/X-lab/chimera-iceclaw
bash deploy/pre_release_check.sh
```

### 2. focused control-plane tests

```bash
cd /Users/sourcefire/X-lab/chimera-iceclaw
cargo test control_plane --features libsql -- --nocapture
```

### 3. 跨仓 smoke

```bash
cd /Users/sourcefire/X-lab/chimera-core-prod
bash deploy/chimera_s10a_controlplane_it.sh
```

## 结果摘要

### A. 本地预发布检查通过

`bash deploy/pre_release_check.sh` 通过，包含：

1. `cargo build` 通过
2. `bash deploy/macos-service.sh status` 正常
3. `GET /api/gateway/status` 正常
4. `GET /api/runtime/health` 返回 `ok=true`
5. `GET /v1/models` 正常返回模型列表

关键返回：

- `llm_backend = openai_compatible`
- `llm_model = qwen3.5-plus-2026-02-15`
- `enabled_channels = ["http","gateway"]`
- `runtime/health.ok = true`

### B. Focused tests 通过

`cargo test control_plane --features libsql -- --nocapture` 通过：

- `8 passed`
- `0 failed`

关键测试：

- `accept_task_intent_persists_receipt_dispatch_and_event`
- `duplicate_same_payload_returns_same_receipt_without_duplicate_event`
- `duplicate_conflicting_payload_is_rejected_safely`
- `mismatched_user_is_rejected_before_writing`
- `accept_route_returns_structured_receipt`
- `legacy_accept_route_returns_structured_receipt`
- `test_control_plane_invalid_token_returns_json_error`
- `test_control_plane_legacy_route_invalid_token_returns_json_error`

说明：

- 这是 `chimera-iceclaw` 新目录中的首次 test profile 构建，耗时较长，但最终通过。

### C. 跨仓 smoke 通过

`chimera-core` 执行结果：

- 预检查通过：
  - `gateway-status`
  - `runtime-health`
  - `models`
- primary route 通过
- primary idempotency 通过
- canonical route 通过
- cross-route idempotency 通过
- 最终结果：
  - `PASS`

关键证据：

- `receipt_id = f0f6c0bb-72ed-4454-ba22-a99eaec4d855`
- `task_id = 74061dfc-b869-4cdd-8ef9-91660df87a0a`
- `intent_id = intent-s10a-live-2026-04-08-001`

## 日志补充说明

`bash deploy/macos-service.sh logs 200` 中看到的时间戳为 UTC 风格输出，例如：

- `2026-04-08T00:24:31Z`

换算到 `Asia/Shanghai` 为：

- `2026-04-08 08:24:31 +08:00`

因此日志时间与本地时区感知不一致属于正常现象，不是运行异常。

## 结论

当前可以确认：

1. `/Users/sourcefire/X-lab/chimera-iceclaw` 已具备预发布基线资格
2. `chimera-core -> chimera-iceclaw` 的 control-plane smoke 已打通
3. 旧目录 `/Users/sourcefire/X-lab/ironelf` 继续保留为回退基线，但不再是主验证入口
4. 当前尚未发现阻止进入下一轮预发布/联调的阻断性问题
