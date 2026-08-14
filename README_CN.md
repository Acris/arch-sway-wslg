# arch-sway-wslg

[English](README.md) | [简体中文](README_CN.md)

`arch-sway-wslg` 用于在 Microsoft WSL2/WSLg 中安装并运行以 Wayland 为主的 Sway 桌面。项目面向 WSL 中的 Arch Linux，不是通用的裸机
Sway 发行版。

<img alt="Sway" src="https://github.com/user-attachments/assets/7bbac63f-4e80-4c44-9ff2-a9dc18fccefc" style="max-width: 1200px; width: 100%;" />

## 功能

- 上游 Sway，并在需要时兼容 XWayland
- Waybar、SwayNC、Fuzzel、Foot、swaynag、nwg-look 和 Yazi
- 全局 Catppuccin Mocha 风格
- UI 使用 Sarasa UI SC，终端使用 Maple Mono NF CN
- 与 Windows 双向同步 UTF-8 纯文本剪贴板
- 使用 WSLg PulseAudio 音频
- 通过一个命令管理 Sway 后台会话、状态、日志、诊断、清理和崩溃恢复
- 带可选备份的事务式配置安装

默认会话保持紧凑。截图和外层 WSLg 窗口由 Windows 管理；本项目不会额外安装锁屏、电源管理、电池、网络或截图工具。

## 前置条件

