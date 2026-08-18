# arch-sway-wslg

[English](README.md) | [简体中文](README_CN.md)

`arch-sway-wslg` installs and runs a polished, Wayland-first Sway session inside Microsoft WSL2/WSLg. It targets Arch
Linux on WSL and is not a general bare-metal Sway distribution.

<img alt="Sway" src="https://github.com/user-attachments/assets/7bbac63f-4e80-4c44-9ff2-a9dc18fccefc" style="max-width: 1200px; width: 100%;" />

## Features

- Upstream Sway with lazy XWayland compatibility
- Waybar, SwayNC, Fuzzel, Foot, swaynag, nwg-look, and Yazi
- Catppuccin Mocha styling throughout the desktop
- Sarasa UI SC for the UI and Maple Mono NF CN for the terminal
- Automatic UTF-8 plain-text clipboard synchronization with Windows, gated so it never interferes with typing
- Up to four nested outputs, each shown as its own WSLg window
- Your own settings live in override files that updates never touch
- A browser of your choice, wired to `BROWSER` inside the session
- Secret Service integration for Seahorse, browsers, and IDEs
- WSLg PulseAudio integration
- One command to start, stop, inspect and diagnose the session

The default session is deliberately compact. Windows owns screenshots and the outer WSLg window; the guest does not
install screen locking, power management, battery, network, or screenshot tools.

## Prerequisites

