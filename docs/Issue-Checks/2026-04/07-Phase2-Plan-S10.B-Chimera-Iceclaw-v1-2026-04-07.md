# 07-Phase2-Plan-S10.B-Chimera-Iceclaw-v1-2026-04-07

## 目标

在 S10.B 第一波 inventory / naming matrix / deploy-alignment 基础上，给出第二波拆解计划，但仍坚持：

1. 不直接破坏当前 `chimera-core <-> ironelf` 联调主线
2. 不把 package rename、目录切换、远程切换、运行面切换揉成一次性提交
3. 每一波都必须有独立回滚路径

## 第二波的核心问题

第一波已经解决：

- 文档与 deploy/service canonical naming
- legacy service 与 restart alias 保留
- repo metadata 已前移到 `chimera-iceclaw`

第二波需要继续解决但尚未落地的内容：

1. CLI / binary 仍然是 `ironclaw`
2. 默认配置目录仍然是 `~/.ironclaw`
3. runtime labels / worker image / container name 仍然是 `ironclaw*`
4. `.github` / `tests` / E2E fixtures 存在大量 `ironclaw` hardcode
5. 本地目录与 git remote 仍未切到 canonical 基线

## 推荐拆分原则

### 原则 1：先 alias，再 rename

如果某一层已经被外部脚本、运维命令、测试夹具或用户习惯直接依赖，先加兼容 alias，再讨论 hard rename。

### 原则 2：先读路径兼容，再写路径迁移

对于 `~/.ironclaw` 这类状态目录：

- 优先让代码支持双路径读取或迁移探测
- 再引入新默认写路径
- 最后才考虑删除旧路径

### 原则 3：运行面和协作面分开

- runtime labels / image / container name 属于运行面
- git remote / 本地目录 / 文档口径属于协作面

这两类不要混在一个提交内。

## Phase 2 建议拆包

### P2.A CLI / Binary Compatibility

目标：

- 评估 `ironclaw` 是否保留为主命令
- 是否新增 `chimera-iceclaw` wrapper / alias
- completion、help output、service install/uninstall 的兼容策略

当前关键落点：

- `src/cli/mod.rs`
- `src/main.rs`
- `src/cli/service.rs`
- `src/cli/snapshots/*`

建议顺序：

1. 先增加 alias / wrapper 能力
2. 补帮助文案说明 canonical name
3. 再评估是否需要改 clap program name

### P2.B Config Base Dir Migration

目标：

- 为 `~/.ironclaw -> ~/.chimera-iceclaw` 建立迁移与兼容读取方案

当前关键落点：

- `src/bootstrap.rs`
- `src/config/*`
- `src/settings.rs`
- `src/setup/wizard.rs`
- `src/setup/README.md`
- `src/llm/session.rs`
- `src/llm/openai_codex_session.rs`
- `src/pairing/store.rs`

建议顺序：

1. 设计双路径探测策略
2. 引入只读兼容：先读新目录，不存在则回退旧目录
3. 提供一次性迁移命令或脚本
4. 最后才切默认写路径

### P2.C Runtime Label / Image Alignment

目标：

- 将 `ironclaw-worker:latest`
- `ironclaw.job_id`
- `ironclaw-worker-*`
- `ironclaw-claude-*`

这些运行面标识逐步切到 `chimera-iceclaw*`

当前关键落点：

- `src/orchestrator/job_manager.rs`
- `src/orchestrator/reaper.rs`
- `src/config/sandbox.rs`
- `src/sandbox/config.rs`
- `src/settings.rs`

风险：

- 会直接影响收容器、回收器、worker 归档、日志排查和已有容器清理

建议顺序：

1. 先让 reaper / manager 同时识别新旧 label
2. 再允许 image / container name 通过配置切换
3. 最后修改默认值

### P2.D CI / Tests / Release Hardcode Cleanup

目标：

- 清理 `.github/workflows`
- `tests/e2e`
- snapshots
- release metadata

中的 `ironclaw` hardcode

当前关键落点：

- `.github/workflows/*`
- `tests/e2e/conftest.py`
- `tests/e2e/helpers.py`
- `tests/e2e/README.md`
- `tests/e2e/CLAUDE.md`
- `src/cli/snapshots/*`

建议顺序：

1. 先把硬编码收集成清单
2. 再区分：
   - 必须改
   - 可保留兼容
   - 暂不改

### P2.E Remote / Directory Cutover

目标：

- 把协作入口逐步切到 `chimera-iceclaw`

范围：

- 新 remote 添加与切换
- 新目录 clone / worktree
- 老目录继续保留

建议顺序：

1. 在当前仓新增 `chimera-iceclaw` remote
2. 验证 push/pull
3. 在 `/Users/sourcefire/X-lab/chimera-iceclaw` 建立新工作目录
4. 只在 smoke 通过后，才升级 canonical 开发入口

## 推荐执行顺序

建议第二波按以下顺序推进：

1. `P2.B Config Base Dir Migration` 设计先行
2. `P2.A CLI / Binary Compatibility`
3. `P2.C Runtime Label / Image Alignment`
4. `P2.D CI / Tests / Release Hardcode Cleanup`
5. `P2.E Remote / Directory Cutover`

原因：

- 配置目录是后续 CLI、setup、session、skills、tool-path 的共同底座
- runtime labels 变更最容易误伤执行平面，不应过早进入
- remote / directory cutover 应当最后做，避免开发主线在未稳定前提前切换

## 最低验收标准

第二波每个子包至少要给出：

1. inventory 或 file hotspot list
2. compatibility strategy
3. rollback strategy
4. focused test / smoke plan

## 当前建议

下一步最值得先开的是：

- `P2.B Config Base Dir Migration`

因为这是未来从 `ironelf` 过渡到 `chimera-iceclaw` 时最容易形成历史负担的一层，也是对运行可靠性影响最大的底座问题。
