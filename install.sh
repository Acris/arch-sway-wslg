#!/usr/bin/env bash
set -Eeuo pipefail
umask 077

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
NAME="arch-sway-wslg"
VERSION="$(<"$ROOT/VERSION")"

if [[ "${XDG_CONFIG_HOME:-}" == /* ]]; then
    CONFIG_HOME="$XDG_CONFIG_HOME"
else
    CONFIG_HOME="$HOME/.config"
fi
if [[ "${XDG_DATA_HOME:-}" == /* ]]; then
    DATA_HOME="$XDG_DATA_HOME"
else
    DATA_HOME="$HOME/.local/share"
fi
if [[ "${XDG_STATE_HOME:-}" == /* ]]; then
    STATE_HOME="$XDG_STATE_HOME"
else
    STATE_HOME="$HOME/.local/state"
fi
LOCAL_BIN_DIR="$HOME/.local/bin"
LOCAL_LIBEXEC_DIR="$HOME/.local/libexec/$NAME"
APPLICATIONS_DIR="$DATA_HOME/applications"
BACKUP_BASE="$STATE_HOME/$NAME/backups"
CONTROL_LOCK_FILE="$STATE_HOME/$NAME/control.lock"
DESKTOP_OVERRIDES_DIR="$ROOT/extras/desktop-overrides"
LIBEXEC_PAYLOAD_DIR="$ROOT/.local/libexec/$NAME"
PACKAGE_MANIFEST="$ROOT/packages.conf"

MANAGED_CONFIG_DIRS=(sway waybar swaync swaynag foot fuzzel yazi)
BOOTSTRAP_PACKAGES=()
MAIN_PACKAGES=()
YAZI_INTEGRATION_PACKAGES=(fd ripgrep fzf zoxide jq 7zip)
YAZI_PREVIEW_PACKAGES=(ffmpeg poppler resvg imagemagick)

note() { printf '%s\n' "$*"; }
warn() { printf 'WARNING: %s\n' "$*" >&2; }
die() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }

usage() {
    cat <<EOF2
Usage: $0

Installs the package set and the $NAME user configuration.
Managed configuration directories are replaced exactly. The installer asks
about a backup, Sway output scale, and optional desktop-entry masks.
EOF2
}

prompt_yes_no() {
    local question="$1" default_answer="$2" answer suffix
    case "$default_answer" in
        yes) suffix='[Y/n]' ;;
        no)  suffix='[y/N]' ;;
        *)   die "invalid prompt default: $default_answer" ;;
    esac

    while true; do
        printf '%s %s ' "$question" "$suffix" >&2
        if ! IFS= read -r answer; then
            printf '\n' >&2
            die "input closed before the question was answered"
        fi
        answer="${answer,,}"
        case "$answer" in
            y|yes) return 0 ;;
            n|no)  return 1 ;;
            '')
                if [[ "$default_answer" == yes ]]; then
                    return 0
                fi
                return 1
                ;;
            *)     warn "please answer y or n" ;;
        esac
    done
}

prompt_scale() {
    local answer

    while true; do
        printf 'Sway output scale [1]: ' >&2
        if ! IFS= read -r answer; then
            printf '\n' >&2
            die "input closed before the scale was answered"
        fi
        [[ -n "$answer" ]] || answer="1"
        [[ "$answer" == .* ]] && answer="0$answer"

        if [[ "$answer" =~ ^[0-9]+([.][0-9]+)?$ ]] && \
           [[ ! "$answer" =~ ^0+([.]0+)?$ ]]; then
            printf '%s\n' "$answer"
            return 0
        fi
        warn "scale must be a positive decimal number (for example: 1, 1.25, 1.5, 2)"
    done
}

preflight() {
    (( EUID != 0 )) || die "run this installer as your normal Arch user, not as root"

    local command_name config_name helper_name
    for command_name in paru bash cp env flock install mkdir mktemp mount mv readlink rm \
                        runuser sed sudo unshare; do
        command -v "$command_name" >/dev/null 2>&1 || \
            die "required command not found: $command_name"
    done

    grep -qi microsoft /proc/sys/kernel/osrelease 2>/dev/null || \
        die "this installer is only supported inside WSL2"

    [[ -s "$ROOT/VERSION" ]] || die "version file is missing or empty: $ROOT/VERSION"
    [[ -d "$ROOT/.config" ]] || die "configuration payload is missing: $ROOT/.config"
    for config_name in "${MANAGED_CONFIG_DIRS[@]}"; do
        [[ -d "$ROOT/.config/$config_name" ]] || \
            die "managed configuration payload is missing: $ROOT/.config/$config_name"
    done
    [[ -x "$ROOT/.local/bin/$NAME" ]] || die "launcher payload is missing: $ROOT/.local/bin/$NAME"
    [[ -d "$LIBEXEC_PAYLOAD_DIR" ]] || \
        die "private helper payload is missing: $LIBEXEC_PAYLOAD_DIR"
    for helper_name in clipboard-protocol.sh windows-clipboard-setter.ps1 windows-clipboard-watcher.ps1; do
        [[ -s "$LIBEXEC_PAYLOAD_DIR/$helper_name" ]] || \
            die "private helper payload is missing: $LIBEXEC_PAYLOAD_DIR/$helper_name"
    done
    grep -Fq "VERSION=\"$VERSION\"" "$ROOT/.local/bin/$NAME" || \
        die "launcher version does not match VERSION"
    [[ -s "$PACKAGE_MANIFEST" ]] || die "package manifest is missing or empty: $PACKAGE_MANIFEST"
    compgen -G "$DESKTOP_OVERRIDES_DIR/*.desktop" >/dev/null || \
        die "desktop-entry overrides are missing"

    if ! locale -a 2>/dev/null | grep -Eiq '^en_US[.]utf-?8$'; then
        die "generate en_US.UTF-8 before installing; see README.md"
    fi
    [[ "$(locale charmap 2>/dev/null || true)" == "UTF-8" ]] || \
        die "the current locale is not UTF-8; see README.md"

    if [[ ! -S /mnt/wslg/runtime-dir/wayland-0 || ! -S /mnt/wslg/PulseServer ]]; then
        warn "WSLg Wayland/Pulse sockets are not active; installation will continue"
        warn "run '$NAME doctor' from a WSLg session before starting Sway"
    fi
    if [[ ! -d /tmp/.X11-unix ]]; then
        warn "WSLg's /tmp/.X11-unix mapping is missing"
        warn "run '$NAME doctor' after installation for detailed diagnostics"
    fi
    local windows_powershell="${WINDOWS_POWERSHELL:-}"
    if [[ -n "$windows_powershell" ]]; then
        if [[ ! -x "$windows_powershell" ]]; then
            warn "WINDOWS_POWERSHELL is not executable: $windows_powershell"
            warn "installation will continue, but the clipboard bridge will not work until it is corrected"
        fi
    elif command -v powershell.exe >/dev/null 2>&1; then
        : # Preferred: Windows PATH projected into WSL.
    elif [[ -x /mnt/c/Windows/System32/WindowsPowerShell/v1.0/powershell.exe ]]; then
        : # Compatibility fallback for the default WSL automount layout.
    else
        warn "Windows PowerShell 5.1 was not found through WSL interop or the default /mnt/c fallback"
        warn "installation will continue, but the clipboard bridge will not work"
    fi
}

load_packages() {
    local line package section=""
    local -A seen=()
    BOOTSTRAP_PACKAGES=()
    MAIN_PACKAGES=()

    while IFS= read -r line || [[ -n "$line" ]]; do
        line="${line#"${line%%[![:space:]]*}"}"
        line="${line%"${line##*[![:space:]]}"}"
        [[ -n "$line" && "$line" != \#* ]] || continue

        case "$line" in
            "[bootstrap]")
                [[ -z "$section" ]] || die "invalid or repeated [bootstrap] section"
                section="bootstrap"
                continue
                ;;
            "[main]")
                [[ "$section" == "bootstrap" ]] || die "[main] must follow [bootstrap]"
                section="main"
                continue
                ;;
            \[*\]) die "unknown package manifest section: $line" ;;
        esac

        [[ -n "$section" ]] || die "package appears before a manifest section: $line"
        [[ "$line" =~ ^[a-zA-Z0-9@._+:-]+$ ]] || \
            die "invalid package manifest line: $line"
        package="$line"
        [[ -z "${seen[$package]:-}" ]] || die "duplicate package in manifest: $package"
        seen[$package]=1
        if [[ "$section" == "bootstrap" ]]; then
            BOOTSTRAP_PACKAGES+=("$package")
        else
            MAIN_PACKAGES+=("$package")
        fi
    done < "$PACKAGE_MANIFEST"

    ((${#BOOTSTRAP_PACKAGES[@]})) || die "package manifest contains no bootstrap packages"
    ((${#MAIN_PACKAGES[@]})) || die "package manifest contains no main packages"
}

protect_active_session() {
    local installed_launcher="$LOCAL_BIN_DIR/$NAME" status_output

    if [[ -x "$installed_launcher" ]] && \
       status_output="$("$installed_launcher" status 2>&1)"; then
        note "$status_output"
        if ! prompt_yes_no "A managed Sway session is running. Stop it before installing?" yes; then
            die "installation cannot replace launcher files while Sway is running"
        fi
        "$installed_launcher" stop || die "failed to stop the managed Sway session"
    fi

    # Hold the launcher's control lock through package installation and payload
    # replacement so another terminal cannot start a new managed session in the
    # gap after the check above.
    mkdir -p "$STATE_HOME/$NAME"
    chmod 700 "$STATE_HOME/$NAME"
    exec 8>"$CONTROL_LOCK_FILE"
    flock 8

    if [[ -x "$installed_launcher" ]] && "$installed_launcher" status >/dev/null 2>&1; then
        die "a managed Sway session started while the installer was preparing; stop it and retry"
    fi
}

BACKUP_DIR=""
BACKUP_COUNT=0

ensure_backup_dir() {
    [[ -z "$BACKUP_DIR" ]] || return 0
    mkdir -p "$BACKUP_BASE"
    chmod 700 "$BACKUP_BASE"
    BACKUP_DIR="$(mktemp -d "$BACKUP_BASE/$(date +%Y%m%d-%H%M%S).XXXXXX")"
}

backup_item() {
    local source_path="$1" relative_path="$2"
    [[ -e "$source_path" || -L "$source_path" ]] || return 0

    ensure_backup_dir
    mkdir -p "$(dirname -- "$BACKUP_DIR/$relative_path")"
    cp -a -- "$source_path" "$BACKUP_DIR/$relative_path"
    BACKUP_COUNT=$((BACKUP_COUNT + 1))
}

backup_existing_files() {
    local include_desktop_overrides="$1" config_name source_file basename

    for config_name in "${MANAGED_CONFIG_DIRS[@]}"; do
        backup_item "$CONFIG_HOME/$config_name" "config/$config_name"
    done
    backup_item "$LOCAL_BIN_DIR/$NAME" "local/bin/$NAME"
    backup_item "$LOCAL_LIBEXEC_DIR" "local/libexec/$NAME"

    if (( include_desktop_overrides )); then
        for source_file in "$DESKTOP_OVERRIDES_DIR"/*.desktop; do
            basename="${source_file##*/}"
            backup_item "$APPLICATIONS_DIR/$basename" "data/applications/$basename"
        done
    fi

    if (( BACKUP_COUNT == 0 )); then
        note "No existing managed files needed a backup."
        return 0
    fi

    {
        printf 'Created: %s\n' "$(date --iso-8601=seconds 2>/dev/null || date)"
        printf 'Original config root: %s\n' "$CONFIG_HOME"
        printf 'Original data root: %s\n' "$DATA_HOME"
        printf 'Original launcher root: %s\n' "$LOCAL_BIN_DIR"
    } > "$BACKUP_DIR/RESTORE-INFO.txt"
    note "Backup: $BACKUP_DIR"
}

