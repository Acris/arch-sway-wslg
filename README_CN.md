# arch-sway-wslg

[English](README.md) | [简体中文](README_CN.md)

`arch-sway-wslg` 在 Microsoft WSL2/WSLg 中安装并运行一套精心调校、Wayland 优先的 Sway 会话。它面向 WSL 上的 Arch
Linux，并不打算成为通用的裸机 Sway 发行配置。

<img alt="Sway" src="https://github.com/user-attachments/assets/7bbac63f-4e80-4c44-9ff2-a9dc18fccefc" style="max-width: 1200px; width: 100%;" />

## 特性

- 上游 Sway，配合按需启动的 XWayland 兼容层
- Waybar、SwayNC、Fuzzel、Foot、swaynag、nwg-look 与 Yazi
- 整个桌面统一使用 Catppuccin Mocha 配色
- 界面使用 Sarasa UI SC，终端使用 Maple Mono NF CN
- 与 Windows 之间自动同步 UTF-8 纯文本剪贴板，并带有门控，绝不干扰打字
- 最多四个嵌套输出，每个都是独立的 WSLg 窗口
- 你的个人设置放在自定义覆盖文件中，更新永不触碰
- 可自行选择浏览器，并在会话内接入 `BROWSER`
- 集成 WSLg PulseAudio
- 一条命令即可启动、停止、查看与诊断会话

默认会话刻意保持精简。截图与外层 WSLg 窗口由 Windows 负责；客户机内不安装锁屏、电源管理、电池、网络或截图工具。

## 前置条件

