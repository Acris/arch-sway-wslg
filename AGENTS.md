# AGENTS.md

## Project overview

`arch-sway-wslg` installs and runs a Wayland-first Sway desktop as a nested compositor inside Microsoft WSL2/WSLg on
Arch Linux. It is not a general bare-metal Sway distribution, and the supported environment is deliberately narrow:

- Arch Linux under WSL2 with WSLg enabled.
- The official systemd-based Arch WSL setup, including a working systemd user manager.
- A normal default WSL user with working `sudo` and `paru`.
- At least one generated non-`C` locale; no particular locale is required.

`README.md` is the user-facing contract. `README_CN.md` is its Chinese translation and must stay section-for-section
equivalent whenever installation, runtime, or troubleshooting behavior changes.

## Repository layout

| Path                             | Role                                                                         |
|----------------------------------|------------------------------------------------------------------------------|
| `install.sh`                     | Prompts, package installation, backup, staged payload transaction, GSettings |
| `.local/bin/arch-sway-wslg`      | Public launcher and session lifecycle manager                                |
| `.local/libexec/arch-sway-wslg/` | Private helpers started by the session, currently the clipboard bridge       |
| `.config/`                       | Configuration payload installed under the user's XDG config root             |
| `extras/desktop-overrides/`      | Optional `Hidden=true` desktop-entry masks                                   |
| `VERSION`                        | Release version; the installer renders it into the launcher                  |
| `LICENSE`                        | MIT terms plus the separate bundled-wallpaper notice                         |

There is no separate package manifest: the package lists live in `install.sh`. Ordinary dependencies that pacman already
resolves are not listed.

## Setup and validation commands

Static Shell checks are the only validation this project runs by default. Do not add Sway configuration validation or
end-to-end WSLg tests unless the user asks for them.

On an Arch Linux host:

```bash
bash -n install.sh
bash -n .local/bin/arch-sway-wslg
bash -n .local/libexec/arch-sway-wslg/clipboard-bridge
shellcheck -S warning install.sh .local/bin/arch-sway-wslg \
  .local/libexec/arch-sway-wslg/clipboard-bridge
git diff --check
```

`shellcheck -S warning` must report nothing. Remaining `-S info` findings are limited to intentional literals (`SC1003`,
`SC2016`).

On a non-Arch host, run the same checks in a disposable Arch container so the supported Bash version parses the scripts:

```bash
docker run --rm -v "$PWD:/workspace:ro" -w /workspace archlinux:latest bash -lc '
  bash -n install.sh
  bash -n .local/bin/arch-sway-wslg
  bash -n .local/libexec/arch-sway-wslg/clipboard-bridge
  sed -i "s/^#\?DownloadUser.*/#DownloadUser = alpm/" /etc/pacman.conf
  pacman -Sy --noconfirm shellcheck >/dev/null
  shellcheck -S warning install.sh .local/bin/arch-sway-wslg \
    .local/libexec/arch-sway-wslg/clipboard-bridge'
```

The `sed` line disables pacman's sandboxed download user, which some emulated Docker hosts require. It applies to the
disposable container only; never change the host's pacman configuration.

## Behavioral contracts

These are the guarantees the project makes. Implementations may change as long as the guarantees hold.

### Session

- Systemd and the systemd user manager are prerequisites; there is no no-systemd fallback.
- The authoritative session state is the transient user scope `arch-sway-wslg-session.scope`. Sway IPC is only a
  communication and readiness channel.
- The session uses systemd's owner-only `/run/user/$UID` runtime. The parent Wayland connection uses the absolute
  `/mnt/wslg/runtime-dir/wayland-0` socket rather than WSLg's shared runtime directory.
- Every managed graphical session gets a private `dbus-run-session`. Sway publishes its final display and IPC variables
  to that bus; the persistent systemd user environment is left untouched.
- Sudo is used only to create the private X11 mount namespace and bind mount, and the session drops back to the normal
  user immediately. WSLg's `/tmp/.X11-unix` mapping is never unmounted, deleted, replaced, or chmodded; the managed
  process tree gets a private `01777` X11 socket directory in its own mount namespace.
- XWayland stays lazy: a display is reserved, but XWayland starts with the first X11 client.
- User renderer and toolkit overrides are preserved. The launcher may unset `WLR_BACKENDS` to guarantee nested-Wayland
  backend selection, and it sets Qt, browser, and VS Code hints only when the user has not already set them.
- Between one and four nested outputs are supported through `--outputs` / `ARCH_SWAY_WSLG_OUTPUTS`. Portals, Flatpak
  integration, screen sharing, moving WSLg windows from Linux, and automatic restart after an outer WSLg failure are out
  of scope.
- Every wait around systemd, WSLg, IPC, locks, or the clipboard is bounded.

### Installer

