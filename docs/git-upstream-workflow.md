# Git 上游同步与本地定制分支规范

本文档用于维护 `EcoPaste-next` 这类场景：代码长期跟随官方上游 `master` 分支，但本地又需要保留自己的定制改动。

## 分支与 Remote 约定

Remote 角色：

- `upstream`: 官方上游仓库，只用于同步代码。
- `origin`: 自己的 fork 或当前克隆来源，可用于推送个人分支。

当前推荐 remote：

```powershell
git remote add upstream https://github.com/EcoPasteHub/EcoPaste.git
git remote set-url upstream https://github.com/EcoPasteHub/EcoPaste.git
```

分支角色：

- `next`: 上游基线分支，必须跟踪 `upstream/master`，只同步上游，不放本地定制提交。
- `my-next`: 本地长期定制分支，从最新 `next` 派生，所有个人改动都放这里。
- `feature/<name>`: 可选，单个功能开发分支，从 `my-next` 派生。

推荐状态：

```text
upstream/master -> 官方最新代码
next            -> 本地上游基线，跟踪 upstream/master
my-next        -> 本地定制分支
feature/xxx    -> 具体功能分支，可选
```

## 核心原则

- 不在 `next` 上提交本地改动。`next` 只做 upstream mirror（上游镜像）。
- 同步前先检查 dirty worktree（未提交工作区），避免切分支时覆盖改动。
- 更新 `next` 只用 fast-forward，命令使用 `git merge --ff-only upstream/master`。
- `my-next` 是你的工作分支，可通过 `rebase next` 或 `merge next` 接收上游更新。
- 私人分支优先用 `rebase` 保持历史线性；多人共享分支优先用 `merge` 保留协作历史。

## 第一次配置

```powershell
cd D:\KaiFaRuanJian\RustSource\EcoPaste-next

git remote add upstream https://github.com/EcoPasteHub/EcoPaste.git
git fetch upstream

git switch next
git branch --set-upstream-to=upstream/master next
git merge --ff-only upstream/master

git switch -c my-next
```

如果 `upstream` 已存在：

```powershell
git remote set-url upstream https://github.com/EcoPasteHub/EcoPaste.git
git fetch upstream
```

## 日常同步流程

### 1. 同步前检查

```powershell
git status --short --branch
git remote -v
git branch -vv --list next my-next
```

如果工作区有未提交改动，先 commit 或 stash。

临时保存改动：

```powershell
git stash push -u -m "preserve local edits before syncing upstream master"
```

`-u` 会同时保存 untracked files（未跟踪文件）。如果确认只需要保存已跟踪文件，可以去掉 `-u`。

### 2. 更新本地 `next`

```powershell
git switch next
git fetch upstream
git merge --ff-only upstream/master
```

如果这里失败，说明本地 `next` 上有额外提交，不再是纯上游基线。先查看：

```powershell
git log --oneline --decorate --graph next --not upstream/master
```

不要直接 `reset --hard`，除非确认这些提交可以丢弃。

### 3. 更新 `my-next`

先切回本地定制分支：

```powershell
git switch my-next
```

查看 `my-next` 与 `next` 的差异：

```powershell
git rev-list --left-right --count my-next...next
```

输出含义：

```text
左边数字 = my-next 比 next 多出的提交数
右边数字 = next 比 my-next 多出的提交数
```

如果 `my-next` 没有自己的提交，只是落后 `next`：

```powershell
git merge --ff-only next
```

如果 `my-next` 有自己的本地提交，并且只在本机使用，推荐 rebase：

```powershell
git rebase next
```

如果 `my-next` 已经推送给别人共同使用，推荐 merge：

```powershell
git merge next
```

## 恢复 Stash

查看 stash：

```powershell
git stash list
```

恢复最近一次 stash：

```powershell
git stash apply 'stash@{0}'
```

PowerShell 下 `stash@{0}` 要加引号，否则会被 shell 解析。

确认恢复无误后删除该 stash：

```powershell
git stash drop 'stash@{0}'
```

如果恢复时有冲突，先不要 drop。处理完冲突并确认改动完整后，再删除 stash。

## 冲突处理

Rebase 冲突时：

```powershell
git status
```

手动编辑冲突文件，确认内容后：

```powershell
git add <conflicted-file>
git rebase --continue
```

如果决定放弃本次 rebase：

```powershell
git rebase --abort
```

Merge 冲突时：

```powershell
git status
```

手动编辑冲突文件，确认内容后：

```powershell
git add <conflicted-file>
git commit
```

如果决定放弃本次 merge：

```powershell
git merge --abort
```

## 验证同步结果

同步完成后至少运行：

```powershell
git status --short --branch
git branch -vv --list next my-next
git rev-list --left-right --count next...upstream/master
git rev-list --left-right --count my-next...next
```

理想结果：

```text
next...upstream/master = 0 0
```

如果 `my-next` 已经更新到最新基线：

```text
my-next...next = 0 0
```

如果 `my-next` 有自己的定制提交，也可能是：

```text
my-next...next = N 0
```

这表示 `my-next` 比 `next` 多 `N` 个本地定制提交，是正常状态。

## 常用完整命令模板

适合日常同步：

```powershell
cd D:\KaiFaRuanJian\RustSource\EcoPaste-next

git status --short --branch
git stash push -u -m "preserve local edits before syncing upstream master"

git switch next
git fetch upstream
git merge --ff-only upstream/master

git switch my-next
git rebase next

git stash apply 'stash@{0}'
git status --short --branch
```

如果 `my-next` 没有本地提交，也可以把 `git rebase next` 换成：

```powershell
git merge --ff-only next
```

## 推送个人分支

如果需要把 `my-next` 推到自己的远端：

```powershell
git push -u origin my-next
```

如果 `my-next` 使用 rebase 后已经推送过，可能需要 force-with-lease：

```powershell
git push --force-with-lease origin my-next
```

`--force-with-lease` 比 `--force` 更安全，会避免覆盖别人刚推送的新提交。

## 回滚与恢复

查看最近操作：

```powershell
git reflog --date=local --max-count=20
```

查看 stash：

```powershell
git stash list
```

恢复某个 stash 到当前分支：

```powershell
git stash apply 'stash@{n}'
```

如果需要撤销未提交的某个文件改动，先确认 diff：

```powershell
git diff -- <path>
```

再决定是否恢复该文件。不要在不确认的情况下使用 `git reset --hard`。
