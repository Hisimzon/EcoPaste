# Windows 非激活窗口搜索输入改造计划

## Goal

在当前 `my-next` 架构上完善 Windows 剪贴板窗口的 no-activate（不抢占焦点）体验：

- 显示剪贴板窗口时，用户原应用继续保持前台焦点。
- 窗口不可聚焦时，字母、数字和常用符号直接进入 EcoPaste 搜索。
- 被 EcoPaste 消费的按键不再泄漏给原应用。
- 导航、预览、粘贴、隐藏、临时 IME 编辑模式保持现有行为。

本计划参考：

- `docs/windows-noactive-branch-ai-reference.md`
- `D:\KaiFaRuanJian\RustSource\EcoPaste` 的 `windows-noactive` 分支
- 重点历史提交：`d5d87dd`、`bcba0ea`、`10fd26a`、`c501bb4`、`ce6bdaf`

旧分支只作为 behavior reference（行为参考），不直接复制旧 `plugins/window`、`rdev` 或旧前端 store 架构。

## Current State

当前 `my-next` 已具备：

- `src-tauri/src/window/windows.rs`
  - 剪贴板窗口显示前调用 `set_focusable(false)`。
  - 编辑输入框时临时恢复 focusable，并记录/恢复原前台窗口。
  - 隐藏窗口时停用键盘和鼠标钩子。
- `src-tauri/src/keyboard/windows.rs`
  - 使用 `WH_KEYBOARD_LL` 捕获导航键、空格预览和部分 Ctrl 快捷键。
  - 同时吞掉被消费按键的 keydown/keyup，避免孤立 keyup 泄漏。
- `src/hooks/useKeyboardEvent.ts`
  - 将 `keyboard://nav` 转换为前端 `KeyboardEvent`。
  - 输入控件激活时保留浏览器原生键盘和 IME 行为。
- `src/hooks/useClipboardWindowEditableFocus.ts`
  - 输入控件编辑期间临时让窗口获取焦点，blur 后恢复 no-activate。
- `src-tauri/src/commands/clipboard.rs`
  - 粘贴前写回剪贴板并隐藏非固定窗口，再模拟 `Ctrl+V`。

当前主要缺口：

- 普通字母、数字和符号不在 Rust hook 的消费范围内。
- 这些按键不会进入搜索框，且会继续发送给保持焦点的原应用。
- 搜索框当前是非受控输入，Header 只接收防抖后的 `ChangeEvent`，缺少接收原生 hook 文本的稳定接口。

## Scope

### Included

- Windows 不可聚焦状态下的 printable key（可打印字符）捕获、翻译、事件发送和吞键。
- 搜索输入组件接收 native keyboard text event。
- Backspace 删除搜索字符的行为。
- Rust 纯逻辑测试和前端最小行为检查。
- Windows 手动焦点回归。

### Excluded

- 低占用模式、窗口销毁/重建和剪贴板回放队列。
- 拼音索引生成与中文内容搜索增强。
- 托盘监听状态、自动启动、清空历史回流修复。
- macOS 行为调整。
- 引入 `rdev` 或新增第三方依赖。
- 重构整个键盘事件系统。

中文 IME 输入继续使用现有 `Ctrl+F`/点击搜索框后的临时可聚焦模式；本轮非聚焦直输只承诺当前键盘布局可解析的普通文本输入。

## Invariants

- Windows 剪贴板窗口普通显示不得调用 `set_focus()`。
- 原应用保持前台焦点，粘贴目标仍是原应用。
- EcoPaste 消费的 keydown 和 keyup 必须成对吞掉。
- Ctrl/Alt/Win 系统快捷键不得被误当成搜索文本吞掉。
- 空格继续用于按住预览，不作为搜索文本。
- 输入框真实聚焦时停止 Rust 导航/文本钩子，IME 与原生编辑行为不重复触发。
- `keyboard://nav` 保持现有 payload 兼容；新增事件名必须在 Rust 与 `src/constants/events.ts` 同步维护。
- macOS 不新增条件分支或行为变化。
- 不覆盖当前 dirty worktree：`src-tauri/Cargo.lock` 删除和 `docs/windows-noactive-branch-ai-reference.md` 未跟踪状态必须保留。

