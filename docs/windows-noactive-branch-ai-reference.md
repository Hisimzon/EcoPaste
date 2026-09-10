# windows-noactive 分支 AI 参考文档

> 生成时间：2026-07-09  
> 当前分支：`windows-noactive`  
> 对比基线：`master..HEAD`  
> 文档用途：给 AI / 开发者快速理解本分支主要开发内容、技术实现、关键文件、注意事项与后续优化方向。

## 1. 分支定位

`windows-noactive` 分支主要围绕 Windows 平台体验优化展开，核心目标是让 EcoPaste 在 Windows 上以“非抢占焦点”的方式展示剪贴板窗口，并进一步支持低占用模式（Low Resource Mode）。

从提交历史看，本分支不是单点功能分支，而是一组 Windows 桌面交互优化合集，主要包含：

- Windows 主窗口不抢占当前应用焦点。
- 通过全局键盘捕获支持窗口可见但不聚焦时的搜索、选择、粘贴、删除等操作。
- 新增低占用模式：窗口不使用时销毁，仅保留后台监听和托盘入口，唤醒时重建窗口。
- 优化剪贴板监听、历史记录回放、托盘菜单状态、自动粘贴兼容性。
- 为中文搜索增加拼音索引，并控制大文本处理阈值，降低性能风险。
- 修复多处 Windows 场景下的焦点、置顶、偏好设置、右键菜单、搜索清空、删除确认框等交互问题。

## 2. 提交历史归类

### 2.1 Windows 非抢占焦点主线

相关提交：

- `d5d87dd feat: 支持 Windows 平台不抢占其他窗口焦点`
- `bcba0ea feat: windows端不抢占窗口焦点可使用快捷键操作待选项`
- `10fd26a feat: windows平台不抢占焦点搜索支持字母、数字、符号`
- `b63701d feat: windows端非抢占焦点下支持中文搜索`
- `c567883 feat: windows粘贴完成后隐藏窗口`
- `963f055 fix: 修复windwos激活时回到顶部`
- `2d210cc fix: 修复windows置顶功能无效`
- `c501bb4 fix: 低占用模式唤醒时主窗口不抢焦点`
- `ce6bdaf fix: 修复 Windows 低占用唤醒与粘贴焦点问题`

主要能力：

- 主窗口显示时设置为不可激活/不可聚焦，避免打断用户正在输入的窗口。
- 后端通过 `rdev::grab` 捕获全局键盘事件，并发送 `dispatch-event`、`search-input` 给前端。
- 前端根据事件完成搜索字符追加、Backspace 删除搜索字符、Esc 隐藏窗口、Enter 粘贴、方向键切换条目等。
- 粘贴前记录并恢复上一个前台窗口，避免内容粘贴到 EcoPaste 自身。
- Windows 下主窗口通过 `WS_EX_NOACTIVATE`、`SetWindowPos(..., SWP_NOACTIVATE, ...)` 避免抢焦点。

### 2.2 低占用模式

相关提交：

- `0f24289 feat: 新增低占用模式`
- `4f9d67c fix: 修复低占用模式清空历史后旧内容回流`
- `ce6bdaf fix: 修复 Windows 低占用唤醒与粘贴焦点问题`
- `c501bb4 fix: 低占用模式唤醒时主窗口不抢焦点`
- `b43447f fix: 修复 EcoPaste 托盘"停止监听"点击后菜单文本不更新的 bug`

主要能力：

- 用户在偏好设置中开启 `lowResourceMode` 后，Windows 下会销毁主窗口和偏好设置窗口。
- 应用进程继续保留，托盘继续存在，剪贴板监听仍在后台运行。
- 当用户按剪贴板快捷键时，后端重新创建主窗口并显示。
- 低占用期间如果剪贴板变化：
  - 后端尽量读取当前剪贴板 snapshot。
  - snapshot 会进入低占用剪贴板队列。
  - 如果无法可靠读取，则标记 dirty flag，窗口恢复后由前端重新读取最新剪贴板。
- 前端窗口挂载时会 drain 低占用队列，按顺序回放历史记录，并刷新列表。
- 清空历史记录时会同步清空低占用 pending state，避免旧队列在下次唤醒时回流。

关键边界：

