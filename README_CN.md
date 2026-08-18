# arch-sway-wslg

[English](README.md) | [简体中文](README_CN.md)

`arch-sway-wslg` 在 Microsoft WSL2/WSLg 中安装并运行一套精心调校、Wayland 优先的 Sway 会话。它面向 WSL 上的 Arch
Linux，并非通用的裸机 Sway 发行配置。

<img alt="Sway" src="https://github.com/user-attachments/assets/7bbac63f-4e80-4c44-9ff2-a9dc18fccefc" style="max-width: 1200px; width: 100%;" />

## 特性

- 上游 Sway，X11 应用通过 XWayland 支持
- Waybar、SwayNC、Fuzzel、Foot、swaynag、nwg-look 与 Yazi
- 整套桌面统一使用 Catppuccin Mocha 配色
- 界面字体 Sarasa UI SC，终端字体 Maple Mono NF CN
- 在 Sway 中复制的纯文本可以在 Windows 中粘贴，反之亦然
- 最多四块 Sway 屏幕，每块都是独立的 Windows 窗口
- 个人设置放在自定义覆盖文件中，更新会原样保留
- 可自行选择浏览器，会话内的链接会用它打开
- 为 Seahorse、浏览器与 IDE 提供密码钥匙串，并可自动解锁
- 通过 WSLg 输出声音
- 一条命令即可启动、停止、查看与诊断会话

默认会话刻意保持精简。截图与窗口管理由 Windows 负责；桌面内不安装锁屏、电源管理、电池、网络或截图工具。

## 前置条件

