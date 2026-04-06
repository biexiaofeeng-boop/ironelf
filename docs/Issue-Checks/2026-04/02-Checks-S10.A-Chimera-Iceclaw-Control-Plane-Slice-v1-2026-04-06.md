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

- 当前未检出本轮 S10.A 的独立功能提交
- 相关实现仍主要处于本地 working tree

说明：

- 这不影响本轮“实现检查”和“focused test”判断
- 但会影响后续 merge / 回滚 / 闭环归档的清晰度
- 建议开发线程在合流前形成单独功能提交

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

判断：

- 改动主要围绕 control-plane ingress / auth / persistence / event trail
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

### 当前发现的高优先级边界问题

#### 问题 1：跨仓默认路由未对齐

当前 `ironelf` 路由：

- `/api/control/tasks/accept`

而 `chimera-core` 默认 dispatch path 仍是：

- `/api/controlplane/task-intents`

影响：

- `ironelf` 单侧测试可过
- 但 `chimera-core -> ironelf` 默认联调路径并未天然打通

判断：

- 这是当前最重要的协议对齐风险
- 若部署时没有显式配置覆盖，这会成为真实 handoff 的阻断项

#### 问题 2：accepted event 的时间语义偏弱

在 `src/control_plane.rs` 中，`build_accepted_event()` 目前将事件时间写为：

- `record.observed_at_utc`
- `record.observed_at_local`

而不是 acceptance 真正发生时的：

- `record.accepted_at_utc`

影响：

- 对后续审计/事件重放来说，`task.accepted` 事件更像“源请求观察时间”，而不是“control plane 接收时间”
- 当前不会阻断最小链路，但会削弱后续 durable event 的时间语义

判断：

- 属于中优先级残余风险
- 建议下一轮尽早统一事件时间口径

## 4. 工作树状态

检查时存在以下额外情况：

- 若干 `.DS_Store` / `.idea` / `.cargo` 等无关未跟踪文件
- 本轮新增实现文件仍未提交

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

- `6 passed`
- `0 failed`
- focused control-plane tests 完成
- 本轮输出中未见阻断性 warning / panic

### 本轮通过的关键测试

- `control_plane::tests::accept_task_intent_persists_receipt_dispatch_and_event`
- `control_plane::tests::duplicate_same_payload_returns_same_receipt_without_duplicate_event`
- `control_plane::tests::duplicate_conflicting_payload_is_rejected_safely`
- `control_plane::tests::mismatched_user_is_rejected_before_writing`
- `channels::web::handlers::control_plane::tests::accept_route_returns_structured_receipt`
- `channels::web::auth::tests::test_control_plane_invalid_token_returns_json_error`

## 6. 本轮结论

结论：

- `ironelf` 侧最小 durable control-plane slice 已基本落地
- focused cargo tests 已通过
- 当前未发现必须推翻本轮实现的结构性问题
- 但存在一个需要尽快处理的协议对齐风险：默认接收路由与 `chimera-core` 默认 dispatch path 不一致

建议：

1. 先由开发线程把本轮代码形成独立提交
2. 再处理 `chimera-core <-> ironelf` 默认路由对齐
3. 后续补一轮真实跨仓 handoff 联调验证
