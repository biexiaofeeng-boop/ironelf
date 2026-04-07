# 03-Route-Alignment-Fixlist-S10.A-v1-2026-04-06

## 回填结果

本清单已于 2026-04-07 收口完成。

完成情况：

- `/api/control/tasks/accept` 保持为 canonical route
- `/api/controlplane/task-intents` 已作为 compatibility alias 接入同一 handler
- auth middleware 已同时覆盖 `/api/control/` 与 `/api/controlplane/`
- legacy route success / invalid auth tests 已补齐并通过
- `task.accepted` 事件时间已改为 acceptance 真正发生时间，源观察时间保留在 payload 中
- 本机跨仓 live smoke 已验证 `chimera-core` 默认路径兼容

## 目的

给 `ironelf` 开发线程一份直接可执行的“路由对齐修复点”清单，用于收口 S10.A 最小 control-plane handoff。

## 背景

当前 `ironelf` 单侧 control-plane slice 已实现并通过 focused tests，但跨仓默认联调仍存在一个关键不对齐：

- `chimera-core` 默认 dispatch path: `/api/controlplane/task-intents`
- `ironelf` 当前 accept route: `/api/control/tasks/accept`

如果不做路由兼容或默认路径统一，真实 `chimera-core -> ironelf` handoff 仍可能失败。

## 必须修复

### 1. 接收端增加兼容路由

在 `ironelf` web server 中，除了现有：

- `/api/control/tasks/accept`

还应兼容接受：

- `/api/controlplane/task-intents`

建议做法：

- 两个路由都映射到同一个 handler
- 不要复制两套逻辑
- 不要让 legacy 路由走不同鉴权/不同返回结构

目标：

- 先保证默认跨仓联调可通
- 后续再决定长期 canonical route

### 2. auth middleware 同步覆盖 legacy 路由

当前 `auth.rs` 中 control-plane 路由识别是：

- `request.uri().path().starts_with("/api/control/")`

这会漏掉：

- `/api/controlplane/task-intents`

建议修复：

- control-plane 路由识别同时覆盖：
  - `/api/control/`
  - `/api/controlplane/`

目标：

- 新旧路由都返回同一类结构化 control-plane auth error

### 3. handler tests 增加 legacy route 验证

在现有 handler tests 基础上，增加至少一条：

- POST `/api/controlplane/task-intents`
- 合法 token
- 合法 `TaskIntent`
- 返回结构化 `TaskReceipt`

目标：

- 防止后续有人删掉 legacy 兼容路由而不自知

### 4. auth tests 增加 legacy route 认证失败验证

补一条测试：

- POST `/api/controlplane/task-intents`
- 非法 token
- 应返回结构化 control-plane error envelope

目标：

- 保证 legacy 路由不是“能进 handler 但认证错误格式错乱”

## 建议本轮一起修

### 5. 给 accept route 选定 canonical 路径并注释说明

建议明确：

- 哪个是 canonical route
- 哪个是 compatibility alias

推荐：

- 内部/长期 canonical 继续用 `/api/control/tasks/accept`
- `/api/controlplane/task-intents` 作为 `chimera-core` 兼容入口

原因：

- 这样不打断当前 `ironelf` 设计语义
- 又能兼容现有 `chimera-core` 默认配置

### 6. 修正 accepted event 时间语义

当前 `build_accepted_event()` 使用的是：

- `record.observed_at_utc`
- `record.observed_at_local`

建议至少改为：

- `observed_at_utc = record.accepted_at_utc`

如果想保留源观察时间，可放入 payload，例如：

- `source_observed_at_utc`
- `source_observed_at_local`

目标：

- `task.accepted` 事件代表“control plane 真正接收时间”
- 源请求观察时间仍可追溯

## 建议不要在本修复中做的事

### 7. 不要在同一波同时做大规模 rename

这轮修复的目标是“让 handoff 默认可通”，不是全仓品牌重命名。

本轮不建议顺手改：

- Cargo package name
- crate 名
- 二进制名
- shell completion 文件名
- deploy/env/service 名
- README / homepage / repository 全量替换

原因：

- 会显著扩大 diff
- 会降低回归定位能力
- 会把“协议修复”和“品牌/项目基线调整”耦合在一起

## 最小验收标准

开发完成后至少通过以下验证：

### A. focused cargo tests

```bash
cd /Users/sourcefire/X-lab/ironelf
cargo test control_plane --features libsql -- --nocapture
```

### B. 新增 legacy route tests 通过

至少应覆盖：

- legacy route success
- legacy route invalid auth

### C. 跨仓默认 smoke 应可通

要求：

- `chimera-core` 不额外改 dispatch_path
- 默认打到 `ironelf`
- 能返回最小 `TaskReceipt`

## 交付物要求

开发线程交付时应提供：

1. 修改文件列表
2. 为什么选择 canonical route / alias 策略
3. focused tests 输出
4. 是否已验证 `chimera-core` 默认路径兼容