## Design

### 1. Native key translation

扩展 `src-tauri/src/keyboard/windows.rs` 的现有 `WH_KEYBOARD_LL` hook，不新增第二套 hook。

- 使用 Windows 原生键盘布局 API 将 `vkCode` 转为文本，优先复用当前 `winapi` 依赖。
- 翻译时读取 Shift/Caps Lock 和当前 foreground thread keyboard layout。
- Ctrl、普通 Alt、Win 修饰期间不产生搜索文本；避免吞掉应用/系统快捷键。
- dead key 返回值不直接发送，并清理必要的转换状态，避免污染下一次翻译。
- 空格、导航键、预览键和已支持的 Ctrl 快捷键继续走现有分支，保持优先级。
- printable keydown 发送文本事件；对应 keyup 通过 `consumed_keys` 吞掉。
- 自动重复 keydown 应重复发送字符，但只需保持一个 consumed VK 记录。

新增事件契约：

```text
keyboard://search-input
payload: { text: string } | { action: "backspace" }
```

如实现阶段发现统一 payload 更简单，可使用：

```text
payload: { action: "append", text: string } | { action: "backspace" }
```

事件字符串在以下位置同步：

- `src-tauri/src/keyboard/mod.rs`
- `src/constants/events.ts`

### 2. Search input state

调整 `src/pages/Clipboard/components/SearchInput.tsx`，让组件持有即时显示值，同时保持 Header/Valtio 为查询状态 owner：

- native `onChange`：立即更新组件本地值；非 composition 状态通知父级。
- `compositionend`：补发最终值，维持现有 IME 语义。
- `keyboard://search-input` append：追加文本并立即通知父级。
- backspace：按 Unicode code point 删除最后一个字符并通知父级。
- `clearToken`：清空本地值，不再依赖仅 remount DOM 的隐式行为。
- 输入框真实聚焦时 Rust hook 已禁用，不会同时收到 native text event。

将 Header 的回调从 `ChangeEvent<HTMLInputElement>` 收窄为 `string`：

- `src/pages/Clipboard/components/Header.tsx` 继续 debounce 写入 `clipboardViewState.keyword`。
- `clearSearch()` 同时清理 pending debounce、Valtio keyword 和 SearchInput 本地值。
- 不在 Valtio 中增加第二份“输入框即时值”。

### 3. Backspace behavior

- 搜索文本非空：Backspace 删除最后一个字符并被吞掉。
- 搜索文本为空：首期不触发记录删除；删除记录继续使用 Delete 或现有明确快捷键，避免 Tauri 广播事件被多个组件同时处理。
- 若产品必须保留“空搜索 Backspace 删除记录”，后续单独增加由页面 owner 决策的 keyboard action router，不在多个 listener 中竞争处理。

## Files To Touch

预计最小写集：

- `src-tauri/src/keyboard/mod.rs`
- `src-tauri/src/keyboard/windows.rs`
- `src/constants/events.ts`
- `src/pages/Clipboard/components/SearchInput.tsx`
- `src/pages/Clipboard/components/Header.tsx`

可能新增测试文件或测试模块：

- `src-tauri/src/keyboard/windows.rs` 内 `#[cfg(test)] mod tests`

不计划修改：

- `src-tauri/src/window/windows.rs`
- `src-tauri/src/commands/clipboard.rs`
- `src-tauri/src/keystroke/windows.rs`

除非实现验证证明现有窗口/粘贴不变量不成立。

## Implementation Checklist

- [ ] 给 keyboard 模块增加集中维护的 search-input event 常量。
- [ ] 提取可测试的 modifier 判定和 key translation 边界逻辑。
- [ ] 在现有 hook 分支中加入 printable key 处理，保持 Ctrl/nav/preview 优先级。
- [ ] printable keydown emit 后记录 consumed VK；keyup 配对吞掉。
- [ ] Backspace 发送 search-input action 并配对吞键。
- [ ] 扩展 TypeScript `TAURI_EVENT` 常量。
- [ ] SearchInput 增加本地即时值和 native event listener。
- [ ] 保持 composition start/end、focus token、blur token 行为。
- [ ] Header 改为 debounce string，不复制搜索状态。
- [ ] 检查窗口隐藏/编辑模式切换时无残留 consumed key。
- [ ] `rg` 检查事件名不存在 raw string 漂移。