请先完成 [ArchWiki 的 Arch Linux on WSL 安装指南](https://wiki.archlinux.org/title/Install_Arch_Linux_on_WSL)。安装器要求：

1. 在启用 WSLg 的 WSL2 中运行 Arch Linux。
2. 已创建普通用户并设为默认 WSL 用户，且 `sudo` 可用。
3. 已按照 [ArchWiki 的 locale 说明](https://wiki.archlinux.org/title/Locale) 配置 locale，并使用有效的 UTF-8 locale。
4. 已配置 WSLg 硬件加速，并保持 Windows 和主机显卡驱动为最新。
5. 普通用户已安装 `base-devel`、Git 和 `paru`。
6. 已启用 Windows 互操作，使 WSL 可以运行 `powershell.exe`。

在 Windows 中保持 WSL 更新：

```powershell
wsl --update
wsl --shutdown
```

systemd 不是必需的。启动器会优先复用可用的用户 D-Bus，否则使用私有的 `dbus-run-session`；不会修改全局 D-Bus 或 systemd
激活环境。

硬件加速有助于获得流畅的嵌套合成效果。如果出现渲染问题，先更新 Windows，执行 `wsl --update`，并安装主机显卡的最新驱动。
如果问题仍然存在，可以在启动 Sway 前尝试软件渲染：

```bash
export LIBGL_ALWAYS_SOFTWARE=1
arch-sway-wslg start
```

软件渲染会更慢。取消该变量或重新打开 shell 即可恢复硬件加速。

## 快速开始

使用普通 Arch 用户执行，不能使用 root：

```bash
git clone https://github.com/Acris/arch-sway-wslg.git
cd arch-sway-wslg
./install.sh
```

请在 `paru` 显示 AUR PKGBUILD 时先检查内容，再确认安装。安装器会：

- 校验所需 payload 文件；
- 询问是否备份现有的受管理路径；
- 列出所有可选 desktop-entry mask 并询问是否安装；
- 询问嵌套 Sway 输出缩放比例，默认值为 `1`；
- 检测正在运行的受管理 Sway 会话，并在更新前询问是否停止；
- 更新 Arch，先安装 `packages.conf` 中的 bootstrap provider，再安装其余桌面软件；
- 将完整 payload 放入 staging，应用选择的缩放比例，并在替换文件前校验最终的 Shell、Sway、Foot 和 Fuzzel 配置；
- 如果部署失败，回滚已替换的路径；
- 将公开启动器安装到 `~/.local/bin`，将私有辅助程序安装到 `~/.local/libexec/arch-sway-wslg`；
- 使用 `gsettings` 应用推荐的 GTK 外观默认值；
- 安装完成后打印可选的 Yazi 集成和增强预览 `paru` 命令。

安装器会在校验 Sway 前解释 sudo 请求：root 权限只用于创建临时、私有的 X11 mount namespace。校验过程仍以普通用户运行， 不会改变
WSLg 的全局 X11 映射。

如果 `~/.local/bin` 不在 `PATH` 中，安装器会打印对应的 Bash 或 Fish 命令。之后启动会话：

```bash
arch-sway-wslg doctor
arch-sway-wslg start
arch-sway-wslg status
```

如果启动未完成，查看日志：

```bash
arch-sway-wslg logs
```

## 调整和最大化 WSLg 窗口

以下快捷键操作的是外层 Windows/WSLg 窗口，而不是 Sway 容器：

- `Win+Up`：最大化 WSLg 窗口。
- `Win+Shift+Left/Right`：将窗口移动到另一台显示器。
- 如果窗口没有正确铺满目标屏幕，先按 `Win+Left` 或 `Win+Right`，再按 `Win+Up`。

若想让窗口占用任务栏下面的完整高度，可启用 Windows 任务栏自动隐藏。也可以把任务栏留在不重要的主显示器上， 将 WSLg
窗口放到另一台显示器。以上快捷键由 Windows 处理，不需要添加 Sway 绑定。

## 安装与更新

受管理的配置目录会被完整替换，旧版本残留文件不会继续存在，但目录中的自定义文件也会被替换：

```text
~/.config/foot
~/.config/fuzzel
~/.config/sway
~/.config/swaynag
~/.config/swaync
~/.config/waybar
~/.config/yazi
```

如果这些目录包含本地修改，请接受备份提示。备份位于 `$XDG_STATE_HOME/arch-sway-wslg/backups`，未设置时默认是
`~/.local/state/arch-sway-wslg/backups`。

更新项目：

```bash
git pull --ff-only
./install.sh
```

Desktop-entry mask 只包含 `Hidden=true`，用于在 Fuzzel 中隐藏辅助程序，不会卸载软件。拒绝提示不会删除或修改已有的同名
desktop 文件。

## 命令

```bash
arch-sway-wslg doctor
arch-sway-wslg start
arch-sway-wslg status
arch-sway-wslg logs
arch-sway-wslg restart
arch-sway-wslg stop
arch-sway-wslg version
```

`start` 和 `restart` 会解释 sudo 请求并申请一次权限，用于创建会话的隔离 X11 mount namespace，然后以普通用户启动会话。 Sway
及桌面应用不会以 root 运行。`stop` 不需要 sudo；它先使用 Sway IPC，若合成器没有正常退出，再执行有界的进程组清理。 即使 PID
状态丢失，固定 IPC 路径仍支持恢复。

## 快捷键

| 按键                         | 操作                     |
|------------------------------|--------------------------|
| `Alt+Enter`                  | 打开 Foot                |
| `Alt+D`                      | 打开 Fuzzel              |
| `Alt+Y`                      | 在 Foot 中打开 Yazi      |
| `Alt+H/J/K/L` 或方向键       | 移动焦点                 |
| `Alt+Shift+H/J/K/L` 或方向键 | 移动当前容器             |
| `Alt+1..0`                   | 切换工作区 1–10          |
| `Alt+Shift+1..0`             | 将容器移动到工作区 1–10  |
| `Alt+B/V`                    | 选择水平或垂直分割       |
| `Alt+S/W/E`                  | 选择堆叠、标签或分割布局 |
| `Alt+F`                      | 切换全屏                 |
| `Alt+Shift+F`                | 切换浮动                 |
| `Alt+R`                      | 进入调整大小模式         |
| `Alt+Shift+N`                | 切换 SwayNC 控制中心     |
| `Alt+Ctrl+N`                 | 切换免打扰               |
| `Alt+Shift+Q`                | 关闭当前窗口             |
| `Alt+Shift+C`                | 重新加载 Sway 配置       |
| `Alt+Shift+E`                | 确认并退出 Sway 会话     |

Windows 占用 `Alt+Tab` 和 `Alt+Space`，所以配置不会使用这些组合键。截图仍可使用 Windows 的 `Win+Shift+S`。

## Yazi

按 `Alt+Y` 可在 Foot 中打开 Yazi。内置主题使用 Catppuccin Mocha。Yazi 是终端文件管理器，不会替换 GTK/Qt 文件选择器，
也不会为每种文件类型安装应用程序。

常用默认绑定：

| 按键               | 操作                                           |
|--------------------|------------------------------------------------|
| `h/j/k/l` 或方向键 | 离开、移动或进入目录                           |
| `Enter`            | 打开选中的文件或目录                           |
| `Space`            | 切换选择                                       |
| `y` / `x` / `p`    | 复制 / 剪切 / 粘贴                             |
| `d` / `D`          | 移入回收站 / 永久删除                          |
| `a` / `r`          | 新建 / 重命名                                  |
| `.`                | 显示或隐藏隐藏文件                             |
| `f`                | 过滤当前目录                                   |
| `s` / `S`          | 使用 `fd` 搜索文件名 / 使用 `ripgrep` 搜索内容 |
| `z` / `Z`          | 使用 `fzf` / `zoxide` 导航                     |
| `F1` 或 `~`        | 打开 Yazi 帮助                                 |
| `q`                | 退出 Yazi                                      |

完整默认键位请参阅 [Yazi 快速入门键位说明](https://yazi-rs.github.io/docs/quick-start/#keybindings)。

[Yazi 官方安装指南](https://yazi-rs.github.io/docs/installation/) 推荐以下工具。安装器已经包含 Yazi、Nerd Font provider 和
Wayland 剪贴板支持；其余搜索、导航、JSON 和归档集成可按需安装：

```bash
paru -S --needed fd ripgrep fzf zoxide jq 7zip
```

- `fd`：快速查找文件名。
- `ripgrep`：搜索文件内容。
- `fzf`：模糊选择。
- `zoxide`：按使用频率导航目录。
- `jq`：格式化和预览 JSON。
- `7zip`：预览和解压更多归档格式。

增强预览是可选的：

```bash
paru -S --needed ffmpeg poppler resvg imagemagick
```

- `ffmpeg`：提取视频缩略图和媒体元数据。
- `poppler`：提供 PDF 渲染和文本提取工具。
- `resvg`：渲染 SVG 预览。
- `imagemagick`：转换和识别额外的图像、字体格式。

Foot 通过内置 Sixel 支持 Yazi 图像预览。若希望 shell 在退出 Yazi 后切换到最后访问的目录，请参阅
[Yazi shell wrapper](https://yazi-rs.github.io/docs/quick-start/#shell-wrapper)；本项目不会修改 shell 启动文件。

## 外观

安装器通过 `gsettings` 应用以下 GTK 默认值：

- GTK 主题：`adw-gtk3-dark`
- 配色方案：`prefer-dark`
- 图标主题：`Papirus-Dark`
- UI 字体：`Sarasa UI SC 11`
- 光标：`Adwaita`，大小 `28`

Sway、Waybar、SwayNC、Fuzzel、Foot、swaynag 和 Yazi 已内置 Catppuccin Mocha。GTK 使用 Adwaita Dark，因为历史上的 Catppuccin GTK
移植版已经归档。进入 Sway 后运行 `nwg-look`，可以查看或修改 GTK、图标、字体和光标设置。

## Windows 剪贴板桥接

桥接程序双向同步 UTF-8 纯文本，不同步图片、HTML 或文件列表。

- 一个持久运行的 Windows PowerShell 5.1 setter 负责写入 Windows 剪贴板。
- 每次 setter 请求都会返回明确的成功或失败确认。
- Windows watcher 使用剪贴板序列号，避免重复启动进程。
- 协议行使用 LF，Linux 读取端会防御性接受 CRLF。
- 反射哈希短时间有效、按 payload 区分，并且只消费一次。
- 桥接启动时会先把 Windows 中支持的文本发布到新的 Sway 剪贴板，再开始 Sway 到 Windows 的监听。
- `nil`、清空事件以及标记为 `CLIPBOARD_STATE=sensitive` 的内容默认不会覆盖 Windows 剪贴板。

如需同步标记为 sensitive 的剪贴板内容，在启动 Sway 前设置：

```bash
export ARCH_SWAY_WSLG_SYNC_SENSITIVE=1
arch-sway-wslg start
```

不建议对密码管理器内容启用此选项。如果 Windows 挂载路径不同，可设置 `WINDOWS_POWERSHELL` 覆盖 PowerShell 查找路径。

## 故障排查

先运行诊断：

```bash
arch-sway-wslg doctor
```

如果缺少 WSLg Wayland、PulseAudio 或 X11 映射，请从 Windows 关闭 WSL 后重试：

```powershell
wsl --shutdown
```

遇到渲染错误时，先更新 Windows、WSL 和主机显卡驱动；如果硬件渲染仍不稳定，请尝试前置条件中的
`LIBGL_ALWAYS_SOFTWARE=1` 回退方案，它会牺牲性能以换取兼容性。

`doctor` 只检查前置条件，不申请 sudo，也不改变 mount 状态。真正的私有 namespace 和 Sway 配置校验发生在 `start` 期间。 在
Sway 启动的 Foot 中执行 `echo "$DISPLAY"`，即使还没有启动 X11 应用，也应显示 Sway 分配的嵌套 display。空值表示 XWayland
初始化失败，请查看 `arch-sway-wslg logs`。Sway 会选择嵌套 display 编号，不需要与 WSLg 的父 display `:0` 相同。

要明确测试 X11 路径，可在 Sway 中运行：

```bash
GDK_BACKEND=x11 nwg-look
```

如果 Sway 已运行但启动器状态被中断，使用 `arch-sway-wslg stop`；固定 IPC socket 支持恢复。不要手动删除 `/tmp/.X11-unix`。

## 软件包说明

`packages.conf` 是安装器唯一的软件包清单。`[bootstrap]` 部分先安装后续软件所需的 portal、字体、Nerd Font 和 JACK provider，
成功后再安装 `[main]` 中的桌面软件和应用。清单不会重复声明 pacman 自动解析的普通依赖。

- `xdg-desktop-portal-gtk-dummy` 满足 Arch GTK 要求，不安装本 WSLg 会话不需要的 guest portal 栈。
- `jack2` 是 Waybar JACK 库依赖的默认 provider，不会被启动，音频仍通过 WSLg PulseAudio。如果已经安装 `pipewire-jack`
  ，安装器会保留它并跳过
  `jack2`，因为两个 provider 互相冲突。
- `qt5-wayland` 为 Qt 5 应用提供原生 Wayland 支持。
- `maplemono-nf-cn-unhinted` 为 Foot 提供 Maple Mono NF CN。
- `ttf-nerd-fonts-symbols-mono` 满足 Yazi 的 Nerd Font 要求，让回退图标保持终端单元格对齐；它不会替换终端字体。

## 运行原理与限制

Sway 直接连接 WSLg 的绝对父级 Wayland socket：`/mnt/wslg/runtime-dir/wayland-0`。WSLg 管理发行版范围的 `/tmp/.X11-unix`
映射， 因此启动器只为受管理的 Sway 进程树在独立 mount namespace 中创建私有的 `01777` X11 socket 目录。一次简短的 sudo
操作创建该 namespace 和 bind mount；root shell 随即通过 `runuser` 降回普通用户，再继续运行项目启动器。会话仍处于普通 WSL 用户
namespace 中，因此 Sway 内的
`sudo` 等 setuid 工具保持正常。父级 WSLg 映射从不卸载、删除或替换，也不需要修改 `/etc/wsl.conf`。Sway 会预留 display，并在第一个
X11 客户端连接时启动 XWayland。会话结束时 mount namespace 自动消失。

会话以 Wayland 为主。Qt 和 SDL2 使用 Wayland 并在需要时回退到 X11；GTK、SDL3、Firefox 和较新的 Electron 应用使用各自的原生后端选择。
使用分数缩放的 XWayland 应用可能不如原生 Wayland 应用清晰。

本项目创建一个嵌套 Sway 输出；多输出模拟和多个独立 WSLg 窗口不在默认范围内。

## 致谢

项目遵循上游 Sway 约定，并吸收了 [jordankoehn/sway-wsl2](https://github.com/jordankoehn/sway-wsl2) 中有用的嵌套
Sway、XWayland 和 Windows 剪贴板序列号思路。与其启动脚本相比，本项目不会重启用户 systemd 服务，也不会全局卸载、删除和重建
`/tmp/.X11-unix`； 它只为单个受管理会话创建隔离的 mount 视图，并在降回普通用户后启动项目。这样可以保持 WSLg 映射不变，不需要
WSL 启动配置，能够适应 WSLg 目录重建，并保留 Sway 内正常的 `sudo` 行为；同时避免每次复制都启动进程以及依赖前台窗口标题的门控逻辑。

其他参考：

- [Sway 示例配置](https://github.com/swaywm/sway/blob/master/config.in)
- [Sway 手册](https://man.archlinux.org/man/sway.5.en)
- [Microsoft WSLg](https://github.com/microsoft/wslg)
- [Waybar](https://github.com/Alexays/Waybar)
- [SwayNC](https://github.com/ErikReider/SwayNotificationCenter)
- [Yazi](https://yazi-rs.github.io/)
- [Catppuccin](https://catppuccin.com/)
- [Maple Mono](https://github.com/subframe7536/maple-font)
- [Sarasa Gothic](https://github.com/be5invis/Sarasa-Gothic)

## 许可证

MIT。详见 [LICENSE](LICENSE)。
