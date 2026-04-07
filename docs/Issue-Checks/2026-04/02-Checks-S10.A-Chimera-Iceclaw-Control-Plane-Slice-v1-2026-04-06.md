# 02-Checks-S10.A-Chimera-Iceclaw-Control-Plane-Slice-v1-2026-04-06

## 检查目标

对 `ironelf` 本轮 S10.A durable control-plane slice 做四类确认：

1. 当前提交/工作树状态
2. 改动范围是否聚焦
3. 代码抽检是否存在明显边界问题
4. 针对性回测是否通过

## 1. 提交状态确认

检查时分支：

- `codex/s10-control-plane-slice-v1`

检查结论：

- 本轮 S10.A 实现已在 `codex/s10-control-plane-slice-v1` 分支收口
- 相关实现、route alignment 修复、checks 回填与 live smoke 证据均已准备进入独立功能提交

说明：

- 本轮判断以实现、focused tests、route alignment 和跨仓 live smoke 为准
- 建议保留本轮独立功能提交，便于后续 merge / rollback / close-out 归档

## 2. 改动范围确认

检查到的核心改动集中在以下文件：

- `src/control_plane.rs`
- `src/channels/web/handlers/control_plane.rs`
- `src/channels/web/server.rs`
- `src/channels/web/auth.rs`
- `src/db/mod.rs`
- `src/db/postgres.rs`
- `src/db/libsql/mod.rs`
- `src/db/libsql/control_plane.rs`
- `src/db/libsql_migrations.rs`
- `src/history/store.rs`
- `src/history/mod.rs`
- `src/lib.rs`
- `migrations/V15__control_plane.sql`
- `docs/Issue-Checks/2026-04/00-INDEX-2026-04.md`
- `docs/Issue-Checks/2026-04/02-Checks-S10.A-Chimera-Iceclaw-Control-Plane-Slice-v1-2026-04-06.md`
- `docs/Issue-Checks/2026-04/03-Route-Alignment-Fixlist-S10.A-v1-2026-04-06.md`

判断：

- 改动主要围绕 control-plane ingress / auth / persistence / event trail
- route alignment 修复仍保持在 control-plane ingress / auth / docs 范围内
- 范围总体聚焦，符合任务包目标

## 3. 代码抽检结论

### 已完成的核心内容

- `src/control_plane.rs` 已定义最小协议对象：
  - `TaskIntent`
  - `TaskReceipt`
  - `DispatchRequest`
  - `ExecutionResult`
  - `ControlTaskRecord`
  - `TaskEventRecord`
- `ControlPlaneManager::accept_task_intent(...)` 已落地最小 acceptance 流程
- `src/channels/web/handlers/control_plane.rs` 已提供 web ingress handler
- `src/channels/web/auth.rs` 已补 control-plane 路由的结构化认证错误
- `src/db/libsql/control_plane.rs` 与 `src/history/store.rs` 已补最小持久化
- `src/db/libsql_migrations.rs` 与 `migrations/V15__control_plane.sql` 已补 control-plane schema
- `src/channels/web/server.rs` 已将 `/api/control/tasks/accept` 作为 canonical route，并增加 `/api/controlplane/task-intents` compatibility alias
- `src/channels/web/handlers/control_plane.rs` 已补 legacy route success test
- `src/channels/web/auth.rs` 已补 legacy route invalid auth test
- `src/control_plane.rs` 已将 `task.accepted` 事件时间调整为真正的 acceptance 时间，并在 payload 中保留源观察时间

### 本轮已收口的边界问题

#### 问题 1：跨仓默认路由未对齐

当前状态：

- canonical route:
  - `/api/control/tasks/accept`
- compatibility alias:
  - `/api/controlplane/task-intents`

结论：

- `chimera-core` 默认 dispatch path 已被兼容
- 新旧路由走同一 handler、同一鉴权、同一返回结构
- 本机跨仓 live smoke 已验证两条路由都返回同一份 `TaskReceipt`

#### 问题 2：accepted event 的时间语义偏弱

当前状态：

- `task.accepted` 事件时间已改为 `record.accepted_at_utc`
- 源侧观察时间已保留在 payload 中：
  - `source_observed_at_utc`
  - `source_observed_at_local`

结论：

- acceptance event 的时间语义已与 durable control-plane 接收动作对齐
- 源请求观察时间仍可追溯，不再混淆事件发生时间与源请求时间

## 4. 工作树状态

检查时存在以下额外情况：

- 若干 `.DS_Store` / `.idea` / `.cargo` 等无关未跟踪文件
- 本轮业务实现文件与 docs backfill 已准备按独立功能提交封口

说明：

- 本次检查未将这些无关文件纳入业务判断
- 但合流前应避免把噪音文件一并提交

## 5. 针对性回测

### 执行命令

```bash
cd /Users/sourcefire/X-lab/ironelf
cargo test control_plane --features libsql -- --nocapture
```

### 结果

- `8 passed`
- `0 failed`
- focused control-plane tests 与 legacy route tests 完成
- 本轮输出中未见阻断性 warning / panic

### 本轮通过的关键测试

- `control_plane::tests::accept_task_intent_persists_receipt_dispatch_and_event`
- `control_plane::tests::duplicate_same_payload_returns_same_receipt_without_duplicate_event`
- `control_plane::tests::duplicate_conflicting_payload_is_rejected_safely`
- `control_plane::tests::mismatched_user_is_rejected_before_writing`
- `channels::web::handlers::control_plane::tests::accept_route_returns_structured_receipt`
- `channels::web::handlers::control_plane::tests::legacy_accept_route_returns_structured_receipt`
- `channels::web::auth::tests::test_control_plane_invalid_token_returns_json_error`
- `channels::web::auth::tests::test_control_plane_legacy_route_invalid_token_returns_json_error`

### 补充回测

- `cargo clippy --tests -- -D warnings`

结果：

- clippy clean
- 未检出本轮 control-plane route alignment 修复引入的 warning

## 6. 本轮结论

结论：

- `ironelf` 侧最小 durable control-plane slice 已落地
- route alignment 修复已完成
- focused cargo tests 与 clippy 已通过
- `chimera-core -> ironelf` 默认跨仓 handoff 路径已在本机 live smoke 中打通
- 当前未发现必须推翻本轮实现的结构性问题

### 本机 live smoke 证据

- 预检查通过：
  - `GET /api/gateway/status`
  - `GET /api/runtime/health`
  - `GET /v1/models`
- 默认联调路径通过：
  - `POST /api/controlplane/task-intents`
- canonical 路径通过：
  - `POST /api/control/tasks/accept`
- 幂等通过：
  - `receipt_id=8a414deb-7539-4d92-aa21-022f5b8a800e`
  - `task_id=fc10b0fe-763e-4d68-92cf-6b96928396e1`
  - 重复提交与跨路由提交结果一致

建议：

1. 由开发线程形成本轮独立功能提交并推远端
2. 将 `chimera-core` 新增联调脚本作为后续回归入口保留
3. 下一轮进入 `DispatchRequest -> runtime dispatch -> ExecutionResult` 闭环能力
