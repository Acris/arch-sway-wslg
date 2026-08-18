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
LIBEXEC_PAYLOAD_DIR="$ROOT/.local/libexec/$NAME"
SETTINGS_DIR="$CONFIG_HOME/$NAME"
BROWSER_FILE="$SETTINGS_DIR/browser"

DESKTOP_OVERRIDE_FILES=("$ROOT/extras/desktop-overrides/"*.desktop)
[[ -e "${DESKTOP_OVERRIDE_FILES[0]}" ]] || DESKTOP_OVERRIDE_FILES=()

MANAGED_CONFIG_DIRS=(sway waybar swaync swaynag foot fuzzel yazi)
# Files and directories inside the managed configuration that belong to the
# user. They survive every reinstall and are read after the managed files.
LOCAL_OVERRIDE_PATHS=(sway/config.d foot/local.ini fuzzel/local.ini)
# Bundled wallpaper, relative to the managed configuration root.
WALLPAPER_RELATIVE="sway/wallpapers/dark-star.jpg"

# Providers and shared prerequisites that have to be resolved before the
# desktop stack. jack2 is the default JACK provider and is skipped when
# pipewire-jack is already installed. Maple Mono ships Nerd Font glyphs but its
# AUR package does not declare the ttf-font-nerd provider Yazi depends on.
BOOTSTRAP_PACKAGES=(noto-fonts jack2 ttf-nerd-fonts-symbols-mono)

# Remaining top-level packages. Ordinary dependencies are left to pacman.
MAIN_PACKAGES=(
    # Desktop stack
    sway xorg-xwayland swaybg swayidle waybar swaync foot fuzzel nwg-look
    qt5-wayland qt6-wayland
    # Terminal file manager
    yazi
    # Credential storage; oo7 is skipped when another Secret Service exists
    oo7 seahorse
    # Appearance
    adw-gtk-theme papirus-icon-theme ttf-sarasa-gothic
    maplemono-nf-cn-unhinted noto-fonts-emoji
    # WSLg integration and desktop plumbing
    wl-clipboard xdg-utils
)

YAZI_INTEGRATION_PACKAGES=(fd ripgrep fzf zoxide jq 7zip)
YAZI_PREVIEW_PACKAGES=(ffmpeg poppler resvg imagemagick)

BROWSER_KEYS=(firefox chromium chrome edge none)
declare -A BROWSER_LABELS=(
    [firefox]="Firefox"
    [chromium]="Chromium"
    [chrome]="Google Chrome"
    [edge]="Microsoft Edge"
    [none]="No browser"
)
declare -A BROWSER_PACKAGES=(
    [firefox]="firefox"
    [chromium]="chromium"
    [chrome]="google-chrome"
    [edge]="microsoft-edge-stable-bin"
)
declare -A BROWSER_SOURCES=(
    [firefox]="official repository"
    [chromium]="official repository"
    [chrome]="AUR"
    [edge]="AUR"
)
declare -A BROWSER_COMMANDS=(
    [firefox]="firefox"
    [chromium]="chromium"
    [chrome]="google-chrome-stable"
    [edge]="microsoft-edge-stable"
)
BROWSER_CHOICE="firefox"
declare -A BROWSER_INSTALLED=()

SWAY_SCALE_MAX=4
SWAY_SCALE=1
SYSTEMD_RUNTIME_DIR="/run/user/$EUID"
# GSettings and every systemd call have to reach the persistent user manager,
# which is also the bus the managed session uses. Bind it explicitly: an
# inherited address may be stale or belong to another bus entirely.
USER_BUS_ADDRESS="unix:path=$SYSTEMD_RUNTIME_DIR/bus"
# WSL can recreate /run/user/$UID underneath a running distribution, which
# leaves the bus socket present but unanswered, so no call to it is unbounded.
USER_BUS_TIMEOUT=10
CONTROL_LOCK_TIMEOUT=30

