# EcoPaste Rust DB Migration Plan

## 背景与结论

当前 EcoPaste 的 history 数据库操作主要在前端完成：`src/database/index.ts` 通过 `@tauri-apps/plugin-sql` + Kysely 初始化 SQLite，`src/database/history.ts` 暴露 `selectHistory`、`insertHistory`、`updateHistory`、`deleteHistory`。Rust 侧目前更多负责窗口、低占用模式、剪贴板监听与粘贴动作，没有统一的 DB repository / service 层。

如果把“这类对数据库的操作”迁移到 Rust 层，重构成本为 **中高**：

- **中等成本**：只迁移 history CRUD 与清空历史相关逻辑，保持前端仍负责剪贴板解析、搜索字段生成、UI 状态管理。
- **较高成本**：把 clipboard ingest（剪贴板读取、去重、排序、插入/更新、清理图片文件、低占用 replay）整体迁到 Rust，前端只订阅事件和渲染列表。
- **不建议一次性全量迁移**：Kysely 的灵活 query builder 当前被多个 UI 场景直接使用，一次性搬到 Rust 会放大 Regression（回归）风险。

推荐采用 **strangler migration（绞杀式迁移）**：先在 Rust 新增 DB service 与 Tauri command facade（门面层），前端保留同名 TS wrapper；然后按 use case 逐步替换，最后移除 Kysely 依赖。

## Scope（范围）

### In Scope

- 新增 Rust DB 模块或插件，封装 SQLite 连接、schema 初始化、history repository。
- 将 history 相关操作迁移到 Rust command：
  - list/query history
  - find by `type + value`
  - insert history
  - update history
  - delete history
  - clear history by filter
  - prune expired / over-limit history
- 将清空历史与低占用队列清理做成一个 Rust-side atomic use case（原子用例），降低 race condition。
- 保留前端 `src/database/history.ts` 作为 compatibility facade（兼容门面），逐步把内部实现从 Kysely 切到 `invoke`。
- 保留现有 SQLite 文件路径和 table schema，避免用户数据迁移成本。

### Out of Scope（第一阶段不做）

- 不重写 UI 状态管理、列表虚拟滚动、搜索框交互。
- 不改变用户现有数据库文件位置和 schema 字段含义。
- 不一次性移除所有 `@tauri-apps/plugin-sql`、Kysely 依赖。
- 不把所有 clipboard parsing 都迁入 Rust，除非进入第二/三阶段。

## Invariants（不变量）

- Existing data must remain readable：已有 `history` 表数据必须可读。
- Schema compatibility：字段名、类型语义保持兼容：`id`、`type`、`group`、`value`、`search`、`count`、`width`、`height`、`favorite`、`createTime`、`note`、`subtype`。
- UI ordering unchanged：默认按 `createTime desc` 排序。
- Favorite semantics unchanged：清理历史时，未勾选删除收藏不得删除 `favorite = true` 记录。
- Image cleanup preserved：删除 image history 时仍要删除对应图片文件。
- Low-resource replay semantics preserved：低占用模式复制的数据仍能在主窗口恢复时进入历史，但清空历史后不应回流旧 snapshot。
- Cross-platform behavior unchanged：Windows/macOS/Linux 的主流程行为一致。

## Target Architecture（目标架构）

```text
Frontend React
  ├─ src/database/history.ts         compatibility facade（先保留 API）
  ├─ src/hooks/useClipboard.ts       调用 facade，不直接碰 SQL
  └─ src/hooks/useHistoryList.ts     调用 facade，不直接拼 SQL

Tauri invoke boundary
  └─ plugin:eco-db 或 app-level commands

Rust DB layer
  ├─ DbState / connection pool
  ├─ migration/schema init
  ├─ history repository
  └─ history service/use cases

SQLite file
  └─ existing EcoPaste database path
```

建议优先新建 `tauri-plugin-eco-db` 或在 `src-tauri/src` 下新增 `database` 模块。若希望和当前插件风格一致，推荐新建插件：

- `src-tauri/src/plugins/database/Cargo.toml`
- `src-tauri/src/plugins/database/src/lib.rs`
- `src-tauri/src/plugins/database/src/commands.rs`
- `src-tauri/src/plugins/database/src/history.rs`
- `src-tauri/src/plugins/database/permissions/default.toml`

## Dependency Choice（依赖选择）

### Option A：`rusqlite`（推荐）