- 队列最大长度为 `256`，超过会丢弃最旧 snapshot。
- 队列会去重相邻重复 snapshot。
- dirty flag 使用消费式读取，读取后会清零。
- 低占用模式关闭时会取消窗口销毁定时器，并清空 pending clipboard state。

### 2.3 中文搜索与拼音索引

相关提交：

- `a44d81c feat: 文本中的中文字符生成拼音，忽略英文/符号/代码实现中文搜索`
- `8b7d034 fix: 粘贴文本去除拼音内容`
- `7769115 feat: 修复html、rtf格式粘贴携带拼音索引，修复备注可能显示异常`
- `b63701d feat: windows端非抢占焦点下支持中文搜索`
- `b9d4faf feat: 新增大文本处理阈值配置项，避免复制大文本导致进程等待时间过长`

主要能力：

- 历史记录的 `search` 字段会追加拼音索引，格式中包含不可见 marker：
  - `SEARCH_PINYIN_MARKER = "\x1FPINYIN\x1F"`
  - `NOTE_PINYIN_MARKER = "\x1FNOTE\x1F"`
- 拼音索引基于 `pinyin-pro` 生成：
  - 全拼：如 `你好` -> `nihao`
  - 首字母：如 `你好` -> `nh`
- 生成索引前只扫描有限长度文本，避免大文本阻塞。
- 粘贴或写入富文本时会通过 `stripSearchIndex` 去除拼音索引，避免用户粘贴出隐藏搜索内容。
- 备注弹窗也会处理拼音 marker，避免备注展示异常。

大文本阈值：

- 默认：`4096` 字符。
- 最小：`512` 字符。
- 最大：`20000` 字符。
- 配置项通过 `normalizeTextThreshold` 统一规范化。

### 2.4 自动粘贴兼容性

相关提交：

- `4f38bf2 feat: 将自动粘贴按键由 Shift+Insert 调整为 Ctrl+V，提升文件粘贴兼容性`
- `ce6bdaf fix: 修复 Windows 低占用唤醒与粘贴焦点问题`

主要能力：

- Windows 自动粘贴改为模拟 `Ctrl+V`。
- 后端使用 `enigo` 发送键盘事件。
- 粘贴前通过 WinEvent hook 记录上一个前台窗口，并在粘贴时调用 `SetForegroundWindow` 恢复目标窗口。
- 前端在写入剪贴板后先隐藏 EcoPaste，再等待短暂 settle delay，最后调用后端 paste。

注意：

- `Ctrl+V` 对文件粘贴兼容性更好，但目标应用仍可能因安全策略、焦点策略或权限限制拒绝粘贴。
- 低占用唤醒后粘贴尤其依赖“上一个窗口”记录是否准确。

### 2.5 托盘与监听状态

相关提交：

- `b43447f fix: 修复 EcoPaste 托盘"停止监听"点击后菜单文本不更新的 bug`
- `0f24289 feat: 新增低占用模式`

主要能力：

- 托盘 ID 固定为 `app-tray`。
- 前端 `useTray` 会创建或复用托盘图标，避免重复创建。
- 普通模式下，托盘菜单由前端通过 Tauri API 创建。
- 低占用模式下，Windows 后端也会确保托盘存在，并根据配置文件读取：
  - 当前语言。
  - 是否显示菜单栏图标。
  - 监听开关状态。
- “开始监听/停止监听”会直接更新菜单项文本，避免 Windows 上整菜单重建失效。

注意：

- 低占用模式下前端窗口可能已销毁，所以托盘相关行为不能完全依赖前端状态。
- 后端通过读取 `.store.json` / `.store.dev.json` 获取部分设置，这是跨层读取，需要注意字段结构变更风险。

### 2.6 搜索、删除和右键菜单交互修复

相关提交：

- `341dc0c fix: 修复清空搜索后列表未恢复的问题`
- `610457d fix: 搜索模式下esc有确认删除弹窗优先取消弹窗`
- `5768094 feat: 删除弹窗最多提示一次，避免堆积`
- `3b3ab90 fix: 修复了点击右键菜单项卡顿、无法设置备注问题`
- `784b97b feat: windows端esc可关闭备注弹窗以及主窗口`
- `07e961c fix: 修复删除历史记录页面渲染`

