# AGENTS.md

## Project overview

`arch-sway-wslg` installs a Wayland-first Sway session that runs as a nested compositor in Arch Linux on WSL2/WSLg. It
is not a bare-metal Sway distribution. A supported environment has WSLg, systemd with a working user manager, and a
normal default user with `sudo` and `paru`. The installer neither checks nor changes locales.

`README.md` is the user-facing contract: install, use, customise, update, uninstall, troubleshoot. It describes
behaviour the user can observe and stays free of implementation detail. Keep `README_CN.md` section-for-section
equivalent with it and update the English document first. Contributor material lives here and in
`clipboard/README.md`; do not put it into the user documents.

The project uses CalVer `YYYY.M.RELEASE`: the release counter starts at `1` for the first release in each month and
increments for subsequent releases in that month. Keep this policy consistent in `VERSION` and both READMEs.

## Repository layout

| Path                             | Contents                                                                    |
|----------------------------------|-----------------------------------------------------------------------------|
| `install.sh`                     | Prompts, package lists, oo7 setup, backups, payload staging, and GSettings  |
| `.local/bin/arch-sway-wslg`      | Public launcher: session lifecycle, `status`, `doctor`, and `logs`          |
| `.local/libexec/arch-sway-wslg/` | Prebuilt Linux clipboard broker, Win32 agent, and `clipboard.sha256`        |
| `clipboard/`                     | Rust workspace for the clipboard data plane; see `clipboard/README.md`      |
| `.config/`                       | Configuration payload installed into the user's configuration home          |
| `extras/desktop-overrides/`      | Optional desktop entry masks                                                |
| `.github/workflows/`             | CI for the Rust workspace; verifies the checked-in payload, uploads nothing |
| `VERSION`                        | Release version rendered into the installed launcher                        |
| `LICENSE`                        | Project and wallpaper notices                                               |

There is no separate package manifest: the package lists live in `install.sh`, and dependencies pacman resolves on its
own are not listed. The installer replaces markers such as `__ARCH_SWAY_WSLG_VERSION__` in the staged payload, so the
files under `.config/` and `.local/` are templates rather than the installed result.

## Setup commands

- Shell checks need `bash` and `shellcheck`; on a non-Arch host use a disposable Arch container (see below).
- Rust checks need the stable toolchain with `clippy` and `rustfmt`, plus the `x86_64-pc-windows-msvc` target.
- Rebuilding the Windows agent needs `cargo-xwin`: `cargo install cargo-xwin --locked`.
- There is no automated end-to-end WSLg test and no Sway configuration validation; do not add either unless asked.

## Testing instructions

Run every command below and report the results. All of them must pass; `shellcheck -S warning` and Clippy must stay
clean. The only intentional ShellCheck `-S info` findings are `SC1003` and `SC2016`.

```bash
bash -n install.sh
bash -n .local/bin/arch-sway-wslg
shellcheck -S warning install.sh .local/bin/arch-sway-wslg
git diff --check
cd clipboard
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo check -p clipboard-agent-win --target x86_64-pc-windows-msvc
```

On a non-Arch host run the shell checks in a disposable Arch container. Pacman's download user has to be disabled
inside that container, and only there:

```bash
docker run --rm -v "$PWD:/workspace:ro" -w /workspace archlinux:latest bash -lc '
  bash -n install.sh
  bash -n .local/bin/arch-sway-wslg
  sed -i "s/^#\?DownloadUser.*/#DownloadUser = alpm/" /etc/pacman.conf
  pacman -Sy --noconfirm shellcheck >/dev/null
  shellcheck -S warning install.sh .local/bin/arch-sway-wslg'
```

Whenever any Rust file under `clipboard/` changes, rebuild both release binaries, replace the files under
`.local/libexec/arch-sway-wslg/`, regenerate `clipboard.sha256`, and run `sha256sum -c` before finishing. The exact
commands are in `clipboard/README.md`. Never leave a payload built from older source in the worktree.