install_packages() {
    local package
    local -a selected_bootstrap=()

    if paru -Qq pipewire-jack >/dev/null 2>&1; then
        for package in "${BOOTSTRAP_PACKAGES[@]}"; do
            [[ "$package" == jack2 ]] || selected_bootstrap+=("$package")
        done
        note "pipewire-jack is already installed; skipping the conflicting jack2 fallback."
    else
        selected_bootstrap=("${BOOTSTRAP_PACKAGES[@]}")
    fi

    note ""
    note "Updating Arch and installing ${#selected_bootstrap[@]} bootstrap packages..."
    note "Paru may request sudo to update the system and install the packages listed in packages.conf."
    paru -Syu --needed "${selected_bootstrap[@]}"

    note ""
    note "Installing ${#MAIN_PACKAGES[@]} remaining packages..."
    paru -S --needed "${MAIN_PACKAGES[@]}"
}

TRANSACTION_TARGETS=()
TRANSACTION_ROLLBACKS=()
TRANSACTION_HAD_OLD=()

transaction_replace() {
    local staged="$1" target="$2" rollback had_old=0
    rollback="${target}.${NAME}.rollback.$$"
    if [[ -e "$rollback" || -L "$rollback" ]]; then
        warn "stale transaction rollback path exists: $rollback"
        return 1
    fi

    if [[ -e "$target" || -L "$target" ]]; then
        mv -- "$target" "$rollback" || return 1
        had_old=1
    fi
    if ! mv -- "$staged" "$target"; then
        (( had_old )) && mv -- "$rollback" "$target" 2>/dev/null || true
        return 1
    fi

    TRANSACTION_TARGETS+=("$target")
    TRANSACTION_ROLLBACKS+=("$rollback")
    TRANSACTION_HAD_OLD+=("$had_old")
}