Complete the [ArchWiki installation guide for Arch Linux on WSL](https://wiki.archlinux.org/title/Install_Arch_Linux_on_WSL)
first. The installer expects:

1. Arch Linux running under WSL2 with WSLg enabled.
2. A normal user configured as the default WSL user, with working `sudo` access.
3. Systemd enabled, with a working systemd user manager for that normal user.
4. WSLg hardware acceleration configured. Keep Windows and the host GPU driver up to date.
5. `base-devel`, Git, and `paru` installed for the normal user.

The installer does not check or change locales. A UTF-8 locale is recommended for the non-ASCII text used by the bar,
launcher, and Yazi; `C.UTF-8` is fine.

Keep WSL current from Windows:

```powershell
wsl --update
wsl --shutdown
```

Systemd is required. The current official Arch image installed by `wsl --install -d archlinux` enables it by default.
For an older or imported distribution where `systemctl status` reports that systemd is not running, add the following to
`/etc/wsl.conf`, then run `wsl --shutdown` from Windows:

```ini
[boot]
systemd=true
```

If rendering is unstable, first update Windows, run `wsl --update`, and install the latest driver for your host GPU.

## Quick Start

Run the following as your normal Arch user, never as root:

```bash
git clone https://github.com/Acris/arch-sway-wslg.git
cd arch-sway-wslg
./install.sh
```

Review AUR PKGBUILDs displayed by `paru` before accepting them. The installer checks the prerequisites, asks about desktop
entry masks, browser, output scale, backup, and GTK appearance, then stages and checks the complete payload before
replacing anything. It stops a running managed session only after asking.

Then start the session:

```bash
arch-sway-wslg doctor
arch-sway-wslg start
arch-sway-wslg status
```

If startup does not complete, read the log with `arch-sway-wslg logs`.

## Commands

```bash
arch-sway-wslg start [--outputs N]    # start the session with N nested outputs (1-4, default 1)
arch-sway-wslg stop
arch-sway-wslg restart [--outputs N]
arch-sway-wslg status
arch-sway-wslg logs
arch-sway-wslg doctor
arch-sway-wslg version
```

`start` and `restart` request sudo once for session setup, then run Sway and its applications as your normal user in a
transient systemd user scope. `stop` needs no sudo. The systemd scope is the authoritative session state.

## Multiple Monitors

Sway can drive one to four nested outputs. Each output is a separate top-level WSLg window:

```bash
arch-sway-wslg start --outputs 2
```

`ARCH_SWAY_WSLG_OUTPUTS=2 arch-sway-wslg start` has the same effect. Both forms accept only whole numbers from 1 through
4. Outputs are named `WL-1`, `WL-2`, and so on; bind workspaces to them from `~/.config/sway/config.d/10-local.conf`:

```
workspace 1 output WL-1
workspace 2 output WL-1
workspace 9 output WL-2
workspace 10 output WL-2
```

Move and maximize the WSLg windows with Windows shortcuts such as `Win+Shift+Left/Right` and `Win+Up`. This project does
not arrange them automatically.

## Customizing Without Losing Changes

Managed configuration directories are replaced on every install. These paths are always yours and are never replaced:

| Path                             | Purpose                                        |
|----------------------------------|------------------------------------------------|
| `~/.config/sway/config.d/*.conf` | Sway settings, read after everything else      |
| `~/.config/foot/local.ini`       | Foot options, applied after the bundled ones   |
| `~/.config/fuzzel/local.ini`     | Fuzzel options, applied after the bundled ones |

The installer creates them with commented examples on the first installation and preserves them later. Settings there win:

```
# ~/.config/sway/config.d/10-local.conf
output * scale 1.5
bindsym $mod+p exec firefox
```

Waybar, SwayNC, swaynag, and Yazi are fully managed. Keep personal versions of those files outside the managed
directories, or use the backup offered before each update.

These directories are replaced (an absolute `$XDG_CONFIG_HOME` replaces `~/.config`):

```text
~/.config/foot
~/.config/fuzzel
~/.config/sway
~/.config/swaynag
~/.config/swaync
~/.config/waybar
~/.config/yazi
```

## Session Environment

The launcher sets the nested display, runtime, audio, desktop identity, and persistent user bus. It preserves your own
renderer and toolkit settings, and only adds defaults for Qt, Java, VS Code, and similar applications when you have not
set them.

The browser you pick at install time is recorded in `~/.config/arch-sway-wslg/browser`. Edit that file (a single
executable name) or export `BROWSER` yourself to change it.

## Key Bindings

| Key                           | Action                                   |
|-------------------------------|------------------------------------------|
| `Alt+Enter`                   | Open Foot                                |
| `Alt+D`                       | Open Fuzzel                              |
| `Alt+Y`                       | Open Yazi in Foot                        |
| `Alt+Shift+V`                 | Read the Windows clipboard right now     |
| `Alt+H/J/K/L` or arrows       | Move focus                               |
| `Alt+Shift+H/J/K/L` or arrows | Move the focused container               |
| `Alt+1..0`                    | Switch to workspace 1–10                 |
| `Alt+Shift+1..0`              | Move a container to workspace 1–10       |
| `Alt+B/V`                     | Select a horizontal or vertical split    |
| `Alt+S/W/E`                   | Select stacking, tabbed, or split layout |
| `Alt+F`                       | Toggle fullscreen                        |
| `Alt+Shift+F`                 | Toggle floating                          |
| `Alt+R`                       | Enter resize mode                        |
| `Alt+Shift+N`                 | Toggle the SwayNC control center         |
| `Alt+Ctrl+N`                  | Toggle Do Not Disturb                    |
| `Alt+Shift+Q`                 | Close the focused window                 |
| `Alt+Shift+C`                 | Reload Sway configuration                |
| `Alt+Shift+E`                 | Confirm and exit the Sway session        |

Windows owns `Alt+Tab` and `Alt+Space`, so the configuration avoids those combinations. Screenshots remain available
through Windows `Win+Shift+S`. Users who prefer a dedicated modifier can set `$mod` to `Mod3` in
`~/.config/sway/config.d/` and map a Windows key to Mod3 with a Windows keyboard-remapping tool.

## Clipboard

WSLg already synchronizes its own Wayland clipboard with Windows in both directions. The bundled bridge therefore only
mirrors UTF-8 plain text between the nested Sway session and the parent WSLg socket. No Windows helper process or
`powershell.exe` is required.

- Images, HTML, and file lists are not synchronized.
- Selections marked `sensitive` by the source application (password managers) are skipped by default.
- Sway starts the bridge, so it lives and dies with the session.

Copies made in Sway are forwarded immediately. Windows clipboard changes are read after the session has been quiet for
two seconds, so they do not interfere with typing; press `Alt+Shift+V` to read one immediately. `arch-sway-wslg status`
reports whether automatic reads are enabled.

Export these before `arch-sway-wslg start`; changing them in a terminal inside the session has no effect:

```bash
# poll interval in seconds; values below 0.2 are rejected
export ARCH_SWAY_WSLG_CLIPBOARD_POLL=5

# quiet period before reading (whole seconds, minimum 1)
export ARCH_SWAY_WSLG_CLIPBOARD_IDLE=5

# only forward Sway -> Windows, never read the Windows clipboard
export ARCH_SWAY_WSLG_CLIPBOARD=to-windows

# no clipboard bridging at all
export ARCH_SWAY_WSLG_CLIPBOARD=off
```

With inbound reads disabled, `Alt+Shift+V` also stops working. You can still read the Windows clipboard on demand with
`WAYLAND_DISPLAY=/mnt/wslg/runtime-dir/wayland-0 wl-paste`.

To include sensitive selections, export this before starting Sway; it is not recommended for password managers:

```bash
export ARCH_SWAY_WSLG_SYNC_SENSITIVE=1
arch-sway-wslg start
```

## Waybar Layout

The right side of the bar keeps five pills: resources, volume, tray, notifications, and the clock. Memory usage is
always visible; hovering it slides out CPU and disk usage, so system information is available without crowding the bar.

## Appearance

The installer shows the current values and asks before applying these GTK defaults. The prompt defaults to yes; answering
no leaves the current values unchanged.

- GTK theme: `adw-gtk3-dark`
- Color scheme: `prefer-dark`
- Icon theme: `Papirus-Dark`
- UI font: `Sarasa UI SC 11`
- Cursor: `Adwaita`, size `28`

Catppuccin Mocha is bundled for the desktop components. Run `nwg-look` inside Sway to review or change GTK, icon, font,
and cursor settings.

The installer asks for an output scale from 1 through 4, including decimals such as `1.25`. Match it to Windows display
scaling (`125%` is `1.25`, `150%` is `1.5`), or change it later with `output * scale 1.25` in
`~/.config/sway/config.d/`.

The bundled wallpaper comes from the
[walls-catppuccin-mocha](https://github.com/orangci/walls-catppuccin-mocha) collection and is excluded from this
project's MIT license. Its upstream image license is unspecified; distributors must verify permission before
redistributing it.

## Yazi

Press `Alt+Y` to open Yazi in Foot. See the [Yazi quick-start keybindings](https://yazi-rs.github.io/docs/quick-start/#keybindings)
and [installation guide](https://yazi-rs.github.io/docs/installation/) for more.

The installer prints the two recommended commands after a successful run:

```bash
paru -S --needed fd ripgrep fzf zoxide jq 7zip        # search, navigation, JSON, archives
paru -S --needed ffmpeg poppler resvg imagemagick     # rich previews
```

Foot renders Yazi image previews through Sixel. This project does not edit shell startup files; add the
[Yazi shell wrapper](https://yazi-rs.github.io/docs/quick-start/#shell-wrapper) yourself if you want directory tracking.

## Updating

```bash
git pull --ff-only
./install.sh
```

Every run offers a timestamped backup before replacing managed files. Backups include `RESTORE-INFO.txt` and are never
deleted automatically. Answer the installer's questions again after updates, then run `arch-sway-wslg restart`.

## Uninstalling

Stop the session first:

```bash
arch-sway-wslg stop
```

Remove the packages this project installed. Drop anything you want to keep and append the browser you chose (`firefox`,
`chromium`, `google-chrome`, or `microsoft-edge-stable-bin`):

```bash
paru -Rns sway xorg-xwayland swaybg swayidle waybar swaync foot fuzzel nwg-look \
  qt5-wayland qt6-wayland yazi oo7 seahorse adw-gtk-theme papirus-icon-theme \
  ttf-sarasa-gothic maplemono-nf-cn-unhinted noto-fonts-emoji noto-fonts \
  ttf-nerd-fonts-symbols-mono wl-clipboard xdg-utils jack2
```

If pacman reports a dependency you want to keep, remove that package from the command and run it again. `jack2` and `oo7`
are optional in some installations; the Yazi helper packages are never installed by this project.

Remove the files:

```bash
rm -rf ~/.config/sway ~/.config/waybar ~/.config/swaync ~/.config/swaynag \
       ~/.config/foot ~/.config/fuzzel ~/.config/yazi ~/.config/arch-sway-wslg
rm -rf ~/.local/libexec/arch-sway-wslg ~/.local/state/arch-sway-wslg
rm -f  ~/.local/bin/arch-sway-wslg
```

If you accepted the desktop-entry masks, remove their `Hidden=true` files under `~/.local/share/applications`. GTK
appearance values stay in dconf; reset them with `gsettings reset-recursively org.gnome.desktop.interface` if needed.

## Troubleshooting

Run diagnostics first:

```bash
arch-sway-wslg doctor
```

`doctor` checks systemd, the runtime and shared user bus, important bus names, required commands, the clipboard bridge,
WSLg mappings, Sway configuration, and audio. It never requests sudo or changes mount state.

If the WSLg Wayland, PulseAudio, or X11 mappings are missing, close WSL and run `wsl --shutdown` from Windows before
trying again.

If a single keystroke produces two characters, run `arch-sway-wslg status` and check `arch-sway-wslg logs`. You can stop
automatic Windows reads with `ARCH_SWAY_WSLG_CLIPBOARD=to-windows`.

If notifications never appear, run `arch-sway-wslg doctor`. If another process holds
`org.freedesktop.Notifications`, stop it with `systemctl --user stop swaync.service` and restart the session.

The Secret Service is available on the shared user bus, but the oo7 keyring may start locked in WSL. If Seahorse,
`secret-tool`, a browser, or an IDE keeps asking for a password, store the keyring password as a systemd user credential
(systemd 258 or newer):

```bash
mkdir -p ~/.config/credstore.encrypted
systemd-ask-password -n | systemd-creds encrypt --user \
  --name=oo7.keyring-encryption-password - \
  ~/.config/credstore.encrypted/oo7.keyring-encryption-password
```

Use `~/.config` for this file even if your shell sets another `XDG_CONFIG_HOME`. Anyone who can read the file and use the
TPM, including root, can decrypt it. Alternatively, run `oo7-cli unlock` once per boot. See
[ArchWiki: Oo7](https://wiki.archlinux.org/title/Oo7) for both methods.

If WSLg stops responding after sleep, a display change, or an update, check `/mnt/wslg/weston.log`, run `wsl --update`,
and restart the session after WSLg is healthy. The launcher does not restart against an unhealthy WSLg compositor.

If an X11 application fails, run `echo "$DISPLAY"` in a Foot terminal and inspect `arch-sway-wslg logs`. You can test
the X11 path with `GDK_BACKEND=x11 nwg-look`.

If a managed session is wedged, use `arch-sway-wslg stop`. Never delete `/tmp/.X11-unix`.

## Limitations

This project is designed for Arch Linux on WSL2/WSLg:

- Windows handles screenshots, taskbar behavior, and WSLg window placement. The project does not arrange or move those
  windows and does not automatically recover from a failed WSLg compositor.
- The session uses the persistent user D-Bus so systemd-activated services such as Secret Service work. Desktop singleton
  names are shared with the rest of the WSL user, and services activated outside the session may outlive `stop`.
- The session publishes no display environment to the user bus. Sway's own applications inherit the session environment
  directly.
- Portals, Flatpak integration, screen sharing, and moving WSLg windows from Linux are not supported.
- Up to four nested outputs are supported; applications using XWayland may be less sharp than native Wayland clients.

## Credits

The nested-Sway approach and the idea of driving several nested outputs come from
[jordankoehn/sway-wsl2](https://github.com/jordankoehn/sway-wsl2).

Additional references:

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

## License

The software and configuration are MIT licensed; the bundled `dark-star.jpg` wallpaper is excluded from the MIT grant,
and its upstream collection does not state a redistribution license. See [LICENSE](LICENSE).