- 优点：轻量、同步 API 简单、SQLite 直连，适合桌面应用本地 DB。
- 优点：比 `tauri-plugin-sql` 更容易在 Rust 层做 transaction（事务）和串行化控制。
- 风险：需要手写 SQL 和类型转换。

### Option B：`sqlx`

- 优点：异步生态成熟，可做 compile-time query check。
- 风险：引入 runtime/feature 较多，构建复杂度更高，对当前桌面小型 SQLite 场景略重。

### Option C：继续从 Rust 调 `tauri-plugin-sql`

- 不推荐：该插件主要服务 JS invoke 边界；Rust 内部 repository 的控制力和测试便利性较弱。

## API Design（Rust Commands）

建议命令按 use case 设计，不把 SQL 细节暴露给前端：

```text
history_init()
history_list(filter: HistoryListFilter) -> Vec<HistoryItem>
history_find_duplicate(type: String, value: String) -> Option<HistoryItem>
history_insert(item: HistoryItem) -> HistoryItem
history_update(id: String, patch: HistoryPatch) -> Option<HistoryItem>
history_delete(id: String) -> bool
history_clear(filter: HistoryClearFilter) -> HistoryClearResult
history_prune(policy: HistoryPrunePolicy) -> HistoryClearResult
history_upsert_clipboard(item: ClipboardHistoryDraft, options: UpsertOptions) -> HistoryUpsertResult
```

第一阶段可以只实现与现有 TS API 对齐的命令，第二阶段再引入 `history_upsert_clipboard` 这类更高层用例。

## Step-by-step Implementation Checklist

### Phase 0：Baseline & Contract（基线与契约）

- [ ] 记录当前 history 表 schema、默认排序、过滤规则、图片删除规则。
- [ ] 补充最小 E2E/manual repro：复制、重复复制、收藏、备注、清空历史、低占用 replay。
- [ ] 为 `src/database/history.ts` 定义稳定的 facade interface，禁止业务组件直接使用 Kysely。
- [ ] 确认数据库文件路径来源，Rust 侧必须复用 `getSaveDatabasePath()` 等价路径。

### Phase 1：Rust DB Infrastructure（基础设施）

- [ ] 新增 Rust DB plugin/module，注册到 `src-tauri/src/lib.rs`。
- [ ] 引入 `rusqlite`，建立 `DbState`，内部使用 `Mutex<Connection>` 或单线程 worker 串行执行 DB 任务。
- [ ] 实现 schema init：`CREATE TABLE IF NOT EXISTS history (...)`，与现有前端 schema 完全一致。
- [ ] 实现 Rust struct：`HistoryItem`、`HistoryPatch`、`HistoryListFilter`、`HistoryClearFilter`。
- [ ] 增加 serde rename / optional field 处理，确保 TS camelCase 字段如 `createTime` 不变。

### Phase 2：CRUD Migration（低风险 CRUD 迁移）

- [ ] 在 Rust 实现 `history_list`，支持 group、favorite、search、offset、limit、order。
- [ ] 在 Rust 实现 `history_find_duplicate(type, value)`。
- [ ] 在 Rust 实现 `history_insert`、`history_update`、`history_delete`。
- [ ] `history_delete` 内保留 image 文件删除逻辑，路径规则与现有 `src/database/history.ts` 一致。
- [ ] 改造 `src/database/history.ts`：保留函数名，但内部改为 `invoke` Rust commands。
- [ ] 保留 Kysely fallback 开关（例如 dev-only flag），便于快速回滚。

### Phase 3：Clear History Atomic Use Case（清空历史原子化）

- [ ] 新增 Rust command：`history_clear(filter)`，由 Rust 一次性完成 query + delete + image cleanup。
- [ ] 在 `history_clear` transaction 前后清理低占用 clipboard queue / dirty flag。
- [ ] 为低占用 replay 引入 clear generation（清空世代）或 clear timestamp，丢弃清空前的 pending snapshot。
- [ ] 前端清空历史弹窗改为调用 `history_clear(filter)`，不再 `selectHistory()` 后逐条 `deleteHistory()`。
- [ ] `history_clear` 完成后再 emit `REFRESH_CLIPBOARD_LIST`。

### Phase 4：Clipboard Upsert Consolidation（剪贴板写入收敛）

