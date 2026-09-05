# AGENTS.md

## Scope and project

These instructions apply throughout this repository. Inspect `git status --short` before editing, preserve unrelated
changes, and do not commit unless asked. Preserve the behavioral contracts below unless the user explicitly changes
them; implementations may change freely within those contracts.

`arch-sway-wslg` installs a nested, Wayland-first Sway session on Arch Linux under WSL2/WSLg. It requires systemd with
a working user manager and a normal default user with `sudo` and `paru`. Do not add bare-metal, non-systemd, or
other-distribution fallbacks. The installer neither checks nor changes locales.

## Repository map

| Path | Responsibility |
| --- | --- |
| `install.sh` | Package lists, prompts, oo7 setup, backups, payload staging/replacement, GSettings |
| `.local/bin/arch-sway-wslg` | Lifecycle launcher, `status`, `doctor`, `logs`, internal clipboard startup |
| `.config/` | Managed Sway, Waybar, SwayNC, swaynag, Foot, Fuzzel, and Yazi configuration |
| `.local/libexec/arch-sway-wslg/` | Checked-in Linux broker, Windows agent, and `clipboard.sha256` |
| `clipboard/crates/clipboard-core/` | Shared protocol, text normalization, and echo-suppression state |
| `clipboard/crates/clipboard-broker/` | Linux Wayland transport, synchronization, agent supervision |
| `clipboard/crates/clipboard-agent-win/` | Native Windows clipboard access |
| `clipboard/README.md` | Clipboard validation and exact release-payload rebuild commands |
| `extras/desktop-overrides/` | Optional desktop-entry masks |
| `.github/workflows/clipboard.yml` | Linux/Windows checks and builds; verifies payload, uploads nothing |
| `VERSION`, `README.md`, `README_CN.md`, `LICENSE` | Release, user documentation, project and wallpaper notices |

Package lists live in `install.sh`; do not duplicate transitive dependencies. Configuration and launcher files contain
installation markers such as `__ARCH_SWAY_WSLG_VERSION__`: edit the templates, preserving staged rendering.

## Installer and managed configuration

- Run installation as the normal user; delegate packages to `paru`. Preflight installation prerequisites, including
  paru's. Refresh package databases without upgrading installed packages; confirm when the system is behind.
- Preserve the single browser prompt, installed-browser detection, and recorded `BROWSER` choice; choosing none clears
  the record. Skip package installation for browsers already installed. Ask once for scale `1`–`4`, default `1`;
  do not infer Windows scaling from WSLg.
- Keep oo7 user-unit setup and optional encrypted user credentials, using the user manager's configuration home and
  skipping credentials on older systemd. Bind every bus call explicitly to the persistent user bus.
- Finish prompts and package work before taking the control lock. Hold it through backup, payload replacement, browser
  recording, and GSettings; never hold it across `paru`.
- Stage the whole payload on the destination filesystems, preserve overrides, render markers, and check required files,
  launcher syntax, executables, and clipboard checksums before replacing anything. Clean staging on every exit.
- `replace_path` moves the old path aside and restores it if the new path's rename fails. Replacement is per path,
  **not an atomic transaction across the installation**. Keep the default-yes timestamped backup and `RESTORE-INFO.txt`
  as the documented recovery route. Helpers called in conditions must explicitly propagate every write failure.
- Support absolute `XDG_CONFIG_HOME`, including spaces and dollar signs; reject paths Sway cannot express. Validate
  destructive targets against the resolved per-user roots. Never remove user-owned overrides or WSLg-owned paths.
- Preserve and seed these user extension points, read last: `sway/config.d/*.conf`, `foot/local.ini`, `fuzzel/local.ini`,
  `waybar/local.css`, `swaync/local.css`. Both CSS overrides must always exist or GTK discards the importing stylesheet.
- Keep Waybar/SwayNC layouts, swaynag, and Yazi fully managed. Declining desktop-entry masks leaves existing files alone.
  Apply approved GSettings only after payload replacement, then read them back.
- Prefer upstream sizes; retain deliberate font, color, and pill styling. Remove settings that merely repeat upstream
  defaults. Where no default exists, use common Sway conventions and multiples of four. Start Sway children directly;
  `swaybar_command` accepts no shell syntax.
- Sway expands variables when parsing each use: late overrides can add or rebind settings, but cannot retune values
  already consumed. Do not promise otherwise in documentation.

## WSLg lifecycle

- Treat `arch-sway-wslg-session.scope` as authoritative session state; use Sway IPC for messages and readiness.
- Use `/run/user/$UID` and explicitly bind `/run/user/$UID/bus`. Address WSLg through
  `/mnt/wslg/runtime-dir/wayland-0` and `/mnt/wslg/PulseServer`. Never publish nested activation environments to the
  shared bus or persistent user manager; session children inherit their environment directly.
- Use sudo only for the private X11 mount namespace, immediately returning managed processes to the normal user.
  Preserve WSLg's `/tmp/.X11-unix` mapping and lazy XWayland startup. Do not hard-code `/mnt/c` paths.
- Override inherited environment only where the session must own it; fill toolkit/browser defaults only when unset.
  Validate both `--outputs` and `ARCH_SWAY_WSLG_OUTPUTS` as counts from one through four.
- Keep `status` and `doctor` read-only: no sudo, control lock, or mutations. Keep command lists aligned with launcher
  and Sway usage; probe both clipboard binaries and report shared-bus names without guessing ownership. Media commands
  supplied only by dependencies are advisory. `logs` must work without systemd, the bus, or a running session.
- Bound all waits, including locks, user-bus calls, WSLg, IPC, and shutdown. Portal scope is GTK file choosers in separate
  WSLg windows; Flatpak integration, screenshot/screen-sharing portals, moving WSLg windows, and automatic WSLg recovery
  are out of scope.

