# arch-sway-wslg

[English](README.md) | [简体中文](README_CN.md)

`arch-sway-wslg` 在 Microsoft WSL2/WSLg 中安装并运行一套 Wayland 优先的 Sway 桌面会话。它面向 WSL 上的 Arch Linux，并非通用的裸机
Sway 发行版。

<img alt="Sway" src="https://github.com/user-attachments/assets/7bbac63f-4e80-4c44-9ff2-a9dc18fccefc" style="max-width: 1200px; width: 100%;" />

## 特性

- 包含 Waybar、SwayNC、Fuzzel、Foot、swaynag、nwg-look 与 Yazi 的 Sway 会话
- Catppuccin Mocha 配色方案，界面使用 Sarasa UI SC 字体，终端使用 Maple Mono NF CN 字体
- 通过 XWayland 支持 X11 应用，通过 WSLg 支持声音
- 在 Sway 中复制的纯文本可以在 Windows 中粘贴，反之亦然
- 最多支持四块 Sway 屏幕，每块屏幕都在独立的 Windows 窗口中运行
- 可选浏览器，以及支持自动解锁的密码钥匙串
- 个人设置存放在覆盖文件中，更新时原样保留
- 一条命令即可启动、停止、查看与诊断桌面

桌面环境刻意保持精简。Windows 负责截图与窗口管理，因此不安装锁屏、电源管理、电池、网络或截图工具。

## 前置条件