主要能力：

- 搜索框清空时会同步清理 `rootState.search`，避免列表无法恢复。
- Windows 下 Esc：
  - 如果存在删除确认框，优先取消确认框。
  - 否则隐藏主窗口。
- 删除确认框限制最多一个，避免重复堆积。
- 输入模式会临时允许主窗口聚焦，避免备注 Modal 输入被全局键盘拦截影响。
- 右键菜单、备注更新、删除历史后选中状态都做了修正。

## 3. 使用到的技术栈

### 3.1 桌面壳与原生层

- `Tauri 2`
  - 跨平台桌面应用框架。
  - 使用 `tauri::Builder` 注册插件、窗口事件、托盘事件、运行事件。
- `Rust 2021`
  - 原生窗口控制、剪贴板监听、全局键盘捕获、托盘低占用逻辑。
- `tauri-plugin-*`
  - `tauri-plugin-sql`：SQLite 数据库。
  - `tauri-plugin-global-shortcut`：全局快捷键。
  - `tauri-plugin-clipboard-x`：剪贴板读写与监听。
  - `tauri-plugin-autostart`：开机自启。
  - `tauri-plugin-log`：日志输出到 stdout、log dir、Webview。
  - `tauri-plugin-updater`：应用更新。
  - `tauri-plugin-fs` / `tauri-plugin-fs-pro`：文件系统访问。
  - `tauri-plugin-opener`：打开 URL / 文件。
  - `tauri-plugin-process`：退出、重启。
  - `tauri-plugin-single-instance`：单实例。
  - `tauri-plugin-prevent-default`：禁用 Webview 默认行为。
- 自定义 Tauri 插件：
  - `tauri-plugin-eco-window`：窗口显示、隐藏、低占用模式、置顶、输入模式。
  - `tauri-plugin-eco-paste`：自动粘贴。
  - `tauri-plugin-eco-autostart`：自启动判断。

### 3.2 Windows 原生 API

使用范围集中在 `src-tauri/src/core/setup/windows.rs`、`src-tauri/src/plugins/window/src/commands/windows.rs`、`src-tauri/src/plugins/paste/src/commands/windows.rs`。

主要 API / 技术点：

- `winapi`
  - `GetForegroundWindow`
  - `GetWindowTextLengthW`
  - `GetWindowTextW`
  - `GetCursorPos`
  - `SetForegroundWindow`
  - `SetWinEventHook`
  - `EVENT_SYSTEM_FOREGROUND`
- `windows` crate
  - `GetWindowLongPtrW`
  - `SetWindowLongPtrW`
  - `SetWindowPos`
  - `WS_EX_NOACTIVATE`
  - `SWP_NOACTIVATE`
  - `HWND_TOPMOST`
- `rdev`
  - `grab`：全局键盘捕获并可拦截事件。
  - `listen`：监听鼠标点击。
  - 用于不聚焦窗口时仍能操作 EcoPaste。
- `enigo`
  - 模拟 `Ctrl+V` 完成自动粘贴。

### 3.3 前端技术栈

- `React 18`
  - 主窗口、偏好设置、列表、弹窗等 UI。
- `Vite 5`
  - 前端构建。
- `TypeScript`
  - 类型约束。
- `Ant Design 5`
  - 偏好设置、输入框、菜单等基础组件。
- `UnoCSS`
  - 原子化样式。
- `Sass`
  - 模块样式和全局样式。
- `Valtio`
  - 全局 store：`globalStore`、`clipboardStore`。
- `ahooks`
  - 生命周期、快捷键、响应式辅助、自定义 hook 基础。
- `react-virtuoso`
  - 历史列表虚拟滚动。
- `Kysely` + `kysely-dialect-tauri`
  - 前端访问 SQLite 的类型安全查询。
- `i18next` / `react-i18next`
  - 多语言。
- `pinyin-pro`
  - 中文拼音索引。
- `DOMPurify`、`react-markdown`、`rehype-raw`、`rtf.js`
  - 富文本 / HTML / RTF 展示与安全处理。

### 3.4 工程工具

- `pnpm`
  - 包管理器，`preinstall` 限制只允许 pnpm。
- `Biome`
  - 格式化和 lint。