请先完成
[ArchWiki 上的 WSL Arch Linux 安装指南](https://wiki.archlinux.org/title/Install_Arch_Linux_on_WSL)。安装脚本要求：

1. Arch Linux 运行在启用了 WSLg 的 WSL2 中。
2. 有一个普通用户被配置为 WSL 默认用户，且 `sudo` 可用。
3. 已启用 systemd，并且该普通用户的 systemd 用户管理器工作正常。
4. 按照 [ArchWiki 的 locale 说明](https://wiki.archlinux.org/title/Locale)
   配置 locale。不要求使用特定 locale，安装脚本既不检查也不写入 locale，但强烈建议使用 UTF-8 locale：状态栏、启动器与 Yazi
   都会显示非 ASCII 文本。
5. 已配置 WSLg 硬件加速。请保持 Windows 与宿主 GPU 驱动为最新版本。
6. 为该普通用户安装 `base-devel`、Git 与 `paru`。

在 Windows 侧保持 WSL 为最新：

```powershell
wsl --update
wsl --shutdown
```

systemd 是必需的。当前由 `wsl --install -d archlinux` 安装的官方 Arch 镜像默认已启用它。 如果是较旧或导入的发行版，
`systemctl status` 报告 systemd 未运行，请在
`/etc/wsl.conf` 中加入以下内容，然后在 Windows 中执行 `wsl --shutdown`：

```ini
[boot]
systemd=true
```

嵌套合成器要流畅运行必须有硬件加速。如果渲染不稳定，请先更新 Windows、执行
`wsl --update`，并安装宿主 GPU 的最新驱动。

## 快速开始

以普通 Arch 用户身份执行下列命令，切勿使用 root：

```bash
git clone https://github.com/Acris/arch-sway-wslg.git
cd arch-sway-wslg
./install.sh
```

接受 `paru` 展示的 AUR PKGBUILD 之前请先审阅。安装脚本会：

- 在改动任何软件包之前检查前置条件与载荷；
- 询问是否安装可选的桌面条目屏蔽文件；
- 询问安装哪个浏览器：Firefox（默认）、Chromium、Google Chrome、Microsoft Edge，或不安装；已经安装的浏览器会标注
  `[installed]`，选中后只接入 `BROWSER`，不会重复安装；
- 询问 Sway 输出缩放（1 到 4，可含小数）；
- 询问是否把当前的托管文件复制到 `~/.local/state/arch-sway-wslg/backups/<timestamp>`，默认为“是”；
- 询问后停止正在运行的托管会话；
- 更新 Arch、安装引导所需的提供者，然后安装桌面组件与所选浏览器（若该浏览器已安装则跳过）；
- 打印当前与建议的 GTK 外观设置，并在修改前询问；
- 暂存整个载荷并完成检查，之后才替换任何内容。

然后启动会话：

```bash
arch-sway-wslg doctor
arch-sway-wslg start
arch-sway-wslg status
```

如果启动没有完成，用 `arch-sway-wslg logs` 查看日志。

## 命令

```bash
arch-sway-wslg start [--outputs N]    # start the session with N nested outputs (1-4, default 1)
arch-sway-wslg stop
arch-sway-wslg restart [--outputs N]
arch-sway-wslg status
arch-sway-wslg logs
arch-sway-wslg doctor
arch-sway-wslg version
```

`start` 与 `restart` 会说明用途并一次性请求 sudo，用于创建会话隔离的 X11 挂载命名空间，随后以你的普通用户身份在瞬态
systemd 用户 scope 中、配合私有 D-Bus 会话启动托管会话。Sway 及其桌面应用绝不以 root 运行。`stop` 不需要 sudo：它先通过 IPC
请求 Sway 退出，必要时再停止该 scope。systemd scope 才是权威的会话状态；IPC 只是通信通道。

## 多显示器

Sway 可以驱动多个嵌套输出。每个输出都是独立的顶层 WSLg 窗口，你可以把它移动到不同的 Windows 显示器上：

```bash
arch-sway-wslg start --outputs 2
```

`ARCH_SWAY_WSLG_OUTPUTS=2 arch-sway-wslg start` 效果相同；两种形式都只接受 1 到 4 的整数，其他任何值都会被拒绝。输出依次命名为
`WL-1`、`WL-2` 等。在你自己的配置中把工作区绑定到它们，例如写在
`~/.config/sway/config.d/10-local.conf`：

```
workspace 1 output WL-1
workspace 2 output WL-1
workspace 9 output WL-2
workspace 10 output WL-2
```

用 Windows 快捷键摆放窗口：`Win+Shift+Left/Right` 把窗口移到另一台显示器，`Win+Up`
最大化窗口。如果窗口没有铺满目标屏幕，先按 `Win+Left` 或 `Win+Right`，再按 `Win+Up`。为 Windows
任务栏启用自动隐藏，才能用满显示器的高度。本项目不会替你移动或最大化 WSLg 窗口。

## 自定义而不丢失修改

托管的配置目录在每次安装时都会被完整替换，因此旧版本残留的文件不可能在更新后存活。有三个路径始终属于你，永不被替换：

| 路径                             | 用途                              |
|----------------------------------|-----------------------------------|
| `~/.config/sway/config.d/*.conf` | Sway 设置，在其余全部内容之后读取 |
| `~/.config/foot/local.ini`       | Foot 选项，在内置选项之后应用     |
| `~/.config/fuzzel/local.ini`     | Fuzzel 选项，在内置选项之后应用   |

首次安装时，安装脚本会创建这些文件并附带注释示例，之后每次安装都会原样保留。由于它们最后被读取，你在其中的设置优先生效：

```
# ~/.config/sway/config.d/10-local.conf
output * scale 1.5
bindsym $mod+p exec firefox
```

Waybar、SwayNC、swaynag 与 Yazi 没有类似的 include 机制，所以 `~/.config/waybar`、`~/.config/swaync`、
`~/.config/swaynag` 与 `~/.config/yazi` 完全由项目托管。请把这些文件的个人版本保存在托管目录之外，或者接受安装脚本在每次更新前提供的备份，再从其中拷回。

以下目录会被替换（这里显示的是默认根目录；绝对路径的 `$XDG_CONFIG_HOME` 会替代 `~/.config`）：

```text
~/.config/foot
~/.config/fuzzel
~/.config/sway
~/.config/swaynag
~/.config/swaync
~/.config/waybar
~/.config/yazi
```

## 会话环境

以下提示只在你自己没有设置过时才会被设置，因此你自己的值总是优先：

| 变量                                  | 值            | 原因                                         |
|---------------------------------------|---------------|----------------------------------------------|
| `QT_QPA_PLATFORM`                     | `wayland;xcb` | Qt 5 从不自行选择 Wayland；保留 xcb 作为回退 |
| `QT_WAYLAND_DISABLE_WINDOWDECORATION` | `1`           | 边框由 Sway 绘制，Qt 不应再画一套自己的      |
| `DONT_PROMPT_WSL_INSTALL`             | `1`           | 阻止 VS Code 在 Sway 里建议改用 Windows 版本 |
| `_JAVA_AWT_WM_NONREPARENTING`         | `1`           | 没有它时 Swing 与 JetBrains IDE 会渲染出灰窗 |
| `BROWSER`                             | 你的选择      | `xdg-open` 把 Sway 视为通用桌面并遵循该变量  |

以下值由会话自己决定，并且总是覆盖继承来的值，因为过期的值会让会话无法工作：

| 变量                                      | 值                      | 原因                              |
|-------------------------------------------|-------------------------|-----------------------------------|
| `XDG_RUNTIME_DIR`                         | `/run/user/$UID`        | 会话从不使用 WSLg 共享的运行时    |
| `WAYLAND_DISPLAY`、`SWAYSOCK`             | 父级套接字，随后由 Sway | Sway 会用它的嵌套值替换它们       |
| `XDG_SESSION_TYPE`、`XDG_CURRENT_DESKTOP` | `wayland`、`sway`       | 工具包与 `xdg-open` 的桌面探测    |
| `WLR_WL_OUTPUTS`                          | `--outputs N`           | 仅在请求多个输出时设置            |
| `PULSE_SERVER`                            | WSLg 套接字             | 音频始终送往 WSLg PulseAudio 端点 |

同时安装 `qt5-wayland` 与 `qt6-wayland`，让两代 Qt 都有可用的 Wayland 平台插件。较新的 Firefox 与 Chromium 版本默认选择
Wayland，因此不为它们设置额外参数。

安装时选择的浏览器记录在 `~/.config/arch-sway-wslg/browser`。编辑该文件（单个可执行文件名）或自行导出
`BROWSER` 即可更改。

## 快捷键

| 按键                         | 动作                       |
|------------------------------|----------------------------|
| `Alt+Enter`                  | 打开 Foot                  |
| `Alt+D`                      | 打开 Fuzzel                |
| `Alt+Y`                      | 在 Foot 中打开 Yazi        |
| `Alt+Shift+V`                | 立即读取 Windows 剪贴板    |
| `Alt+H/J/K/L` 或方向键       | 移动焦点                   |
| `Alt+Shift+H/J/K/L` 或方向键 | 移动聚焦的容器             |
| `Alt+1..0`                   | 切换到工作区 1–10          |
| `Alt+Shift+1..0`             | 把容器移动到工作区 1–10    |
| `Alt+B/V`                    | 选择水平或垂直分割         |
| `Alt+S/W/E`                  | 选择堆叠、标签页或分割布局 |
| `Alt+F`                      | 切换全屏                   |
| `Alt+Shift+F`                | 切换浮动                   |
| `Alt+R`                      | 进入调整大小模式           |
| `Alt+Shift+N`                | 切换 SwayNC 控制中心       |
| `Alt+Ctrl+N`                 | 切换免打扰                 |
| `Alt+Shift+Q`                | 关闭聚焦的窗口             |
| `Alt+Shift+C`                | 重新加载 Sway 配置         |
| `Alt+Shift+E`                | 确认并退出 Sway 会话       |

`Alt+Tab` 与 `Alt+Space` 归 Windows 所有，因此配置避开了这些组合。截图仍可通过 Windows 的
`Win+Shift+S` 完成。偏好使用独立修饰键的用户可以在 `~/.config/sway/config.d/` 中把 `$mod` 设为
`Mod3`，并用 Windows 的键盘重映射工具把某个 Windows 键映射到 Mod3。

## 剪贴板

WSLg 本身已经在双向同步它自己的 Wayland 剪贴板与 Windows 剪贴板。因此内置的桥接只在嵌套 Sway 会话与父级 WSLg 套接字之间镜像
UTF-8 纯文本，这已足以让 Sway 中的 `Ctrl+C` 能在 Windows 中粘贴，反之亦然。整个过程不涉及任何 Windows 辅助进程，也不需要
`powershell.exe`。

- 图片、HTML 与文件列表不参与同步。
- 源应用标记为 `sensitive` 的选区（密码管理器）默认被跳过。
- 桥接由 Sway 启动，因此与会话同生共死。

两个方向的工作方式并不相同。Sway 实现了 wlroots 的 data-control 协议，因此会话内的复制会在发生的瞬间被转发。WSLg 的 Weston
完全没有实现任何 data-control 协议，因此外层剪贴板无法通过事件监听，只能改用一次性读取：每秒一次，且每次读取都有 3 秒超时上限。

这样的读取必须在 WSLg 上打开一个 1 像素的 surface，会把键盘焦点从会话短暂夺走。焦点回来时 wlroots
会把当时按住的键重放一遍，因此与打字重叠的读取会造成字符重复或丢失。为此桥接只在会话两秒没有输入之后才读取外层剪贴板。它自己启动并监督一个
`swayidle` 来得知这一时刻；如果 `swayidle` 无法保持运行，桥接会在日志中明确说明并停止读取，而不是接着往你的按键里插。
`arch-sway-wslg status` 会报告桥接处于这两种状态中的哪一种。

这也意味着在 Windows 上复制的文本会在会话静止之后才到达，而不是在你工作的过程中。想要立即拿到时，按
`Alt+Shift+V`：无论会话正在做什么，它都会当场读取 Windows 剪贴板。

下面三个变量由桥接读取，而桥接由 Sway 启动，因此请在 `arch-sway-wslg start` **之前** export； 在会话内的终端里设置它们没有任何效果：

```bash
# 降低读取 Windows 剪贴板的频率；小于 200ms 的值会被拒绝
export ARCH_SWAY_WSLG_CLIPBOARD_POLL=5

# 读取前等待更长的会话静止时间（整秒，最小 1）
export ARCH_SWAY_WSLG_CLIPBOARD_IDLE=5

# 只转发 Sway -> Windows，不再读取 Windows 剪贴板
export ARCH_SWAY_WSLG_CLIPBOARD=to-windows

# 完全不做剪贴板桥接
export ARCH_SWAY_WSLG_CLIPBOARD=off
```

入向方向被关闭后，`Alt+Shift+V` 也不再工作，但在会话内的任何终端里执行
`WAYLAND_DISPLAY=/mnt/wslg/runtime-dir/wayland-0 wl-paste` 仍可按需读取 Windows 剪贴板。

若要包含敏感选区，请在启动 Sway 之前导出下面的变量；对密码管理器不建议这样做：

```bash
export ARCH_SWAY_WSLG_SYNC_SENSITIVE=1
arch-sway-wslg start
```

## Waybar 布局

状态栏右侧保留五个胶囊：资源、音量、托盘、通知与时钟。内存占用始终可见；把鼠标悬停在上面会滑出 CPU
与磁盘占用，这样既能看到系统信息，又不会让状态栏过于拥挤。

## 外观

安装脚本会先显示当前值，再显示下面这些建议的 GTK 默认值，并在修改前询问。该提示默认为“是”；回答“否”则所有 GSettings 值保持不变。

- GTK 主题：`adw-gtk3-dark`
- 配色方案：`prefer-dark`
- 图标主题：`Papirus-Dark`
- 界面字体：`Sarasa UI SC 11`
- 光标：`Adwaita`，尺寸 `28`

Sway、Waybar、SwayNC、Fuzzel、Foot、swaynag 与 Yazi 都内置了 Catppuccin Mocha。GTK 使用 Adwaita Dark，因为历史上的 Catppuccin
GTK 移植已归档。在 Sway 内运行 `nwg-look` 可以查看或修改 GTK、图标、字体与光标设置。

安装脚本会询问输出缩放，接受 1 到 4 之间的任意值，包括 `1.25` 这样的小数。它无法被自动探测：Wayland 自身无法表达的缩放 由
WSLg 在 Windows 侧完成，其父输出始终通告 scale 1，因此 Windows 上 125% 的设置从 Linux 侧完全不可见。请按你的 Windows
显示缩放填写（125% 填 `1.25`，150% 填 `1.5`），并可随时在 `~/.config/sway/config.d/` 中用 `output * scale 1.25` 修改。

内置壁纸安装在 `~/.config/sway/wallpapers/dark-star.jpg`，其解析后的绝对路径会写入 Sway 配置。壁纸来自
[walls-catppuccin-mocha](https://github.com/orangci/walls-catppuccin-mocha) 图集，不在本项目 MIT
许可的授权范围内。该图集没有为图片声明许可，因此再分发者必须先确认获得许可。

## Yazi

按 `Alt+Y` 可在 Foot 中打开 Yazi；内置主题使用 Catppuccin Mocha 配色。默认键位见
[Yazi 快速上手键位说明](https://yazi-rs.github.io/docs/quick-start/#keybindings)，可选集成见
[Yazi 安装指南](https://yazi-rs.github.io/docs/installation/)。

安装成功后，安装脚本会打印两条推荐命令：

```bash
paru -S --needed fd ripgrep fzf zoxide jq 7zip        # search, navigation, JSON, archives
paru -S --needed ffmpeg poppler resvg imagemagick     # rich previews
```

Foot 通过内置的 Sixel 实现渲染 Yazi 的图片预览。本项目不会修改 shell 启动文件，如果你需要目录跟踪，请自行添加
[Yazi shell 包装函数](https://yazi-rs.github.io/docs/quick-start/#shell-wrapper)。

## 更新

```bash
git pull --ff-only
./install.sh
```

每次运行都会在替换任何内容之前，询问是否把此前的托管状态复制到 `~/.local/state/arch-sway-wslg/backups/<timestamp>`
，每个备份都包含一个写明确切恢复命令的
`RESTORE-INFO.txt`。旧备份永远不会被自动删除；不再需要的请自行移除。

一次更新可能会引入上一个版本不需要的软件包，因此请重新回答安装脚本的提问，不要假定软件包集合没有变化，之后用
`arch-sway-wslg restart` 重启会话。

## 卸载

先停止会话：

```bash
arch-sway-wslg stop
```

移除本项目安装的软件包。删掉你想保留的部分，并追加你所选择的浏览器（`firefox`、`chromium`、`google-chrome` 或
`microsoft-edge-stable-bin`）：

```bash
paru -Rns sway xorg-xwayland swaybg swayidle waybar swaync foot fuzzel nwg-look \
  qt5-wayland qt6-wayland yazi oo7 seahorse adw-gtk-theme papirus-icon-theme \
  ttf-sarasa-gothic maplemono-nf-cn-unhinted noto-fonts-emoji noto-fonts \
  ttf-nerd-fonts-symbols-mono wl-clipboard xdg-utils jack2
```

如果 pacman 因为你要保留的某个东西仍然依赖该列表中的包而拒绝执行，请把那个包从命令中去掉再重新运行。例如保留
Chromium、Chrome 或 Edge 会让 `xdg-utils` 仍然被需要，保留 Firefox 会让某个字体包作为它的 `ttf-font`
提供者仍然被需要。

`jack2` 只是作为 Waybar 的 JACK 提供者被安装，`oo7` 只在没有其他 Secret Service 后端时才安装；如果安装脚本跳过了它们，它们就不存在。可选的
Yazi 辅助工具（`fd`、`ripgrep`、`fzf`、`zoxide`、`jq`、`7zip`、
`ffmpeg`、`poppler`、`resvg`、`imagemagick`）从来不由本项目安装。

移除文件：

```bash
rm -rf ~/.config/sway ~/.config/waybar ~/.config/swaync ~/.config/swaynag \
       ~/.config/foot ~/.config/fuzzel ~/.config/yazi ~/.config/arch-sway-wslg
rm -rf ~/.local/libexec/arch-sway-wslg ~/.local/state/arch-sway-wslg
rm -f  ~/.local/bin/arch-sway-wslg
```

如果你接受了桌面条目屏蔽文件，还要删除 `~/.local/share/applications` 下那些 `Hidden=true` 的文件 （
`avahi-discover.desktop`、`bssh.desktop`、`bvnc.desktop`、`foot-server.desktop`、`footclient.desktop`、
`lstopo.desktop`、`qv4l2.desktop`、`qvidcap.desktop`、`xgps.desktop`、`xgpsspeed.desktop`）。通过 GSettings 设置的 GTK 外观值保存在
dconf 中；用 `gsettings reset-recursively org.gnome.desktop.interface` 重置它们。

## 故障排查

先运行诊断：

```bash
arch-sway-wslg doctor
```

`doctor` 会检查 systemd 用户管理器与运行时、会话所需的每一个命令（合成器、空闲通知器、`wl-clipboard`，以及 Sway
配置启动的那些应用）、剪贴板桥接、WSLg 映射、Sway 配置是否可读以及音频连通性。它从不请求 sudo，也不改动挂载状态。

如果 WSLg 的 Wayland、PulseAudio 或 X11 映射缺失，请关闭 WSL 并在 Windows 中执行 `wsl --shutdown`，然后再试。

如果按一次键却输入了两个字符，说明外层剪贴板正在你打字时被读取。请执行
`arch-sway-wslg status`：剪贴板那一行会说明这些读取是否以会话空闲为前提进行门控。如果没有，请在
`arch-sway-wslg logs` 中查找 `swayidle` 警告，或者用 `ARCH_SWAY_WSLG_CLIPBOARD=to-windows` 启动会话，直接停止这些读取。

宿主休眠、网络变化、显示器或任务栏变化，以及 WSLg Weston 故障，都可能中断父级 Wayland 连接，从而终止嵌套的 Sway 会话。启动器会清理托管的
cgroup，但不会在父合成器不健康时自动重启 Sway。请检查 `/mnt/wslg/weston.log` 与
`/mnt/wslg/versions.txt`，执行 `wsl --update`，等 WSLg 恢复正常后再执行 `arch-sway-wslg stop`，然后
`arch-sway-wslg start`。

在由 Sway 启动的 Foot 终端里，即使第一个 X11 应用还没启动，`echo "$DISPLAY"` 也应当输出 Sway 预留的嵌套显示号。若结果为空，说明
XWayland 初始化失败；请检查 `arch-sway-wslg logs`。 嵌套显示号由 Sway 选择，因此不必与 WSLg 父级的 `:0` 一致。若要显式验证
X11 路径，可在 Sway 内运行
`GDK_BACKEND=x11 nwg-look`。

如果托管会话卡死，使用 `arch-sway-wslg stop`；systemd 会停止整个会话 cgroup。切勿删除
`/tmp/.X11-unix`。

## 设计说明与限制

**私有 X11 挂载命名空间。** 发行版范围内的 `/tmp/.X11-unix` 映射由 WSLg 拥有并以只读方式挂载，因此嵌套的 XWayland
无法在其中创建套接字。启动器只为托管的 Sway 进程树在其专属挂载命名空间中提供一个 `01777` 的 X11 套接字目录。一个简短、固定的
sudo 步骤负责创建该命名空间与绑定挂载，随后立即用 `runuser`
退回到你的用户身份。父级 WSLg 映射从不被卸载、删除或替换，`/etc/wsl.conf` 也从不被修改。命名空间随会话一起消失。

**systemd 运行时。** 启动器要求使用 systemd 仅属主可访问的 `/run/user/$UID` 运行时，并把控制文件放在
`/run/user/$UID/arch-sway-wslg`。它会忽略 WSLg 共享的 `XDG_RUNTIME_DIR` 值，同时仍以绝对路径连接位于
`/mnt/wslg/runtime-dir/wayland-0` 的 WSLg Wayland 套接字。

**Wayland 优先。** 启动器会移除继承来的 `WLR_BACKENDS` 值，使 Sway 能选择其嵌套 Wayland 后端，但会保留你设置的渲染器变通选项。应用自行选择
Wayland 或 XWayland。使用分数缩放的 XWayland 应用可能不如原生 Wayland 应用清晰。

**私有 D-Bus 及其代价。** 每个托管会话都运行一个私有的 `dbus-run-session`。好处是在 Sway 内被激活的服务会继承嵌套显示，而不是打开到外层
WSLg 桌面上，并且一切都随会话结束。代价是真实存在的，值得了解：

- 持久用户总线上的服务无法从会话内访问，而会话内的应用可能激活某个服务的 *第二个*实例，例如 `oo7` Secret
  Service。避免同时在两条总线上使用消费密钥的应用。
- GSettings 值不受影响：dconf 把它们存放在共享文件中，因此安装脚本写入的值在会话内可见；只有变更通知无法跨总线传递。
- 加入 XDG Desktop Portal 后端需要针对这条私有总线做专门集成，这也是基于 portal 的文件选择器、Flatpak portal 访问与
  Wayland 屏幕共享不在支持范围内的原因。

**范围。** 最多支持四个嵌套输出。外层 WSLg 故障后的自动重启、portal、屏幕共享，以及从 Linux 侧移动 WSLg 窗口都不支持。

## 致谢

嵌套 Sway 的思路以及驱动多个嵌套输出的想法来自
[jordankoehn/sway-wsl2](https://github.com/jordankoehn/sway-wsl2)。

其他参考：

- [Sway sample configuration](https://github.com/swaywm/sway/blob/master/config.in)
- [Sway manual](https://man.archlinux.org/man/sway.5.en)
- [Microsoft WSLg](https://github.com/microsoft/wslg)
- [Waybar](https://github.com/Alexays/Waybar)
- [SwayNC](https://github.com/ErikReider/SwayNotificationCenter)
- [Yazi](https://yazi-rs.github.io/)
- [Catppuccin](https://catppuccin.com/)
- [walls-catppuccin-mocha](https://github.com/orangci/walls-catppuccin-mocha)
- [Maple Mono](https://github.com/subframe7536/maple-font)
- [Sarasa Gothic](https://github.com/be5invis/Sarasa-Gothic)

## 许可

软件与配置以 MIT 许可发布；内置的 `dark-star.jpg` 壁纸不在 MIT 授权范围内，其上游图集也未声明再分发许可。参见
[LICENSE](LICENSE)。
