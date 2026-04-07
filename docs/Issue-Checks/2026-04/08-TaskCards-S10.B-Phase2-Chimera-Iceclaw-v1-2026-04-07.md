# 08-TaskCards-S10.B-Phase2-Chimera-Iceclaw-v1-2026-04-07

## 目标

把 S10.B 第二波拆成可以独立执行、独立验证、独立回滚的任务卡。

## T11 CLI Alias Layer

范围：

- `src/cli/mod.rs`
- `src/main.rs`
- `src/cli/service.rs`
- `src/cli/snapshots/*`

任务：

1. 盘点 `ironclaw` 命令名、帮助文案、service 子命令输出
2. 设计 `chimera-iceclaw` wrapper 或 alias 方案
3. 明确是否需要保留 `ironclaw` 为主命令

验收：

- `ironclaw` 继续可用
- 新 alias 方案可说明
- 帮助文案不引入歧义

## T12 Base Dir Compatibility

范围：

- `src/bootstrap.rs`
- `src/config/*`
- `src/settings.rs`
- `src/setup/*`
- `src/llm/session.rs`
- `src/pairing/store.rs`

任务：

1. 梳理所有 `~/.ironclaw` 读写点
2. 设计新旧目录探测顺序
3. 设计迁移脚本或迁移子命令

验收：

- 给出双路径读写策略
- 给出迁移/回滚步骤
- 明确哪些状态文件必须一起迁

## T13 Runtime Label Compatibility

范围：

- `src/orchestrator/job_manager.rs`
- `src/orchestrator/reaper.rs`
- `src/config/sandbox.rs`
- `src/sandbox/config.rs`
- `src/settings.rs`

任务：

1. 盘点 `ironclaw.job_id` / `ironclaw-worker:*` / container name
2. 设计新旧 label 双识别
3. 设计配置开关或灰度切换参数

验收：

- 旧 worker / 旧容器仍能被识别
- 新 label 不影响收口与回收
- rollback 明确

## T14 Web/UI Local Storage Naming

范围：

- `src/channels/web/static/app.js`
- `src/channels/web/static/theme-init.js`
- `src/channels/web/static/i18n/index.js`

任务：

1. 盘点 `ironclaw-theme` / `ironclaw_language`
2. 设计 localStorage key 兼容策略
3. 避免升级后丢用户偏好

验收：

- 升级前后的主题和语言设置可继承
- 不出现空白页或首屏回归

## T15 CI / Test Fixture Cleanup

范围：

- `.github/workflows/*`
- `tests/e2e/*`
- `src/cli/snapshots/*`

任务：

1. 盘点测试和 workflow 对 `ironclaw` binary / path / repo 名的依赖
2. 区分必须改与可保留兼容
3. 输出最小修改波次

验收：

- 给出 hotfile list
- 给出拆批策略
- 不与生产切换混做

## T16 Remote Cutover Prep

范围：

- git remote
- `/Users/sourcefire/X-lab/chimera-iceclaw`
- `/Users/sourcefire/X-lab/ironelf`

任务：

1. 新增 `chimera-iceclaw` remote
2. 准备新目录 clone / worktree
3. 形成切换日 checklist

验收：

- push / fetch 命令明确
- 新旧目录并存策略明确
- rollback 到旧目录明确

## T17 Production Cutover Checklist

范围：

- `deploy/*.service`
- `deploy/setup.sh`
- `deploy/restart.sh`
- 运维手册

任务：

1. 明确 canonical service 切换窗口
2. 明确 `.env`、unit、container、日志核查步骤
3. 明确失败回滚步骤

验收：

- 有逐条 checklist
- 有切换前/切换后验证项
- 有 rollback checklist

## T18 Close-Out Gate

任务：

1. 统一回填 checks
2. 统一列出修改文件
3. 统一列出 build/test/smoke 结果
4. 统一列出风险与未决项

验收：

- 可以直接作为 merge 前审阅材料