- `commitlint`
  - commit message 规范。
- `simple-git-hooks` + `lint-staged`
  - 提交前检查。
- `release-it`
  - 版本发布。
- `tsx`
  - 构建脚本执行，例如 icon / portable 构建。

## 4. 关键文件地图

### 4.1 Rust / Tauri

- `src-tauri/src/lib.rs`
  - Tauri 应用入口。
  - 注册插件。
  - Windows 低占用托盘菜单。
  - 低占用剪贴板监听与 snapshot 捕获。
  - 窗口关闭、退出、托盘点击、菜单事件处理。

- `src-tauri/src/core/setup/windows.rs`
  - Windows 平台初始化。
  - `rdev::grab` 捕获键盘。
  - `rdev::listen` 监听鼠标点击窗口外隐藏。
  - 非聚焦状态下转发搜索字符和快捷操作。
  - 低占用快捷键唤醒主窗口。

- `src-tauri/src/plugins/window/src/commands/mod.rs`
  - 自定义窗口插件公共状态。
  - 低占用模式全局状态。
  - 剪贴板 snapshot queue / dirty flag。
  - 动态创建主窗口、偏好设置窗口。
  - 监听页面加载完成后再显示窗口。

- `src-tauri/src/plugins/window/src/commands/windows.rs`
  - Windows 窗口命令实现。
  - `WS_EX_NOACTIVATE` / `SWP_NOACTIVATE`。
  - show/hide、输入模式、低占用模式、置顶、任务栏图标。

- `src-tauri/src/plugins/paste/src/commands/windows.rs`
  - Windows 自动粘贴实现。
  - 记录上一个前台窗口。
  - 恢复目标窗口焦点。
  - 发送 `Ctrl+V`。

- `src-tauri/src/plugins/window/permissions/default.toml`
  - 自定义窗口插件命令权限。

- `src-tauri/capabilities/default.json`
  - Tauri capabilities 权限配置。

### 4.2 前端

- `src/pages/Main/index.tsx`
  - 主窗口核心逻辑。
  - 注册剪贴板监听、全局快捷键、托盘、低占用模式同步。
  - 消费后端 `dispatch-event`。

- `src/hooks/useClipboard.ts`
  - 剪贴板变化处理。
  - 历史记录插入/更新。
  - 低占用队列回放。
  - 拼音索引写入 `search` 字段。

- `src/plugins/window.ts`
  - 前端调用 `eco-window` 命令的封装。
  - show/hide、低占用、输入模式、同步窗口位置。

- `src/plugins/clipboard.ts`
  - 写入剪贴板、粘贴入口。
  - 粘贴前去除拼音索引。
  - 等待焦点稳定后调用后端 paste。

- `src/pages/Main/components/SearchInput/index.tsx`
  - Windows 下消费 `search-input` 和 `dispatch-event`。
  - Backspace / Escape / allowClear 修复。
  - 非 Windows 下仍走输入框聚焦逻辑。

- `src/utils/pinyin.ts`
  - 拼音索引生成。
  - marker 清理。
  - 兼容旧数据中可能没有 marker 的拼音拼接。

- `src/utils/threshold.ts`
  - 大文本阈值常量与规范化。

- `src/hooks/useTray.ts`
  - 前端托盘创建、菜单、监听开关、语言同步。

- `src/pages/Preference/components/General/index.tsx`
  - 低占用模式配置入口。

- `src/pages/Preference/components/Clipboard/index.tsx`
  - 搜索框配置和大文本阈值配置。

- `src/pages/Preference/components/History/components/Delete/index.tsx`
  - 删除历史时清理低占用 pending clipboard state。

## 5. 关键数据流

### 5.1 普通剪贴板监听

1. 前端 `useClipboard` 调用 `startListening()`。
2. `tauri-plugin-clipboard-x` 监听剪贴板变化。
3. 前端 `onClipboardChange(result)` 收到剪贴板内容。
4. `processClipboardChange` 识别类型：
   - files
   - html
   - rtf
   - text
   - image
5. 文本类内容生成 subtype 和拼音搜索索引。
6. 通过 Kysely 写入 SQLite `history` 表。
7. 当前分组可见时同步更新 `state.list`。