transaction_rollback() {
    local index target rollback had_old
    for (( index=${#TRANSACTION_TARGETS[@]}-1; index>=0; index-- )); do
        target="${TRANSACTION_TARGETS[$index]}"
        rollback="${TRANSACTION_ROLLBACKS[$index]}"
        had_old="${TRANSACTION_HAD_OLD[$index]}"
        rm -rf -- "$target"
        if (( had_old )); then
            mv -- "$rollback" "$target" 2>/dev/null || \
                warn "manual recovery required: $rollback -> $target"
        fi
    done
}

transaction_finish() {
    local rollback
    for rollback in "${TRANSACTION_ROLLBACKS[@]}"; do
        rm -rf -- "$rollback"
    done
    TRANSACTION_TARGETS=()
    TRANSACTION_ROLLBACKS=()
    TRANSACTION_HAD_OLD=()
}

cleanup_staging_paths() {
    local config_stage="$1" local_stage="$2" data_stage="${3:-}"
    [[ -z "$config_stage" ]] || rm -rf -- "$config_stage"
    [[ -z "$local_stage" ]] || rm -rf -- "$local_stage"
    [[ -z "$data_stage" ]] || rm -rf -- "$data_stage"
}

payload_transaction_fail() {
    local message="$1" config_stage="$2" local_stage="$3" data_stage="${4:-}"
    transaction_rollback
    cleanup_staging_paths "$config_stage" "$local_stage" "$data_stage"
    die "$message; previous files were restored"
}

validate_staged_payload() {
    local config_stage="$1" local_stage="$2"
    local sway_config="$config_stage/sway/config"
    local validation_runtime="$local_stage/validation-runtime"
    local validation_x11="$local_stage/validation-x11" x11_target user_name

    command -v Xwayland >/dev/null 2>&1 || {
        warn "Xwayland is not installed"
        return 1
    }
    grep -Eq '^[[:space:]]*xwayland[[:space:]]+enable([[:space:]]|$)' "$sway_config" || {
        warn "xwayland enable is missing from the staged Sway config"
        return 1
    }
    bash -n "$local_stage/bin/$NAME" || return 1
    bash -n "$local_stage/libexec/$NAME/clipboard-protocol.sh" || return 1

    x11_target="$(readlink -f -- /tmp/.X11-unix)"
    [[ -d "$x11_target" ]] || {
        warn "WSLg X11 socket directory target is unavailable: $x11_target"
        return 1
    }
    mkdir -p "$validation_runtime" "$validation_x11"
    chmod 700 "$validation_runtime"
    chmod 1777 "$validation_x11"
    user_name="$(id -un)"

    note "Sudo is required only to validate Sway inside a temporary private X11 mount namespace."
    note "Validation runs as your normal user and does not change the global WSLg X11 mapping."
    sudo -v || {
        warn "sudo authorization failed"
        return 1
    }
    if ! sudo -n -- unshare --mount --propagation private -- \
        sh -c 'mount --bind "$1" "$2" && exec runuser -u "$3" -- "$4" __validate "$5" "$6" "$7"' \
        _ "$validation_x11" "$x11_target" "$user_name" "$local_stage/bin/$NAME" \
        "$sway_config" "$validation_runtime" /mnt/wslg/runtime-dir/wayland-0; then
        warn "staged Sway config validation failed"
        return 1
    fi
    XDG_CONFIG_HOME="$config_stage" foot -C || {
        warn "staged Foot config validation failed"
        return 1
    }
    XDG_CONFIG_HOME="$config_stage" fuzzel --check-config || {
        warn "staged Fuzzel config validation failed"
        return 1
    }
}

install_payload() {
    local include_desktop_overrides="$1" sway_scale="$2"
    local config_name source_file basename sway_config
    local config_stage local_stage data_stage

    note ""
    note "Staging and validating the configuration payload before installation..."
    mkdir -p "$CONFIG_HOME" "$HOME/.local" "$LOCAL_BIN_DIR" "$HOME/.local/libexec"
    (( include_desktop_overrides )) && mkdir -p "$APPLICATIONS_DIR"
    config_stage="$(mktemp -d "$CONFIG_HOME/.${NAME}.config.XXXXXX")" || \
        die "failed to create configuration staging directory"
    local_stage="$(mktemp -d "$HOME/.local/.${NAME}.local.XXXXXX")" || {
        cleanup_staging_paths "$config_stage" "" ""
        die "failed to create launcher staging directory"
    }
    data_stage=""
    if (( include_desktop_overrides )); then
        data_stage="$(mktemp -d "$APPLICATIONS_DIR/.${NAME}.desktop.XXXXXX")" || {
            cleanup_staging_paths "$config_stage" "$local_stage" ""
            die "failed to create desktop-entry staging directory"
        }
    fi
    trap 'transaction_rollback; cleanup_staging_paths "$config_stage" "$local_stage" "$data_stage"; die "payload transaction interrupted; previous files were restored"' INT TERM HUP

    for config_name in "${MANAGED_CONFIG_DIRS[@]}"; do
        cp -a -- "$ROOT/.config/$config_name" "$config_stage/$config_name" || \
            payload_transaction_fail "failed to stage config/$config_name" \
                "$config_stage" "$local_stage" "$data_stage"
    done
    mkdir -p "$local_stage/bin" "$local_stage/libexec" || \
        payload_transaction_fail "failed to create launcher staging layout" \
            "$config_stage" "$local_stage" "$data_stage"
    install -m 755 -- "$ROOT/.local/bin/$NAME" "$local_stage/bin/$NAME" || \
        payload_transaction_fail "failed to stage the launcher" \
            "$config_stage" "$local_stage" "$data_stage"
    cp -a -- "$LIBEXEC_PAYLOAD_DIR" "$local_stage/libexec/$NAME" || \
        payload_transaction_fail "failed to stage private launcher helpers" \
            "$config_stage" "$local_stage" "$data_stage"
    if (( include_desktop_overrides )); then
        for source_file in "$DESKTOP_OVERRIDES_DIR"/*.desktop; do
            install -m 644 -- "$source_file" "$data_stage/${source_file##*/}" || \
                payload_transaction_fail "failed to stage ${source_file##*/}" \
                    "$config_stage" "$local_stage" "$data_stage"
        done
    fi

    # Validate the complete staged payload before any managed path is touched.
    sway_config="$config_stage/sway/config"
    if [[ "$(grep -Ec '^[[:space:]]*scale[[:space:]]+[0-9]+([.][0-9]+)?[[:space:]]*$' "$sway_config")" != 1 ]]; then
        cleanup_staging_paths "$config_stage" "$local_stage" "$data_stage"
        die "expected exactly one Sway output scale directive in $sway_config"
    fi
    sed -i -E \
        "s/^([[:space:]]*)scale[[:space:]]+[0-9]+([.][0-9]+)?[[:space:]]*$/\\1scale $sway_scale/" \
        "$sway_config" || payload_transaction_fail "failed to stage the selected Sway scale" \
            "$config_stage" "$local_stage" "$data_stage"
    validate_staged_payload "$config_stage" "$local_stage" || \
        payload_transaction_fail "staged payload validation failed" \
            "$config_stage" "$local_stage" "$data_stage"

    TRANSACTION_TARGETS=()
    TRANSACTION_ROLLBACKS=()
    TRANSACTION_HAD_OLD=()
    for config_name in "${MANAGED_CONFIG_DIRS[@]}"; do
        if ! transaction_replace "$config_stage/$config_name" "$CONFIG_HOME/$config_name"; then
            payload_transaction_fail "payload transaction failed while replacing config/$config_name" \
                "$config_stage" "$local_stage" "$data_stage"
        fi
    done
    if ! transaction_replace "$local_stage/bin/$NAME" "$LOCAL_BIN_DIR/$NAME" || \
       ! transaction_replace "$local_stage/libexec/$NAME" "$LOCAL_LIBEXEC_DIR"; then
        payload_transaction_fail "payload transaction failed while replacing launcher files" \
            "$config_stage" "$local_stage" "$data_stage"
    fi
    if (( include_desktop_overrides )); then
        for source_file in "$data_stage"/*.desktop; do
            basename="${source_file##*/}"
            if ! transaction_replace "$source_file" "$APPLICATIONS_DIR/$basename"; then
                payload_transaction_fail "payload transaction failed while installing $basename" \
                    "$config_stage" "$local_stage" "$data_stage"
            fi
        done
    fi

    transaction_finish
    cleanup_staging_paths "$config_stage" "$local_stage" "$data_stage"
    trap - INT TERM HUP
    if (( include_desktop_overrides )); then
        note "Desktop-entry overrides: installed"
    else
        note "Desktop-entry overrides: skipped; every existing same-named file was left unchanged"
    fi
}

list_desktop_overrides() {
    local source_file
    note "Optional desktop-entry overrides:"
    for source_file in "$DESKTOP_OVERRIDES_DIR"/*.desktop; do
        printf '  %s\n' "$APPLICATIONS_DIR/${source_file##*/}"
    done
}

APPEARANCE_SUMMARY="not applied"

apply_appearance_defaults() {
    local failed=0
    gsettings set org.gnome.desktop.interface gtk-theme 'adw-gtk3-dark' || failed=1
    gsettings set org.gnome.desktop.interface color-scheme 'prefer-dark' || failed=1
    gsettings set org.gnome.desktop.interface icon-theme 'Papirus-Dark' || failed=1
    gsettings set org.gnome.desktop.interface font-name 'Sarasa UI SC 11' || failed=1
    gsettings set org.gnome.desktop.interface cursor-theme 'Adwaita' || failed=1
    gsettings set org.gnome.desktop.interface cursor-size 28 || failed=1
    if (( ! failed )); then
        APPEARANCE_SUMMARY="applied with gsettings"
        note "GTK appearance defaults applied with gsettings."
    else
        APPEARANCE_SUMMARY="partially applied; review with nwg-look"
        warn "some GTK appearance defaults could not be applied"
        warn "use nwg-look after starting Sway to review them"
    fi
}

show_path_hint() {
    case ":${PATH:-}:" in
        *":$LOCAL_BIN_DIR:"*) return 0 ;;
    esac

    warn "$LOCAL_BIN_DIR is not in PATH"
    case "${SHELL##*/}" in
        bash)
            note 'Add this to ~/.bashrc, then open a new shell:'
            note '  export PATH="$HOME/.local/bin:$PATH"'
            ;;
        fish)
            note 'Run this once from Fish:'
            note '  fish_add_path $HOME/.local/bin'
            ;;
        *)
            note 'Add ~/.local/bin to your shell PATH, for example:'
            note '  export PATH="$HOME/.local/bin:$PATH"'
            ;;
    esac
}

