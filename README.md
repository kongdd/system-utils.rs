# system-media

[![CI](https://github.com/kongdd/system-media.rs/actions/workflows/ci.yml/badge.svg)](https://github.com/kongdd/system-media.rs/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/kongdd/system-media.rs/graph/badge.svg)](https://codecov.io/gh/kongdd/system-media.rs)

`clipboard-image` 将剪贴板图片导出为 PNG，`audio-player` 播放本地 MP3 文件，`add-path` 将目录加入 Windows 当前用户的 PATH。

```bash
clipboard-image --stdout > shot.png  # 输出 PNG 二进制
clipboard-image --output shot.png    # 保存到指定路径
audio-player song.mp3                # 播放 MP3
lines --exts rs,R                    # 统计当前目录
lines -d crates/lines --exts rs,R    # 统计指定目录
```

[`lines`](crates/lines/README.md) 按目录树统计代码行数，也可按阈值筛选过长的文件。

在 Windows 上运行 `add-path "C:\Tools"`，然后打开新终端。已存在的目录不会重复添加。`add-path -f ~/.local/bin` 会展开 `~`，并在添加 PATH 前递归创建目录。

`clipboard-image` 需要访问桌面剪贴板，输出的 PNG 不得超过 18 MiB。剪贴板无图片、图片超限或写入失败时，程序向标准错误输出错误信息，并以非零状态退出。Pi 扩展的用法见 [Pi 扩展文档](docs/pi-clipimg.md)。

## 安装与构建

所有 workspace 成员共用根目录 `Cargo.toml` 的 Release 配置，优先减小可执行文件体积。新工具加入 `workspace.members` 即可沿用，无需重复定义 profile。

在仓库根目录构建命令行工具，产物默认位于 `target/release/`：

```bash
cargo build --workspace --bins --release
```

也可分别安装：

```bash
cargo install --path crates/clipboard-image
cargo install --path crates/audio-player
cargo install --path crates/add-path  # 仅 Windows
cargo install --path crates/lines
```

交叉编译 Windows x64 整套工具（需 rustup 和 MinGW-w64）：

```bash
./scripts/build-dist.sh windows-x64
```

`dist/windows-x64/` 包含 `add-path.exe`、`clipboard-image.exe`、`audio-player.exe`、`lines.exe`、`edge-tts-rust.exe` 和 `edge-tts-rust` 的 MIT 许可证。

在 Apple Silicon Mac 上构建整套工具（需 rustup 和 Xcode 命令行工具）：

```bash
./scripts/build-dist.sh macos-arm64
```

`dist/macos-arm64/` 包含 `clipboard-image`、`audio-player`、`lines`、`edge-tts-rust` 和 `edge-tts-rust` 的 MIT 许可证。