### 5.2 低占用剪贴板监听

1. 用户开启 `globalStore.app.lowResourceMode`。
2. 前端保存 store，并调用 `setLowResourceMode(true, shortcut)`。
3. Windows 后端设置 `LOW_RESOURCE_MODE = true`。
4. 后端销毁主窗口和偏好设置窗口。
5. 剪贴板变化时：
   - 如果主窗口不存在，后端读取 clipboard snapshot。
   - 读取成功则 push 到 `LOW_RESOURCE_CLIPBOARD_QUEUE`。
   - 读取失败则设置 `LOW_RESOURCE_CLIPBOARD_DIRTY = true`。
6. 用户按剪贴板快捷键。
7. 后端匹配快捷键，调用 `show_main_window`。
8. 窗口重建并加载完成后显示。
9. 前端 `useClipboard` 挂载：
   - `drainLowResourceClipboardQueue()`
   - `consumeLowResourceClipboardDirty()`
   - 必要时 `readClipboard()`
   - 回放数据并刷新列表。

### 5.3 Windows 非聚焦搜索与快捷操作

1. 主窗口显示但不激活。
2. 用户键盘输入仍属于原前台应用。
3. 后端 `rdev::grab` 捕获事件。
4. 如果是 EcoPaste 操作快捷键：
   - 转换成 `dispatch-event`。
   - 前端处理选择、粘贴、删除、隐藏等。
5. 如果是可打印字符：
   - 后端转换成字符。
   - 发送 `search-input`。
   - 前端追加到搜索框状态。
6. Ctrl / Alt 组合键不转成搜索字符，避免污染搜索条件。

### 5.4 自动粘贴

1. 用户选择历史条目并触发粘贴。
2. 前端根据配置写入剪贴板：
   - 原格式写入：text / rtf / html / image / files。
   - 纯文本粘贴：写入 stripped search text。
3. 前端隐藏主窗口。
4. 等待 `PASTE_FOCUS_SETTLE_DELAY = 120ms`。
5. 后端恢复上一个前台窗口。
6. 后端模拟 `Ctrl+V`。

## 6. 配置与状态说明

### 6.1 Store

相关类型在 `src/types/store.d.ts`。

重要字段：

- `globalStore.app.lowResourceMode`
  - 是否启用低占用模式。
- `globalStore.app.showMenubarIcon`
  - 是否显示托盘图标。
- `globalStore.shortcut.clipboard`
  - 剪贴板窗口快捷键，也是低占用唤醒快捷键。
- `clipboardStore.content.textThreshold`
  - 大文本处理阈值。
- `clipboardStore.search.autoClear`
  - 激活/失焦后是否自动清空搜索框。
- `clipboardStore.search.defaultFocus`
  - 非 Windows 平台是否默认聚焦搜索框。
- `clipboardStore.window.position`
  - 主窗口位置策略：`remember` / `follow` / `center`。
- `clipboardStore.window.backTop`
  - 激活时是否回到顶部。

### 6.2 后端全局状态

低占用 / 窗口状态：

- `LOW_RESOURCE_MODE`
- `LOW_RESOURCE_CLIPBOARD_DIRTY`
- `LOW_RESOURCE_CLIPBOARD_QUEUE`
- `LOW_RESOURCE_CLIPBOARD_SHORTCUT`
- `MAIN_WINDOW_VISIBLE`
- `INPUT_MODE`
- `PINNED`
- `MAIN_WINDOW_DESTROY_SEQ`

窗口生命周期辅助：

- `JUST_CREATED_WINDOW_LABELS`
- `PENDING_SHOW_WINDOW_LABELS`
- `PAGE_LOADED_WINDOW_LABELS`

这些状态大多是进程内状态，应用重启后不会保留。

## 7. 注意事项与风险

### 7.1 Windows 焦点风险

- 非抢占焦点依赖 Windows 原生窗口样式，不同 Windows 版本、不同 WebView2 行为可能存在差异。
- `WS_EX_NOACTIVATE` 会导致输入框天然不能直接输入，因此 Windows 搜索依赖 `rdev` 全局捕获。
- Modal / 备注输入必须进入 `INPUT_MODE`，否则键盘输入可能被后端拦截。
- 如果改动 `enter_input_mode` / `exit_input_mode`，需要验证备注弹窗、删除确认框、搜索框、粘贴快捷键。