## Behavioural contracts

Implementations may change freely as long as these hold. Preserve them unless the user changes them explicitly.

### Session

- Systemd and its user manager are required; there is no fallback for a session without them.
- `arch-sway-wslg-session.scope` is the authoritative session state. Sway IPC only carries messages and readiness.
- The session runtime is `/run/user/$UID`. The parent Wayland socket is `/mnt/wslg/runtime-dir/wayland-0` and the audio
  endpoint is `/mnt/wslg/PulseServer`, both addressed by absolute path.
- The session shares the persistent user bus at `/run/user/$UID/bus`, because Arch activates services such as the Secret
  Service through the systemd user manager. Bind that address explicitly in every launcher and installer bus call, and
  publish nothing back to the bus or the user manager: the session's own children inherit the nested values already.
- Sudo is used only to create the private X11 mount namespace. Never alter WSLg's `/tmp/.X11-unix` mapping; managed
  processes return to the normal user immediately, and XWayland stays lazy.
- Override an inherited environment value only where the session must own it. Toolkit, Java, VS Code and browser
  defaults are filled in only when the user has set nothing.
- Validate the nested output count wherever it enters, `--outputs` and `ARCH_SWAY_WSLG_OUTPUTS` alike; one through four
  outputs are supported, and an inherited count never reaches the compositor unchecked.
- Keep `status` and `doctor` truthful, and keep both of them observers: they never ask for sudo, take the control lock,
  or change anything, because the installer also runs `status` while holding that lock. `doctor` covers the commands the
  session needs, the applications the Sway configuration starts, and both clipboard binaries, and it reports names on
  the shared bus as they are instead of inferring who owns them. A command no installed package provides directly is
  reported without failing the check.
- `logs` reads an ordinary file, and a session that refuses to start is exactly when it is needed, so it must not depend
  on systemd, the bus, or any other part of the session.
- Bound every wait around systemd, the user bus, WSLg, IPC, locks, and the clipboard.
- Portal support is limited to GTK file choosers shown as separate WSLg windows. Flatpak integration, screenshot
  portals, screen sharing, moving WSLg windows, and recovering automatically after WSLg itself fails are out of scope.

### Installer

- Run as the normal user and delegate package changes to `paru`. Preflight the commands an installation run depends on,
  `paru`'s own included; it is not an audit of every call the script makes.
- Refresh the package databases only; upgrading installed packages belongs to the user's own schedule. Arch does not
  support partial upgrades, so ask for confirmation when the refresh shows the installed system is behind.
- Ask for a browser once, mark the ones already installed, do not reinstall those, record the choice for `BROWSER`, and
  remove a previously recorded choice when the user declines a browser.
- When `oo7` is present, enable and start its user unit and offer to store the keyring password as an encrypted systemd
  user credential, resolved against the user manager's own configuration home and skipped on older systemd.
- Accept an absolute `XDG_CONFIG_HOME`, including spaces and dollar signs, and render the resolved paths into the staged
  payload. Reject a configuration root the Sway configuration cannot express instead of mangling it.
- Ask once for a scale from `1` to `4`, defaulting to `1`. Do not try to detect Windows scaling: WSLg performs it on the
  Windows side and reports a scale of 1.
- Stage and check the whole payload on the same filesystem before replacing any managed directory, and remove the
  staging directories on every exit, an interrupt included. The check covers the files the session cannot start without,
  the imported override stylesheets among them. Preserve the user override paths and seed them on a first installation.
  Declining the optional desktop-entry masks leaves any existing same-named files unchanged.
- Report a failed write instead of continuing: a helper called in a condition runs without `errexit`, so every step in
  it reports its own failure.
- Offer a timestamped backup on every run, default to yes, and keep it the single documented way back: it replaces the
  removed rollback machinery.
- Apply approved GSettings through the explicitly bound persistent user bus after the payload commits, then read the
  values back.
