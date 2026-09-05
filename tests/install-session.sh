#!/usr/bin/env bash
set -Eeuo pipefail
umask 077

# Fault injection for the installer guards; no real bus, packages or user files.
source "$(dirname -- "${BASH_SOURCE[0]}")/../install.sh"
TEST_DIR="$(mktemp -d)"
trap 'rm -rf -- "$TEST_DIR"' EXIT
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
