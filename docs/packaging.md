# Packaging Guide

本文档记录 EcoPaste 常用打包命令。命令默认在项目根目录执行：

```powershell
cd D:\KaiFaRuanJian\RustSource\EcoPaste
```

## Prerequisites

- Node.js 18+
- pnpm
- Rust stable
- Windows 安装包需要 NSIS 相关依赖可下载或已缓存

首次构建前安装依赖：

```powershell
pnpm install
```

## Frontend Build

只构建前端资源：

```powershell
pnpm build
```

产物路径：

```text
dist/
```

## Windows Portable

免安装版（portable / 解压即用）推荐使用：

```powershell
pnpm package:windows:portable
```

这条命令会先执行：

```powershell
pnpm tauri build --no-bundle
```

然后执行：

```powershell
pnpm portable:windows
```

产物路径：

```text
dist/portable/EcoPaste-<version>-windows-x64-portable.zip
```

解压后目录结构：

```text
EcoPaste.exe
portable
README.txt
assets/tray.ico
```

`portable` 是便携模式 marker file（标记文件）。存在该文件时，EcoPaste 会把主要数据写入 exe 同级的 `data/`：

```text
data/.store.json
data/.window-state.json
data/EcoPaste.db
```

删除 `portable` 文件后，会回到系统默认 AppData 路径。

## Windows Standalone Exe

只编译 release exe，不生成安装包：

```powershell
pnpm package:windows:exe
```

等价命令：

```powershell
pnpm tauri build --no-bundle
```

产物路径：

```text
target/release/EcoPaste.exe
```

这个 exe 不是完整 portable 包；如果要分发，建议使用 `pnpm package:windows:portable`，因为 portable 包会带上资源目录和 marker file。

## Windows Installer

生成 Windows 安装包（NSIS）：

```powershell
pnpm package:windows:installer
```

等价命令：

```powershell
pnpm tauri build --bundles nsis
```

产物路径：

```text
target/release/bundle/nsis/
```

如果 NSIS 阶段报网络下载错误，例如 `nsis_tauri_utils.dll` 下载失败，可以先使用 portable 打包：

```powershell
pnpm package:windows:portable
```

Portable 打包不依赖 NSIS bundling。

## macOS Bundle

在 macOS 上执行：

```bash
pnpm tauri build --bundles app,dmg
```

常见产物路径：

```text
target/release/bundle/macos/
target/release/bundle/dmg/
```

macOS 签名、公证、权限需要按发布环境单独配置。

## Linux Packages

在 Linux 上执行：

```bash
pnpm tauri build --bundles appimage,deb,rpm
```

常见产物路径：

```text
target/release/bundle/appimage/
target/release/bundle/deb/
target/release/bundle/rpm/
```

Linux 依赖见 `src-tauri/tauri.linux.conf.json`，当前 deb/rpm 配置包含 GStreamer 依赖。

## Clean Rebuild

前端资源异常时可先清理 `dist/` 后重建：

```powershell
Remove-Item -Recurse -Force dist
pnpm build
```

Rust release 产物异常时可清理 release 目录后重建：

```powershell
Remove-Item -Recurse -Force target/release
pnpm package:windows:portable
```

注意：`target/release` 清理后完整编译会更慢。