请先完成 [ArchWiki 上的 WSL Arch Linux 安装指南](https://wiki.archlinux.org/title/Install_Arch_Linux_on_WSL)。安装脚本要求：

1. Arch Linux 运行在启用了 WSLg 的 WSL2 中。
2. 已将一个普通用户配置为 WSL 默认用户，并可正常使用 `sudo`。
3. 已启用 systemd，且该普通用户的 systemd 用户管理器工作正常。
4. 已配置 WSLg 硬件加速，并保持 Windows 与宿主 GPU 驱动为最新。
5. 为该普通用户安装 `base-devel`、Git 与 `paru`。

状态栏、启动器与 Yazi 会显示非 ASCII 文本，因此建议使用 UTF-8 locale；`C.UTF-8` 也可以。安装脚本不会改动 locale 设置。

在 Windows 侧保持 WSL 为最新：

```powershell
wsl --update
wsl --shutdown
```

systemd 是必需的。用 `wsl --install -d archlinux` 安装的官方 Arch 镜像默认已启用。若发行版较旧或为导入而来，且
`systemctl status` 提示 systemd 未运行，请在 `/etc/wsl.conf` 中加入以下内容，再从 Windows 执行 `wsl --shutdown`：

```ini
[boot]
systemd=true
```

如果渲染不稳定，请先更新 Windows、执行 `wsl --update`，并安装宿主 GPU 的最新驱动。

## 快速开始

以普通 Arch 用户身份执行下列命令，切勿使用 root：

```bash
git clone https://github.com/Acris/arch-sway-wslg.git
cd arch-sway-wslg
./install.sh
```

请先用 `paru -Syu` 升级系统：安装脚本只刷新软件包数据库，不会自行升级已安装的软件包。

接受 `paru` 展示的 AUR PKGBUILD 之前请先审阅。安装脚本会检查前置条件，并询问桌面条目屏蔽文件、浏览器、输出缩放、备份、 钥匙串解锁与
GTK 外观设置。在替换当前配置之前，它会先把要安装的内容检查一遍，也只会在询问后停止正在运行的会话。

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

`start` 与 `restart` 会为搭建会话一次性请求 sudo 密码，之后 Sway 与所有桌面应用都以普通用户身份运行。`stop` 不需要
sudo，它会结束整个会话，包括会话启动的一切。

## 多显示器

Sway 可以显示 1 到 4 块屏幕，每块都是独立的 Windows 窗口：

```bash
arch-sway-wslg start --outputs 2
```

`ARCH_SWAY_WSLG_OUTPUTS=2 arch-sway-wslg start` 效果相同。两种形式都只接受 1 到 4 的整数。屏幕依次命名为 `WL-1`、
`WL-2`，工作区的分配写在 `~/.config/sway/config.d/10-local.conf` 里：

```
workspace 1 output WL-1
workspace 2 output WL-1
workspace 9 output WL-2
workspace 10 output WL-2
```

这些窗口的移动与最大化请使用 Windows 快捷键，例如 `Win+Shift+Left/Right` 与 `Win+Up`。本项目不会自动排布它们。

## 自定义而不丢失修改

托管的配置目录在每次安装时都会被整体替换。下列路径始终属于用户，永远不会被替换：

| 路径                             | 用途                              |
|----------------------------------|-----------------------------------|
| `~/.config/sway/config.d/*.conf` | Sway 设置，在其他所有配置之后读取 |
| `~/.config/foot/local.ini`       | Foot 选项，在随附选项之后生效     |
| `~/.config/fuzzel/local.ini`     | Fuzzel 选项，在随附选项之后生效   |
| `~/.config/waybar/local.css`     | Waybar 样式，在随附样式表之后生效 |
| `~/.config/swaync/local.css`     | SwayNC 样式，在随附样式表之后生效 |

安装脚本会在首次安装时带注释示例创建它们，之后一律保留。写在其中的设置优先：

```
# ~/.config/sway/config.d/10-local.conf
output * scale 1.5
bindsym $mod+p exec firefox
```

```css
/* ~/.config/waybar/local.css */
* {
    font-size: 16px;
}
```

两个样式表都在随附样式表之后读取，因此其中的规则优先。即使内容为空也请保留这两个文件：删除任意一个都会让 Waybar 或 SwayNC
失去样式。

Waybar 与 SwayNC 的布局、swaynag 以及 Yazi 没有可以安全使用的 include 机制，因此这些文件完全托管。个人版本请放在托管目录
之外，或使用每次更新前提供的备份。

以下目录会被替换（若设置了 `XDG_CONFIG_HOME`，它们位于该目录下而非 `~/.config`）：

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

启动器会准备好会话所需的一切：屏幕、声音，以及应用会去查找的桌面标识。环境中已有的取值会被保留，Qt、Java、VS Code
等应用的常用默认值只在尚未设置时才补齐。

安装时选择的浏览器记录在 `~/.config/arch-sway-wslg/browser` 中。编辑该文件（其中只有一个可执行文件名）或导出 `BROWSER`
即可更改。

## 快捷键

| 按键                         | 动作                     |
|------------------------------|--------------------------|
| `Alt+Enter`                  | 打开 Foot                |
| `Alt+D`                      | 打开 Fuzzel              |
| `Alt+Y`                      | 在 Foot 中打开 Yazi      |
| `Alt+Shift+V`                | 立即读取 Windows 剪贴板  |
| `Alt+H/J/K/L` 或方向键       | 移动焦点                 |
| `Alt+Shift+H/J/K/L` 或方向键 | 移动当前容器             |
| `Alt+1..0`                   | 切换到工作区 1–10        |
| `Alt+Shift+1..0`             | 把容器移动到工作区 1–10  |
| `Alt+B/V`                    | 选择水平或垂直分割       |
| `Alt+S/W/E`                  | 选择堆叠、标签或分割布局 |
| `Alt+F`                      | 切换全屏                 |
| `Alt+Shift+F`                | 切换浮动                 |
| `Alt+R`                      | 进入调整大小模式         |
| `Alt+Shift+N`                | 切换 SwayNC 控制中心     |
| `Alt+Ctrl+N`                 | 切换勿扰模式             |
| `Alt+Shift+Q`                | 关闭当前窗口             |
| `Alt+Shift+C`                | 重载 Sway 配置           |
| `Alt+Shift+E`                | 确认并退出 Sway 会话     |

`Alt+Tab` 与 `Alt+Space` 归 Windows 所有，因此配置避开了这两个组合。截图仍可使用 Windows 的 `Win+Shift+S`。如需专用修饰
键，可在 `~/.config/sway/config.d/` 中把 `$mod` 设为 `Mod3`，再用 Windows 上的键盘重映射工具把某个 Windows 键映射为 Mod3。

## 剪贴板

在 Sway 中复制的文本可以在 Windows 中粘贴，在 Windows 中复制的文本也可以在 Sway 中粘贴。共享随会话一起启动与停止。

- 只共享纯文本：图片、HTML 与文件列表不在其中。
- 应用标记为敏感的选区（例如密码管理器中的条目）默认跳过。

在 Sway 中复制会立即送达 Windows。反方向会稍晚一些：会话会等输入出现短暂停顿（默认两秒）之后再去查看 Windows 剪贴板。 通常在切回
Sway 窗口时文本已经就位；按 `Alt+Shift+V` 可立即取回。执行 `arch-sway-wslg status` 可以查看会话是否正在读取 Windows 剪贴板。

以下变量需在 `arch-sway-wslg start` **之前** 导出；在会话内的终端里修改不会生效：

```bash
# how often the Windows clipboard is checked, in seconds; values below 0.2 are rejected
export ARCH_SWAY_WSLG_CLIPBOARD_POLL=5

# how long a pause in typing to wait for, in whole seconds (minimum 1)
export ARCH_SWAY_WSLG_CLIPBOARD_IDLE=5

# only send Sway -> Windows, never bring the Windows clipboard in
export ARCH_SWAY_WSLG_CLIPBOARD=to-windows

# no clipboard sharing at all
export ARCH_SWAY_WSLG_CLIPBOARD=off
```

关闭该方向后，`Alt+Shift+V` 也随之失效。

敏感选区也可以一并共享，但在使用密码管理器时不建议这样做：

```bash
export ARCH_SWAY_WSLG_SYNC_SENSITIVE=1
arch-sway-wslg start
```

## Waybar 布局

状态栏右侧显示五个元素：资源、音量、托盘、通知与时钟。内存占用常驻显示，鼠标悬停时向外展开 CPU 与磁盘占用，让系统信息 既能看到又不拥挤。

## 外观

安装脚本会先显示当前取值，再询问是否应用下列 GTK 默认设置。该询问默认为“是”，回答“否”则保持现状不变。

- GTK 主题：`adw-gtk3-dark`
- 配色方案：`prefer-dark`
- 图标主题：`Papirus-Dark`
- 界面字体：`Sarasa UI SC 11`
- 光标主题：`Adwaita`

桌面组件随附 Catppuccin Mocha 配色。在 Sway 中运行 `nwg-look` 可以查看或修改 GTK、图标、字体与光标设置。

安装脚本会询问 1 到 4 之间的输出缩放，支持 `1.25` 这样的小数。请与 Windows 显示缩放保持一致（`125%` 对应 `1.25`，
`150%` 对应 `1.5`），也可以之后在 `~/.config/sway/config.d/` 中用 `output * scale 1.25` 修改。

随附壁纸来自 [walls-catppuccin-mocha](https://github.com/orangci/walls-catppuccin-mocha) 合集，不在本项目的 MIT 许可范围
内。其上游图片许可未作说明，分发者在再分发前必须自行确认授权。

## Yazi

按 `Alt+Y` 可在 Foot 中打开 Yazi。更多内容参见
[Yazi 快速上手快捷键](https://yazi-rs.github.io/docs/quick-start/#keybindings)
与[安装指南](https://yazi-rs.github.io/docs/installation/)。

随附主题遵循 [catppuccin/yazi](https://github.com/catppuccin/yazi)，并附带用于文件预览语法高亮的 Catppuccin Mocha 主题。

安装成功后，脚本会打印两条推荐命令：

```bash
paru -S --needed fd ripgrep fzf zoxide jq 7zip        # search, navigation, JSON, archives
paru -S --needed ffmpeg poppler resvg imagemagick     # rich previews
```

Foot 通过 Sixel 渲染 Yazi 的图片预览。本项目不会改动 shell 启动文件；如需目录跟随，请自行添加
[Yazi shell wrapper](https://yazi-rs.github.io/docs/quick-start/#shell-wrapper)。

## 更新

```bash
git pull --ff-only
./install.sh
```

每次运行都会在替换托管文件前提供一次带时间戳的备份。备份中包含 `RESTORE-INFO.txt`，并且永远不会被自动删除。更新后请重新
回答安装脚本的提问，然后执行 `arch-sway-wslg restart`。系统本身请按自己的节奏用 `paru -Syu` 升级，安装脚本不会代劳。

## 卸载

先停止会话；若安装脚本启用过钥匙串守护进程，也一并停止：

```bash
arch-sway-wslg stop
systemctl --user disable --now oo7-daemon.service
rm -f ~/.config/credstore.encrypted/oo7.keyring-encryption-password
```

移除本项目安装的软件包。想保留的请从命令中删除，并把所选浏览器追加进去（`firefox`、`chromium`、`google-chrome`、
`microsoft-edge-stable-bin` 或 `brave-bin`）：

```bash
paru -Rns sway xorg-xwayland swaybg swayidle waybar swaync foot fuzzel nwg-look \
  qt5-wayland qt6-wayland yazi oo7 seahorse adw-gtk-theme papirus-icon-theme \
  ttf-sarasa-gothic maplemono-nf-cn-unhinted noto-fonts-emoji noto-fonts \
  ttf-nerd-fonts-symbols-mono wl-clipboard xdg-utils jack2
```

如果 pacman 提示某个软件包仍被需要，请把它从命令中去掉再执行一次。`jack2` 与 `oo7` 在部分环境下是可选的；Yazi 的辅助软件
包本项目从不安装。

删除文件：

```bash
rm -rf ~/.config/sway ~/.config/waybar ~/.config/swaync ~/.config/swaynag \
       ~/.config/foot ~/.config/fuzzel ~/.config/yazi ~/.config/arch-sway-wslg
rm -rf ~/.local/libexec/arch-sway-wslg ~/.local/state/arch-sway-wslg
rm -f  ~/.local/bin/arch-sway-wslg
```

若当初接受了桌面条目屏蔽文件，它们是 `~/.local/share/applications` 下带 `Hidden=true` 的文件，需要在那里删除。GTK 外观取
值保存在 dconf 中，必要时可用 `gsettings reset-recursively org.gnome.desktop.interface` 重置。

## 故障排查

先运行诊断：

```bash
arch-sway-wslg doctor
```

`doctor` 会检查 systemd、会话所需的程序、剪贴板共享、WSLg 集成、Sway 配置与音频。它只做检查：既不请求 sudo，也不改动任何 东西。

如果缺少 WSLg 的 Wayland、PulseAudio 或 X11 映射，请关闭 WSL 并在 Windows 中执行 `wsl --shutdown`，然后重试。

如果按一次键有时会出现两个字符，说明读取 Windows 剪贴板的时机不合适。查看 `arch-sway-wslg status` 中的剪贴板一行以及
`arch-sway-wslg logs` 中的告警；用 `ARCH_SWAY_WSLG_CLIPBOARD=to-windows` 启动会话可以关闭该方向。

如果通知始终不出现，请运行 `arch-sway-wslg doctor`。若有其他进程占用 `org.freedesktop.Notifications`，用
`systemctl --user stop swaync.service` 停止它，再重启会话。

钥匙串对同一 WSL 用户下的所有应用可用。安装了 oo7 时，安装脚本会随用户会话启动它，并询问是否保存钥匙串密码以便自动解锁；
该询问需要 systemd 258 或更高版本，否则会被跳过。如果 Seahorse、`secret-tool`、浏览器或 IDE 反复索要密码，可重新运行安装
脚本并接受该询问，也可以手动完成：

```bash
systemctl --user enable --now oo7-daemon.service
mkdir -p ~/.config/credstore.encrypted
systemd-ask-password -n | systemd-creds encrypt --user \
  --name=oo7.keyring-encryption-password - \
  ~/.config/credstore.encrypted/oo7.keyring-encryption-password
systemctl --user restart oo7-daemon.service
```

即使 shell 设置了其他 `XDG_CONFIG_HOME`，该文件也请放在 `~/.config` 下。任何能读取该文件并使用 TPM 的人（包括 root）都能
解密它。另一种方式是每次开机执行一次 `oo7-cli unlock`。两种方式参见 [ArchWiki: Oo7](https://wiki.archlinux.org/title/Oo7)。

如果 WSLg 在休眠、显示变更或更新后停止响应，Sway 会话可能随之结束。请查看 `/mnt/wslg/weston.log`，执行 `wsl --update`， 待
WSLg 恢复正常后再启动会话；启动器不会自行处理。

如果 X11 应用无法运行，请在 Foot 终端中执行 `echo "$DISPLAY"` 并检查 `arch-sway-wslg logs`。X11 通路可用
`GDK_BACKEND=x11 nwg-look` 测试。

如果会话卡住，`arch-sway-wslg stop` 总能结束它。切勿删除 `/tmp/.X11-unix`。

## 限制

本项目专为 WSL2/WSLg 上的 Arch Linux 设计：

- 截图、任务栏行为以及桌面窗口的位置都由 Windows 负责。本项目不会排布或移动它们，WSLg 自身失效时也不会自动恢复。
- 钥匙串、通知这类桌面服务与同一 WSL 用户下的其他进程共享，这正是它们能在此工作的原因。会话之外启动的服务在 `stop`
  之后仍会继续运行。
- 只有由 Sway 启动的应用才会获得会话的设置。
- 不支持 portal、Flatpak 集成、屏幕共享，也不支持从 Linux 侧移动桌面窗口。
- 最多支持四块屏幕；使用 XWayland 的应用可能不如原生 Wayland 应用清晰。

## 致谢

嵌套 Sway 的思路以及使用多块屏幕的想法来自
[jordankoehn/sway-wsl2](https://github.com/jordankoehn/sway-wsl2)。

其他参考：

- [Sway 示例配置](https://github.com/swaywm/sway/blob/master/config.in)
- [Sway 手册](https://man.archlinux.org/man/sway.5.en)
- [Microsoft WSLg](https://github.com/microsoft/wslg)
- [Waybar](https://github.com/Alexays/Waybar)
- [SwayNC](https://github.com/ErikReider/SwayNotificationCenter)
- [Yazi](https://yazi-rs.github.io/)
- [Catppuccin](https://catppuccin.com/)
- [walls-catppuccin-mocha](https://github.com/orangci/walls-catppuccin-mocha)
- [Maple Mono](https://github.com/subframe7536/maple-font)
- [Sarasa Gothic](https://github.com/be5invis/Sarasa-Gothic)

## 许可

软件与配置采用 MIT 许可；随附的 `dark-star.jpg` 壁纸不在 MIT 授权范围内，其上游合集也未说明再分发许可。参见
[LICENSE](LICENSE)。