print_paru_command() {
    local package
    printf '  paru -S --needed'
    for package in "$@"; do
        printf ' %s' "$package"
    done
    printf '\n'
}

show_yazi_recommendations() {
    note "Recommended optional Yazi integrations:"
    print_paru_command "${YAZI_INTEGRATION_PACKAGES[@]}"
    note ""
    note "Optional Yazi rich previews:"
    print_paru_command "${YAZI_PREVIEW_PACKAGES[@]}"
}

main() {
    case "${1:-}" in
        '') ;;
        -h|--help) usage; return 0 ;;
        *) usage >&2; return 2 ;;
    esac

    preflight
    load_packages

    local create_backup=0 install_desktop_overrides=0 sway_scale
    if prompt_yes_no "Back up existing managed configuration before replacing it?" yes; then
        create_backup=1
    else
        warn "managed configuration directories will be replaced without a backup"
    fi
    list_desktop_overrides
    note ""
    if prompt_yes_no "Install the desktop-entry overrides listed above?" yes; then
        install_desktop_overrides=1
    fi
    note ""
    sway_scale="$(prompt_scale)"

    protect_active_session

    if (( create_backup )); then
        backup_existing_files "$install_desktop_overrides"
    fi

    install_packages
    install_payload "$install_desktop_overrides" "$sway_scale"
    apply_appearance_defaults

    if [[ -e "$LOCAL_BIN_DIR/start-sway-wslg" ]]; then
        warn "legacy launcher still exists: $LOCAL_BIN_DIR/start-sway-wslg"
        warn "remove it after confirming that '$NAME' works"
    fi
    show_path_hint

    cat <<EOF2

Installation complete.

Version:                 $VERSION
Installed configuration: $CONFIG_HOME
Installed launcher:      $LOCAL_BIN_DIR/$NAME
Sway output scale:       $sway_scale

GTK appearance:           $APPEARANCE_SUMMARY
Run nwg-look inside Sway if you want to review or change the appearance.
EOF2

    note ""
    show_yazi_recommendations

    cat <<EOF2

Next:
  1. Run: $NAME doctor
  2. Run: $NAME start
  3. Verify: $NAME status
  4. Test clipboard synchronization in both directions
EOF2
}

if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
    main "$@"
fi
