# lines

`lines` 用来检查代码文件是否过长：按目录树统计行数，也可仅列出需要关注的文件。

在仓库根目录执行 `cargo install --path crates/lines` 安装。

```sh
lines --exts rs,R                         # 统计当前目录
lines -d crates --exts rs                 # 统计指定目录
lines --warn --exts rs                    # 仅列出警告区间内的文件
lines --bad --exts rs                     # 仅列出超过上限的文件
lines --n-warn=100 -d crates --exts rs    # 临时调整警告下限
lines config list                         # 查看当前生效的阈值
lines config --n-warn 300 --n-bad 600     # 保存个人阈值
```

- 文件名与行数同色：默认 **低于 300 行**使用终端默认文字色，**300–600 行**标黄，**超过 600 行**标红。黄、红项及目录合计加粗，行数使用千位分隔符并右对齐。文件行数右侧显示 `+` 长度条，尺度按扫描目录中最长的文件自动调整，每格至少 10 行，不满一格向上取整，最多显示 20 格。设置 `NO_COLOR=1` 可关闭颜色。
- `--exts` 接受逗号分隔、不带点的扩展名，区分大小写。筛选时可省略它，此时跳过开头含 NUL 的文件。
- `--warn` 和 `--bad` 可同时指定；没有匹配项时，以绿色粗体提示当前检查通过。临时指定 `--n-warn` 或 `--n-bad` 时，若未指定筛选标记，默认启用 `--warn`。
- 默认跳过名为 `test` 或 `tests` 的子目录；加 `--tests` 可纳入统计和筛选。遵循 `.gitignore`，跳过隐藏项，不跟随符号链接。行数包含空行和末尾无换行符的行。

个人阈值保存在 `$XDG_CONFIG_HOME/lines/thresholds`（默认 `~/.config/lines/thresholds`），Windows 使用 `%APPDATA%\lines\thresholds`。
