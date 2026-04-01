# 给 ironelf Rust Dev 的启动提示词（R3）

你在分支 `codex/r3-worker-native-web-v1` 上工作。

目标：让 `ironelf` Worker 原生支持 `web_fetch` 和 `web_search`，并在 bridge 路由中把这两类请求从 `ClaudeCode` 收回到 Worker；`browser` 暂不处理。

参考范围：

1. 工具层参考：
   `/Users/sourcefire/X-lab/chimera-core/nanobot/agent/tools/web.py`
2. 路由层只用于理解边界，不直接迁移：
   `/Users/sourcefire/X-lab/chimera-core/nanobot/intel/web_intel_router.py`
3. browser/vision adapter 只用于理解 deferred boundary：
   `/Users/sourcefire/X-lab/chimera-core/nanobot/intel/adapters/browser_session.py`
   `/Users/sourcefire/X-lab/chimera-core/nanobot/intel/adapters/vision_rpa.py`

硬约束：

1. 只迁移 tool/runtime 层，不迁移 orchestration/router 层。
2. 先做 `web_fetch`，再做 `web_search`。
3. 不要把 `browser` 或 `vision/rpa` 偷渡进这个任务包。
4. 更新路由前，必须先保证 Worker 里真实存在对应工具。
5. 所有输出都要结构化，特别是 evidence、provider_used、fallback_used、timeout、auth/rate-limit 错误。

执行顺序：

1. T01-T03：钉死边界和 tool contract
2. T04-T06：落 `web_fetch`
3. T07-T10：落 `web_search`
4. T11-T13：更新 runtime 注册、路由、准入策略
5. T14-T16：测试、回填 checks、总结 handoff

交付物：

1. Worker-native `web_fetch` / `web_search` 实现
2. route selection 更新
3. `15-Checks-R3-Worker-Native-Web-v1-2026-03-29.md` 回填证据
4. changed files + commit hash
5. known limits，明确写出 `browser deferred`
