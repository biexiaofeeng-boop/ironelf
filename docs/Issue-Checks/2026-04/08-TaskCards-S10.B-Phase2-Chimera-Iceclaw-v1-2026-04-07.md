# 08-TaskCards-S10.B-Phase2-Chimera-Iceclaw-v1-2026-04-07

## 目标

把 S10.B 后续动作拆成可以独立执行、独立验证、独立回滚的任务卡，并明确“目录/remote 优先，内部实现名可保留”。

## T11 Remote Canonicalization

范围：

- 当前仓 `git remote`
- 新仓 `git@github.com:biexiaofeeng-boop/chimera-iceclaw.git`

任务：

1. 在旧工作区接入 `chimera-iceclaw` remote
2. 同步 `main` 与当前开发分支
3. 保留旧 `origin` 为回退基线

验收：

- 新 remote 可 fetch / push
- 旧 remote 不丢
- 当前联调分支可在新仓存在

## T12 Local Directory Cutover

范围：

- `/Users/sourcefire/X-lab/chimera-iceclaw`
- `/Users/sourcefire/X-lab/ironelf`

任务：

1. 创建新 canonical 工作目录
2. 在新目录中以 `chimera-iceclaw` 作为 `origin`
3. 将旧仓保留为 `ironelf-legacy` 或回退参照

验收：

- 新目录可正常 `git status`
- 新目录可正常切到当前开发分支
- 旧目录仍可独立工作

## T13 Top-Level Project Naming

范围：

- `README*`
- `Cargo.toml` metadata
- 项目索引文档

任务：

1. 保持顶层对外项目名使用 `chimera-iceclaw`
2. 明确说明内部 `ironclaw` 命名仍保留
3. 降低外部命名与内部实现名之间的歧义

验收：

- 对外项目名统一
- 内部实现名保留策略明确

## T14 Rollback Baseline

范围：

- 旧目录
- 旧 remote
- 当前运行面

任务：

1. 明确保留旧目录 `/Users/sourcefire/X-lab/ironelf`
2. 明确保留旧 remote 与回退命令
3. 明确 smoke 失败后的回切方式

验收：

- rollback 命令明确
- 不依赖现场临时记忆

## T15 Optional Long-Term Cleanup Backlog

范围：

- `src/cli/*`
- `src/bootstrap.rs`
- `src/settings.rs`
- `src/orchestrator/*`
- `.github/workflows/*`
- `tests/e2e/*`

任务：

1. 将以下项目列为长期可选项，而非当前切换阻塞项：
   - CLI alias
   - `~/.ironclaw` 迁移
   - runtime labels/image rename
   - CI/tests hardcode cleanup
2. 只在确有业务收益时再单独开包

验收：

- backlog 定义清楚
- 不误导为当前必做项

## T16 Close-Out Gate

任务：

1. 统一回填 checks
2. 统一列出修改文件
3. 统一列出 build/test/smoke 结果
4. 统一列出风险与未决项

验收：

- 可以直接作为 merge 前审阅材料