## Windows clipboard bridge

- The Linux broker connects directly to nested Sway's `ext-data-control-v1`; one persistent Win32 agent uses the native
  clipboard API and a message-only window. They communicate through inherited pipes, never the parent Wayland socket.
  Nothing is installed on Windows; clipboard contents stay in memory and never enter files or logs.
- Forward UTF-8 plain text only, at most 16 MiB. Reject malformed text, embedded NUL, and empty/unreadable selections
  without clearing the other clipboard. Exclude the KDE password-manager sensitivity hint unless
  `ARCH_SWAY_WSLG_SYNC_SENSITIVE=1`; do not claim arbitrary sensitivity detection. Normalize line endings in one pass:
  CRLF toward Windows, LF toward Sway, lone CR treated as a line break.
- Preserve `SyncSlots` serialization and `MirrorState` sequence/hash echo suppression. Commit Windows writes after ACK;
  commit Wayland publishes when announced and read back through the same bounded offer path. Never rely on timing or
  client order. Keep replaced sources serving pending paste requests until canceled.
- Give existing Windows text priority at startup, including changes during Wayland initialization. The agent's startup
  snapshot must pair the sequence number with the text it read. Queue undelivered text, retry refused access once, and
  let newer selections supersede retries. Agent restarts must not overwrite a Windows change with stale Sway text.
- `degraded` indicates transport failure. Rejected selections and superseded writes do not degrade health; a second
  refusal of the same text degrades health only if no newer selection has superseded it. Rewrite status only when its
  content changes.
- Bound frames, allocations, transfers, Hello, heartbeats, restarts, and shutdown; reset restart budgets after stable
  operation. Fail immediately after registry discovery if `ext_data_control_manager_v1` or a seat is missing; treat
  Hello/heartbeat timeouts and exhausted restart budgets as errors. Keep `BrokerError::exit_code` aligned with launcher
  `RestartPreventExitStatus`: permanent failures exit `3`, a vanished data-control device exits `1`, and compositor
  disconnection exits cleanly.
- Sway invokes only `__start_clipboard`; the broker runs as a transient user service bound to the session scope.
  Close the agent's stdin first, wait a bounded grace period, then kill if needed and reap it on every broker exit.
- Any frame layout change (including the 28-byte header or payload layout), or message-set change, requires a
  `PROTOCOL_VERSION` bump. After any clipboard workspace change, rebuild and replace **both** release binaries and
  regenerate/verify `clipboard.sha256` using `clipboard/README.md`. Never leave checked-in payloads built from older
  source.

## Code style and documentation

- Target current Arch Bash with `set -Eeuo pipefail`, `umask 077`, four-space indentation, quoted expansions, arrays,
  `[[ ... ]]`, and function-local variables. Match surrounding comments and explain why.
- Use stable Rust, edition 2024. Keep platform APIs in `wayland.rs` and `windows.rs`, shared logic in `clipboard-core`,
  and Win32 unsafe blocks small. Reuse `clipboard/crates/clipboard-broker/src/io.rs` for bounded pipe I/O; set each
  descriptor nonblocking once when acquired.
- Update user-visible behavior in `README.md` first, then keep `README_CN.md` section-for-section equivalent. Keep
  contributor details here or in `clipboard/README.md`; remove stale documentation.
- Use CalVer `YYYY.M.RELEASE`, starting at `1` each month. Package-list changes require a `VERSION` bump and matching
  uninstall lists in both READMEs.
- When a commit is authorized, always use Conventional Commits: `<type>[optional scope]: <imperative description>`,
  lowercase after the colon, no trailing period, at most 72 characters. Types: `feat`, `fix`, `refactor`, `perf`, `docs`,
  `style`, `chore`.
  Explain why in a wrapped body; mark broken contracts with `!` or a `BREAKING CHANGE:` footer and migration action.

## Setup and validation

Use Arch Bash/ShellCheck, a C compiler/linker (Arch `base-devel`), and stable Rust with `rustfmt`, `clippy`,
and target `x86_64-pc-windows-msvc`.
Rebuilding the Windows payload also needs `cargo install cargo-xwin --locked`; build the Linux payload on Linux.
Run every check below and report results, including blockers, before finishing or opening a PR:

```bash
# From the repository root, on Arch Linux:
bash -n install.sh
bash -n .local/bin/arch-sway-wslg
shellcheck -S warning install.sh .local/bin/arch-sway-wslg
git diff --check
bash tests/install-session.sh
cd clipboard
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo check -p clipboard-agent-win --target x86_64-pc-windows-msvc
```

On a non-Arch host, run shell checks in a disposable Arch container (explicit x86-64 also supports ARM hosts):

```bash
docker run --rm --platform linux/amd64 -v "$PWD:/workspace:ro" -w /workspace archlinux:latest bash -lc '
  set -eu
  bash -n install.sh
  bash -n .local/bin/arch-sway-wslg
  sed -i "s/^#\?DownloadUser.*/#DownloadUser = alpm/" /etc/pacman.conf
  pacman -Sy --noconfirm shellcheck >/dev/null
  shellcheck -S warning install.sh .local/bin/arch-sway-wslg'
```

Disable pacman's download user only inside that container. Run broker checks on Linux; with a read-only repository
mount, set `CARGO_TARGET_DIR` to a writable container directory. ShellCheck warnings and Clippy must stay clean;
the only intentional ShellCheck info findings are `SC1003` and `SC2016`. There is no automated WSLg end-to-end test or
Sway configuration validation; do not add either unless asked. Sway configuration runtime errors belong in the managed
session log. Report runtime validation separately from static checks.
