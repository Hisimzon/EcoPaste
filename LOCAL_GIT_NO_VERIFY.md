# 本地提交跳过 Git Hooks

为了避免本地提交时运行耗时的 `cargo clippy`，使用：

```bash
git commit --no-verify -m "提交说明"
git push
```

`--no-verify` 加在 `git commit`，不是 `git push`。它会跳过本项目的本地 Git hooks，因此不会自动运行：

- `biome check`：前端格式和部分代码规范检查。
- `cargo fmt`：Rust 格式检查和整理。
- `cargo clippy`：Rust 编译检查及代码警告检查。

## 什么是 Rust 警告

Rust 警告通常不会阻止普通编译，但可能表示存在无效代码、可疑写法或潜在问题，例如：

- 未使用的 import、变量或函数。
- 永远不会执行的代码。
- 不必要的复制、借用或类型转换。
- Clippy 检测出的容易出错或低效写法。

项目的 Clippy 命令包含 `-D warnings`，会把所有警告当成错误处理，所以出现警告时本地 hook 会阻止提交。

使用 `--no-verify` 只代表跳过本地检查，不代表代码已经通过检查。正式发布前应由 GitHub CI 或手动检查确认。

## 提交前的轻量检查

使用 `--no-verify` 提交前，可以执行不包含完整 `cargo clippy` 的轻量检查：

- 查看本次 `git diff`，检查明显的类型、生命周期、条件分支和跨平台问题。
- 检查未使用的 import、函数调用参数和 `Result` 错误处理。
- 运行较轻量的 `cargo fmt --check`，确认 Rust 格式正确。
- 前端只对本次修改文件运行 Biome 格式和代码规范检查。

以上检查可以减少明显问题，但不能完全替代 `cargo clippy`、完整编译和实际功能验证。