note() { printf '%s\n' "$*"; }
warn() { printf 'WARNING: %s\n' "$*" >&2; }
die() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }

usage() {
    cat <<EOF2
Usage: $0

Installs the package set and the $NAME user configuration.

Managed configuration directories are replaced exactly, except for the local
override paths (${LOCAL_OVERRIDE_PATHS[*]}), which are always preserved.

The installer asks for a backup of the current state, the Sway output scale, a
browser, the optional desktop-entry masks and the GTK appearance defaults.
Backups are written to $BACKUP_BASE.
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
                [[ "$default_answer" == yes ]] && return 0
                return 1
                ;;
            *) warn "please answer y or n" ;;
        esac
    done
}

# A browser that is already installed only contributes its BROWSER value; the
# installer must not run a second package transaction for it.
detect_installed_browsers() {
    local key
    for key in "${BROWSER_KEYS[@]}"; do
        [[ "$key" != none ]] || continue
        if paru -Qq "${BROWSER_PACKAGES[$key]}" >/dev/null 2>&1; then
            BROWSER_INSTALLED[$key]=1
        else
            BROWSER_INSTALLED[$key]=0
        fi
    done
}

prompt_browser() {
    local index key answer marker

    detect_installed_browsers

    note ""
    note "Web browser (sets BROWSER inside the Sway session):"
    for index in "${!BROWSER_KEYS[@]}"; do
        key="${BROWSER_KEYS[$index]}"
        if [[ "$key" == none ]]; then
            printf '  %d) %-15s install none; xdg-open keeps the current default\n' \
                $((index + 1)) "${BROWSER_LABELS[$key]}"
        else
            marker=""
            (( BROWSER_INSTALLED[$key] )) && marker=" [installed]"
            printf '  %d) %-15s %s (%s)%s\n' $((index + 1)) "${BROWSER_LABELS[$key]}" \
                "${BROWSER_PACKAGES[$key]}" "${BROWSER_SOURCES[$key]}" "$marker"
        fi
    done

    while true; do
        printf 'Choose a browser [1-%d, default: 1]: ' "${#BROWSER_KEYS[@]}" >&2
        if ! IFS= read -r answer; then
            printf '\n' >&2
            die "input closed before the browser was chosen"
        fi
        [[ -n "$answer" ]] || answer=1
        if [[ "$answer" =~ ^[0-9]+$ ]] && \
           (( answer >= 1 && answer <= ${#BROWSER_KEYS[@]} )); then
            BROWSER_CHOICE="${BROWSER_KEYS[$((answer - 1))]}"
            return 0
        fi
        warn "enter a number from 1 through ${#BROWSER_KEYS[@]}"
    done
}

# Activation details describe how a process was started, not which bus to use;
# dropping them keeps an inherited value from redirecting children. Callers
# treat the timeout like any other failure to reach the bus.
user_bus_command() {
    timeout "$USER_BUS_TIMEOUT" \
        env -u DBUS_STARTER_ADDRESS -u DBUS_STARTER_BUS_TYPE \
        XDG_RUNTIME_DIR="$SYSTEMD_RUNTIME_DIR" \
        DBUS_SESSION_BUS_ADDRESS="$USER_BUS_ADDRESS" "$@"
}

systemctl_user() {
    user_bus_command systemctl --user "$@"
}

systemd_user_usable() {
    [[ -d /run/systemd/system && -d "$SYSTEMD_RUNTIME_DIR" && \
       -O "$SYSTEMD_RUNTIME_DIR" && -w "$SYSTEMD_RUNTIME_DIR" && \
       -S "$SYSTEMD_RUNTIME_DIR/bus" ]] || return 1
    systemctl_user show-environment >/dev/null 2>&1
}

gsettings_user() {
    user_bus_command gsettings "$@"
}

preflight() {
    (( EUID != 0 )) || die "run this installer as your normal Arch user, not as root"

    local command_name config_name
    for command_name in paru flock sudo systemctl; do
        command -v "$command_name" >/dev/null 2>&1 || \
            die "required command not found: $command_name"
    done

    grep -qi microsoft /proc/sys/kernel/osrelease 2>/dev/null || \
        die "this installer is only supported inside WSL2"
    systemd_user_usable || \
        die "systemd and its user manager are required; restart this Arch WSL distribution with systemd"

    [[ -s "$ROOT/VERSION" ]] || die "version file is missing or empty: $ROOT/VERSION"
    for config_name in "${MANAGED_CONFIG_DIRS[@]}"; do
        [[ -d "$ROOT/.config/$config_name" ]] || \
            die "managed configuration payload is missing: $ROOT/.config/$config_name"
    done
    [[ -x "$ROOT/.local/bin/$NAME" ]] || die "launcher payload is missing: $ROOT/.local/bin/$NAME"
    [[ -x "$LIBEXEC_PAYLOAD_DIR/clipboard-bridge" ]] || \
        die "clipboard bridge payload is missing: $LIBEXEC_PAYLOAD_DIR/clipboard-bridge"
    ((${#DESKTOP_OVERRIDE_FILES[@]})) || die "desktop-entry overrides are missing"
}

# -----------------------------------------------------------------------------
# Sway output scale
# -----------------------------------------------------------------------------
# WSLg performs the scaling Wayland cannot express itself on the Windows side
# and keeps advertising scale 1 on its parent wl_output, so a fractional Windows
# setting such as 125% is not observable from Linux. Ask instead of guessing.
prompt_scale() {
    local answer

    note ""
    note "Sway output scale (match your Windows display scaling):"
    note "  100% -> 1    125% -> 1.25    150% -> 1.5    175% -> 1.75    200% -> 2"

    while true; do
        printf 'Sway output scale [1-%d, default: 1]: ' "$SWAY_SCALE_MAX" >&2
        if ! IFS= read -r answer; then
            printf '\n' >&2
            die "input closed before the scale was answered"
        fi
        [[ -n "$answer" ]] || answer=1
        if [[ "$answer" =~ ^[1-3]([.][0-9]+)?$ || "$answer" =~ ^4([.]0+)?$ ]]; then
            SWAY_SCALE="$answer"
            note "Change it any time in $CONFIG_HOME/sway/config.d/, for example: output * scale 1.5"
            return 0
        fi
        warn "enter a number from 1 through $SWAY_SCALE_MAX, for example: 1, 1.25, 1.5, 2"
    done
}

# -----------------------------------------------------------------------------
# Packages
# -----------------------------------------------------------------------------
install_packages() {
    local package
    local -a selected_bootstrap=()
    local -a selected_main=("${MAIN_PACKAGES[@]}")

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
    note "Paru may request sudo to update the system and install packages."
    paru -Syu --needed "${selected_bootstrap[@]}"

    if paru -T org.freedesktop.secrets >/dev/null 2>&1; then
        selected_main=()
        for package in "${MAIN_PACKAGES[@]}"; do
            [[ "$package" == oo7 ]] || selected_main+=("$package")
        done
        note "An org.freedesktop.secrets credential manager is already installed; skipping oo7."
    fi
    if [[ "$BROWSER_CHOICE" != none ]]; then
        if (( ${BROWSER_INSTALLED[$BROWSER_CHOICE]:-0} )); then
            note "${BROWSER_LABELS[$BROWSER_CHOICE]} is already installed; skipping ${BROWSER_PACKAGES[$BROWSER_CHOICE]}."
        else
            selected_main+=("${BROWSER_PACKAGES[$BROWSER_CHOICE]}")
        fi
    fi

    note ""
    note "Installing ${#selected_main[@]} remaining packages..."
    paru -S --needed "${selected_main[@]}"
}

write_browser_choice() {
    if [[ "$BROWSER_CHOICE" == none ]]; then
        # A recorded choice from an earlier installation would otherwise keep
        # setting BROWSER inside the session.
        rm -f -- "$BROWSER_FILE"
        return 0
    fi
    local browser_command="${BROWSER_COMMANDS[$BROWSER_CHOICE]}"

    mkdir -p "$SETTINGS_DIR"
    printf '%s\n' "$browser_command" > "$BROWSER_FILE"
    note "BROWSER inside Sway: $browser_command"
}

# -----------------------------------------------------------------------------
# Session protection
# -----------------------------------------------------------------------------
stop_active_session() {
    local installed_launcher="$LOCAL_BIN_DIR/$NAME" status_output

    [[ -x "$installed_launcher" ]] || return 0
    status_output="$("$installed_launcher" status 2>&1)" || return 0

    note "$status_output"
    prompt_yes_no "A managed Sway session is running. Stop it before installing?" yes || \
        die "installation cannot replace launcher files while Sway is running"
    "$installed_launcher" stop || die "failed to stop the managed Sway session"
}

# The launcher control lock is only held around the payload replacement, so a
# long package update never blocks another terminal from starting Sway.
take_control_lock() {
    local installed_launcher="$LOCAL_BIN_DIR/$NAME"

    mkdir -p "$STATE_HOME/$NAME"
    chmod 700 "$STATE_HOME/$NAME"
    exec 8>"$CONTROL_LOCK_FILE"
    flock -w "$CONTROL_LOCK_TIMEOUT" 8 || \
        die "another $NAME command has held the control lock for more than ${CONTROL_LOCK_TIMEOUT}s"

    if [[ -x "$installed_launcher" ]] && "$installed_launcher" status >/dev/null 2>&1; then
        die "a managed Sway session started while the installer was preparing; stop it and retry"
    fi
}

# -----------------------------------------------------------------------------
# Backup
# -----------------------------------------------------------------------------
# The backup is the only safety net for the replacement below, so it is offered
# on every run and enabled by default. Users who track their configuration
# elsewhere can decline it.
BACKUP_DIR=""
BACKUP_REQUESTED=0
BACKUP_SUMMARY="declined"
BACKUP_COUNT=0

prompt_backup() {
    note ""
    if prompt_yes_no "Copy the current managed files to a timestamped backup before replacing them?" yes; then
        BACKUP_REQUESTED=1
        BACKUP_SUMMARY="requested; pending installation"
    else
        BACKUP_SUMMARY="declined"
        warn "no backup will be made; the current managed files will be replaced"
    fi
}

backup_item() {
    local source_path="$1" relative_path="$2"
    [[ -e "$source_path" || -L "$source_path" ]] || return 0

    mkdir -p "$(dirname -- "$BACKUP_DIR/$relative_path")"
    cp -a -- "$source_path" "$BACKUP_DIR/$relative_path"
    BACKUP_COUNT=$((BACKUP_COUNT + 1))
}

backup_existing_files() {
    local include_desktop_overrides="$1" config_name source_file basename

    (( BACKUP_REQUESTED )) || return 0

    mkdir -p "$BACKUP_BASE"
    chmod 700 "$BACKUP_BASE"
    BACKUP_DIR="$(mktemp -d "$BACKUP_BASE/$(date +%Y%m%d-%H%M%S).XXXXXX")"

    for config_name in "${MANAGED_CONFIG_DIRS[@]}"; do
        backup_item "$CONFIG_HOME/$config_name" "config/$config_name"
    done
    backup_item "$LOCAL_BIN_DIR/$NAME" "local/bin/$NAME"
    backup_item "$LOCAL_LIBEXEC_DIR" "local/libexec/$NAME"

    if (( include_desktop_overrides )); then
        for source_file in "${DESKTOP_OVERRIDE_FILES[@]}"; do
            basename="${source_file##*/}"
            backup_item "$APPLICATIONS_DIR/$basename" "data/applications/$basename"
        done
    fi

    if (( BACKUP_COUNT == 0 )); then
        rmdir "$BACKUP_DIR"
        BACKUP_DIR=""
        BACKUP_SUMMARY="none needed; nothing was installed yet"
        note "No existing managed files needed a backup."
        return 0
    fi

    {
        printf 'Created: %s\n' "$(date --iso-8601=seconds 2>/dev/null || date)"
        printf 'Version: %s\n' "$VERSION"
        printf 'Original config root: %s\n' "$CONFIG_HOME"
        printf 'Original data root: %s\n' "$DATA_HOME"
        printf 'Original launcher root: %s\n' "$LOCAL_BIN_DIR"
        printf '\nRestore a directory with:\n'
        printf '  rm -rf "%s/sway" && cp -a "%s/config/sway" "%s/sway"\n' \
            "$CONFIG_HOME" "$BACKUP_DIR" "$CONFIG_HOME"
    } > "$BACKUP_DIR/RESTORE-INFO.txt"
    BACKUP_SUMMARY="$BACKUP_DIR"
    note "Backup: $BACKUP_DIR"
}

# -----------------------------------------------------------------------------
# Payload staging
# -----------------------------------------------------------------------------
# Every managed path is staged on the same filesystem and checked before
# anything is replaced, so the replacement itself is a pair of renames that
# fails only when the target cannot be moved at all.
STAGE_CONFIG=""
STAGE_LOCAL=""
STAGE_DATA=""

cleanup_staging() {
    local path
    for path in "$STAGE_CONFIG" "$STAGE_LOCAL" "$STAGE_DATA"; do
        if [[ -n "$path" ]]; then
            rm -rf -- "$path"
        fi
    done
    STAGE_CONFIG=""
    STAGE_LOCAL=""
    STAGE_DATA=""
}

# The old path is moved aside first, so a failing second rename can put it back,
# and it is deleted only once the new one is in place.
replace_path() {
    local staged="$1" target="$2"
    local previous="${target}.${NAME}.old.$$"

    if [[ -e "$previous" || -L "$previous" ]]; then
        warn "a leftover replacement path is in the way: $previous"
        return 1
    fi
    if [[ -e "$target" || -L "$target" ]] && ! mv -- "$target" "$previous"; then
        return 1
    fi
    if ! mv -- "$staged" "$target"; then
        if [[ -e "$previous" || -L "$previous" ]]; then
            mv -- "$previous" "$target" || \
                warn "manual recovery required: $previous -> $target"
        fi
        return 1
    fi
    rm -rf -- "$previous"
}

payload_fail() {
    cleanup_staging
    if [[ -n "$BACKUP_DIR" ]]; then
        warn "restore the previous state from $BACKUP_DIR; see its RESTORE-INFO.txt"
    fi
    die "$1"
}

# -----------------------------------------------------------------------------
# Payload rendering
# -----------------------------------------------------------------------------
render_marker() {
    local file="$1" marker="$2" value="$3" content occurrences

    occurrences="$(grep -Fc -- "$marker" "$file" || true)"
    if [[ "$occurrences" != 1 ]]; then
        warn "expected exactly one $marker in $file"
        return 1
    fi
    content="$(<"$file")"
    printf '%s\n' "${content//"$marker"/"$value"}" > "$file"
}

# The Sway config quotes this value, which covers spaces, and doubles a dollar
# sign to keep it literal. A quote, backslash, or newline cannot be expressed
# there at all, so such a configuration root is rejected instead of mangled.
sway_wallpaper_value() {
    local path="$CONFIG_HOME/$WALLPAPER_RELATIVE"

    case "$path" in
        *'"'*|*'\'*|*$'\n'*|*$'\r'*)
            warn "configuration root contains a quote, backslash, or newline unsupported by the Sway config"
            return 1
            ;;
    esac
    printf '%s\n' "${path//\$/\$\$}"
}

seed_local_override() {
    local target="$1"
    shift
    [[ -e "$target" ]] && return 0
    printf '%s\n' "$@" > "$target"
}

# Keep the user's own override files and create them on a first installation.
stage_local_overrides() {
    local relative

    for relative in "${LOCAL_OVERRIDE_PATHS[@]}"; do
        if [[ -e "$CONFIG_HOME/$relative" ]]; then
            cp -a -- "$CONFIG_HOME/$relative" "$STAGE_CONFIG/$relative" || return 1
        fi
    done

    mkdir -p "$STAGE_CONFIG/sway/config.d" || return 1
    seed_local_override "$STAGE_CONFIG/sway/config.d/10-local.conf" \
        '# Personal Sway configuration. This directory is never replaced by' \
        '# arch-sway-wslg and is included after every managed setting.' \
        '#' \
        '# Examples:' \
        '#   output * scale 1.5' \
        '#   workspace 1 output WL-1' \
        '#   workspace 9 output WL-2' \
        '#   bindsym $mod+p exec firefox' \
        '#' \
        '# Do not add dbus-update-activation-environment here. The session runs' \
        '# on your persistent user bus, where that call reaches the systemd user' \
        '# manager: the nested display values would outlive the session and could' \
        '# not be removed again. Programs started by Sway inherit them already.'
    seed_local_override "$STAGE_CONFIG/foot/local.ini" \
        '# Personal Foot configuration; never replaced by arch-sway-wslg.' \
        '# Options set here win, for example:' \
        '#   [main]' \
        '#   font=Maple Mono NF CN:size=12'
    seed_local_override "$STAGE_CONFIG/fuzzel/local.ini" \
        '# Personal Fuzzel configuration; never replaced by arch-sway-wslg.' \
        '# Options set here win, for example:' \
        '#   [main]' \
        '#   lines=12'
}

render_staged_payload() {
    local wallpaper

    render_marker "$STAGE_LOCAL/bin/$NAME" "__ARCH_SWAY_WSLG_VERSION__" "$VERSION" || return 1
    render_marker "$STAGE_CONFIG/sway/config" "__ARCH_SWAY_WSLG_SCALE__" "$SWAY_SCALE" || return 1
    wallpaper="$(sway_wallpaper_value)" || return 1
    render_marker "$STAGE_CONFIG/sway/config" "__ARCH_SWAY_WSLG_WALLPAPER__" "$wallpaper" || return 1
    render_marker "$STAGE_CONFIG/foot/foot.ini" "__ARCH_SWAY_WSLG_FOOT_LOCAL__" \
        "$CONFIG_HOME/foot/local.ini" || return 1
    render_marker "$STAGE_CONFIG/fuzzel/fuzzel.ini" "__ARCH_SWAY_WSLG_FUZZEL_LOCAL__" \
        "$CONFIG_HOME/fuzzel/local.ini" || return 1
}

check_staged_payload() {
    [[ -s "$STAGE_CONFIG/$WALLPAPER_RELATIVE" ]] || {
        warn "staged Sway wallpaper payload is missing"
        return 1
    }
    bash -n "$STAGE_LOCAL/bin/$NAME" || return 1
    bash -n "$STAGE_LOCAL/libexec/$NAME/clipboard-bridge" || return 1
}

install_payload() {
    local include_desktop_overrides="$1" config_name source_file basename

    note ""
    note "Staging and checking the configuration payload before installation..."
    mkdir -p "$CONFIG_HOME" "$HOME/.local" "$LOCAL_BIN_DIR" "$HOME/.local/libexec"
    (( include_desktop_overrides )) && mkdir -p "$APPLICATIONS_DIR"

    STAGE_CONFIG="$(mktemp -d "$CONFIG_HOME/.${NAME}.config.XXXXXX")" || \
        die "failed to create configuration staging directory"
    STAGE_LOCAL="$(mktemp -d "$HOME/.local/.${NAME}.local.XXXXXX")" || {
        cleanup_staging
        die "failed to create launcher staging directory"
    }
    if (( include_desktop_overrides )); then
        STAGE_DATA="$(mktemp -d "$APPLICATIONS_DIR/.${NAME}.desktop.XXXXXX")" || {
            cleanup_staging
            die "failed to create desktop-entry staging directory"
        }
    fi

    for config_name in "${MANAGED_CONFIG_DIRS[@]}"; do
        cp -a -- "$ROOT/.config/$config_name" "$STAGE_CONFIG/$config_name" || \
            payload_fail "failed to stage config/$config_name"
    done
    mkdir -p "$STAGE_LOCAL/bin" "$STAGE_LOCAL/libexec" || \
        payload_fail "failed to create launcher staging layout"
    install -m 755 -- "$ROOT/.local/bin/$NAME" "$STAGE_LOCAL/bin/$NAME" || \
        payload_fail "failed to stage the launcher"
    cp -a -- "$LIBEXEC_PAYLOAD_DIR" "$STAGE_LOCAL/libexec/$NAME" || \
        payload_fail "failed to stage private launcher helpers"
    if (( include_desktop_overrides )); then
        install -m 644 -- "${DESKTOP_OVERRIDE_FILES[@]}" "$STAGE_DATA/" || \
            payload_fail "failed to stage desktop-entry overrides"
    fi

    stage_local_overrides || payload_fail "failed to stage the local override files"
    render_staged_payload || payload_fail "failed to render the staged payload"
    check_staged_payload || payload_fail "staged payload check failed"

    for config_name in "${MANAGED_CONFIG_DIRS[@]}"; do
        replace_path "$STAGE_CONFIG/$config_name" "$CONFIG_HOME/$config_name" || \
            payload_fail "failed to replace config/$config_name"
    done
    replace_path "$STAGE_LOCAL/bin/$NAME" "$LOCAL_BIN_DIR/$NAME" || \
        payload_fail "failed to replace the launcher"
    # Replacing the whole directory also removes helpers dropped by an earlier
    # release, which a file-by-file copy would leave behind.
    replace_path "$STAGE_LOCAL/libexec/$NAME" "$LOCAL_LIBEXEC_DIR" || \
        payload_fail "failed to replace the private helpers"
    if (( include_desktop_overrides )); then
        for source_file in "${DESKTOP_OVERRIDE_FILES[@]}"; do
            basename="${source_file##*/}"
            replace_path "$STAGE_DATA/$basename" "$APPLICATIONS_DIR/$basename" || \
                payload_fail "failed to install $basename"
        done
    fi

    cleanup_staging
    if (( include_desktop_overrides )); then
        note "Desktop-entry overrides: installed"
    else
        note "Desktop-entry overrides: skipped; every existing same-named file was left unchanged"
    fi
}

list_desktop_overrides() {
    local source_file
    note "Optional desktop-entry overrides:"
    for source_file in "${DESKTOP_OVERRIDE_FILES[@]}"; do
        printf '  %s\n' "$APPLICATIONS_DIR/${source_file##*/}"
    done
}

# -----------------------------------------------------------------------------
# GTK appearance
# -----------------------------------------------------------------------------
APPEARANCE_SCHEMA="org.gnome.desktop.interface"
APPEARANCE_KEYS=(gtk-theme color-scheme icon-theme font-name cursor-theme cursor-size)
APPEARANCE_VALUES=("'adw-gtk3-dark'" "'prefer-dark'" "'Papirus-Dark'" \
                   "'Sarasa UI SC 11'" "'Adwaita'" '28')
APPEARANCE_REQUESTED=0
APPEARANCE_SUMMARY="not requested"

prompt_appearance_defaults() {
    local current index
    if ! command -v gsettings >/dev/null 2>&1; then
        APPEARANCE_SUMMARY="not applied; GSettings is unavailable"
        warn "gsettings is unavailable; GTK appearance defaults were not applied"
        return 0
    fi

    note ""
    note "Current GTK appearance settings:"
    for index in "${!APPEARANCE_KEYS[@]}"; do
        if current="$(gsettings_user get "$APPEARANCE_SCHEMA" \
            "${APPEARANCE_KEYS[$index]}" 2>&1)"; then
            printf '  %-14s %s\n' "${APPEARANCE_KEYS[$index]}:" "$current"
        else
            printf '  %-14s unavailable (%s)\n' "${APPEARANCE_KEYS[$index]}:" "$current"
        fi
    done

    note ""
    note "Proposed GTK appearance settings:"
    for index in "${!APPEARANCE_KEYS[@]}"; do
        printf '  %-14s %s\n' "${APPEARANCE_KEYS[$index]}:" "${APPEARANCE_VALUES[$index]}"
    done
    note ""
    if ! prompt_yes_no "Apply the proposed GTK appearance settings?" yes; then
        APPEARANCE_SUMMARY="skipped by user"
        note "GTK appearance settings left unchanged."
        return 0
    fi

    APPEARANCE_REQUESTED=1
    APPEARANCE_SUMMARY="approved; pending installation"
}

apply_appearance_defaults() {
    local current index verified=1

    (( APPEARANCE_REQUESTED )) || return 0

    for index in "${!APPEARANCE_KEYS[@]}"; do
        if ! gsettings_user set "$APPEARANCE_SCHEMA" "${APPEARANCE_KEYS[$index]}" \
            "${APPEARANCE_VALUES[$index]}"; then
            verified=0
            continue
        fi
        current="$(gsettings_user get "$APPEARANCE_SCHEMA" \
            "${APPEARANCE_KEYS[$index]}" 2>/dev/null)" || {
            verified=0
            continue
        }
        [[ "$current" == "${APPEARANCE_VALUES[$index]}" ]] || verified=0
    done
    if (( verified )); then
        APPEARANCE_SUMMARY="applied with gsettings"
        note "GTK appearance defaults applied through the systemd user bus."
        return 0
    fi

    APPEARANCE_SUMMARY="partially applied; review with nwg-look"
    warn "some GTK appearance defaults could not be applied or verified"
    warn "use nwg-look after starting Sway to review them"
}

# -----------------------------------------------------------------------------
# Reporting
# -----------------------------------------------------------------------------
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

show_yazi_recommendations() {
    note "Recommended optional Yazi integrations:"
    printf '  paru -S --needed %s\n' "${YAZI_INTEGRATION_PACKAGES[*]}"
    note ""
    note "Optional Yazi rich previews:"
    printf '  paru -S --needed %s\n' "${YAZI_PREVIEW_PACKAGES[*]}"
}

main() {
    case "${1:-}" in
        '') ;;
        -h|--help) usage; return 0 ;;
        *) usage >&2; return 2 ;;
    esac

    preflight

    local install_desktop_overrides=0
    list_desktop_overrides
    note ""
    if prompt_yes_no "Install the desktop-entry overrides listed above?" yes; then
        install_desktop_overrides=1
    fi
    prompt_browser
    prompt_scale
    prompt_backup

    stop_active_session
    install_packages
    prompt_appearance_defaults

    take_control_lock
    backup_existing_files "$install_desktop_overrides"
    install_payload "$install_desktop_overrides"
    write_browser_choice
    apply_appearance_defaults

    show_path_hint

    cat <<EOF2

Installation complete.

Version:                 $VERSION
Installed configuration: $CONFIG_HOME
Installed launcher:      $LOCAL_BIN_DIR/$NAME
Sway output scale:       $SWAY_SCALE
Browser:                 ${BROWSER_LABELS[$BROWSER_CHOICE]}
Backup:                  $BACKUP_SUMMARY

GTK appearance:          $APPEARANCE_SUMMARY
Run nwg-look inside Sway if you want to review or change the appearance.

Your own settings belong in:
  $CONFIG_HOME/sway/config.d/*.conf
  $CONFIG_HOME/foot/local.ini
  $CONFIG_HOME/fuzzel/local.ini
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