- Take the control lock once the questions and the package installation are done, and hold it to the end of the run: the
  backup, the replacement and the recorded choices have to describe one installation. Never hold it across `paru`.

### Clipboard

- Synchronise UTF-8 plain text only; images, HTML, and file lists are out of scope. Text over 16 MiB, malformed UTF-8,
  embedded NUL, and empty selections are never forwarded: an empty or unreadable selection on one side leaves the other
  side untouched. Selections carrying the KDE password-manager sensitivity hint stay excluded unless the user opts in
  with `ARCH_SWAY_WSLG_SYNC_SENSITIVE=1`. Do not claim detection of arbitrary hints.
- Line endings are normalised in one pass: CRLF towards Windows, LF towards Sway, a lone CR becomes a line break. The
  conversion is documented in the READMEs; keep them in step.
- The Linux broker talks directly to Sway's `ext-data-control-v1` protocol and the single persistent Win32 agent uses
  the native Windows clipboard API; neither direction visits the parent WSLg Wayland socket, and the agent creates only
  a message-only window. Nothing is installed on the Windows side.
- The broker fails fast instead of waiting forever: a compositor without `ext_data_control_manager_v1` or a seat, an
  agent that never sends `Hello` within `HELLO_TIMEOUT`, a missed heartbeat, and an exhausted restart budget are all
  errors. Failures another start would only repeat (a configuration error, a compositor without the globals, an
  exhausted restart budget) exit with status `3`, which the launcher lists in `RestartPreventExitStatus` so the
  clipboard stays down for the rest of the session instead of cycling; a vanished data-control device exits `1` and is
  retried within the unit's start limit; the compositor closing the connection (Sway exiting) is a clean exit. Keep
  `BrokerError::exit_code` and the launcher property in step.
- Both directions are serialised in the broker state machine with `SyncSlots` (one pending text per direction) and
  `MirrorState` (echo suppression). A Windows write commits only after its request ACK. A Wayland publish commits when
  the compositor announces and serves the selection back to it, which wlroots does for every data-control device
  including the one that set it. Every offer follows the same bounded read path, and its SHA-256 identifies a publish
  without assuming an order between clients. Echoes are suppressed by sequence numbers and committed hashes, never by
  wall-clock comparisons, and clipboard payloads never touch the disk or the log.
- Cold startup gives an existing Windows text selection priority, including a Windows change that arrives while the
  Wayland side is still initialising; the agent's startup snapshot pairs the sequence number with the text it read.
- Text that cannot be delivered is queued, not dropped: a Sway selection made while the agent is unavailable is sent
  once it is back, a refused Windows write is retried once after a short delay unless a newer selection on either side
  has superseded it, and an agent restart cannot push an old Sway selection over a Windows change made while it was
  unavailable. The agent likewise reads a Windows update it could not open once more before reporting the selection
  unavailable.
- `degraded` means a transport failure (agent or Wayland). Rejecting a single selection, and a refused write that a
  newer selection replaces anyway, is logged but does not change the health; only the second refusal of the same text
  does. The status file under the clipboard state directory is rewritten only when its content changes.
- Bound clipboard access, IPC payloads, heartbeats, transfers, shutdown, and restarts. The restart budget resets after
  stable operation. A replaced data source keeps serving `send` requests until the compositor cancels it, so a paste
  that races a new copy never receives empty data.
- Sway starts only the short-lived internal launcher command (`__start_clipboard`) that knows the nested display. The
  broker itself is a transient systemd user service bound to the session scope, and the Win32 agent exits when its
  inherited pipe closes: shutdown closes the agent's stdin first, waits a bounded grace period, and only then kills.
  The broker reaps the agent the same way before it exits itself, whichever way its loop ended.
