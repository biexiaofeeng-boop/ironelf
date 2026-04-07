# 07-Phase2-Plan-S10.B-Chimera-Iceclaw-v1-2026-04-07

## 目标

在 S10.B 第一波 inventory / naming matrix / deploy-alignment 基础上，重新收敛后续计划，但仍坚持：

1. 不直接破坏当前 `chimera-core <-> ironelf` 联调主线
2. 不把 package rename、目录切换、远程切换、运行面切换揉成一次性提交
3. 每一波都必须有独立回滚路径

## 第二波的核心问题

第一波已经解决：

- 文档与 deploy/service canonical naming
- legacy service 与 restart alias 保留
- repo metadata 已前移到 `chimera-iceclaw`

当前真正还需要继续解决的内容：

1. 本地目录仍然是 `/Users/sourcefire/X-lab/ironelf`
2. git remote 仍然是 `ironelf`
3. 需要建立新目录与新 remote 的平稳接管路径
4. 内部实现名是否保持 `ironclaw`，现在答案是“可以保持”

## 推荐拆分原则

### 原则 1：目录/remote 优先，内部实现名保留

当前优先完成项目管理基线切换：

- 本地目录
- git remote
- 顶层项目展示名

而不是继续深入 package / crate / binary / config dir rename。

### 原则 2：先 alias，再 rename

如果某一层已经被外部脚本、运维命令、测试夹具或用户习惯直接依赖，先加兼容 alias，再讨论 hard rename。

### 原则 3：内部路径能不动就不动

对于以下内容，当前都允许继续保留 `ironclaw`：

- `Cargo package`
- crate 名
- binary 名
- `~/.ironclaw`
- runtime labels
- CI/test hardcode

## 下一阶段建议拆包

### P2.A Remote / Directory Cutover

目标：

- 把项目管理与开发入口逐步切到 `chimera-iceclaw`

范围：

- 新 remote 接入与同步
- 新目录 clone / worktree
- 老目录继续保留

建议顺序：

1. 在当前仓新增 `chimera-iceclaw` remote
2. 验证 push/pull
3. 在 `/Users/sourcefire/X-lab/chimera-iceclaw` 建立新工作目录
4. 只在 smoke 通过后，才升级 canonical 开发入口

### P2.B Optional Long-Term Cleanup

目标：

- 如果将来确实有收益，再考虑是否处理以下长期项：
  - CLI alias
  - config dir migration
  - runtime labels/image
  - CI/tests hardcode

结论：

- 这些都不是当前切换 `chimera-iceclaw` 项目基线的前置条件。

## 最低验收标准

第二波每个子包至少要给出：

1. inventory 或 file hotspot list
2. compatibility strategy
3. rollback strategy
4. focused test / smoke plan

## 当前建议

下一步最值得先开的是：

- `P2.A Remote / Directory Cutover`

因为这才是当前最贴近真实业务目标的迁移动作，而且不会无谓扩大与 upstream `ironclaw` 的差异面。