### 7.2 全局键盘捕获风险

- `rdev::grab` 是全局拦截，逻辑错误会影响用户当前应用输入。
- Ctrl / Alt 组合键过滤很重要，避免快捷键被当成搜索字符。
- modifier 状态可能漂移，本分支已在低占用唤醒后 reset modifier state。
- 需要特别验证：
  - Alt+C 唤醒。
  - Ctrl+W 隐藏。
  - Ctrl+P 置顶。
  - Ctrl+, 打开偏好设置。
  - Backspace 删除搜索字符。
  - Delete 删除条目。
  - Esc 优先关闭确认框，再隐藏窗口。

### 7.3 低占用模式风险

- 窗口会被 destroy，不是普通 hide，因此前端内存状态会丢失。
- 所有必须跨窗口重建保留的状态都应进入 store、数据库或后端队列。
- 后端读取 `.store.json` 是为了低占用时没有前端也能构造托盘菜单，但这对 store schema 有耦合。
- 低占用期间 clipboard snapshot 可能读取失败，此时只能用 dirty flag 在唤醒后读取最新剪贴板，无法恢复中间多次变化。
- 队列上限 `256`，高频复制场景会丢弃更旧记录。
- 清空历史必须调用 `clearLowResourceClipboardState`，否则旧队列可能再次写回。

### 7.4 剪贴板与历史记录风险

- `search` 字段混合了原始搜索文本和拼音索引，读取/粘贴时必须 strip marker。
- HTML / RTF 写入剪贴板时必须传入纯文本版本，不能把拼音索引带出去。
- 文件类型 `value` 在数据库中是 JSON 字符串，在前端状态中是数组，修改时要注意转换。
- 图片类型会把路径转换成 full path。
- 重复内容会更新 `createTime`，是否移动到顶部由 `clipboardStore.content.autoSort` 控制。

### 7.5 托盘风险

- 普通模式托盘主要由前端管理，低占用模式托盘需要后端兜底。
- 不要重复创建同 ID 托盘；应优先 `TrayIcon.getById("app-tray")` 或后端 `tray_by_id`。
- Windows 上更新托盘菜单可能不稳定，本分支通过直接 `setText` 或后端重建菜单规避。
- 退出/重启时低占用模式会阻止默认退出，需要通过 `allow-app-exit` 或后端 `ALLOW_LOW_RESOURCE_EXIT_REQUEST` 放行。

### 7.6 性能风险

- 大文本拼音生成可能阻塞，所以必须使用 `normalizeTextThreshold`。
- `stripSearchIndex` 针对无 marker 的旧数据有 split + 比对逻辑，大文本会提前返回避免高开销。
- 低占用队列回放时是顺序处理，极端队列长度下可能造成窗口初次显示后的列表刷新延迟。

### 7.7 跨平台风险

- 本分支大量逻辑是 Windows 特化。
- macOS / Linux 的 `set_low_resource_mode` 当前更多是记录状态或走常规路径，不具备 Windows 完整低占用销毁/唤醒能力。
- 前端搜索框逻辑明确区分：
  - Windows：后端捕获输入。
  - 非 Windows：真实输入框聚焦。
- 修改共享 API 时必须检查 `macos.rs`、`linux.rs`、`windows.rs` 三个平台命令签名一致。

## 8. AI 优化参考建议

后续让 AI 基于此分支继续优化时，建议按以下优先级理解和验证：

1. 先读 `src-tauri/src/plugins/window/src/commands/mod.rs`，理解窗口重建、低占用队列和全局状态。
2. 再读 `src-tauri/src/plugins/window/src/commands/windows.rs`，理解 Windows 非激活显示。
3. 再读 `src-tauri/src/core/setup/windows.rs`，理解键盘/鼠标全局捕获。
4. 再读 `src/pages/Main/index.tsx` 和 `src/pages/Main/components/SearchInput/index.tsx`，理解后端事件如何落到前端交互。
5. 再读 `src/hooks/useClipboard.ts`、`src/plugins/clipboard.ts`、`src/utils/pinyin.ts`，理解历史写入、拼音索引和粘贴清理。
6. 最后读 `src/hooks/useTray.ts` 和 `src-tauri/src/lib.rs` 中 Windows tray 相关逻辑，理解普通模式和低占用模式的托盘差异。

