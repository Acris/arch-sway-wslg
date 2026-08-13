#!/usr/bin/env bash

# Private protocol helpers sourced by arch-sway-wslg. Callers provide the
# history paths, FIFO path, TTL, and standard note/warn functions.

clipboard_protocol_normalize_line() {
    REPLY="${1%$'\r'}"
}

clipboard_history_consume() {
    # Consume at most one matching reflection token. Keeping the same hash in
    # the cache after a match would suppress deliberate repeated copies during
    # the TTL window.
    local file="$1" wanted="$2" now ts hash tmp found=1 consumed=0
    now="$(date +%s)"
    tmp="${file}.tmp.$$"
    : > "$tmp"

    if [[ -r "$file" ]]; then
        while read -r ts hash; do
            [[ "$ts" =~ ^[0-9]+$ && "$hash" =~ ^[0-9a-f]{64}$ ]] || continue
            (( now >= ts && now - ts <= CLIPBOARD_ECHO_TTL )) || continue
            if (( ! consumed )) && [[ "$hash" == "$wanted" ]]; then
                found=0
                consumed=1
                continue
            fi
            printf '%s %s\n' "$ts" "$hash" >> "$tmp"
        done < "$file"
    fi

    mv -f "$tmp" "$file"
    return "$found"
}

clipboard_history_remember() {
    local file="$1" hash="$2"
    printf '%s %s\n' "$(date +%s)" "$hash" >> "$file"
}

clipboard_history_forget() {
    local file="$1" unwanted="$2" ts hash tmp
    tmp="${file}.tmp.$$"
    : > "$tmp"
    if [[ -r "$file" ]]; then
        while read -r ts hash; do
            [[ "$hash" == "$unwanted" ]] && continue
            printf '%s %s\n' "$ts" "$hash" >> "$tmp"
        done < "$file"
    fi
    mv -f "$tmp" "$file"
}

clipboard_wait_for_setter_ack() {
    local wanted="$1" deadline=$((SECONDS + 2)) message
    while (( SECONDS < deadline )); do
        if IFS= read -r -t 0.2 message < "$CLIPBOARD_SETTER_ACK_FIFO"; then
            clipboard_protocol_normalize_line "$message"
            message="$REPLY"
            case "$message" in
                "OK:$wanted") return 0 ;;
                "ERR:$wanted") return 1 ;;
            esac
        fi
    done
    return 2
}
