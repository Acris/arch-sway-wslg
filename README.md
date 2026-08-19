# arch-sway-wslg

[English](README.md) | [简体中文](README_CN.md)

`arch-sway-wslg` installs and runs a Wayland-first Sway desktop inside Microsoft WSL2/WSLg. It targets Arch Linux on WSL
and is not a general bare-metal Sway distribution.

<img alt="Sway" src="https://github.com/user-attachments/assets/7bbac63f-4e80-4c44-9ff2-a9dc18fccefc" style="max-width: 1200px; width: 100%;" />

## Features

- Sway with Waybar, SwayNC, Fuzzel, Foot, swaynag, nwg-look, and Yazi
- Catppuccin Mocha styling, Sarasa UI SC for the interface and Maple Mono NF CN for the terminal
- X11 applications through XWayland, and sound through WSLg
- Plain text copied in Sway can be pasted in Windows, and the other way round
- Up to four Sway screens, each in its own Windows window
- A browser of choice, and a password keyring that can unlock itself
- Personal settings in override files that updates keep in place
- One command to start, stop, inspect, and diagnose the desktop

The desktop is deliberately compact. Windows keeps its own screenshots and window management, so no screen locking,
power management, battery, network, or screenshot tools are installed.

## Prerequisites

Complete
the [ArchWiki installation guide for Arch Linux on WSL](https://wiki.archlinux.org/title/Install_Arch_Linux_on_WSL)
first. The installer expects:

1. Arch Linux on WSL2 with WSLg enabled, on an up-to-date Windows and GPU driver.
2. A normal user configured as the default WSL user, with working `sudo` access.
3. Systemd enabled, including a working systemd user manager for that user.
4. `base-devel`, Git, and `paru` installed for that user.
5. A UTF-8 locale for the non-ASCII text used by the bar, launcher, and Yazi; `C.UTF-8` is fine. Locale settings are
   left untouched.

Keep WSL current from Windows:

```powershell
wsl --update
wsl --shutdown
```

If `systemctl status` reports that systemd is not running, add the following to `/etc/wsl.conf` and run `wsl --shutdown`
from Windows:

```ini
[boot]
systemd=true
```

WSL may stop the Arch instance after its last terminal closes, which also ends the Sway window. Keep an Arch terminal
open while using the desktop, or disable both idle timers in `%UserProfile%\.wslconfig`:

```ini
[general]
instanceIdleTimeout=-1

[wsl2]
vmIdleTimeout=-1
```

Run `wsl --shutdown` from Windows after changing the file, then start WSL again.
See [microsoft/WSL#13291](https://github.com/microsoft/WSL/issues/13291).

## Install

Run the following as a normal Arch user, never as root:

```bash
paru -Syu
git clone https://github.com/Acris/arch-sway-wslg.git
cd arch-sway-wslg
./install.sh
```

The installer asks about desktop entry masks, a browser, the output scale, and a backup, then installs the packages.
Keyring unlocking and GTK appearance are asked afterwards, so the run needs attention again once the packages are in
place. The managed configuration is replaced only after the whole payload has been staged and checked. Review AUR
PKGBUILDs shown by `paru` before accepting them.

Upgrading the system stays with `paru -Syu`, because Arch does not support partial upgrades: when the refreshed package
databases show that this system is behind, the installer reports it and stops unless continuing is confirmed.

Then start the desktop:

```bash
arch-sway-wslg doctor
arch-sway-wslg start
arch-sway-wslg status
```

If startup does not complete, read the log with `arch-sway-wslg logs`.

## Commands

```bash
arch-sway-wslg start [--outputs N]    # start with N screens (1-4, default 1)
arch-sway-wslg stop
arch-sway-wslg restart [--outputs N]
arch-sway-wslg status
arch-sway-wslg logs
arch-sway-wslg doctor
arch-sway-wslg version
```

`start` and `restart` ask for the sudo password once while setting the session up; Sway and all desktop applications
then run as the normal user. `stop` needs no sudo and ends everything the session started.

## Key Bindings

| Key                           | Action                                   |
|-------------------------------|------------------------------------------|
| `Alt+Enter`                   | Open Foot                                |
| `Alt+D`                       | Open Fuzzel                              |
| `Alt+Y`                       | Open Yazi in Foot                        |
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

Windows owns `Alt+Tab` and `Alt+Space`, so the configuration avoids those combinations, and screenshots stay on Windows
`Win+Shift+S`.

## Multiple Monitors

Sway can show one to four screens, each in its own Windows window:

```bash
arch-sway-wslg start --outputs 2
```

`ARCH_SWAY_WSLG_OUTPUTS=2 arch-sway-wslg start` has the same effect; both forms accept whole numbers from 1 through 4.
The screens are named `WL-1`, `WL-2`, and so on, and workspaces can be assigned to them:

```
# ~/.config/sway/config.d/10-local.conf
workspace 1 output WL-1
workspace 9 output WL-2
```

Move and maximize those windows with Windows shortcuts such as `Win+Shift+Left/Right` and `Win+Up`. This project does
not arrange them.

## Customizing

Managed configuration directories are replaced on every install. These paths always belong to the user, are read after
the bundled files, and are never replaced:

| Path                             | Purpose        |
|----------------------------------|----------------|
| `~/.config/sway/config.d/*.conf` | Sway settings  |
| `~/.config/foot/local.ini`       | Foot options   |
| `~/.config/fuzzel/local.ini`     | Fuzzel options |
| `~/.config/waybar/local.css`     | Waybar styling |
| `~/.config/swaync/local.css`     | SwayNC styling |

The installer creates them with commented examples on the first installation and keeps them afterwards. Settings there
win:

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

Keep both stylesheets in place even when they are empty: removing one leaves Waybar or SwayNC unstyled. The Waybar and
SwayNC layouts, swaynag, and Yazi have no safe include mechanism and are fully managed, so keep personal versions
outside the managed directories or use the backup the installer offers.

These directories are replaced (with `XDG_CONFIG_HOME` set, they live under it instead of `~/.config`):

```text
~/.config/foot
~/.config/fuzzel
~/.config/sway
~/.config/swaynag
~/.config/swaync
~/.config/waybar
~/.config/yazi
```

The browser chosen at install time is recorded in `~/.config/arch-sway-wslg/browser` and opens the links followed inside
the session. Editing that file, which holds a single executable name, or exporting `BROWSER` changes it.

## Clipboard

Text copied in Sway can be pasted in Windows, and text copied in Windows can be pasted in Sway. Sharing starts and stops
with the session.

- Only plain text is shared: images, HTML, and file lists are not.
- Selections an application marks as sensitive, such as an entry from a password manager, are skipped by default.

Neither direction is instant, because sharing is timed to stay out of the way of typing. A copy made in Sway reaches
Windows a moment later. The other direction waits for a short pause in typing and then looks at the Windows clipboard,
less often the longer the session goes untouched; text copied in Windows is usually already there once the Sway window
is back in focus, and a brief pause is enough if a paste comes sooner. `arch-sway-wslg status` reports the current
state.

Export any of these before `arch-sway-wslg start`; setting them in a terminal inside the session has no effect:

```bash
# how often the Windows clipboard is checked once typing pauses, in seconds (minimum 0.2)
export ARCH_SWAY_WSLG_CLIPBOARD_POLL=5

# how long a pause in typing to wait for, in whole seconds (minimum 1)
export ARCH_SWAY_WSLG_CLIPBOARD_IDLE=5

# only send Sway -> Windows, never bring the Windows clipboard in
export ARCH_SWAY_WSLG_CLIPBOARD=to-windows

# no clipboard sharing at all
export ARCH_SWAY_WSLG_CLIPBOARD=off

# share sensitive selections too; not recommended alongside a password manager
export ARCH_SWAY_WSLG_SYNC_SENSITIVE=1
```

## Appearance

Sizes follow the defaults of each bundled program; fonts, colors, and the rounded bar elements are the deliberate
differences. Both `local.css` files can adjust them.

On the right of the bar are resources, volume, tray, notifications, and the clock. Memory usage is always visible;
hovering it slides out CPU and disk usage.

The installer shows the current GTK values and asks before applying its own defaults; answering no keeps the current
ones:

- GTK theme: `adw-gtk3-dark`
- Color scheme: `prefer-dark`
- Icon theme: `Papirus-Dark`
- UI font: `Sarasa UI SC 11`
- Cursor theme: `Adwaita`

Run `nwg-look` inside Sway to review or change GTK, icon, font, and cursor settings.

The installer also asks for an output scale from 1 through 4, decimals included. Match it to Windows display scaling
(`125%` is `1.25`, `150%` is `1.5`), or set `output * scale 1.25` in `~/.config/sway/config.d/` later.

The bundled wallpaper is `arch-black-4k.png` from the
[catppuccin-wallpapers](https://github.com/zhichaoh/catppuccin-wallpapers) collection, which is MIT licensed. Another
image can be set with `output * bg /path/to/image fill` in `~/.config/sway/config.d/`.

## Yazi

Press `Alt+Y` to open Yazi in Foot. Its theme follows [catppuccin/yazi](https://github.com/catppuccin/yazi) and includes
the matching syntax highlighting for file previews. See
the [Yazi documentation](https://yazi-rs.github.io/docs/quick-start/) for keybindings and features.

The installer prints two recommended commands after a successful run:

```bash
paru -S --needed fd ripgrep fzf zoxide jq 7zip        # search, navigation, JSON, archives
paru -S --needed ffmpeg poppler resvg imagemagick     # rich previews
```

Image previews are rendered through Sixel. The
[Yazi shell wrapper](https://yazi-rs.github.io/docs/quick-start/#shell-wrapper) has to be added manually, because this
project does not edit shell startup files.

## Updating

```bash
git pull --ff-only
./install.sh
```

An update can add packages, so answer the installer's questions again, then run `arch-sway-wslg restart`. Every run
offers a timestamped backup of the managed files before replacing them; backups include `RESTORE-INFO.txt` and are never
deleted automatically.

## Uninstalling

The paths below are the defaults; adjust them when `XDG_CONFIG_HOME` or `XDG_STATE_HOME` is set.

Stop the session first, and stop the keyring daemon if the installer enabled it:

```bash
arch-sway-wslg stop
systemctl --user disable --now oo7-daemon.service
rm -f ~/.config/credstore.encrypted/oo7.keyring-encryption-password
```

Remove the packages this project installed. Drop anything worth keeping and append the browser that was chosen
(`firefox`, `chromium`, `google-chrome`, `microsoft-edge-stable-bin`, or `brave-bin`):

```bash
paru -Rns sway xorg-xwayland swaybg swayidle waybar swaync foot fuzzel nwg-look \
  qt5-wayland qt6-wayland yazi oo7 seahorse adw-gtk-theme papirus-icon-theme \
  ttf-sarasa-gothic maplemono-nf-cn-unhinted noto-fonts-emoji noto-fonts \
  ttf-nerd-fonts-symbols-mono wl-clipboard xdg-utils jack2
```

If pacman reports that a package is still needed, drop it from the command and run it again; a kept browser, for
example, still needs `xdg-utils` and a font package.

Remove the files:

```bash
rm -rf ~/.config/sway ~/.config/waybar ~/.config/swaync ~/.config/swaynag \
       ~/.config/foot ~/.config/fuzzel ~/.config/yazi ~/.config/arch-sway-wslg
rm -rf ~/.local/libexec/arch-sway-wslg ~/.local/state/arch-sway-wslg
rm -f  ~/.local/bin/arch-sway-wslg
```

The desktop-entry masks, if they were accepted, are `Hidden=true` files under `~/.local/share/applications` and have to
be removed there. GTK appearance values stay in dconf; reset them with
`gsettings reset-recursively org.gnome.desktop.interface` if needed.

## Troubleshooting

Run diagnostics first:

```bash
arch-sway-wslg doctor
```

It checks systemd, the programs the desktop needs, WSLg integration, and audio, and it changes nothing.

If the WSLg Wayland, PulseAudio, or X11 mappings are missing, close WSL and run `wsl --shutdown` from Windows before
trying again.

If a keystroke sometimes produces two characters, the Windows clipboard is being read at the wrong moment. Check the
clipboard line in `arch-sway-wslg status` and any warnings in `arch-sway-wslg logs`; starting the session with
`ARCH_SWAY_WSLG_CLIPBOARD=to-windows` switches that direction off.

If notifications never appear, run `arch-sway-wslg doctor`. When another process already holds
`org.freedesktop.Notifications`, stop it with `systemctl --user stop swaync.service` and restart the session.

The keyring is shared with the rest of the WSL user. When oo7 is installed, the installer starts it and offers to store
the keyring password so that it opens on its own, which needs systemd 258 or newer. If Seahorse, `secret-tool`, a
browser, or an IDE keeps asking for a password, run the installer again and accept that question, or do the same by
hand:

```bash
systemctl --user enable --now oo7-daemon.service
mkdir -p ~/.config/credstore.encrypted
systemd-ask-password -n | systemd-creds encrypt --user \
  --name=oo7.keyring-encryption-password - \
  ~/.config/credstore.encrypted/oo7.keyring-encryption-password
systemctl --user restart oo7-daemon.service
```

Use `~/.config` for this file even when the shell sets another `XDG_CONFIG_HOME`. Anyone who can read it and use the
TPM, including root, can decrypt it. Running `oo7-cli unlock` once per boot is the alternative; see
[ArchWiki: Oo7](https://wiki.archlinux.org/title/Oo7) for both.

If WSLg stops responding after sleep, a display change, or an update, the Sway session can end with it. Check
`/mnt/wslg/weston.log`, run `wsl --update`, and start the session again once WSLg is healthy.

If an X11 application fails, run `echo "$DISPLAY"` in a Foot terminal and inspect `arch-sway-wslg logs`. The X11 path
can be tested with `GDK_BACKEND=x11 nwg-look`.

If a session is stuck, `arch-sway-wslg stop` always ends it. Never delete `/tmp/.X11-unix`.

## Limitations

- Windows handles screenshots, taskbar behavior, and where the desktop windows sit. This project does not move them, and
  it does not recover on its own when WSLg itself stops working.
- The keyring, notifications, and other desktop services are shared with the rest of the WSL user, which is what makes
  them work here. One that was started outside the session keeps running after `stop`.
- Portals, Flatpak integration, and screen sharing are not supported.
- Applications using XWayland may look less sharp than native Wayland ones.

## Credits

The nested-Sway approach and the idea of using several screens come from
[jordankoehn/sway-wsl2](https://github.com/jordankoehn/sway-wsl2).

Additional references:

- [Sway sample configuration](https://github.com/swaywm/sway/blob/master/config.in)
- [Sway manual](https://man.archlinux.org/man/sway.5.en)
- [Microsoft WSLg](https://github.com/microsoft/wslg)
- [Waybar](https://github.com/Alexays/Waybar)
- [SwayNC](https://github.com/ErikReider/SwayNotificationCenter)
- [Yazi](https://yazi-rs.github.io/)
- [Catppuccin](https://catppuccin.com/)
- [catppuccin-wallpapers](https://github.com/zhichaoh/catppuccin-wallpapers)
- [Maple Mono](https://github.com/subframe7536/maple-font)
- [Sarasa Gothic](https://github.com/be5invis/Sarasa-Gothic)

## License

The software and configuration are MIT licensed, as are the bundled wallpaper and the Yazi syntax highlighting theme,
which carry their own upstream copyright notices. See [LICENSE](LICENSE).