- The installer runs as the normal user; package changes are delegated to `paru`, which may request sudo.
- Preflight checks cover only actual prerequisites and the payload the installer needs.
- Any generated non-`C` locale is accepted; no locale is required, selected, or written.
- The Sway output scale is answered once, between `1` and `4` with decimals allowed, and defaults to `1`. It is not
  detected: WSLg scales the factors Wayland cannot express on the Windows side and keeps advertising scale 1 on its
  parent output. Users change it afterwards in their own Sway override files.
- An absolute custom `XDG_CONFIG_HOME`, including ordinary spaces and dollar signs, is supported. Resolved absolute
  paths are rendered into the staged payload through markers.
- The bundled wallpaper comes from `orangci/walls-catppuccin-mocha`, is managed with the Sway directory, and keeps its
  separate license notice.
- Managed config directories are replaced exactly, with same-filesystem staging, payload checks before any replacement,
  and reverse-order rollback on failure or signal.
- The user's override paths (`sway/config.d`, `foot/local.ini`, `fuzzel/local.ini`) survive every installation and are
  seeded with commented examples on a first run.
- The previous managed state is copied to a timestamped backup before anything is replaced.
- The launcher control lock is held only around the payload transaction, never across package installation.
- The installer prints current and proposed GTK settings and asks before changing GSettings, defaulting to applying
  them, and writes and reads back approved values through the persistent systemd user bus after the payload commits.

### Clipboard bridge

- The bridge synchronizes UTF-8 plain text in both directions only. Images, HTML, and file lists are out of scope, and
  selections marked sensitive are excluded unless the user opts in.
- WSLg already mirrors its own Wayland clipboard to and from Windows, so the bridge mirrors text between the nested
  session and the parent WSLg socket and requires no Windows helper process.
- Echo suppression and parent change detection are serialized under one lock, so a forwarded selection can never loop
  back, and a parent value read while an outgoing selection is still taking effect is not mistaken for a new copy.
- Sway implements data-control, so the nested selection is watched through events. WSLg's Weston implements no
  data-control protocol, so the parent side is polled with one-shot reads instead. Because such a read briefly takes
  focus on the parent compositor, the inbound direction and the interval stay configurable, and the interval is rejected
  below an explicit floor rather than merely being non-zero.
- A parent read happens under the shared lock, so it is bounded by its own timeout: a compositor that never focuses the
  temporary surface costs one skipped poll instead of stalling the outgoing direction as well.
- A parent read must never overlap user input. Taking focus on the parent makes wlroots replay the keys held at that
  moment, which duplicates or drops characters, so reads happen only while the session reports no input activity. The
  idle notifier is a session prerequisite, and the idle state lives in the bridge process and is signalled to it,
  without any additional state file.
- Sway starts the bridge, so it inherits the nested display and belongs to the session scope.
- Worker restarts are bounded, and an unhealthy outer WSLg compositor never triggers an unbounded restart loop.

### Configuration payload

- Programs that support an include mechanism get a user-owned override that is read last: Sway includes
  `config.d/*.conf`, Foot and Fuzzel include `local.ini` from a trailing `[main]` section.
- Waybar's `include` gives precedence to the including file, so Waybar, SwayNC, swaynag, and Yazi are fully managed and
  documented as such instead of gaining a fake extension point.
- Sway's direct children inherit its final environment, so session services are started plainly and never wrapped to
  pass variables. `swaybar_command` in particular is executed directly with only `-b <id>` appended and accepts no shell
  syntax. The final nested values are published once to the D-Bus activation environment, for the services the bus
  activates itself.
- Sway configuration is not validated by this project; Sway reports runtime and configuration failures in the managed
  session log.

## Code style

- Target current Arch Linux Bash, not macOS Bash 3 or POSIX `sh`.
- Keep `set -Eeuo pipefail` and `umask 077` in the installer, the launcher, and the private helpers.
- Use four-space indentation, quoted expansions, Bash arrays for argument lists, `[[ ... ]]` for tests, and `local`
  variables inside functions.
- Prefer Bash built-ins and small, targeted `grep`/`sed` operations over additional external parsers.
- Validate exact targets before `rm -rf`, and derive destructive paths from the fixed per-user state, runtime, and
  config roots.
- Keep compatibility surface small: no support code for unsupported distributions, non-systemd sessions, alternate
  Windows shells, or fixed `/mnt/c` paths.
- Match the surrounding comment density and tone; explain why, not what.

## Change checklist

- Inspect the existing worktree first and preserve unrelated user changes.
- Keep the launcher's required-command list aligned with what start, stop, clipboard, and doctor actually use.
- Keep `VERSION` current; the launcher receives it through marker rendering, so it is never hard-coded twice.
- Update both READMEs for user-visible changes and delete stale claims instead of documenting code that no longer
  exists.
- Preserve the contracts above unless the user explicitly changes them.
- Run the static checks and report exactly which ones were performed.
