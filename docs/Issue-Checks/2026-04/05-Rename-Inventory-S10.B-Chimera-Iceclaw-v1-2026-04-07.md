# 05-Rename-Inventory-S10.B-Chimera-Iceclaw-v1-2026-04-07

## 目标

为 `ironelf / ironclaw -> chimera-iceclaw` 建立第一版 inventory 与 naming matrix，并明确当前目标是“项目基线迁移”，不是“内部实现名全量 rename”。

## 1. 当前基线判断

当前工作区与联调基线：

- 开发目录仍为 `/Users/sourcefire/X-lab/ironelf`
- 当前 git remote 仍为 `origin = git@github.com:biexiaofeeng-boop/ironelf.git`
- 新 canonical 远程目标为 `https://github.com/biexiaofeeng-boop/chimera-iceclaw`
- 当前 `chimera-core <-> ironelf` 联调主线必须保持不破

## 2. Rename Inventory

### 2.1 高密度热点分类

| 分类 | 典型位置 | 当前状态 | 风险判断 |
|---|---|---|---|
| Cargo package / workspace crate | `Cargo.toml`, `crates/ironclaw_common`, `crates/ironclaw_safety` | 包名、crate 名、workspace member 仍为 `ironclaw*` | 当前允许保留，不作为迁移阻塞项 |
| 二进制 / CLI 命令 | `src/cli/mod.rs`, `src/main.rs`, `src/worker/*` | 可执行名与帮助文本仍为 `ironclaw` | 当前允许保留，不作为迁移阻塞项 |
| 默认配置目录 / 本地状态 | `~/.ironclaw`, `src/bootstrap`, `src/settings.rs`, `src/setup/*` | 配置、DB、日志、skills、channels 全部挂在 `.ironclaw` | 当前允许保留，不作为迁移阻塞项 |
| Runtime / sandbox 标识 | `ironclaw-worker`, `ironclaw.job_id`, Docker label / container name | 运行时标签和镜像名仍是 `ironclaw*` | 当前允许保留，避免误伤运行面 |
| Deploy / service / env | `deploy/*.service`, `deploy/setup.sh`, `deploy/restart.sh`, `deploy/macos-service.sh`, `deploy/env.example` | 可按需要采用 `chimera-iceclaw` 对外名称 | 可保留现状，不要求继续深挖 |
| README / docs / metadata | `README*`, `docs/Issue-Checks/2026-04/*`, `Cargo.toml` homepage/repository | 对外项目名适合前移到 `chimera-iceclaw` | 本轮优先 |
| Git remote / local directory | `origin`, `/Users/sourcefire/X-lab/ironelf` | 仍停留在旧项目基线 | 本轮最高优先级 |
| CI / release / tests | `.github/workflows/*`, `tests/e2e/*` | 存在大量 `ironclaw` hardcode | 当前允许保留，不作为目录迁移阻塞项 |

### 2.2 粗粒度分布

基于本轮 inventory 搜索：

- `src/` 内存在约 `154` 个命名热点文件
- `tests/` 内存在约 `90` 个命名热点文件
- `docs/` 内存在约 `33` 个命名热点文件
- `deploy/` 下 `6` 个文件全部涉及命名切换

结论：

1. 全仓一次性 rename 风险过高，也不是当前真实目标。
2. 当前最重要的是 remote、目录、项目展示名切换。
3. Rust package、binary、config dir、runtime label 可以持续保留 `ironclaw`，以减少与 upstream 偏离。

## 3. Naming Matrix

| 项目面 | Canonical Name Now | Compatibility Keep | 处理策略 |
|---|---|---|---|
| 对外项目名 | `chimera-iceclaw` | 文档中说明兼容 `ironelf / ironclaw` 过渡期 | 本轮执行 |
| Git 元数据 / remote | `chimera-iceclaw` | 当前 `origin` 可暂时保留旧仓记录 | 本轮执行 |
| 本地目录 | `/Users/sourcefire/X-lab/chimera-iceclaw` | 保留 `/Users/sourcefire/X-lab/ironelf` | 本轮执行 |
| Cargo homepage / repository | 指向 `chimera-iceclaw` | 包名仍是 `ironclaw` | 本轮执行 |
| Rust package / crate / binary | `ironclaw` | `ironclaw` | 允许长期保留 |
| `~/.ironclaw` 配置目录 | `~/.ironclaw` | `~/.ironclaw` | 允许长期保留 |
| Docker image / runtime labels | `ironclaw*` | `ironclaw*` | 允许长期保留 |
| deploy / service 名称 | 允许 `chimera-iceclaw` 对外名 | 兼容 `ironclaw` | 按运维需要保留双入口 |
| CI / test hardcode | `ironclaw` | `ironclaw` | 当前不动 |

## 4. 本轮第一波实现范围

本轮直接落地：

1. `Cargo.toml` 的 `homepage` / `repository`
2. README 顶部 canonical naming 说明
3. 新 git remote 接入与同步
4. 新本地目录 `/Users/sourcefire/X-lab/chimera-iceclaw`
5. 旧目录 `/Users/sourcefire/X-lab/ironelf` 保留作回退基线

本轮明确不动：

1. `name = "ironclaw"`
2. workspace crate 目录名
3. CLI 子命令名 `ironclaw`
4. `~/.ironclaw` 默认目录
5. runtime label / container label / worker image 名
6. 大批测试、CI、发布脚本 rename

## 5. 目录与 remote 策略建议

### 本地目录

- canonical 目标目录：
  - `/Users/sourcefire/X-lab/chimera-iceclaw`
- 过渡期保留目录：
  - `/Users/sourcefire/X-lab/ironelf`

建议：

1. 先在新目录做独立 clone 或 worktree。
2. 不要立即改掉现有 `ironelf` 工作目录。
3. 生产切换稳定后，再决定是否引入 `chimera-iceclaw-dev`。

### Git remote

建议切换顺序：

1. 当前仓增加新 remote：
   - `chimera-iceclaw`
2. 先把 `main` 与当前开发分支推到新仓
3. 在新目录中以 `chimera-iceclaw` 远程作为 `origin`
4. 将旧仓保留为 `ironelf-legacy`

## 6. 结论

本轮 S10.B 第一阶段应定义为：

- `chimera-iceclaw` 成为仓库远程、主目录与对外项目名
- `ironclaw` 继续保留为内部实现名与 upstream 兼容名
- `ironelf` 目录与旧 remote 继续保留为过渡期开发与回滚基线
