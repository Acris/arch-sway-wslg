# arch-sway-wslg

[English](README.md) | [简体中文](README_CN.md)

`arch-sway-wslg` installs and runs a polished, Wayland-first Sway session inside Microsoft WSL2/WSLg. It targets Arch
Linux on WSL and is not a general bare-metal Sway distribution.

<img alt="Sway" src="https://github.com/user-attachments/assets/7bbac63f-4e80-4c44-9ff2-a9dc18fccefc" style="max-width: 1200px; width: 100%;" />

## Features

- Upstream Sway, with X11 applications supported through XWayland
- Waybar, SwayNC, Fuzzel, Foot, swaynag, nwg-look, and Yazi
- Catppuccin Mocha styling throughout the desktop
- Sarasa UI SC for the UI and Maple Mono NF CN for the terminal
- Plain text copied in Sway can be pasted in Windows, and the other way round
- Up to four Sway screens, each in its own Windows window
- Personal settings live in override files that updates keep in place
- A browser of choice, opened by links inside the session
- A password keyring for Seahorse, browsers, and IDEs, which can unlock itself
- Sound through WSLg
- One command to start, stop, inspect and diagnose the session

The default session is deliberately compact. Windows keeps its own screenshots and window management; the desktop does
not install screen locking, power management, battery, network, or screenshot tools.

## Prerequisites

