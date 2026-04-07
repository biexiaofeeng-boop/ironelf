# 09-Execution-S10.B-Remote-Directory-Cutover-v1-2026-04-07

## 目标

记录本轮 S10.B 收敛后的实际执行动作：

1. 接入 `chimera-iceclaw` 新 remote
2. 同步基础分支与当前开发分支
3. 建立新的本地目录 `/Users/sourcefire/X-lab/chimera-iceclaw`
4. 保留旧目录 `/Users/sourcefire/X-lab/ironelf` 作为回退基线

## 执行原则

1. 不移动当前工作目录
2. 不移除旧 `origin`
3. 不改 `ironclaw` 内部实现名
4. 新旧目录并存，先验证再切换入口

## 执行命令

```bash
cd /Users/sourcefire/X-lab/ironelf
git remote add chimera-iceclaw git@github.com:biexiaofeeng-boop/chimera-iceclaw.git
git push chimera-iceclaw main
git push chimera-iceclaw codex/s10b-rename-base-alignment-v1
git clone git@github.com:biexiaofeeng-boop/chimera-iceclaw.git /Users/sourcefire/X-lab/chimera-iceclaw
cd /Users/sourcefire/X-lab/chimera-iceclaw
git remote add ironelf-legacy git@github.com:biexiaofeeng-boop/ironelf.git
git checkout codex/s10b-rename-base-alignment-v1
git remote -v
git status --short --branch
```

## 执行结果

### 结果摘要

- 当前旧工作区已接入新 remote：
  - `chimera-iceclaw = git@github.com:biexiaofeeng-boop/chimera-iceclaw.git`
- 已成功同步以下分支到新仓：
  - `main`
  - `codex/s10b-rename-base-alignment-v1`
- 已成功创建新目录：
  - `/Users/sourcefire/X-lab/chimera-iceclaw`
- 新目录中的 remote 关系已完成：
  - `origin = git@github.com:biexiaofeeng-boop/chimera-iceclaw.git`
  - `ironelf-legacy = git@github.com:biexiaofeeng-boop/ironelf.git`
- 新目录当前分支：
  - `codex/s10b-rename-base-alignment-v1`
- 新目录当前状态：
  - working tree clean

### 新目录核验结果

```bash
cd /Users/sourcefire/X-lab/chimera-iceclaw
git remote -v
git status --short --branch
```

结果：

- `origin` 已指向 `chimera-iceclaw`
- `ironelf-legacy` 已作为旧仓回退入口保留
- 分支状态：
  - `## codex/s10b-rename-base-alignment-v1...origin/codex/s10b-rename-base-alignment-v1`

### 结论

本轮“项目基线迁移”已经实际执行到位：

1. 旧目录 `ironelf` 继续保留
2. 新目录 `chimera-iceclaw` 已可作为 canonical 工作目录
3. 新旧远程并存，且回退路径清晰

## 回滚方式

1. 继续回到旧目录：
   - `/Users/sourcefire/X-lab/ironelf`
2. 继续使用旧 remote：
   - `origin = git@github.com:biexiaofeeng-boop/ironelf.git`
3. 若新目录异常，可直接删除新 clone，而不影响旧工作区