请先完成 [ArchWiki 上的 WSL Arch Linux 安装指南](https://wiki.archlinux.org/title/Install_Arch_Linux_on_WSL)。安装程序要求：

1. Arch Linux 运行在启用了 WSLg 的 WSL2 中，并保持 Windows 与 GPU 驱动为最新。
2. 已将一个普通用户配置为默认 WSL 用户，并拥有正常的 `sudo` 权限。
3. 已启用 systemd，且该用户的 systemd 用户管理器正常运行。
4. 已安装 `base-devel`、Git 与 `paru`。
5. 使用 UTF-8 locale 以正常显示状态栏、启动器与 Yazi 中的非 ASCII 字符；`C.UTF-8` 即可。locale 设置将保持不变。

在 Windows 中保持 WSL 为最新：

```powershell
wsl --update
wsl --shutdown
```

如果 `systemctl status` 显示 systemd 未运行，请在 `/etc/wsl.conf` 中添加以下内容并从 Windows 执行 `wsl --shutdown`：

```ini
[boot]
systemd=true
```

关闭最后一个终端后，WSL 可能会停止 Arch 实例，Sway 窗口也会随之消失。使用桌面期间可保持一个 Arch 终端运行，或在
`%UserProfile%\.wslconfig` 中同时禁用两个空闲计时器：

```ini
[general]
instanceIdleTimeout=-1

[wsl2]
vmIdleTimeout=-1
```

修改文件后，请在 Windows 中运行 `wsl --shutdown`，然后重新启动
WSL。详见 [microsoft/WSL#13291](https://github.com/microsoft/WSL/issues/13291)。

## 安装

以普通 Arch 用户身份运行以下命令，切勿使用 root：

```bash
paru -Syu
git clone https://github.com/Acris/arch-sway-wslg.git
cd arch-sway-wslg
./install.sh
```

安装程序会先询问桌面条目屏蔽、浏览器、输出缩放与备份，然后安装软件包；钥匙串解锁与 GTK
外观设置在装包之后才会询问，因此软件包安装完成后仍需再次留意。只有在整套载荷完成暂存与检查后，才会替换托管配置。在接受
`paru` 展示的 AUR PKGBUILD 前请先审阅。

系统升级应使用 `paru -Syu`，因为 Arch 不支持部分升级：当刷新后的软件包数据库显示系统落后时，安装程序会报告并停止，除非明确确认继续。

然后启动桌面：

```bash
arch-sway-wslg doctor
arch-sway-wslg start
arch-sway-wslg status
```

如果启动未成功完成，请使用 `arch-sway-wslg logs` 查看日志。

## 命令

```bash
arch-sway-wslg start [--outputs N]    # 以 N 块屏幕启动（1-4，默认 1）
arch-sway-wslg stop
arch-sway-wslg restart [--outputs N]
arch-sway-wslg status
arch-sway-wslg logs
arch-sway-wslg doctor
arch-sway-wslg version
```

`start` 与 `restart` 在配置会话时会请求一次 sudo 密码；随后 Sway 及所有桌面应用均以普通用户身份运行。`stop` 不需要
sudo，且会结束会话启动的所有进程。

## 快捷键

| 按键                         | 动作                     |
|------------------------------|--------------------------|
| `Alt+Enter`                  | 打开 Foot                |
| `Alt+D`                      | 打开 Fuzzel              |
| `Alt+Y`                      | 在 Foot 中打开 Yazi      |
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

Windows 占用 `Alt+Tab` 与 `Alt+Space`，因此配置避开了这些组合；截图仍使用 Windows 的 `Win+Shift+S`。

## 多显示器

Sway 可以显示一到四块屏幕，每块屏幕都在独立的 Windows 窗口中：

```bash
arch-sway-wslg start --outputs 2
```

`ARCH_SWAY_WSLG_OUTPUTS=2 arch-sway-wslg start` 效果相同；两种形式均接受 1 到 4 的整数。屏幕命名为 `WL-1`、`WL-2`
等，可以为它们分配工作区：

```
# ~/.config/sway/config.d/10-local.conf
workspace 1 output WL-1
workspace 9 output WL-2
```

使用 `Win+Shift+Left/Right` 与 `Win+Up` 等 Windows 快捷键移动或最大化这些窗口。本项目不会自动排布它们。

## 自定义

托管配置目录在每次安装时都会被替换。以下路径始终属于用户，在读取随附文件后加载，且永远不会被替换：

| 路径                             | 用途        |
|----------------------------------|-------------|
| `~/.config/sway/config.d/*.conf` | Sway 设置   |
| `~/.config/foot/local.ini`       | Foot 选项   |
| `~/.config/fuzzel/local.ini`     | Fuzzel 选项 |
| `~/.config/waybar/local.css`     | Waybar 样式 |
| `~/.config/swaync/local.css`     | SwayNC 样式 |

安装程序在首次安装时会创建包含注释示例的文件，并在之后保留。这些文件中的设置优先：

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

即使样式表为空也请保留：删除其中之一会导致 Waybar 或 SwayNC 失去样式。Waybar 与 SwayNC 布局、swaynag 以及 Yazi 没有安全的
include 机制，属于完全托管，因此请将个人版本存放在托管目录之外，或使用安装程序提供的备份。

以下目录会被替换（如果设置了 `XDG_CONFIG_HOME`，则位于该目录下而非 `~/.config`）：

```text
~/.config/foot
~/.config/fuzzel
~/.config/sway
~/.config/swaynag
~/.config/swaync
~/.config/waybar
~/.config/yazi
```

安装时选择的浏览器记录在 `~/.config/arch-sway-wslg/browser` 中，用于打开会话内点击的链接。可以通过编辑该文件（包含单个可执行文件名）或导出
`BROWSER` 来修改。

## 剪贴板

在 Sway 中复制的文本可以在 Windows 中粘贴，在 Windows 中复制的文本也可以在 Sway 中粘贴。共享功能随会话启动而开启，随会话停止而关闭。

- 仅共享纯文本：不支持图片、HTML 与文件列表。
- 默认跳过被应用标记为敏感的选区，例如来自密码管理器的条目。

两个方向都不是即时的，因为共享的时机刻意避开了输入。在 Sway 中复制的内容会在片刻后到达 Windows。反方向则会先等待输入出现停顿，再去查看
Windows 剪贴板，且会话闲置越久查看越稀；在 Windows 中复制的文本通常在切回 Sway 窗口时已经就位，如果粘贴得更早，稍作停顿即可。执行
`arch-sway-wslg status` 可查看当前状态。

在执行 `arch-sway-wslg start` 前导出以下任意变量；在会话内的终端设置它们不会生效：

```bash
# 输入停顿后检查 Windows 剪贴板的间隔，单位为秒（最小 0.2）
export ARCH_SWAY_WSLG_CLIPBOARD_POLL=5

# 需要等待的输入停顿时长，单位为整数秒（最小 1）
export ARCH_SWAY_WSLG_CLIPBOARD_IDLE=5

# 只把 Sway 的内容送往 Windows，不再取回 Windows 剪贴板
export ARCH_SWAY_WSLG_CLIPBOARD=to-windows

# 完全关闭剪贴板共享
export ARCH_SWAY_WSLG_CLIPBOARD=off

# 一并共享敏感选区；与密码管理器同时使用时不推荐
export ARCH_SWAY_WSLG_SYNC_SENSITIVE=1
```

## 外观

尺寸遵循各随附程序的默认值；字体、颜色与状态栏的圆角元素是刻意进行的调整。两个 `local.css` 文件均可对其进行修改。

状态栏右侧显示资源占用、音量、托盘、通知与时钟。内存占用始终可见；鼠标悬停时会通过抽屉效果滑出 CPU 与磁盘占用。

安装程序会显示当前的 GTK 取值并在应用其默认设置前询问；回答“否”将保留当前值：

- GTK 主题：`adw-gtk3-dark`
- 配色方案：`prefer-dark`
- 图标主题：`Papirus-Dark`
- 界面字体：`Sarasa UI SC 11`
- 光标主题：`Adwaita`

在 Sway 内运行 `nwg-look` 可审阅或修改 GTK、图标、字体与光标设置。

安装程序还会询问 1 到 4 之间的输出缩放，支持小数。请将其与 Windows 显示缩放匹配（`125%` 为 `1.25`，`150%` 为 `1.5`），或者稍后在
`~/.config/sway/config.d/` 中设置 `output * scale 1.25`。

随附壁纸为 [catppuccin-wallpapers](https://github.com/zhichaoh/catppuccin-wallpapers) 合集中的 `arch-black-4k.png`，采用
MIT 许可。可在 `~/.config/sway/config.d/` 中通过 `output * bg /path/to/image fill` 更换为其他图片。

## Yazi

按 `Alt+Y` 在 Foot 中打开 Yazi。其主题遵循 [catppuccin/yazi](https://github.com/catppuccin/yazi)
，并包含配套的文件预览语法高亮。快捷键与功能请参阅 [Yazi 文档](https://yazi-rs.github.io/docs/quick-start/)。

安装程序在运行成功后会打印两条推荐命令：

```bash
paru -S --needed fd ripgrep fzf zoxide jq 7zip        # 搜索、导航、JSON、压缩包
paru -S --needed ffmpeg poppler resvg imagemagick     # 丰富预览
```

图片预览通过 Sixel 渲染。由于本项目不编辑 shell
启动文件，必须手动添加 [Yazi shell wrapper](https://yazi-rs.github.io/docs/quick-start/#shell-wrapper)。

## 更新

```bash
git pull --ff-only
./install.sh
```

更新可能会增加软件包，因此请再次回答安装程序的问题，然后运行 `arch-sway-wslg restart`。每次运行都会在替换托管文件前提供一次带时间戳的备份；备份包含
`RESTORE-INFO.txt` 且永远不会被自动删除。

## 卸载

以下路径为默认值；若设置了 `XDG_CONFIG_HOME` 或 `XDG_STATE_HOME`，请相应调整。

请先停止会话，并停止安装程序启用的钥匙串守护进程：

```bash
arch-sway-wslg stop
systemctl --user disable --now oo7-daemon.service
rm -f ~/.config/credstore.encrypted/oo7.keyring-encryption-password
```

移除本项目安装的软件包。请去掉需要保留的内容，并追加安装时选择的浏览器（`firefox`、`chromium`、`google-chrome`、
`microsoft-edge-stable-bin` 或 `brave-bin`）：

```bash
paru -Rns sway xorg-xwayland swaybg swayidle waybar swaync foot fuzzel nwg-look \
  qt5-wayland qt6-wayland yazi oo7 seahorse adw-gtk-theme papirus-icon-theme \
  ttf-sarasa-gothic maplemono-nf-cn-unhinted noto-fonts-emoji noto-fonts \
  ttf-nerd-fonts-symbols-mono wl-clipboard xdg-utils jack2
```

如果 pacman 报告某个软件包仍被需要，请将其从命令中删除并重新运行；例如，保留的浏览器仍需要 `xdg-utils` 和字体包。

删除文件：

```bash
rm -rf ~/.config/sway ~/.config/waybar ~/.config/swaync ~/.config/swaynag \
       ~/.config/foot ~/.config/fuzzel ~/.config/yazi ~/.config/arch-sway-wslg
rm -rf ~/.local/libexec/arch-sway-wslg ~/.local/state/arch-sway-wslg
rm -f  ~/.local/bin/arch-sway-wslg
```

如果接受了桌面条目屏蔽，它们是 `~/.local/share/applications` 下的 `Hidden=true` 文件，必须在那里删除。GTK 外观取值保留在
dconf 中；如有需要，请使用 `gsettings reset-recursively org.gnome.desktop.interface` 重置。

## 故障排查

请先运行诊断：

```bash
arch-sway-wslg doctor
```

该命令会检查 systemd、桌面所需的程序、WSLg 集成以及音频，且不会做任何改动。

如果缺少 WSLg 的 Wayland、PulseAudio 或 X11 映射，请先关闭 WSL 并在 Windows 中运行 `wsl --shutdown`，然后重试。

如果按键有时会产生两个字符，说明 Windows 剪贴板的读取时机不当。请检查 `arch-sway-wslg status` 中的剪贴板相关行以及
`arch-sway-wslg logs` 中的警告；使用 `ARCH_SWAY_WSLG_CLIPBOARD=to-windows` 启动会话可关闭该方向。

如果通知始终不显示，请运行 `arch-sway-wslg doctor`。当另一个进程已占用 `org.freedesktop.Notifications` 时，请使用
`systemctl --user stop swaync.service` 停止它并重启会话。

钥匙串由该 WSL 用户下的所有程序共享。安装 oo7 后，安装程序会启动它并询问是否存储钥匙串密码以便自动解锁，这需要 systemd 258
或更高版本。如果 Seahorse、`secret-tool`、浏览器或 IDE 持续要求输入密码，请再次运行安装程序并接受该询问，或者手动完成：

```bash
systemctl --user enable --now oo7-daemon.service
mkdir -p ~/.config/credstore.encrypted
systemd-ask-password -n | systemd-creds encrypt --user \
  --name=oo7.keyring-encryption-password - \
  ~/.config/credstore.encrypted/oo7.keyring-encryption-password
systemctl --user restart oo7-daemon.service
```

即使 shell 设置了另一个 `XDG_CONFIG_HOME`，该文件也请使用 `~/.config` 路径。任何能够读取该文件并使用 TPM 的用户（包括
root）都可以解密它。另一种选择是每次启动运行一次 `oo7-cli unlock`
；详见 [ArchWiki: Oo7](https://wiki.archlinux.org/title/Oo7)。

如果 WSLg 在休眠、显示更改或更新后停止响应，Sway 会话可能会随之终止。请检查 `/mnt/wslg/weston.log`，运行 `wsl --update`，并在
WSLg 恢复健康后再次启动会话。

如果 X11 应用失败，请在 Foot 终端中运行 `echo "$DISPLAY"` 并检查 `arch-sway-wslg logs`。可以使用
`GDK_BACKEND=x11 nwg-look` 测试 X11 通路。

如果会话卡住，`arch-sway-wslg stop` 总能结束它。切勿删除 `/tmp/.X11-unix`。

## 限制

- Windows 负责处理截图、任务栏行为以及桌面窗口的定位。本项目不会移动它们，且在 WSLg 自身停止工作时不会自动恢复。
- 钥匙串、通知及其他桌面服务与 WSL 用户共享，这正是它们能在此工作的原因。在会话外启动的服务在执行 `stop` 后仍会继续运行。
- 不支持 portal、Flatpak 集成以及屏幕共享。
- 使用 XWayland 的应用可能不如原生 Wayland 应用清晰。

## 致谢

嵌套 Sway 的方法以及使用多块屏幕的想法源自 [jordankoehn/sway-wsl2](https://github.com/jordankoehn/sway-wsl2)。

其他参考资料：

- [Sway 示例配置](https://github.com/swaywm/sway/blob/master/config.in)
- [Sway 手册](https://man.archlinux.org/man/sway.5.en)
- [Microsoft WSLg](https://github.com/microsoft/wslg)
- [Waybar](https://github.com/Alexays/Waybar)
- [SwayNC](https://github.com/ErikReider/SwayNotificationCenter)
- [Yazi](https://yazi-rs.github.io/)
- [Catppuccin](https://catppuccin.com/)
- [catppuccin-wallpapers](https://github.com/zhichaoh/catppuccin-wallpapers)
- [Maple Mono](https://github.com/subframe7536/maple-font)
- [Sarasa Gothic](https://github.com/be5invis/Sarasa-Gothic)

## 许可

软件与配置采用 MIT 许可协议，随附壁纸与 Yazi 语法高亮主题同样采用 MIT 许可，并保留各自的上游版权声明。详见
[LICENSE](LICENSE)。