## Automated Validation

### Rust focused checks

为不依赖真实全局 hook 的纯逻辑增加测试：

- 无 Ctrl/Alt/Win 时允许 printable translation。
- Ctrl/Alt/Win 快捷键不进入搜索文本路径。
- Shift/数字/常用 OEM 符号转换边界。
- Space 不进入搜索文本路径。
- Backspace action 与普通 printable action 分离。

命令：

```powershell
cd D:\KaiFaRuanJian\RustSource\EcoPaste-next\src-tauri
cargo fmt --check
cargo test keyboard
cargo clippy -- -D warnings
```

### Frontend checks

```powershell
cd D:\KaiFaRuanJian\RustSource\EcoPaste-next
pnpm lint
pnpm tsc
pnpm build:vite
```

### Current blocker

当前 Windows 环境拒绝创建任何 `Cargo.lock`，包括临时 worktree 中的同名文件，报错：

```text
Access to the path '...\src-tauri\Cargo.lock' is denied.
```

在安全软件/系统策略解除前，Rust build/test 可能无法执行。实现会先做前端检查与静态 Rust review；完整 Rust gate 必须在 `Cargo.lock` 可创建后补跑。

## Manual Windows Regression

普通 no-activate 模式：

- [ ] 在记事本/浏览器输入框保持光标，打开 EcoPaste，原窗口仍是 foreground。
- [ ] 输入 `abc123-_=.,/`，EcoPaste 搜索更新，原输入框不出现这些字符。
- [ ] 按住字符键产生正常重复输入，原应用不收到 keydown/keyup。
- [ ] Backspace 每次删除一个搜索字符。
- [ ] ArrowUp/ArrowDown、Tab、Enter、Escape、Space 行为不回归。
- [ ] Ctrl+C、Ctrl+V、Alt+Tab、Win 键等系统/应用快捷键不被错误吞掉。
- [ ] Enter 粘贴后窗口隐藏，内容进入原应用。
- [ ] 固定窗口下粘贴仍进入原应用。

编辑/IME 模式：

- [ ] 点击搜索框或按 Ctrl+F 后窗口临时可聚焦。
- [ ] 中文 IME composition 正常，不重复字符。
- [ ] 输入框 blur 后恢复 no-activate，并把 foreground 还给之前窗口。
- [ ] 备注、分组名称等其他 editable 控件输入不被全局 hook 干扰。

边界：

- [ ] 隐藏窗口后键盘输入完全回到原应用。
- [ ] 快速显示/隐藏不会残留 Ctrl 或 consumed key 状态。
- [ ] 切换不同键盘布局时不会崩溃；无法翻译的键应放行而非生成乱码。

## Risks

- Windows keyboard layout 与 dead key 状态复杂，错误调用 `ToUnicodeEx` 可能影响后续输入。
- Tauri event 是异步的；快速输入必须保证顺序稳定，不能在前端 debounce 之前丢字符。
- SearchInput 从纯 DOM 非受控改为组件本地 state 后，IME composition 是主要 regression risk。
- 全局 hook 回调必须保持短小，不能做阻塞操作或持有锁后 emit 大量工作。
- 键盘 hook 安装失败时当前代码仅写日志；本轮不新增用户提示，但手测必须覆盖失败可观察性。

## Rollback

若 printable-key 路径出现输入法、快捷键或稳定性问题：

1. 保留当前 `keyboard://nav` 导航实现。
2. 回滚 `keyboard://search-input` emit 和前端 listener。
3. SearchInput 恢复现有非受控实现。
4. 用户仍可通过 Ctrl+F/点击搜索框进入临时聚焦模式搜索，不影响基础可用性。

该回滚不需要数据库、设置或 migration 操作。
