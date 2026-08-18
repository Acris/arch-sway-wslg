# AGENTS.md

## Project

`arch-sway-wslg` installs a Wayland-first Sway session as a nested compositor in Arch Linux on WSL2/WSLg. It is not a
bare-metal Sway distribution. Supported environments have WSLg, systemd with a working user manager, and a normal
default user with `sudo` and `paru`; the installer does not check or change locales.

`README.md` is the user-facing contract. Keep `README_CN.md` section-for-section equivalent with it, updating the English
document first. Do not duplicate user instructions here unless an agent needs the rule to modify the project safely.

## Important paths

- `install.sh`: package selection, prompts, backups, staging, payload replacement, and GSettings.
- `.local/bin/arch-sway-wslg`: public launcher and session lifecycle.
- `.local/libexec/arch-sway-wslg/`: private session helpers, currently the clipboard bridge.
- `.config/`: installed configuration payload; `extras/desktop-overrides/` contains optional desktop masks.
- `VERSION`: release version rendered into the installed launcher; `LICENSE`: project and wallpaper notices.

There is no separate package manifest. Package lists are in `install.sh`; ordinary dependencies resolved by pacman are not
listed.

## Validation

Static Shell checks are the default. Do not add Sway configuration validation or end-to-end WSLg tests unless requested.
Run on Arch Linux:

```bash
bash -n install.sh
bash -n .local/bin/arch-sway-wslg
bash -n .local/libexec/arch-sway-wslg/clipboard-bridge
shellcheck -S warning install.sh .local/bin/arch-sway-wslg \
  .local/libexec/arch-sway-wslg/clipboard-bridge
git diff --check
```

`shellcheck -S warning` must be clean. Existing `-S info` findings are limited to intentional `SC1003` and `SC2016`.
On non-Arch hosts, use a disposable Arch container; if needed, disable pacman's download user inside that container only:

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

## Rules that changes must preserve

### Session

- Systemd and its user manager are required; there is no fallback for non-systemd sessions.
- `arch-sway-wslg-session.scope` is the authoritative session state. Sway IPC is only for communication and readiness.
- Use `/run/user/$UID` for the session runtime and `/mnt/wslg/runtime-dir/wayland-0` for the parent Wayland socket.
- Share `/run/user/$UID/bus` with the session and bind that address explicitly in every launcher or installer bus call. Never
  publish the nested session environment to D-Bus or the systemd user manager.
- The launcher uses sudo only to create the private X11 mount namespace. Never alter WSLg's `/tmp/.X11-unix` mapping;
  managed processes must return to the normal user immediately. XWayland remains lazy.
- Preserve user renderer and toolkit overrides except for values the session must own. Validate `--outputs` and
  `ARCH_SWAY_WSLG_OUTPUTS` at every boundary; support one through four outputs.
- Keep `status` and `doctor` truthful, and bound every wait involving systemd, the user bus, WSLg, IPC, locks, or the
  clipboard. Portals, Flatpak integration, screen sharing, window moving, and automatic recovery after WSLg failure are
  out of scope.

### Installer

- Run as the normal user. Delegate package changes to `paru`; preflight only commands the installer or session actually
  uses.
- Accept an absolute `XDG_CONFIG_HOME`, including spaces and dollar signs. Render resolved paths into staged payloads.
- Prompt once for a scale from `1` to `4`, defaulting to `1`; do not attempt to detect Windows scaling.
- Stage and validate payloads on the same filesystem before replacing managed directories. Preserve user override paths,
  offer a timestamped backup on every run, and remove revoked optional choices.
- Apply approved GSettings through the explicitly bound persistent user bus after the payload commits, then read values back.
- Keep the control lock around payload replacement only, never around package installation.

### Clipboard

- Sync UTF-8 plain text only; images, HTML, and file lists are out of scope. Sensitive selections stay excluded unless the
  user opts in.
- Mirror between the nested Sway and the parent WSLg socket; do not add a Windows helper process.
- Serialize forwarding, echo suppression, and parent reads under one lock. Automatic parent reads require the supervised
  quiet-period notifier; if it is unhealthy, disable inbound reads loudly. Explicit reads remain available.
- Keep bridge and notifier restarts, clipboard reads, and all other waits bounded. The bridge is started by Sway and must
  remain in the session scope.

### Configuration

- User extension points are `sway/config.d/*.conf`, `foot/local.ini`, and `fuzzel/local.ini`; preserve and seed them.
- Waybar, SwayNC, swaynag, and Yazi are fully managed because their include behavior cannot provide a safe user extension.
- Start Sway children directly. Do not add shell syntax to `swaybar_command`, and never publish an activation environment.
- This project does not validate Sway configuration; runtime errors belong in the managed session log.

## Style and safety

- Target current Arch Bash, not POSIX `sh` or macOS Bash 3. Keep `set -Eeuo pipefail` and `umask 077` in scripts.
- Use four-space indentation, quoted expansions, arrays for argument lists, `[[ ... ]]` for tests, and `local` variables in
  functions. Prefer built-ins and small targeted `grep`/`sed` operations.
- Before destructive operations, validate exact targets and derive paths from fixed per-user state, runtime, and config
  roots. Do not add compatibility code for unsupported distributions, shells, or fixed `/mnt/c` paths.
- Match surrounding comment density and explain why rather than what.

## Change checklist

- Inspect the worktree first and preserve unrelated user changes.
- Keep launcher command lists aligned with `start`, `stop`, clipboard, and `doctor`; keep installer preflight aligned with
  installer commands.
- Bump `VERSION` for package-list changes and update uninstall lists in both READMEs. Delete stale documentation instead of
  describing behavior the code no longer has.
- For user-visible behavior, update `README.md` first and then translate it into `README_CN.md`.
- Preserve the rules above unless the user explicitly changes the contract. Run and report the exact validation commands.
- Do not commit unless the user asks.

## Commits

Use Conventional Commits: `<type>[optional scope]: <imperative description>`, lower case after the colon, no trailing
period, and no more than 72 characters. Use the project types `feat`, `fix`, `refactor`, `perf`, `docs`, `style`, and
`chore`; scopes follow the affected subsystem. Explain why in a wrapped body and mark contract breaks with `!` or a
`BREAKING CHANGE:` footer, including required user actions.
