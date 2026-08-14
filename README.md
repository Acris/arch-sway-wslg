# arch-sway-wslg

`arch-sway-wslg` installs and runs a polished, Wayland-first Sway session inside Microsoft WSL2/WSLg. It targets Arch
Linux on WSL and is not intended to be a general bare-metal Sway distribution.

<img width="3840" height="2160" alt="Sway" src="https://github.com/user-attachments/assets/7bbac63f-4e80-4c44-9ff2-a9dc18fccefc" />

## Features

- Upstream Sway with lazy XWayland compatibility
- Waybar, SwayNC, Fuzzel, Foot, swaynag, nwg-look, and Yazi
- Catppuccin Mocha styling throughout the desktop
- Sarasa UI SC for the UI and Maple Mono NF CN for the terminal
- Automatic UTF-8 plain-text clipboard synchronization with Windows
- WSLg PulseAudio integration
- One-command background lifecycle management with status, logs, diagnostics, cleanup, and crash recovery
- Transactional configuration installation with optional backups

The default session is deliberately compact. Windows owns screenshots and the outer WSLg window; the guest does not
install screen locking, power management, battery, network, or screenshot tools.

## Prerequisites

Complete
the [ArchWiki installation guide for Arch Linux on WSL](https://wiki.archlinux.org/title/Install_Arch_Linux_on_WSL)
first. The installer expects:

1. Arch Linux running under WSL2 with WSLg enabled.
2. A normal user configured as the default WSL user, with working `sudo` access.
3. The `en_US.UTF-8` locale generated and an active UTF-8 locale.
4. WSLg hardware acceleration already configured.
5. `base-devel`, Git, and `paru` installed for the normal user.
6. Windows interoperability enabled so `powershell.exe` can run from WSL.

Keep WSL current from Windows:

```powershell
wsl --update
wsl --shutdown
```

Systemd is optional. The launcher reuses a working user D-Bus session when available and otherwise starts a private
`dbus-run-session`; it does not modify the global D-Bus or systemd activation environment.

## Quick Start

Run the following as your normal Arch user, never as root:

```bash
git clone https://github.com/Acris/arch-sway-wslg.git
cd arch-sway-wslg
./install.sh
```

Review AUR PKGBUILDs displayed by `paru` before accepting them. The installer then:

- verifies the required payload files;
- asks whether to back up existing managed paths;
- lists every optional desktop-entry mask and asks whether to install them;
- asks for the nested Sway output scale, defaulting to `1`;
- detects a running managed Sway session and asks to stop it before updating;
- updates Arch and installs the bootstrap providers from `packages.conf`, then installs the remaining desktop packages;
- stages the complete payload, applies the selected scale, and validates the final Shell, Sway, Foot, and Fuzzel
  configuration before replacing anything;
- rolls back already-replaced paths if any deployment step fails;
- installs the public launcher into `~/.local/bin` and private helpers into
  `~/.local/libexec/arch-sway-wslg`;
- applies the recommended GTK appearance defaults with `gsettings`.

The installer explains its sudo request before validating Sway: root permission is used only to create a temporary,
private X11 mount namespace. The validation itself runs as the normal user and leaves WSLg's global X11 mapping
unchanged.

If `~/.local/bin` is not in `PATH`, the installer prints the appropriate Bash or Fish command. Then start the session:

```bash
arch-sway-wslg doctor
arch-sway-wslg start
arch-sway-wslg status
```

Open the session log if startup does not complete:

```bash
arch-sway-wslg logs
```

## Installation and Updates

Managed configuration directories are replaced exactly. This prevents stale files from older releases surviving an
update, but it also means custom files inside these directories are replaced:

```text
~/.config/foot
~/.config/fuzzel
~/.config/sway
~/.config/swaynag
~/.config/swaync
~/.config/waybar
~/.config/yazi
```

Accept the backup prompt if those directories contain local changes. Backups are stored under
`$XDG_STATE_HOME/arch-sway-wslg/backups`, or by default under
`~/.local/state/arch-sway-wslg/backups`.

To update, pull the repository and run the installer again:

```bash
git pull --ff-only
./install.sh
```

Desktop-entry masks contain only `Hidden=true`; they hide helper applications from Fuzzel without uninstalling them.
Declining the prompt never deletes or modifies an existing same-named desktop file.

## Package Notes

`packages.conf` is the installer's only package manifest. Its `[bootstrap]` section is installed first and contains the
portal, font, Nerd Font, and JACK providers needed by later packages. After that transaction succeeds, the `[main]`
section installs the desktop stack and applications. The manifest does not repeat ordinary dependencies resolved by
pacman.

- `xdg-desktop-portal-gtk-dummy` satisfies Arch GTK requirements without installing a guest portal stack that is
  unnecessary for this WSLg session.
- `jack2` is the default provider for Waybar's JACK library requirement and is not started; audio continues through WSLg
  PulseAudio. If `pipewire-jack` is already installed, the installer keeps it and skips `jack2` because the two
  providers conflict.
- `qt5-wayland` provides native Wayland support for Qt 5 applications.
- `maplemono-nf-cn-unhinted` supplies Maple Mono NF CN for Foot.
- `ttf-nerd-fonts-symbols-mono` satisfies Yazi's Nerd Font requirement and keeps fallback icons aligned to terminal
  cells; it does not replace the terminal font.

## Yazi

Press `Alt+Y` to open Yazi in Foot. The bundled theme uses the Catppuccin Mocha palette. Yazi is a terminal file
manager; it does not replace GTK/Qt file chooser dialogs or install applications for every file type.

Useful default Yazi bindings:

| Key                 | Action                                           |
|---------------------|--------------------------------------------------|
| `h/j/k/l` or arrows | Leave, move, or enter a directory                |
| `Enter`             | Open the selected file or directory              |
| `Space`             | Toggle selection                                 |
| `y` / `x` / `p`     | Copy / cut / paste selected files                |
| `d` / `D`           | Move to trash / permanently delete               |
| `a` / `r`           | Create / rename                                  |
| `.`                 | Toggle hidden files                              |
| `f`                 | Filter the current directory                     |
| `s` / `S`           | Search names with `fd` / contents with `ripgrep` |
| `z` / `Z`           | Navigate with `fzf` / `zoxide`                   |
| `F1` or `~`         | Open Yazi help                                   |
| `q`                 | Quit Yazi                                        |

See the [Yazi quick-start keybindings](https://yazi-rs.github.io/docs/quick-start/#keybindings)
for the complete default map.

The [official Yazi installation guide](https://yazi-rs.github.io/docs/installation/) recommends the tools below. The
installer already includes Yazi, its Nerd Font provider, and Wayland clipboard support. For the remaining search,
navigation, JSON, and archive integrations, install:

```bash
paru -S --needed fd ripgrep fzf zoxide jq 7zip
```

- `fd` supplies fast filename discovery.
- `ripgrep` supplies content search.
- `fzf` supplies fuzzy selection.
- `zoxide` supplies frecency-based directory navigation.
- `jq` formats and previews JSON.
- `7zip` previews and extracts additional archive formats.

Rich previews are optional:

```bash
paru -S --needed ffmpeg poppler resvg imagemagick
```

- `ffmpeg` extracts video thumbnails and media metadata.
- `poppler` supplies PDF rendering and text-extraction utilities.
- `resvg` renders SVG files for previews.
- `imagemagick` converts and identifies extra image/font formats, including formats not handled by Yazi's basic image
  path.

Foot supports Yazi image previews through its built-in Sixel implementation. For an interactive shell wrapper that
changes the shell's working directory to Yazi's last directory, follow
the [Yazi quick-start wrapper](https://yazi-rs.github.io/docs/quick-start/#shell-wrapper). This project does not edit
shell startup files.

## Appearance

The installer applies these GTK defaults with `gsettings`:

- GTK theme: `adw-gtk3-dark`
- Color scheme: `prefer-dark`
- Icon theme: `Papirus-Dark`
- UI font: `Sarasa UI SC 11`
- Cursor: `Adwaita`, size `28`

Catppuccin Mocha is bundled for Sway, Waybar, SwayNC, Fuzzel, Foot, swaynag, and Yazi. GTK uses Adwaita Dark because the
historical Catppuccin GTK port is archived. Run `nwg-look` inside Sway whenever you want to review or change the GTK,
icon, font, or cursor settings.

## Commands

```bash
arch-sway-wslg doctor
arch-sway-wslg start
arch-sway-wslg status
arch-sway-wslg logs
arch-sway-wslg restart
arch-sway-wslg stop
arch-sway-wslg version
```

`start` and `restart` explain and request sudo once to create the session's isolated X11 mount namespace, then launch
the managed session as the normal user. Sway and its desktop applications never run as root. `stop` does not require
sudo. It first uses Sway IPC, then performs bounded process-group cleanup if the compositor does not exit normally. The
fixed IPC path still allows recovery when PID state is missing.

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
through Windows `Win+Shift+S`.

## Clipboard Bridge

The bridge synchronizes UTF-8 plain text in both directions. It does not sync images, HTML, or file lists.

- One persistent Windows PowerShell 5.1 setter handles Sway-to-Windows writes.
- Every setter request receives an explicit success or failure acknowledgement.
- The Windows watcher uses clipboard sequence numbers instead of repeatedly launching a process.
- Protocol lines use LF and Linux readers defensively accept CRLF.
- Reflection hashes are short-lived, payload-aware, and consumed once.
- On bridge startup, supported Windows text is published to the fresh Sway clipboard before Sway-to-Windows watching
  begins.
- `nil`, clear, and `CLIPBOARD_STATE=sensitive` events do not overwrite the Windows clipboard by default.

To opt into synchronizing clipboard content marked sensitive, export this before starting Sway:

```bash
export ARCH_SWAY_WSLG_SYNC_SENSITIVE=1
arch-sway-wslg start
```

This is not recommended for password-manager content. `WINDOWS_POWERSHELL` may be exported to override PowerShell
discovery when Windows is mounted somewhere other than the default `/mnt/c` layout.

## Runtime Design and Limitations

Sway connects directly to WSLg's absolute parent Wayland socket at
`/mnt/wslg/runtime-dir/wayland-0`. WSLg owns the distribution-wide `/tmp/.X11-unix` mapping, so the launcher gives only
the managed Sway process tree a private `01777` X11 socket directory in a separate mount namespace. A short, fixed sudo
step creates that namespace and bind mount; its fixed root shell invokes `runuser` before the project launcher continues
inside the namespace. The session remains in the normal WSL user namespace, so setuid tools such as `sudo` keep working
inside Sway. The parent WSLg mapping is never unmounted, deleted, or replaced, and no `/etc/wsl.conf` change is needed.
Sway reserves a display in the private namespace and starts XWayland only when the first X11 client connects. The mount
namespace disappears with the managed session.

The session remains Wayland-first. Qt and SDL2 use Wayland with X11 fallback; GTK, SDL3, Firefox, and current Electron
applications use their native backend selection. Fractionally scaled XWayland applications may be less sharp than native
Wayland applications.

The bundle creates one nested Sway output. Multi-output emulation and multiple independent WSLg windows are outside its
default scope.

## Troubleshooting

Run diagnostics first:

```bash
arch-sway-wslg doctor
```

If the WSLg Wayland, PulseAudio, or X11 mappings are missing, close WSL and run the following from Windows before trying
again:

```powershell
wsl --shutdown
```

`doctor` checks prerequisites without requesting sudo or changing mount state. The real private-namespace and Sway
configuration validation happens during `start`. Inside a Foot terminal launched by Sway, `echo "$DISPLAY"` should print
the nested display reserved by Sway even before the first X11 application starts. An empty value means XWayland
initialization failed; inspect `arch-sway-wslg logs`. Sway selects the nested display number, so it does not need to
match WSLg's parent `:0`.

To exercise the X11 path explicitly with a bundled application, run this inside Sway:

```bash
GDK_BACKEND=x11 nwg-look
```

If Sway is already running but its launcher state was interrupted, use
`arch-sway-wslg stop`; the fixed IPC socket supports recovery. Do not manually delete `/tmp/.X11-unix`.

## Credits

The project follows upstream Sway conventions and retains useful nested-Sway, XWayland, and Windows clipboard-sequence
ideas from
[jordankoehn/sway-wsl2](https://github.com/jordankoehn/sway-wsl2). Compared with that project's startup script, this
project does not restart the user systemd service or globally unmount, delete, and recreate `/tmp/.X11-unix`. Its brief
sudo step creates an isolated mount view for one managed session and invokes the project launcher only after dropping
back to the user. This keeps WSLg's mapping intact, needs no WSL boot configuration, survives WSLg directory recreation,
and preserves normal `sudo` behavior inside Sway. It also avoids per-copy process launches and foreground-window-title
gates.

Additional references:

- [Sway sample configuration](https://github.com/swaywm/sway/blob/master/config.in)
- [Sway manual](https://man.archlinux.org/man/sway.5.en)
- [Microsoft WSLg](https://github.com/microsoft/wslg)
- [Waybar](https://github.com/Alexays/Waybar)
- [SwayNC](https://github.com/ErikReider/SwayNotificationCenter)
- [Yazi](https://yazi-rs.github.io/)
- [Catppuccin](https://catppuccin.com/)
- [Maple Mono](https://github.com/subframe7536/maple-font)
- [Sarasa Gothic](https://github.com/be5invis/Sarasa-Gothic)

## License

MIT. See [LICENSE](LICENSE).
