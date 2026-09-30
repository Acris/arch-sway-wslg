#!/usr/bin/env bash
set -Eeuo pipefail
umask 077

# Fault injection for the installer guards; no real bus, packages or user files.
TEST_DIR="$(mktemp -d)"
trap 'rm -rf -- "$TEST_DIR"' EXIT
# CI containers can inherit a mounted HOME owned by the host user. Give the
# installer an owned fixture home before it derives any per-user paths.
export HOME="$TEST_DIR/home"
mkdir -p "$HOME"
# shellcheck source=SCRIPTDIR/../install.sh
source "$(dirname -- "${BASH_SOURCE[0]}")/../install.sh"
STATE_HOME="$TEST_DIR/state"
LOCAL_BIN_DIR="$TEST_DIR/bin"
# Used by take_control_lock in the sourced installer.
# shellcheck disable=SC2034
CONTROL_LOCK_FILE="$STATE_HOME/$NAME/control.lock"
mkdir -p "$LOCAL_BIN_DIR"
cat > "$LOCAL_BIN_DIR/$NAME" <<'LAUNCHER'
#!/usr/bin/env bash
[[ "$1" == stop ]]
LAUNCHER
chmod 755 "$LOCAL_BIN_DIR/$NAME"

systemctl_user() {
    [[ "$*" == "show --property=ActiveState --value arch-sway-wslg-session.scope" ]] || return 99
    [[ "$TEST_STATE" != bus-error ]] || return 1
    printf '%s\n' "$TEST_STATE"
}
prompt_yes_no() { return 0; }

expect_rejected() {
    local command="$1"
    if ("$command") >"$TEST_DIR/output" 2>&1; then
        printf 'FAIL: %s accepted state %s\n' "$command" "$TEST_STATE" >&2
        exit 1
    fi
}

for TEST_STATE in bus-error '' unexpected; do
    expect_rejected stop_active_session
    expect_rejected take_control_lock
done
for TEST_STATE in inactive failed; do
    (stop_active_session)
    (take_control_lock)
done
for TEST_STATE in active activating deactivating reloading refreshing maintenance; do
    (stop_active_session)
    expect_rejected take_control_lock
done
# A session can start during package work even without an installed launcher.
LOCAL_BIN_DIR="$TEST_DIR/missing"
TEST_STATE=active
expect_rejected stop_active_session
expect_rejected take_control_lock
printf 'Installer session guards: passed\n'

fail() {
    printf 'FAIL: %s\n' "$*" >&2
    exit 1
}

# A process inside the session scope, at any depth, must be recognized.
printf '0::/user.slice/user-1000.slice/user@1000.service/app.slice/%s\n' \
    "$SESSION_SCOPE" > "$TEST_DIR/cgroup-inside"
printf '0::/user.slice/user-1000.slice/user@1000.service/app.slice/%s/sub\n' \
    "$SESSION_SCOPE" > "$TEST_DIR/cgroup-nested"
printf '0::/user.slice/user-1000.slice/user@1000.service/app.slice/%s-other.scope\n' \
    "$SESSION_SCOPE" > "$TEST_DIR/cgroup-outside"
running_inside_session "$TEST_DIR/cgroup-inside" || fail "session scope not detected"
running_inside_session "$TEST_DIR/cgroup-nested" || fail "nested session cgroup not detected"
! running_inside_session "$TEST_DIR/cgroup-outside" || fail "similar scope mistaken for the session"
! running_inside_session "$TEST_DIR/missing-cgroup" || fail "missing cgroup file mistaken for the session"
printf 'Installer in-session detection: passed\n'

# Roots that would turn the replacement's deletion onto unrelated files.
# Runs in a subshell so that the installer's die only ends the check.
roots_valid() (
    CONFIG_HOME="$1"
    DATA_HOME="$TEST_DIR/data"
    STATE_HOME="$TEST_DIR/state"
    export CONFIG_HOME DATA_HOME STATE_HOME
    validate_install_roots
)
roots_rejected() {
    roots_valid "$1" >/dev/null 2>&1 && fail "accepted configuration root: $1"
    return 0
}
roots_rejected "$HOME"
roots_rejected "$HOME/"
roots_rejected "$HOME/.local"
roots_rejected /
roots_rejected /mnt/wslg/runtime-dir
roots_rejected "$HOME/../$(basename -- "$HOME")"
roots_valid "$TEST_DIR/config \$HOME dir" || fail "rejected a dedicated configuration root"
printf 'Installer root validation: passed\n'

# Restore lines have to survive being pasted into a shell unchanged.
BACKUP_DIR="$TEST_DIR/backup \$HOME 'x'"
mkdir -p "$BACKUP_DIR/config/sway"
target="$TEST_DIR/restore \$HOME \"q\"/sway"
mkdir -p "$target/sub"
line="$(restore_command config/sway "$target")"
bash -c "$line" || fail "restore line failed: $line"
[[ -d "$target" && ! -e "$target/sub" ]] || fail "restore line did not restore $target"
line="$(restore_command config/missing "$TEST_DIR/restore \$HOME \"q\"/yazi")"
[[ "$line" == "rm -rf -- "* && "$line" != *"cp -a"* ]] || fail "unexpected line for a new path: $line"
printf 'Installer restore commands: passed\n'

# The recorded browser is the default; an earlier installation without a
# record chose no browser.
BROWSER_FILE="$TEST_DIR/browser"
printf 'chromium\n' > "$BROWSER_FILE"
[[ "$(default_browser_index)" == 2 ]] || fail "recorded browser is not the default"
rm -f -- "$BROWSER_FILE"
LOCAL_BIN_DIR="$TEST_DIR/bin"
[[ "$(default_browser_index)" == "${#BROWSER_KEYS[@]}" ]] || fail "earlier 'none' choice is not the default"
LOCAL_BIN_DIR="$TEST_DIR/missing"
[[ "$(default_browser_index)" == 1 ]] || fail "first installation does not default to Firefox"
printf 'Installer browser default: passed\n'
