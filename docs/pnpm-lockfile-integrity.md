# pnpm 依赖与锁文件完整性规范

本文档约束 `EcoPaste-next` 中 `package.json` 与 `pnpm-lock.yaml` 的修改、
上游同步后的处理和本地验证流程。目标是让全新 Windows 构建环境能够稳定执行：

```powershell
pnpm install --frozen-lockfile
```

本文不要求自动打包、不创建 Release，也不负责自动升级依赖版本。

## 适用范围

- 直接依赖或开发依赖的新增、删除、升级、降级与版本范围调整。
- `next` 同步上游后，`my-next` 的 rebase 或 merge。
- `package.json` 或 `pnpm-lock.yaml` 出现冲突，或被手工改动后的恢复。

项目当前通过 `package.json` 固定包管理器版本为 `pnpm@10.33.1`。处理锁文件时
应使用该版本，避免不同 pnpm 版本生成不必要的 diff。

## 核心不变量

`pnpm-lock.yaml` 不只是一个直接依赖版本列表。对每个依赖，以下三层必须一致：

```text
package.json 的版本范围
        ↓ 由 pnpm 解析
pnpm-lock.yaml / importers 中的直接版本
        ↓ 必须存在对应记录
pnpm-lock.yaml / packages 与 snapshots 中的包快照
```

例如，若 `package.json` 声明：

```json
"@tauri-apps/plugin-log": "^2.9.0"
```

锁文件的 importer、`packages` 和 `snapshots` 都必须指向
`@tauri-apps/plugin-log@2.9.0`。只修改前两层而保留其他版本的快照，会造成：

```text
ERR_PNPM_LOCKFILE_MISSING_DEPENDENCY
```

`--frozen-lockfile` 的失败是保护机制：它拒绝在 CI 中猜测或修复锁文件。

## 修改依赖的标准流程

### 使用 pnpm 修改直接依赖

不要手工编辑 `pnpm-lock.yaml`。直接依赖需要变更时，优先让 pnpm 同时更新
清单与完整锁文件。

例如，将 Tauri log 插件同步到上游的 `2.9.x`：

```powershell
pnpm add '@tauri-apps/plugin-log@^2.9.0' --ignore-scripts
```

如果已经有意修改了 `package.json`，用完整安装重新解析锁文件：

```powershell
pnpm install --no-frozen-lockfile --ignore-scripts
```

`--ignore-scripts` 用于避免在仅修复依赖元数据时运行项目生命周期脚本。若变更
本身依赖安装脚本，按该依赖的要求另行处理。

不要依赖下面的命令修复损坏锁文件：

```powershell
pnpm install --lockfile-only --no-frozen-lockfile
```

它可能不会遍历或补齐损坏锁文件中缺失的 package snapshot。本项目的
`plugin-log@2.8.0` 事件已验证：完整的非冻结安装可以修复，单独的
`--lockfile-only` 不能作为修复或验证依据。

### 提交前验证

在依赖清单或锁文件发生变更后，提交前执行：

```powershell
pnpm install --frozen-lockfile --ignore-scripts
git diff --check -- package.json pnpm-lock.yaml
git diff -- package.json pnpm-lock.yaml
```

第一条应成功，才表示 GitHub 的全新环境能够使用锁文件完成依赖解析。

暂存后还应确认依赖清单与锁文件没有被拆到不同提交中：

```powershell
git diff --cached --name-only
```

如果本次改动包含 `package.json` 的依赖版本变化，正常情况下也应包含
`pnpm-lock.yaml`。锁文件单独变化时，应确认它是有意的完整重解析结果。

## 上游同步、rebase 与冲突处理

上游依赖升级和本地依赖固定同时存在时，最容易破坏锁文件。完成
`my-next` 的 rebase 或 merge 后，若 `package.json` 或 `pnpm-lock.yaml` 被冲突处理、
被重放提交改动，必须执行以下收尾步骤：

1. 先确定 `package.json` 中最终想要的版本范围。
2. 让 `pnpm-lock.yaml` 恢复为可解析的 YAML；不要手工拼接 package 或 snapshot
   块，也不要只保留 importer 的版本。
3. 使用 pnpm 重建完整锁文件：

   ```powershell
   pnpm install --no-frozen-lockfile --ignore-scripts
   ```

4. 使用冻结模式验证：

   ```powershell
   pnpm install --frozen-lockfile --ignore-scripts
   ```

5. 审查 `package.json` 与 `pnpm-lock.yaml` 的 diff，再继续 rebase/merge 和推送。

完整的上游分支同步步骤见 [git-upstream-workflow.md](./git-upstream-workflow.md)。

## 失败恢复

当 CI 或本地出现如下错误时：

```text
ERR_PNPM_LOCKFILE_MISSING_DEPENDENCY
Broken lockfile: no entry for '<package>@<version>' in pnpm-lock.yaml
```

按以下顺序恢复：

```powershell
pnpm install --no-frozen-lockfile --ignore-scripts
pnpm install --frozen-lockfile --ignore-scripts
git diff --check -- pnpm-lock.yaml
git diff -- pnpm-lock.yaml
```

第二条命令必须通过。若 diff 包含与本次版本变更无关的大规模升级，不要直接
提交；先确认 pnpm 版本、registry 配置和 `package.json` 是否与预期一致。

## 本次事件的教训

上游提交 `5139d30b` 曾将 `@tauri-apps/plugin-log` 更新为 `2.9.0`。后续本地提交
将 `package.json` 和 lockfile importer 改为 `~2.8.0` / `2.8.0`，但没有替换
`packages` 与 `snapshots` 中的 `2.9.0` 记录。结果是在手动 Windows 构建执行
冻结安装时才暴露错误。

`465857d6 fix(deps): repair pnpm lockfile` 已把这三层重新对齐为 `2.8.0`。
随后应恢复并跟随上游的 `^2.9.0` 范围，当前 lockfile 解析为精确的 `2.9.0`。
以后应把“非冻结重解析 + 冻结验证”视为版本调整和上游同步后的固定收尾步骤；
不要为了绕过锁文件错误而长期偏离上游已确认的依赖版本。

## 可选的远端防线

当前手动 Windows 构建工作流已经会在打包前运行 `pnpm install --frozen-lockfile`，
因此能阻止损坏锁文件进入构建阶段；但发现时机较晚。

若需要更早发现，可以后续增加一个独立的 Windows 依赖完整性工作流，仅在
`my-next` 的 `package.json` 或 `pnpm-lock.yaml` 变化时运行：

```powershell
pnpm install --frozen-lockfile
```

该工作流不执行 `pnpm tauri build`，不创建 tag 或 Release。它是可选防线，是否
启用应单独决定。