- [ ] 将 `select duplicate -> update createTime / insert` 合并为 Rust `history_upsert_clipboard`。
- [ ] 明确 `autoSort`、`copyPlain`、`textThreshold`、pinyin search 仍在前端还是迁到 Rust。
- [ ] 若 search/pinyin 留在前端，Rust command 接收已构造好的 `HistoryItem`。
- [ ] 若 search/pinyin 迁到 Rust，需要替代 `pinyin-pro` 或保留前端生成 search 字段。
- [ ] 让低占用 replay 调用同一个 `history_upsert_clipboard`，避免重复并发路径。

### Phase 5：Cleanup & Dependency Removal（收尾）

- [ ] 移除前端对 Kysely query builder 的业务依赖。
- [ ] 删除或降级 `kysely`、`kysely-dialect-tauri`、`kysely-plugin-serialize`。
- [ ] 若完全不用 JS SQL，移除 `@tauri-apps/plugin-sql` 和 `tauri-plugin-sql`。
- [ ] 更新 skill/docs：DB ownership 从 frontend 改为 Rust。
- [ ] 补充 release note：内部 DB 层迁移，无用户数据变更。

## Validation Strategy（验证策略）

### Static Checks

```powershell
pnpm exec biome check src/database/history.ts src/hooks/useClipboard.ts src/hooks/useHistoryList.ts src/pages/Preference/components/History/components/Delete/index.tsx
cargo check -p EcoPaste --target x86_64-pc-windows-msvc
```

### Rust Tests

如果新增 Rust DB module，建议添加 repository-level tests：

```powershell
cargo test -p tauri-plugin-eco-db
```

测试覆盖：

- schema init 可重复执行。
- insert/list/update/delete 正常。
- duplicate by `type + value` 命中。
- clear history respects favorite flag。
- clear history deletes image file。
- clear generation prevents low-resource replay from re-inserting stale snapshot。

### Manual Regression Matrix

- 普通模式：复制文本、图片、文件，列表立即出现。
- 普通模式：重复复制同一内容，按 `autoSort` 更新排序。
- 搜索：文本、备注、拼音 search 仍可命中。
- 收藏：清空历史未勾选删除收藏时，收藏记录保留。
- 备注：编辑备注后刷新列表仍保留。
- 图片：删除 image 记录后图片文件被清理。
- 低占用模式：主窗口销毁后复制内容，唤醒后能 replay。
- 低占用模式：唤醒后立即清空历史，不再出现旧 snapshot 回流。
- 跨平台：Windows 优先；macOS/Linux 至少跑 smoke test。

## Rollback Notes（回滚）

- Phase 1/2 保留 `src/database/history.ts` facade，因此可以通过 feature flag 或 dev switch 回退到 Kysely 实现。
- 不改变 SQLite 文件位置和 schema，回滚不需要数据迁移。
- 若 Rust `history_clear` 出现问题，可先只回退清空历史入口到旧 TS 实现。
- 若 `rusqlite` 引入导致构建问题，可先停留在 app-level commands，不移除 `tauri-plugin-sql`。

## Risk Assessment（风险评估）

| Risk | Impact | Mitigation |
| --- | --- | --- |
| Query builder 灵活性丢失 | 中 | Rust API 按 use case 设计，覆盖现有过滤/分页/search |
| 前后端类型不一致 | 中 | TS facade + Rust serde contract + snapshot 测试 |
| SQLite connection 并发问题 | 中高 | Rust 单 owner / Mutex / worker queue，事务包裹批量操作 |
| 图片路径清理差异 | 中 | 保留现有路径规则并写测试 |
| 低占用 replay 竞态未彻底解决 | 高 | clear generation + Rust atomic clear/upsert |
| 全量迁移回归面大 | 高 | 分阶段迁移，每阶段可独立验证和回滚 |

## Estimated Cost（成本预估）

- Phase 1 + Phase 2：约 1.5–3 天，完成 Rust CRUD + 前端 facade 替换。
- Phase 3：约 1–2 天，完成清空历史原子化和低占用 race 修复。
- Phase 4：约 2–4 天，取决于 search/pinyin/clipboard normalization 是否迁到 Rust。
- Phase 5：约 0.5–1 天，清理依赖和文档。

整体如果只解决当前 race，并迁移 history clear/upsert 的核心路径，约 **2–4 天**；如果把所有 history DB 操作完整迁入 Rust 并清理前端 SQL 依赖，约 **5–10 天**，属于中高风险重构。

## Recommendation（建议）

优先执行 Phase 1–3：先建立 Rust DB ownership（所有权）和 atomic clear，用最小迁移解决当前低占用清空历史回流问题。Phase 4/5 等核心路径稳定后再做，避免一次性重构影响剪贴板主流程。