Complete
the [ArchWiki installation guide for Arch Linux on WSL](https://wiki.archlinux.org/title/Install_Arch_Linux_on_WSL)
first. The installer expects:

1. Arch Linux running under WSL2 with WSLg enabled.
2. A normal user configured as the default WSL user, with working `sudo` access.
3. Systemd enabled, with a working systemd user manager for that normal user.
4. WSLg hardware acceleration configured. Keep Windows and the host GPU driver up to date.
5. `base-devel`, Git, and `paru` installed for the normal user.

A UTF-8 locale is recommended for the non-ASCII text used by the bar, launcher, and Yazi; `C.UTF-8` is fine. Locale
settings are left untouched by the installer.

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

If rendering is unstable, first update Windows, run `wsl --update`, and install the latest driver for the host GPU.

## Quick Start

Run the following as a normal Arch user, never as root:

```bash
git clone https://github.com/Acris/arch-sway-wslg.git
cd arch-sway-wslg
./install.sh
```

Upgrade the system first with `paru -Syu`: the installer only refreshes the package databases and never upgrades
installed packages on its own.

Review AUR PKGBUILDs displayed by `paru` before accepting them. The installer checks the prerequisites and asks about
desktop entry masks, the browser, the output scale, a backup, keyring unlocking, and the GTK appearance settings. It
checks everything it is about to install before replacing the current configuration, and it stops a running session only
after asking.

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

`start` and `restart` ask for the sudo password once to set the session up; Sway and all desktop applications then run
as the normal user. `stop` needs no sudo and ends the whole session, including anything it started.

## Multiple Monitors

Sway can show one to four screens, each in its own Windows window:

```bash
arch-sway-wslg start --outputs 2
```

`ARCH_SWAY_WSLG_OUTPUTS=2 arch-sway-wslg start` has the same effect. Both forms accept whole numbers from 1 through 4.
The screens are named `WL-1`, `WL-2`, and so on; workspaces are assigned to them from
`~/.config/sway/config.d/10-local.conf`:

```
workspace 1 output WL-1
workspace 2 output WL-1
workspace 9 output WL-2
workspace 10 output WL-2
```

Move and maximize those windows with Windows shortcuts such as `Win+Shift+Left/Right` and `Win+Up`. This project does
not arrange them.

## Customizing Without Losing Changes

Managed configuration directories are replaced on every install. These paths always belong to the user and are never
replaced:

| Path                             | Purpose                                         |
|----------------------------------|-------------------------------------------------|
| `~/.config/sway/config.d/*.conf` | Sway settings, read after everything else       |
| `~/.config/foot/local.ini`       | Foot options, applied after the bundled ones    |
| `~/.config/fuzzel/local.ini`     | Fuzzel options, applied after the bundled ones  |
| `~/.config/waybar/local.css`     | Waybar styling, applied after the bundled sheet |
| `~/.config/swaync/local.css`     | SwayNC styling, applied after the bundled sheet |

The installer creates them with commented examples on the first installation and preserves them later. Settings there
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

Both stylesheets are read after the bundled ones, so rules set there win. Keep the two files in place even when they are
empty: removing one leaves Waybar or SwayNC unstyled.

The Waybar and SwayNC layouts, swaynag, and Yazi have no include mechanism that can be used safely, so those files are
fully managed. Keep personal versions outside the managed directories, or use the backup offered before each update.

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

## Session Environment

The launcher prepares what the session needs: the screen, the sound, and the desktop identity applications look for.
Values already set in the environment are kept, and useful defaults for Qt, Java, VS Code, and similar applications are
only filled in where nothing has been set.

The browser picked at install time is recorded in `~/.config/arch-sway-wslg/browser`. Editing that file (a single
executable name) or exporting `BROWSER` changes it.

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

Windows owns `Alt+Tab` and `Alt+Space`, so the configuration avoids those combinations. Screenshots remain available
through Windows `Win+Shift+S`. A dedicated modifier is possible by setting `$mod` to `Mod3` in
`~/.config/sway/config.d/` and mapping a Windows key to Mod3 with a Windows keyboard-remapping tool.

## Clipboard

Text copied in Sway can be pasted in Windows, and text copied in Windows can be pasted in Sway. Sharing starts and stops
together with the session.

- Only plain text is shared: images, HTML, and file lists are not.
- Selections an application marks as sensitive, such as an entry from a password manager, are skipped by default.

Sharing is timed to stay out of the way of typing, so neither direction is instant. A copy made in Sway reaches Windows
a moment later. The other direction takes longer: the session waits for a short pause in typing, two seconds by default,
before it looks at the Windows clipboard, and it looks less often the longer the session goes untouched. Text copied in
Windows is usually already there by the time the Sway window comes back into focus; if a paste happens sooner, a short
pause is enough. Run `arch-sway-wslg status` to see whether the session is picking the Windows clipboard up.

Export these before `arch-sway-wslg start`; changing them in a terminal inside the session has no effect:

```bash
# how often the Windows clipboard is checked once the session is quiet, in seconds; below 0.2 is rejected
export ARCH_SWAY_WSLG_CLIPBOARD_POLL=5

# how long a pause in typing to wait for, in whole seconds (minimum 1)
export ARCH_SWAY_WSLG_CLIPBOARD_IDLE=5

# only send Sway -> Windows, never bring the Windows clipboard in
export ARCH_SWAY_WSLG_CLIPBOARD=to-windows

# no clipboard sharing at all
export ARCH_SWAY_WSLG_CLIPBOARD=off
```

Sensitive selections can be included as well, which is not recommended alongside a password manager:

```bash
export ARCH_SWAY_WSLG_SYNC_SENSITIVE=1
arch-sway-wslg start
```

## Waybar Layout

The right side of the bar shows five items: resources, volume, tray, notifications, and the clock. Memory usage is
always visible; hovering it slides out CPU and disk usage, so system information is available without crowding the bar.

## Appearance

The installer shows the current values and asks before applying these GTK defaults. The prompt defaults to yes;
answering no leaves the current values unchanged.

- GTK theme: `adw-gtk3-dark`
- Color scheme: `prefer-dark`
- Icon theme: `Papirus-Dark`
- UI font: `Sarasa UI SC 11`
- Cursor theme: `Adwaita`

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

Press `Alt+Y` to open Yazi in Foot. See
the [Yazi quick-start keybindings](https://yazi-rs.github.io/docs/quick-start/#keybindings)
and [installation guide](https://yazi-rs.github.io/docs/installation/) for more.

The bundled theme follows [catppuccin/yazi](https://github.com/catppuccin/yazi) and ships the matching Catppuccin Mocha
theme for syntax-highlighted file previews.

The installer prints the two recommended commands after a successful run:

```bash
paru -S --needed fd ripgrep fzf zoxide jq 7zip        # search, navigation, JSON, archives
paru -S --needed ffmpeg poppler resvg imagemagick     # rich previews
```

Foot renders Yazi image previews through Sixel. This project does not edit shell startup files; the
[Yazi shell wrapper](https://yazi-rs.github.io/docs/quick-start/#shell-wrapper) has to be added manually for directory
tracking.

## Updating

```bash
git pull --ff-only
./install.sh
```

Every run offers a timestamped backup before replacing managed files. Backups include `RESTORE-INFO.txt` and are never
deleted automatically. Answer the installer's questions again after updates, then run `arch-sway-wslg restart`. Upgrade
the system itself with `paru -Syu` on a separate schedule; the installer never does it.

## Uninstalling

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

If pacman reports a dependency that is still needed, remove that package from the command and run it again. `jack2` and
`oo7` are optional in some installations; the Yazi helper packages are never installed by this project.

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

`doctor` checks systemd, the programs the session needs, clipboard sharing, the WSLg integration, the Sway
configuration, and audio. It only looks: it never asks for sudo and changes nothing.

If the WSLg Wayland, PulseAudio, or X11 mappings are missing, close WSL and run `wsl --shutdown` from Windows before
trying again.

If a keystroke sometimes produces two characters, the Windows clipboard is being read at the wrong moment. Check the
clipboard line in `arch-sway-wslg status` and any warnings in `arch-sway-wslg logs`; starting the session with
`ARCH_SWAY_WSLG_CLIPBOARD=to-windows` switches that direction off.

If notifications never appear, run `arch-sway-wslg doctor`. If another process holds
`org.freedesktop.Notifications`, stop it with `systemctl --user stop swaync.service` and restart the session.

The keyring is available to every application in the same WSL user. When oo7 is installed, the installer starts it with
the user session and offers to store the keyring password so that it opens on its own; that offer needs systemd 258 or
newer and is skipped otherwise. If Seahorse, `secret-tool`, a browser, or an IDE keeps asking for a password, run the
installer again and accept that question, or do the same by hand:

```bash
systemctl --user enable --now oo7-daemon.service
mkdir -p ~/.config/credstore.encrypted
systemd-ask-password -n | systemd-creds encrypt --user \
  --name=oo7.keyring-encryption-password - \
  ~/.config/credstore.encrypted/oo7.keyring-encryption-password
systemctl --user restart oo7-daemon.service
```

Use `~/.config` for this file even when the shell sets another `XDG_CONFIG_HOME`. Anyone who can read the file and use
the TPM, including root, can decrypt it. Alternatively, run `oo7-cli unlock` once per boot. See
[ArchWiki: Oo7](https://wiki.archlinux.org/title/Oo7) for both methods.

If WSLg stops responding after sleep, a display change, or an update, the Sway session can end with it. Check
`/mnt/wslg/weston.log`, run `wsl --update`, and start the session again once WSLg is healthy; the launcher does not do
that by itself.

If an X11 application fails, run `echo "$DISPLAY"` in a Foot terminal and inspect `arch-sway-wslg logs`. The X11 path
can be tested with `GDK_BACKEND=x11 nwg-look`.

If a session is stuck, `arch-sway-wslg stop` always ends it. Never delete `/tmp/.X11-unix`.

## Limitations

This project is designed for Arch Linux on WSL2/WSLg:

- Windows handles screenshots, taskbar behavior, and where the desktop windows sit. The project does not arrange or move
  them, and it does not recover on its own when WSLg itself stops working.
- Desktop services such as the keyring and notifications are shared with the rest of the WSL user, which is what makes
  them work here. One that was started outside the session keeps running after `stop`.
- Only the applications Sway starts pick up the session's settings.
- Portals, Flatpak integration, screen sharing, and moving the desktop windows from Linux are not supported.
- Up to four screens are supported; applications using XWayland may look less sharp than native Wayland ones.

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
- [walls-catppuccin-mocha](https://github.com/orangci/walls-catppuccin-mocha)
- [Maple Mono](https://github.com/subframe7536/maple-font)
- [Sarasa Gothic](https://github.com/be5invis/Sarasa-Gothic)

## License

The software and configuration are MIT licensed; the bundled `dark-star.jpg` wallpaper is excluded from the MIT grant,
and its upstream collection does not state a redistribution license. See [LICENSE](LICENSE).
