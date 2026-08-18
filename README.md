# arch-sway-wslg

[English](README.md) | [简体中文](README_CN.md)

`arch-sway-wslg` installs and runs a polished, Wayland-first Sway session inside Microsoft WSL2/WSLg. It targets Arch
Linux on WSL and is not intended to be a general bare-metal Sway distribution.

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
- WSLg PulseAudio integration
- One command to start, stop, inspect and diagnose the session

The default session is deliberately compact. Windows owns screenshots and the outer WSLg window; the guest does not
install screen locking, power management, battery, network, or screenshot tools.

## Prerequisites

Complete
the [ArchWiki installation guide for Arch Linux on WSL](https://wiki.archlinux.org/title/Install_Arch_Linux_on_WSL)
first. The installer expects:

1. Arch Linux running under WSL2 with WSLg enabled.
2. A normal user configured as the default WSL user, with working `sudo` access.
3. Systemd enabled, with a working systemd user manager for that normal user.
4. A locale configured according to the [ArchWiki locale instructions](https://wiki.archlinux.org/title/Locale). No
   specific locale is required and none is checked or written by the installer, but a UTF-8 locale is strongly
   recommended: the bar, the launcher, and Yazi all display non-ASCII text.
5. WSLg hardware acceleration configured. Keep Windows and the host GPU driver up to date.
6. `base-devel`, Git, and `paru` installed for the normal user.

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

Hardware acceleration is required for a smooth nested compositor. If rendering is unstable, first update Windows, run
`wsl --update`, and install the latest driver for your host GPU.

## Quick Start

Run the following as your normal Arch user, never as root:

```bash
git clone https://github.com/Acris/arch-sway-wslg.git
cd arch-sway-wslg
./install.sh
```

Review AUR PKGBUILDs displayed by `paru` before accepting them. The installer:

- checks the prerequisites and the payload before touching any package;
- asks whether to install the optional desktop-entry masks;
- asks which browser to install: Firefox (default), Chromium, Google Chrome, Microsoft Edge, or none; a browser that is
  already installed is marked `[installed]` and is only wired to `BROWSER`, never reinstalled;
- asks for the Sway output scale (1 through 4, decimals allowed);
- asks whether to copy the current managed files to `~/.local/state/arch-sway-wslg/backups/<timestamp>`, defaulting to
  yes;
- stops a running managed session after asking;
- updates Arch, installs the bootstrap providers, then the desktop stack and the chosen browser unless it is already
  installed;
- prints the current and proposed GTK appearance settings and asks before changing them;
- stages the whole payload and checks it before replacing anything.

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

`start` and `restart` explain and request sudo once to create the session's isolated X11 mount namespace, then launch
the managed session as your normal user in a transient systemd user scope with a private D-Bus session. Sway and its
desktop applications never run as root. `stop` needs no sudo: it asks Sway to exit through IPC, then stops the scope if
necessary. The systemd scope is the authoritative session state; IPC is only a communication channel.

## Multiple Monitors

Sway can drive more than one nested output. Each output is a separate top-level WSLg window that you can move to a
different Windows monitor:

```bash
arch-sway-wslg start --outputs 2
```

`ARCH_SWAY_WSLG_OUTPUTS=2 arch-sway-wslg start` has the same effect; both forms accept only whole numbers from 1 through
4 and refuse anything else. The outputs are named `WL-1`, `WL-2`, and so on. Bind workspaces to them from your own
configuration, for example in `~/.config/sway/config.d/10-local.conf`:

```
workspace 1 output WL-1
workspace 2 output WL-1
workspace 9 output WL-2
workspace 10 output WL-2
```

Position the windows with Windows shortcuts: `Win+Shift+Left/Right` moves a window to another monitor and `Win+Up`
maximizes it. If a window does not fill the target screen, press `Win+Left` or `Win+Right` first, then `Win+Up`. Enable
automatic hiding for the Windows taskbar to use the full height of a display. This project does not move or maximize
WSLg windows for you.

## Customizing Without Losing Changes

Managed configuration directories are replaced exactly on every install, so stale files from older releases cannot
survive an update. Three paths are always yours and are never replaced:

| Path                             | Purpose                                        |
|----------------------------------|------------------------------------------------|
| `~/.config/sway/config.d/*.conf` | Sway settings, read after everything else      |
| `~/.config/foot/local.ini`       | Foot options, applied after the bundled ones   |
| `~/.config/fuzzel/local.ini`     | Fuzzel options, applied after the bundled ones |

The installer creates them on a first installation with commented examples and copies them forward on every later
installation. Because they are read last, anything you set there wins:

```
# ~/.config/sway/config.d/10-local.conf
output * scale 1.5
bindsym $mod+p exec firefox
```

Waybar, SwayNC, swaynag, and Yazi have no comparable include mechanism, so `~/.config/waybar`, `~/.config/swaync`,
`~/.config/swaynag`, and `~/.config/yazi` are fully managed. Keep personal versions of those files outside the managed
directories, or accept the backup the installer offers before every update and copy them back from there.

These directories are replaced (the default root is shown; an absolute `$XDG_CONFIG_HOME` replaces `~/.config`):

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

These hints are only set when you have not set them yourself, so your own values always win:

| Variable                              | Value         | Why                                                          |
|---------------------------------------|---------------|--------------------------------------------------------------|
| `QT_QPA_PLATFORM`                     | `wayland;xcb` | Qt 5 never selects Wayland on its own; xcb stays as fallback |
| `QT_WAYLAND_DISABLE_WINDOWDECORATION` | `1`           | Sway draws the borders, so Qt should not add its own         |
| `DONT_PROMPT_WSL_INSTALL`             | `1`           | Stops VS Code from suggesting the Windows build inside Sway  |
| `_JAVA_AWT_WM_NONREPARENTING`         | `1`           | Swing and the JetBrains IDEs render grey windows without it  |
| `BROWSER`                             | your choice   | `xdg-open` treats Sway as a generic desktop and honours it   |

These are decided by the session itself and always overwrite an inherited value, because a stale one breaks the session:

| Variable                                  | Value                    | Why                                               |
|-------------------------------------------|--------------------------|---------------------------------------------------|
| `XDG_RUNTIME_DIR`                         | `/run/user/$UID`         | The session never uses WSLg's shared runtime      |
| `WAYLAND_DISPLAY`, `SWAYSOCK`             | parent socket, then Sway | Sway replaces them with its nested values         |
| `XDG_SESSION_TYPE`, `XDG_CURRENT_DESKTOP` | `wayland`, `sway`        | Toolkit and `xdg-open` desktop detection          |
| `WLR_WL_OUTPUTS`                          | `--outputs N`            | Only set when more than one output is requested   |
| `PULSE_SERVER`                            | WSLg socket              | Audio always goes to the WSLg PulseAudio endpoint |

`qt5-wayland` and `qt6-wayland` are installed so both Qt generations have the Wayland platform plugin available. Recent
Firefox and Chromium releases select Wayland by default; no extra flags are set for them.

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
mirrors UTF-8 plain text between the nested Sway session and the parent WSLg socket, which is enough for
`Ctrl+C` in Sway to paste in Windows and the other way around. No Windows helper process is involved, and
`powershell.exe` is not required.

- Images, HTML, and file lists are not synchronized.
- Selections marked `sensitive` by the source application (password managers) are skipped by default.
- Sway starts the bridge, so it lives and dies with the session.

The two directions do not work the same way. Sway implements the wlroots data-control protocol, so a copy inside the
session is forwarded the moment it happens. WSLg's Weston implements no data-control protocol at all, so the outer
clipboard cannot be watched for events and is read with one-shot reads instead, once per second and each bounded by a
three-second timeout.

Such a read has to open a one-pixel surface on WSLg, which takes keyboard focus away from the session for a moment.
wlroots replays the keys that were held when focus comes back, so a read that overlaps typing duplicates or drops
characters. The bridge therefore reads the outer clipboard only after two seconds without input. It starts and
supervises its own `swayidle` to know when that is; if `swayidle` cannot be kept running, the bridge says so in the log
and stops reading rather than typing into your keystrokes. `arch-sway-wslg status` reports which of the two states the
bridge is in.

That also means text copied on Windows arrives once the session goes quiet, not while you are working. When you want it
immediately, press `Alt+Shift+V`: it reads the Windows clipboard on the spot, whatever the session is doing.

The three variables below are read by the bridge, which Sway starts, so export them **before** `arch-sway-wslg start`;
setting them in a terminal inside the session has no effect:

```bash
# read the Windows clipboard less often; values below 200ms are rejected
export ARCH_SWAY_WSLG_CLIPBOARD_POLL=5

# wait longer for the session to go quiet before reading (whole seconds, minimum 1)
export ARCH_SWAY_WSLG_CLIPBOARD_IDLE=5

# only forward Sway -> Windows, never read the Windows clipboard
export ARCH_SWAY_WSLG_CLIPBOARD=to-windows

# no clipboard bridging at all
export ARCH_SWAY_WSLG_CLIPBOARD=off
```

With the inbound direction disabled, `Alt+Shift+V` stops working too, but
`WAYLAND_DISPLAY=/mnt/wslg/runtime-dir/wayland-0 wl-paste` still reads the Windows clipboard on demand from any terminal
inside the session.

To include sensitive selections, export this before starting Sway; it is not recommended for password managers:

```bash
export ARCH_SWAY_WSLG_SYNC_SENSITIVE=1
arch-sway-wslg start
```

## Waybar Layout

The right side of the bar keeps five pills: resources, volume, tray, notifications, and the clock. Memory usage is
always visible; hovering it slides out CPU and disk usage, so system information is available without crowding the bar.

## Appearance

The installer shows the current values followed by these proposed GTK defaults and asks before changing them. The prompt
defaults to yes; answering no leaves every GSettings value unchanged.

- GTK theme: `adw-gtk3-dark`
- Color scheme: `prefer-dark`
- Icon theme: `Papirus-Dark`
- UI font: `Sarasa UI SC 11`
- Cursor: `Adwaita`, size `28`

Catppuccin Mocha is bundled for Sway, Waybar, SwayNC, Fuzzel, Foot, swaynag, and Yazi. GTK uses Adwaita Dark because the
historical Catppuccin GTK port is archived. Run `nwg-look` inside Sway to review or change GTK, icon, font, and cursor
settings.

The installer asks for the output scale and accepts any value from 1 through 4, including decimals such as `1.25`. It
cannot be detected: WSLg performs the scaling Wayland cannot express itself on the Windows side and keeps advertising
scale 1 on its parent output, so a Windows setting of 125% is invisible from Linux. Match your Windows display scaling
(125% is `1.25`, 150% is `1.5`) and change it at any time with `output * scale 1.25` in `~/.config/sway/config.d/`.

The bundled wallpaper is installed at `~/.config/sway/wallpapers/dark-star.jpg` and the resolved absolute path is
written into the Sway configuration. It comes from the
[walls-catppuccin-mocha](https://github.com/orangci/walls-catppuccin-mocha) collection and is not relicensed by this
project's MIT license. That collection does not publish a license for the image, so distributors must verify permission
before redistributing it.

## Yazi

Press `Alt+Y` to open Yazi in Foot; the bundled theme uses the Catppuccin Mocha palette. See the
[Yazi quick-start keybindings](https://yazi-rs.github.io/docs/quick-start/#keybindings) for the default key map and the
[Yazi installation guide](https://yazi-rs.github.io/docs/installation/) for optional integrations.

The installer prints the two recommended commands after a successful run:

```bash
paru -S --needed fd ripgrep fzf zoxide jq 7zip        # search, navigation, JSON, archives
paru -S --needed ffmpeg poppler resvg imagemagick     # rich previews
```

Foot renders Yazi image previews through its built-in Sixel implementation. This project does not edit shell startup
files, so add the [Yazi shell wrapper](https://yazi-rs.github.io/docs/quick-start/#shell-wrapper) yourself if you want
directory tracking.

## Updating

```bash
git pull --ff-only
./install.sh
```

Every run offers to copy the previous managed state to `~/.local/state/arch-sway-wslg/backups/<timestamp>` before
replacing anything, and each backup contains a `RESTORE-INFO.txt` with the exact restore command. Old backups are never
deleted automatically; remove the ones you no longer need.

An update can add packages the previous release did not need, so answer the installer's questions again rather than
assuming the package set is unchanged, and restart the session afterwards with `arch-sway-wslg restart`.

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

If pacman refuses because something you keep still depends on a package in that list, drop that package from the command
and run it again. Keeping Chromium, Chrome, or Edge for example keeps `xdg-utils` required, and keeping Firefox keeps a
font package required as its `ttf-font` provider.

`jack2` was installed only as Waybar's JACK provider, and `oo7` only when no other Secret Service backend existed;
neither is present if the installer skipped it. Optional Yazi helpers (`fd`, `ripgrep`, `fzf`, `zoxide`, `jq`, `7zip`,
`ffmpeg`, `poppler`, `resvg`, `imagemagick`) were never installed by this project.

Remove the files:

```bash
rm -rf ~/.config/sway ~/.config/waybar ~/.config/swaync ~/.config/swaynag \
       ~/.config/foot ~/.config/fuzzel ~/.config/yazi ~/.config/arch-sway-wslg
rm -rf ~/.local/libexec/arch-sway-wslg ~/.local/state/arch-sway-wslg
rm -f  ~/.local/bin/arch-sway-wslg
```

If you accepted the desktop-entry masks, also remove the `Hidden=true` files under `~/.local/share/applications`
(`avahi-discover.desktop`, `bssh.desktop`, `bvnc.desktop`, `foot-server.desktop`, `footclient.desktop`,
`lstopo.desktop`, `qv4l2.desktop`, `qvidcap.desktop`, `xgps.desktop`, `xgpsspeed.desktop`). GTK appearance values set
through GSettings stay in dconf; reset them with `gsettings reset-recursively org.gnome.desktop.interface`.

## Troubleshooting

Run diagnostics first:

```bash
arch-sway-wslg doctor
```

`doctor` checks the systemd user manager and runtime, every command the session needs (the compositor, the idle
notifier, `wl-clipboard`, and the applications the Sway configuration starts), the clipboard bridge, the WSLg mappings,
Sway config readability, and audio connectivity. It never requests sudo or changes mount state.

If the WSLg Wayland, PulseAudio, or X11 mappings are missing, close WSL and run `wsl --shutdown` from Windows before
trying again.

If a single keystroke ever produces two characters, the outer clipboard is being read while you type. Run
`arch-sway-wslg status`: the clipboard line says whether reads are gated on the session being idle. If they are not,
check `arch-sway-wslg logs` for `swayidle` warnings, or start the session with `ARCH_SWAY_WSLG_CLIPBOARD=to-windows` to
stop those reads outright.

Host sleep, network changes, monitor or taskbar changes, or a WSLg Weston failure can terminate the parent Wayland
connection and therefore the nested Sway session. The launcher cleans up the managed cgroup but does not automatically
restart Sway against an unhealthy parent compositor. Check `/mnt/wslg/weston.log` and `/mnt/wslg/versions.txt`, run
`wsl --update`, then `arch-sway-wslg stop` followed by `arch-sway-wslg start` once WSLg is healthy.

Inside a Foot terminal launched by Sway, `echo "$DISPLAY"` should print the nested display reserved by Sway even before
the first X11 application starts. An empty value means XWayland initialization failed; inspect `arch-sway-wslg logs`.
Sway selects the nested display number, so it does not need to match WSLg's parent `:0`. To exercise the X11 path
explicitly, run `GDK_BACKEND=x11 nwg-look` inside Sway.

If a managed session is wedged, use `arch-sway-wslg stop`; systemd stops the complete session cgroup. Never delete
`/tmp/.X11-unix`.

## Design Notes and Limitations

**Private X11 mount namespace.** WSLg owns the distribution-wide `/tmp/.X11-unix` mapping and mounts it read-only, so a
nested XWayland cannot create its socket there. The launcher gives only the managed Sway process tree a private `01777`
X11 socket directory inside its own mount namespace. A short, fixed sudo step creates that namespace and bind mount and
immediately drops back to your user with `runuser`. The parent WSLg mapping is never unmounted, deleted, or replaced,
and `/etc/wsl.conf` is never edited. The namespace disappears with the session.

**Systemd runtime.** The launcher requires systemd's owner-only `/run/user/$UID` runtime and keeps its control files in
`/run/user/$UID/arch-sway-wslg`. It ignores WSLg's shared `XDG_RUNTIME_DIR` value while still connecting to the absolute
WSLg Wayland socket at `/mnt/wslg/runtime-dir/wayland-0`.

**Wayland first.** The launcher removes an inherited `WLR_BACKENDS` value so Sway can select its nested Wayland backend,
but preserves renderer workarounds you set. Applications choose Wayland or XWayland themselves. Fractionally scaled
XWayland applications may look less sharp than native Wayland applications.

**Private D-Bus, and what it costs.** Each managed session runs a private `dbus-run-session`. The benefit is that
services activated inside Sway inherit the nested display instead of opening on the outer WSLg desktop, and everything
ends with the session. The costs are real and worth knowing:

- Services on your persistent user bus are not reachable from inside the session, and a session application can activate
  a *second* instance of a service such as the `oo7` Secret Service. Avoid using secret-consuming applications on both
  buses at the same time.
- GSettings values are not affected: dconf stores them in a shared file, so the values the installer writes are visible
  inside the session; only change notifications do not cross buses.
- Adding an XDG Desktop Portal backend would require deliberate integration with this private bus, which is why portal
  based file choosers, Flatpak portal access, and Wayland screen sharing are outside the supported scope.

**Scope.** Up to four nested outputs are supported. Automatic restart after an outer WSLg failure, portals, screen
sharing, and moving WSLg windows from Linux are not.

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
