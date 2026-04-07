# 04-TaskPackage-S10.B-Rename-Base-Alignment-v1-2026-04-07

## 目的

将 `ironelf -> chimera-iceclaw` rename / base-alignment 以独立任务包方式登记到 `ironelf` 子项目文档区，避免后续项目内维护断链。

## 上游资料包

- `/Users/sourcefire/X-lab/docs/stack-architecture/2026-04-s10b-rename-base-alignment-v1/00-README.md`
- `/Users/sourcefire/X-lab/docs/stack-architecture/2026-04-s10b-rename-base-alignment-v1/01-current-proposal.md`
- `/Users/sourcefire/X-lab/docs/stack-architecture/2026-04-s10b-rename-base-alignment-v1/02-task-pack-chimera-iceclaw-rename-base-alignment.md`
- `/Users/sourcefire/X-lab/docs/stack-architecture/2026-04-s10b-rename-base-alignment-v1/03-task-cards.md`
- `/Users/sourcefire/X-lab/docs/stack-architecture/2026-04-s10b-rename-base-alignment-v1/04-risk-rollback-and-cutover.md`
- `/Users/sourcefire/X-lab/docs/stack-architecture/2026-04-s10b-rename-base-alignment-v1/05-dev-kickoff-prompt.md`

## 本轮目标摘要

本轮不是继续加功能，而是为 `chimera-iceclaw` 建立新的仓库/目录/项目基线：

1. 明确 canonical 命名
2. 平稳切换本地目录与 git 远程项目名
3. 保留 `ironclaw` 作为内部代码、crate、binary 与配置路径兼容名
4. 为目录与 remote 切换保留可回退的旧开发/旧目录基线

## 本轮约束

- 不把 rename 和其他新功能耦合
- 不破坏当前 control-plane 联调主线
- 不做无回退的一步到位硬切换
- 不把 `Cargo package` / `crate` / `binary` / `~/.ironclaw` 强行 rename 作为当前必做项

## 子项目侧维护说明

`ironelf` 在这轮仍可作为旧开发与检查工作区保留。

未来切换稳定后，再考虑：

- `chimera-iceclaw` 成为主对外仓 / 主目录 / 项目管理基线
- `ironelf` 退回历史兼容或开发过渡目录
- 是否继续长期保留内部 `ironclaw` 实现名，按 upstream 升级便利性再判断
