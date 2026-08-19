# AGENTS.md

## Project overview

`arch-sway-wslg` installs a Wayland-first Sway session that runs as a nested compositor in Arch Linux on WSL2/WSLg. It
is not a bare-metal Sway distribution. A supported environment has WSLg, systemd with a working user manager, and a
normal default user with `sudo` and `paru`. The installer neither checks nor changes locales.

`README.md` is the user-facing contract and is written for users, not for maintainers: it describes what the session
does, not how it is built. Keep `README_CN.md` section-for-section equivalent with it and update the English document
first. Do not duplicate user instructions here unless an agent needs the rule to change the project safely.

## Repository layout

| Path                             | Contents                                                                   |
|----------------------------------|----------------------------------------------------------------------------|
| `install.sh`                     | Prompts, package lists, oo7 setup, backups, payload staging, and GSettings |
| `.local/bin/arch-sway-wslg`      | Public launcher: session lifecycle, `status`, and `doctor`                 |
| `.local/libexec/arch-sway-wslg/` | Private session helpers; currently the clipboard bridge                    |
| `.config/`                       | Configuration payload installed into the user's configuration home         |
| `extras/desktop-overrides/`      | Optional desktop entry masks                                               |
| `VERSION`                        | Release version rendered into the installed launcher                       |
| `LICENSE`                        | Project and wallpaper notices                                              |

There is no separate package manifest: the package lists live in `install.sh`, and dependencies pacman resolves on its
own are not listed. The installer replaces markers such as `__ARCH_SWAY_WSLG_VERSION__` in the staged payload, so the
files under `.config/` and `.local/` are templates rather than the installed result.

## Setup and validation

Static shell checks are the whole test suite. Do not add Sway configuration validation or end-to-end WSLg tests unless
asked. On Arch Linux:

```bash
bash -n install.sh
bash -n .local/bin/arch-sway-wslg
bash -n .local/libexec/arch-sway-wslg/clipboard-bridge
shellcheck -S warning install.sh .local/bin/arch-sway-wslg \
  .local/libexec/arch-sway-wslg/clipboard-bridge
git diff --check
```

`shellcheck -S warning` must stay clean. The remaining `-S info` findings are the intentional `SC1003` and `SC2016`
only. On a non-Arch host use a disposable Arch container; pacman's download user has to be disabled inside that
container, and only there:

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

Run the exact commands above and report their results. Implementations may change freely as long as the contracts below
still hold.

## Behavioural contracts

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
  outputs are supported.
- Keep `status` and `doctor` truthful. `doctor` only inspects: it never asks for sudo, changes nothing, and covers the
  commands the session needs, the applications the Sway configuration starts, and the clipboard bridge.
- Bound every wait around systemd, the user bus, WSLg, IPC, locks, and the clipboard.
- Out of scope: portals, Flatpak integration, screen sharing, moving WSLg windows, and recovering automatically after
  WSLg itself fails.

### Installer

- Run as the normal user and delegate package changes to `paru`. Preflight only the commands the installer or the
  session actually uses.
- Refresh the package databases only; upgrading installed packages belongs to the user's own schedule.
- Ask for a browser once, mark the ones already installed, do not reinstall those, record the choice for `BROWSER`, and
  remove a previously recorded choice when the user declines a browser.
- When `oo7` is present, enable and start its user unit and offer to store the keyring password as an encrypted systemd
  user credential, resolved against the user manager's own configuration home and skipped on older systemd.
- Accept an absolute `XDG_CONFIG_HOME`, including spaces and dollar signs, and render the resolved paths into the staged
  payload. Reject a configuration root the Sway configuration cannot express instead of mangling it.
- Ask once for a scale from `1` to `4`, defaulting to `1`. Do not try to detect Windows scaling: WSLg performs it on the
  Windows side and reports a scale of 1.
- Stage and check the whole payload on the same filesystem before replacing any managed directory. Preserve the user
  override paths, seed them on a first installation, and remove optional choices the user has revoked.
- Offer a timestamped backup on every run, default to yes, and keep it the single documented way back: it replaces the
  removed rollback machinery.
- Apply approved GSettings through the explicitly bound persistent user bus after the payload commits, then read the
  values back.
- Hold the control lock around the payload replacement only, never around package installation.

### Clipboard

- Synchronise UTF-8 plain text only; images, HTML, and file lists are out of scope, and selections marked sensitive stay
  excluded unless the user opts in.
- Mirror between the nested Sway selection and the parent WSLg socket, which WSLg already exchanges with Windows. Do not
  add a Windows helper process.
- Serialise forwarding, echo suppression, and parent reads under one lock, and record a selection as mirrored only after
  the peer really holds it.
- Any visit to the parent takes focus away from the session, whichever direction causes it, so none of them may overlap
  typing. Automatic reads require the supervised quiet-period notifier, and a forward waits for the shortcut that
  triggered it to be released, which also collapses a burst of copies into the newest one. A notifier that cannot be
  kept alive stops those reads loudly instead of releasing them, and `status` reports which of the two applies.
- Automatic reads back off the longer the session stays quiet, and any input restores the configured interval.
- Bound bridge and notifier restarts, every clipboard read, and the poll interval, which is rejected below an explicit
  floor. A restarted watcher keeps what has already been mirrored, so it cannot push a stale selection back over the
  peer. The bridge is started by Sway and stays inside the session scope.

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
- This project does not validate Sway configuration; runtime errors belong in the managed session log.

## Code style

- Target the Bash that current Arch ships, not POSIX `sh` or the Bash on macOS. Keep `set -Eeuo pipefail` and
  `umask 077` in every script.
- Use four-space indentation, quoted expansions, arrays for argument lists, `[[ ... ]]` for tests, and `local` variables
  in functions. Prefer built-ins and small, targeted `grep` and `sed` calls.
- Match the surrounding comment density and explain why, not what.
- Keep the compatibility surface small: no code for unsupported distributions or shells, and no fixed `/mnt/c` paths.

## Safety

- Before a destructive operation, validate the exact target and derive paths from the fixed per-user state, runtime, and
  configuration roots.
- Never remove or recreate paths that belong to WSLg or to the user's own configuration.
- Do not commit unless the user asks.

## Change checklist

- Inspect the worktree first and preserve unrelated changes.
- Keep the launcher's command lists aligned with what `start`, `stop`, the clipboard, and `doctor` really use, and the
  installer's preflight aligned with the commands the installer runs.
- Bump `VERSION` when the package list changes, and update the uninstall list in both READMEs at the same time.
- Update `README.md` for user-visible behaviour first, then translate it into `README_CN.md`. Delete stale documentation
  instead of describing behaviour the code no longer has.
- Preserve the contracts above unless the user changes them explicitly, and report the validation commands you ran.

## Commit and pull request guidelines

Use Conventional Commits: `<type>[optional scope]: <imperative description>`, lower case after the colon, no trailing
period, and no more than 72 characters in the subject. The project types are `feat`, `fix`, `refactor`, `perf`, `docs`,
`style`, and `chore`; scopes follow the affected subsystem. Explain why in a wrapped body, and mark a broken contract
with `!` or a `BREAKING CHANGE:` footer that states the action the user has to take.
