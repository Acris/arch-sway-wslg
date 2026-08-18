# arch-sway-wslg

[English](README.md) | [简体中文](README_CN.md)

`arch-sway-wslg` 在 Microsoft WSL2/WSLg 中安装并运行一套精心调校、Wayland 优先的 Sway 会话。它面向 WSL 上的 Arch
Linux，并不是通用的裸机 Sway 发行配置。

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
- 为 Seahorse、浏览器与 IDE 提供 Secret Service 集成
- 集成 WSLg PulseAudio
- 一条命令即可启动、停止、查看与诊断会话

默认会话刻意保持精简。截图与外层 WSLg 窗口由 Windows 负责；客户机内不安装锁屏、电源管理、电池、网络或截图工具。

## 前置条件

请先完成 [ArchWiki 上的 WSL Arch Linux 安装指南](https://wiki.archlinux.org/title/Install_Arch_Linux_on_WSL)。安装脚本要求：

1. Arch Linux 运行在启用了 WSLg 的 WSL2 中。
2. 有一个普通用户被配置为 WSL 默认用户，且 `sudo` 可用。
3. 已启用 systemd，并且该普通用户的 systemd 用户管理器工作正常。
4. 已配置 WSLg 硬件加速。请保持 Windows 与宿主 GPU 驱动为最新版本。
5. 为该普通用户安装 `base-devel`、Git 与 `paru`。

安装脚本不会检查或修改 locale。状态栏、启动器与 Yazi 会显示非 ASCII 文本，因此建议使用 UTF-8 locale；`C.UTF-8` 也可以。

在 Windows 侧保持 WSL 为最新：

```powershell
wsl --update
wsl --shutdown
```

systemd 是必需的。当前由 `wsl --install -d archlinux` 安装的官方 Arch 镜像默认已启用它。如果是较旧或导入的发行版，
`systemctl status` 报告 systemd 未运行，请在
`/etc/wsl.conf` 中加入以下内容，然后在 Windows 中执行 `wsl --shutdown`：

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

接受 `paru` 展示的 AUR PKGBUILD 之前请先审阅。安装脚本会检查前置条件，询问桌面条目屏蔽文件、浏览器、输出缩放、备份和 GTK 外观，
然后暂存并检查完整载荷，最后才替换文件。它只会在询问后停止正在运行的托管会话。

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

`start` 与 `restart` 会为会话设置一次性请求 sudo，随后以你的普通用户身份在瞬态 systemd 用户 scope 中运行 Sway 及其应用。
`stop` 不需要 sudo。systemd scope 才是权威的会话状态。

## 多显示器

Sway 可以驱动 1 到 4 个嵌套输出。每个输出都是独立的顶层 WSLg 窗口：

```bash
arch-sway-wslg start --outputs 2
```

`ARCH_SWAY_WSLG_OUTPUTS=2 arch-sway-wslg start` 效果相同。两种形式都只接受 1 到 4 的整数。输出依次命名为
`WL-1`、`WL-2` 等；在 `~/.config/sway/config.d/10-local.conf` 中把工作区绑定到它们：

```
workspace 1 output WL-1
workspace 2 output WL-1
workspace 9 output WL-2
workspace 10 output WL-2
```

用 `Win+Shift+Left/Right` 与 `Win+Up` 等 Windows 快捷键移动或最大化 WSLg 窗口。本项目不会自动排列窗口。

## 自定义而不丢失修改

托管的配置目录在每次安装时都会被替换。以下路径始终属于你，永不被替换：

| 路径                             | 用途                              |
|----------------------------------|-----------------------------------|
| `~/.config/sway/config.d/*.conf` | Sway 设置，在其余全部内容之后读取 |
| `~/.config/foot/local.ini`       | Foot 选项，在内置选项之后应用     |
| `~/.config/fuzzel/local.ini`     | Fuzzel 选项，在内置选项之后应用   |

首次安装时，安装脚本会创建这些文件并附带注释示例，之后每次安装都会保留。写在其中的设置优先生效：

```
# ~/.config/sway/config.d/10-local.conf
output * scale 1.5
bindsym $mod+p exec firefox
```

Waybar、SwayNC、swaynag 与 Yazi 完全由项目托管。请把这些文件的个人版本保存在托管目录之外，或者使用每次更新前提供的备份。

以下目录会被替换（绝对路径的 `$XDG_CONFIG_HOME` 会替代 `~/.config`）：

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

启动器会设置嵌套显示、运行时目录、音频、桌面身份与持久用户总线。它会保留你自己的渲染器与工具包设置，只在你没有设置时为
Qt、Java、VS Code 等应用补充默认值。

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
UTF-8 纯文本。整个过程不涉及 Windows 辅助进程，也不需要 `powershell.exe`。

- 图片、HTML 与文件列表不参与同步。
- 源应用标记为 `sensitive` 的选区（密码管理器）默认被跳过。
- 桥接由 Sway 启动，因此与会话同生共死。

在 Sway 中复制的内容会立即转发。Windows 剪贴板的变化会在会话静止两秒后读取，不会干扰打字；按
`Alt+Shift+V` 可以立即读取。`arch-sway-wslg status` 会报告自动读取是否启用。

请在 `arch-sway-wslg start` **之前** export 下面的变量；在会话内的终端里修改它们没有效果：

```bash
# 读取间隔（秒）；小于 0.2 的值会被拒绝
export ARCH_SWAY_WSLG_CLIPBOARD_POLL=5

# 读取前的静止时间（整秒，最小 1）
export ARCH_SWAY_WSLG_CLIPBOARD_IDLE=5

# 只转发 Sway -> Windows，不再读取 Windows 剪贴板
export ARCH_SWAY_WSLG_CLIPBOARD=to-windows

# 完全不做剪贴板桥接
export ARCH_SWAY_WSLG_CLIPBOARD=off
```

关闭入向读取后，`Alt+Shift+V` 也不再工作。仍可用
`WAYLAND_DISPLAY=/mnt/wslg/runtime-dir/wayland-0 wl-paste` 按需读取 Windows 剪贴板。

若要包含敏感选区，请在启动 Sway 之前导出下面的变量；对密码管理器不建议这样做：

```bash
export ARCH_SWAY_WSLG_SYNC_SENSITIVE=1
arch-sway-wslg start
```

## Waybar 布局

状态栏右侧保留五个胶囊：资源、音量、托盘、通知与时钟。内存占用始终可见；把鼠标悬停在上面会滑出 CPU
与磁盘占用，这样既能看到系统信息，又不会让状态栏过于拥挤。

## 外观

安装脚本会显示当前值，并在应用下面这些 GTK 默认值前询问。该提示默认为“是”；回答“否”则当前值保持不变。

- GTK 主题：`adw-gtk3-dark`
- 配色方案：`prefer-dark`
- 图标主题：`Papirus-Dark`
- 界面字体：`Sarasa UI SC 11`
- 光标：`Adwaita`，尺寸 `28`

桌面组件都内置了 Catppuccin Mocha。在 Sway 内运行 `nwg-look` 可以查看或修改 GTK、图标、字体与光标设置。

安装脚本会询问 1 到 4 之间的输出缩放，也接受 `1.25` 这样的小数。请与 Windows 显示缩放匹配（`125%` 填 `1.25`，`150%` 填
`1.5`），之后也可在 `~/.config/sway/config.d/` 中用 `output * scale 1.25` 修改。

内置壁纸来自 [walls-catppuccin-mocha](https://github.com/orangci/walls-catppuccin-mocha) 图集，不在本项目 MIT
许可的授权范围内。上游没有声明图片许可，再分发者必须先确认获得许可。

## Yazi

按 `Alt+Y` 可在 Foot 中打开 Yazi。更多信息见 [Yazi 快速上手键位说明](https://yazi-rs.github.io/docs/quick-start/#keybindings)
与[安装指南](https://yazi-rs.github.io/docs/installation/)。

安装成功后，安装脚本会打印两条推荐命令：

```bash
paru -S --needed fd ripgrep fzf zoxide jq 7zip        # 搜索、导航、JSON、压缩包
paru -S --needed ffmpeg poppler resvg imagemagick     # 丰富的预览
```

Foot 通过 Sixel 渲染 Yazi 图片预览。本项目不会修改 shell 启动文件，如需目录跟踪请自行添加
[Yazi shell 包装函数](https://yazi-rs.github.io/docs/quick-start/#shell-wrapper)。

## 更新

```bash
git pull --ff-only
./install.sh
```

每次运行都会在替换托管文件前询问是否创建带时间戳的备份。备份包含 `RESTORE-INFO.txt`，且不会自动删除。更新后请重新回答安装脚本的问题，
然后执行 `arch-sway-wslg restart`。

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

如果 pacman 提示你要保留的内容仍依赖某个包，请从命令中删掉该包再运行。`jack2` 与 `oo7` 在部分安装中是可选的；Yazi
辅助工具从来不由本项目安装。

移除文件：

```bash
rm -rf ~/.config/sway ~/.config/waybar ~/.config/swaync ~/.config/swaynag \
       ~/.config/foot ~/.config/fuzzel ~/.config/yazi ~/.config/arch-sway-wslg
rm -rf ~/.local/libexec/arch-sway-wslg ~/.local/state/arch-sway-wslg
rm -f  ~/.local/bin/arch-sway-wslg
```

如果你接受了桌面条目屏蔽文件，还要删除 `~/.local/share/applications` 下对应的 `Hidden=true` 文件。通过 GSettings 设置的 GTK
外观值保存在 dconf 中；如有需要，用 `gsettings reset-recursively org.gnome.desktop.interface` 重置它们。

## 故障排查

先运行诊断：

```bash
arch-sway-wslg doctor
```

`doctor` 会检查 systemd、运行时与共用用户总线、重要的总线名字、所需命令、剪贴板桥接、WSLg 映射、Sway 配置与音频。
它从不请求 sudo，也不改动挂载状态。

如果 WSLg 的 Wayland、PulseAudio 或 X11 映射缺失，请关闭 WSL 并在 Windows 中执行 `wsl --shutdown`，然后再试。

如果按一次键却输入了两个字符，请执行 `arch-sway-wslg status` 并检查 `arch-sway-wslg logs`。可以用
`ARCH_SWAY_WSLG_CLIPBOARD=to-windows` 停止自动读取 Windows 剪贴板。

如果会话内始终收不到通知，请执行 `arch-sway-wslg doctor`。如果其他进程占用了
`org.freedesktop.Notifications`，请执行 `systemctl --user stop swaync.service`，然后重启会话。

Secret Service 可通过共用用户总线访问，但 oo7 钥匙串在 WSL 中可能以锁定状态启动。如果 Seahorse、`secret-tool`、浏览器或 IDE
持续要求密码，请将钥匙串密码保存为 systemd 用户凭据（需要 systemd 258 或更新版本）：

```bash
mkdir -p ~/.config/credstore.encrypted
systemd-ask-password -n | systemd-creds encrypt --user \
  --name=oo7.keyring-encryption-password - \
  ~/.config/credstore.encrypted/oo7.keyring-encryption-password
```

即使 shell 设置了其他 `XDG_CONFIG_HOME`，这里也请使用 `~/.config`。任何能读取该文件并使用 TPM 的人（包括 root）都能解密它。
也可以每次开机运行 `oo7-cli unlock`。两种方法见 [ArchWiki: Oo7](https://wiki.archlinux.org/title/Oo7)。

如果 WSLg 在休眠、显示器变化或更新后没有响应，请检查 `/mnt/wslg/weston.log`，执行 `wsl --update`，待 WSLg 恢复后重启会话。
启动器不会在 WSLg 合成器不健康时自动重启。

如果 X11 应用失败，请在 Foot 终端中运行 `echo "$DISPLAY"` 并检查 `arch-sway-wslg logs`。可以用
`GDK_BACKEND=x11 nwg-look` 测试 X11 路径。

如果托管会话卡死，使用 `arch-sway-wslg stop`。切勿删除 `/tmp/.X11-unix`。

## 限制

本项目面向 WSL2/WSLg 上的 Arch Linux：

- 截图、任务栏行为与 WSLg 窗口位置由 Windows 负责。本项目不会自动排列或移动窗口，也不会在 WSLg 合成器故障后自动恢复。
- 会话使用持久用户 D-Bus，因此 Secret Service 等由 systemd 激活的服务可以工作。桌面单例名字与整个 WSL 用户共享，会话外激活的服务可能在
  `stop` 后继续运行。
- 会话不会向用户总线发布显示环境；Sway 自己启动的应用会直接继承会话环境。
- Portal、Flatpak 集成、屏幕共享以及从 Linux 侧移动 WSLg 窗口均不支持。
- 最多支持四个嵌套输出；使用 XWayland 的应用可能不如原生 Wayland 客户端清晰。

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