适合优化的方向：

- 抽象低占用剪贴板 snapshot 的类型，减少 `serde_json::Value` 跨层隐式约定。
- 为 `parse_shortcut` / `key_to_char` 增加 Rust 单元测试，降低快捷键解析回归风险。
- 为 `appendPinyinToSearch` / `stripSearchIndex` 增加 TypeScript 单元测试，覆盖 marker、旧数据、大文本阈值、HTML/RTF 粘贴。
- 把后端读取 `.store.json` 的逻辑集中封装，避免语言、托盘显示等字段路径散落。
- 对低占用队列回放增加可观测日志，方便排查“丢记录”或“旧内容回流”。
- 明确 macOS / Linux 低占用模式的产品语义，避免 UI 开关在非 Windows 平台表现不一致。
- 对 Windows 全局键盘捕获增加失败 fallback 提示，因为 `rdev::grab` 失败后非聚焦搜索会不可用。

## 9. 建议验证清单

### 9.1 基础构建检查

```powershell
cd D:\KaiFaRuanJian\RustSource\EcoPaste
pnpm lint
pnpm build:vite
cargo check --manifest-path src-tauri/Cargo.toml
```

### 9.2 Windows 手动回归

普通模式：

- 打开 EcoPaste 主窗口，不应抢占当前应用焦点。
- 输入字母、数字、符号，搜索框应更新，原应用不应收到这些字符。
- 按 Backspace，搜索框应删除一个字符。
- 清空搜索框后，列表应恢复。
- 按方向键切换条目。
- 按 Enter 粘贴到原窗口。
- 粘贴后 EcoPaste 应隐藏。
- 复制文件后从 EcoPaste 粘贴，目标应用应收到文件。
- 打开备注 Modal 后应能正常输入中文。
- 删除确认框存在时按 Esc，应先取消确认框，不应直接隐藏主窗口。

低占用模式：

- 开启低占用模式后，主窗口/偏好设置窗口应被销毁，托盘仍存在。
- 复制文本/图片/文件后，按剪贴板快捷键唤醒，历史应回放新增记录。
- 高频复制超过多次后，唤醒不应出现重复相邻记录。
- 清空历史后再唤醒，不应出现旧内容回流。
- 点击托盘“停止监听/开始监听”，菜单文本应立即变化。
- 停止监听时复制内容，唤醒后不应新增该内容。
- 退出/重启应用应能正常结束进程，不应被低占用模式拦截。

拼音搜索：

- 复制中文文本后，可以用全拼搜索。
- 可以用首字母搜索。
- 英文/符号/代码片段不应被错误扩展成大量拼音索引。
- 粘贴文本/HTML/RTF 时，不应带出 `PINYIN` marker 或拼音索引。

### 9.3 重点命令

查看本分支相对 master 的提交：

```powershell
cd D:\KaiFaRuanJian\RustSource\EcoPaste
git log --oneline --decorate master..HEAD
```

查看本分支影响文件：

```powershell
cd D:\KaiFaRuanJian\RustSource\EcoPaste
git diff --stat master..HEAD
```

查看当前工作区改动：

```powershell
cd D:\KaiFaRuanJian\RustSource\EcoPaste
git status --short
```

## 10. 不变量（Invariants）

后续优化时尽量保持这些不变量：

- Windows 主窗口显示不应抢占用户当前工作窗口焦点。
- Windows 主窗口非聚焦时仍应可搜索、选择、粘贴、隐藏。
- 粘贴目标应是用户原本所在窗口，而不是 EcoPaste 主窗口。
- 低占用模式下关闭窗口不等于退出应用。
- 低占用模式下托盘必须可用，至少支持唤醒、偏好设置、监听开关、退出/重启。
- 剪贴板历史清空后，低占用队列和 dirty flag 必须同步清空。
- 拼音索引只能用于搜索，不能被粘贴给用户。
- 大文本处理必须受阈值限制。
- macOS / Linux 的命令签名必须与 Windows 保持兼容，避免前端 invoke 失败。

