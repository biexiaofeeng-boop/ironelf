# 06-Checks-S10.B-Rename-Base-Alignment-v1-2026-04-07

## 检查目标

对 S10.B 第一波项目基线迁移做六项交付确认：

1. rename inventory 是否落库
2. naming matrix 是否清晰
3. remote / 目录切换是否明确
4. 改动是否限制在低风险对齐层
5. 验证是否通过
6. rollback 步骤是否明确

## 1. 交付结果总览

本轮产出：

- rename inventory：
  - `docs/Issue-Checks/2026-04/05-Rename-Inventory-S10.B-Chimera-Iceclaw-v1-2026-04-07.md`
- checks / close-out：
  - `docs/Issue-Checks/2026-04/06-Checks-S10.B-Rename-Base-Alignment-v1-2026-04-07.md`
- 第一波实现：
  - 对外 canonical naming 前移到 `chimera-iceclaw`
  - 保留 `ironclaw` 作为内部实现 / binary / config compatibility 名称

## 2. 改动文件列表

本轮核心修改文件：

- `Cargo.toml`
- `README.md`
- `README.zh-CN.md`
- `deploy/env.example`
- `deploy/ironclaw.service`
- `deploy/chimera-iceclaw.service`
- `deploy/restart.sh`
- `deploy/setup.sh`
- `deploy/macos-service.sh`
- `docs/Issue-Checks/2026-04/00-INDEX-2026-04.md`
- `docs/Issue-Checks/2026-04/05-Rename-Inventory-S10.B-Chimera-Iceclaw-v1-2026-04-07.md`
- `docs/Issue-Checks/2026-04/06-Checks-S10.B-Rename-Base-Alignment-v1-2026-04-07.md`

## 3. 实现边界确认

本轮已完成：

1. `chimera-iceclaw` 成为 README 顶部与 repo metadata 的 canonical 对外项目名
2. 当前策略已明确：`ironclaw` 保留为内部实现 / binary / config compatibility 名称
3. 本轮执行重点切到 git remote 与本地目录基线，而不是强推源码 rename

本轮明确未做：

1. 不改 Cargo package 名 `ironclaw`
2. 不改 workspace crate 名与 crate 目录
3. 不改 CLI 命令 `ironclaw`
4. 不改 `~/.ironclaw` 默认目录
5. 不改 worker image、runtime labels、sandbox labels
6. 不做 CI / tests / release 全量 rename

判断：

- 范围聚焦在 metadata / docs / remote / directory cutover
- 未触碰当前 `chimera-core <-> ironelf` 联调关键运行面
- 符合“项目名/目录名迁移，内部实现名保留”的收敛目标

## 4. Build / Test / Static Checks

### 执行命令

```bash
cd /Users/sourcefire/X-lab/ironelf
cargo build
cargo test control_plane --features libsql -- --nocapture
bash -n deploy/macos-service.sh
bash -n deploy/restart.sh
bash -n deploy/setup.sh
bash deploy/macos-service.sh status
```

### 结果

- `cargo build`：通过
- `cargo test control_plane --features libsql -- --nocapture`：通过
  - `8 passed`
  - `0 failed`
- `bash -n deploy/macos-service.sh`：通过
- `bash -n deploy/restart.sh`：通过
- `bash -n deploy/setup.sh`：通过
- `bash deploy/macos-service.sh status`：通过
  - 本机服务仍处于 `running`
  - 当前二进制仍为 `/Volumes/ChimeraData/build-cache/rust-targets/ironelf/debug/ironclaw`
  - 当前日志路径仍为 `~/.ironclaw/logs/ironclaw.current.log`

说明：

- 本轮没有触碰 Rust package / binary / runtime label，因此 focused runtime regression 仍以现有 control-plane 测试和本机服务状态核对为主。
- `ironclaw` 作为内部实现名被明确保留，不再将其视为当前迁移阻塞项。

## 5. 运维切换步骤

### 本轮项目基线切换执行结果

已执行：

1. 当前仓已接入：
   - `chimera-iceclaw = git@github.com:biexiaofeeng-boop/chimera-iceclaw.git`
2. 已同步新仓分支：
   - `main`
   - `codex/s10b-rename-base-alignment-v1`
3. 已建立新 canonical 工作目录：
   - `/Users/sourcefire/X-lab/chimera-iceclaw`
4. 新目录内已保留旧仓回退入口：
   - `ironelf-legacy = git@github.com:biexiaofeeng-boop/ironelf.git`

### 本轮项目基线切换建议

1. 保留当前目录与旧 remote：
   - `/Users/sourcefire/X-lab/ironelf`
   - `origin = git@github.com:biexiaofeeng-boop/ironelf.git`
2. 新增 canonical remote：
   - `git@github.com:biexiaofeeng-boop/chimera-iceclaw.git`
3. 将当前仓的 `main` 和开发分支同步到新 remote
4. 新建 canonical 工作目录：
   - `/Users/sourcefire/X-lab/chimera-iceclaw`
5. 在新目录中将 `origin` 指向 `chimera-iceclaw`
6. 将旧仓接入为 `ironelf-legacy`

## 6. Rollback 步骤

### Remote / 目录回滚

1. 保持 `/Users/sourcefire/X-lab/ironelf` 不动
2. 如果 `/Users/sourcefire/X-lab/chimera-iceclaw` smoke 失败，直接回旧目录继续开发
3. 当前旧工作区仍保留旧 `origin`
4. 新工作区中也已保留 `ironelf-legacy`
5. 若新仓协作异常，继续以旧 remote 推进当前联调主线

## 7. 已知风险

1. `ironclaw` 与 `chimera-iceclaw` 将长期共存：前者偏内部实现名，后者偏项目管理基线。
2. README 多语言与 CI / tests / release 仍存在大量 `ironclaw` hardcode，但当前不视为迁移阻塞。
3. 新 remote 若为空仓，需要先同步基础分支后再让新目录正式接管。

## 8. 结论

本轮 S10.B 第一波 rename / base-alignment 已满足最低交付：

1. rename inventory 已完成
2. naming matrix 已完成，并已明确“内部实现名保留”
3. 当前最小实现聚焦在 metadata / docs / remote / directory cutover
4. 旧目录与旧 remote 被保留为回滚基线
5. 后续若无明确业务收益，不必继续推进 package / binary / config dir / runtime labels rename
