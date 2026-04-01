# 给 ironelf Rust Dev 的启动提示词（R2）

- Date: 2026-03-29
- Status: COMPLETED / EVIDENCE BACKFILLED

你在分支 `codex/r2-dmx-router-compat-v1` 上工作。

目标：让 `ironelf` 通过现有 OpenAI-compatible provider 路径稳定支持 `dmx` router，并满足 `chimera-core -> S9 -> ironelf` 的联通要求。

硬约束：

1. 先最小改动，再考虑 built-in provider；不要先发明新的 backend 类型。
2. 先确认 `LLM_BASE_URL` 的 `/v1` 约定、`/models` 行为、completion/streaming 行为，再决定是否需要代码补丁。
3. 如果 generic `openai_compatible` 已足够，优先落 docs、tests、samples、smoke checks。
4. 如果必须加 `dmx`/`dmxapi` alias/profile，只做轻量一层，底下仍走现有 OpenAI-compatible protocol。
5. 不要把 `Cherry` 集成或数据层改造带进这个任务。

执行顺序：

1. T01-T03：钉死 config contract 和实现路线
2. T04-T08：做兼容性验证与必要的窄修复
3. T09-T10：补齐 README / provider docs / operator guidance
4. T11-T12：回填 checks、总结 handoff

交付物：

1. 代码或文档改动
2. `10-Checks-R2-DMX-Router-Compat-v1-2026-03-29.md` 回填证据
3. commit hash 与 changed files
4. 是否需要 built-in provider 的最终结论
5. 已知限制和对 `S9` 的联调说明

## 执行结果回写

1. 已按最小改动路线完成：`dmx` 继续走现有 `openai_compatible` provider，无需新增 built-in backend。
2. 已完成 live + mock 兼容性验证，并回填到 `10-Checks-R2-DMX-Router-Compat-v1-2026-03-29.md`。
3. 已补齐 operator 文档与样例：
   `README.md`、
   `docs/LLM_PROVIDERS.md`、
   `11-DMX-Config-Samples-R2-v1-2026-03-29.md`。
4. 已知边界：
   `chimera-core -> S9 -> ironelf(dmx)` 的 `ironelf` 侧已联通可用；
   完整跨仓 S9 验收仍需在上游仓继续执行。