- The agent pipe carries length-prefixed frames with a 28-byte header (`PROTOCOL_VERSION` in `clipboard-core`). Any
  change to the frame layout or message set must bump the version, and both binaries must be rebuilt and deployed
  together. The installer refuses a payload that fails `clipboard.sha256`, and `doctor` runs `--probe` on both binaries.

### Configuration

- The user extension points are `sway/config.d/*.conf`, `foot/local.ini`, `fuzzel/local.ini`, `waybar/local.css`, and
  `swaync/local.css`. Preserve them, seed them once, and keep them read last so the user's values win. GTK drops a whole
  stylesheet whose import is missing, so the two CSS files have to exist after every installation.
- The Waybar and SwayNC layouts, swaynag, and Yazi stay fully managed, because their include behaviour cannot offer a
  safe extension point; only the two stylesheets take overrides.
- Sizes are the upstream defaults of each component. Fonts, colours, and the bar's pill styling are the deliberate
  deviations; a value that only restates an upstream default is dropped instead of repeated. Where upstream states no
  default, follow common Sway configurations and prefer multiples of four so fractional output scales stay on whole
  pixels.
- Start Sway's children directly. `swaybar_command` is executed without a shell, so it takes no shell syntax, and
  nothing publishes an activation environment: on the shared user bus that would outlive the session.
- Sway expands a variable while it parses the line that uses it, so `config.d` can add and re-bind, but it cannot retune
  a value the managed configuration has already consumed. Do not document it as if it could.
- This project does not validate Sway configuration; runtime errors belong in the managed session log.

## Code style

- Target the Bash that current Arch ships, not POSIX `sh` or the Bash on macOS. Keep `set -Eeuo pipefail` and
  `umask 077` in every script.
- Use four-space indentation, quoted expansions, arrays for argument lists, `[[ ... ]]` for tests, and `local` variables
  in functions. Prefer built-ins and small, targeted `grep` and `sed` calls.
- Match the surrounding comment density and explain why, not what.
- Keep the compatibility surface small: no code for unsupported distributions or shells, and no fixed `/mnt/c` paths.
- Rust targets stable edition 2024. Keep platform APIs behind target-specific modules (`wayland.rs`, `windows.rs`),
  share protocol, text, and state logic through `clipboard-core`, deny Clippy warnings in validation, bound every
  allocation derived from IPC, and keep `unsafe` Win32 blocks as small as practical.
- Bounded pipe I/O goes through `clipboard-broker/src/io.rs`; do not add a second deadline helper. Its callers make a
  descriptor non-blocking once, when they obtain it, rather than per call.

## Safety

- Before a destructive operation, validate the exact target and derive paths from the fixed per-user state, runtime, and
  configuration roots.
- Never remove or recreate paths that belong to WSLg or to the user's own configuration.
- Do not commit unless the user asks.

## Change checklist

- Inspect the worktree first and preserve unrelated changes.
- Keep the launcher's command lists aligned with what `start`, `stop`, the clipboard, and `doctor` really use.
- After changing anything under `clipboard/`, rebuild both binaries and refresh `clipboard.sha256` as described in
  "Testing instructions".
- Bump `VERSION` when the package list changes, and update the uninstall list in both READMEs at the same time.
- Update `README.md` for user-visible behaviour first, then translate it into `README_CN.md`. Delete stale documentation
  instead of describing behaviour the code no longer has, and keep implementation detail out of both.
- Preserve the contracts above unless the user changes them explicitly, and report the validation commands you ran.

## PR instructions

- Use Conventional Commits: `<type>[optional scope]: <imperative description>`, lower case after the colon, no trailing
  period, and no more than 72 characters in the subject.
- The project types are `feat`, `fix`, `refactor`, `perf`, `docs`, `style`, and `chore`; scopes follow the affected
  subsystem, for example `installer` or `clipboard`.
- Explain why in a wrapped body. Mark a broken contract with `!` or a `BREAKING CHANGE:` footer that states the action
  the user has to take.
- Run the full "Testing instructions" list before opening a pull request and mention the results.
