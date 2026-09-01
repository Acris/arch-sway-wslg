# Clipboard data plane

This workspace builds the two x86-64 programs used by the managed session:

- `arch-sway-wslg-clipboard` is the Linux broker. It owns the synchronization
  state machine and connects directly to Sway through ext-data-control-v1.
- `arch-sway-wslg-clipboard-agent.exe` is the Win32 agent. It owns a
  message-only window and is the only process that accesses the Windows
  clipboard.

The agent is a child of the broker and communicates only through inherited
standard streams. `clipboard-core` defines the bounded binary protocol, text
normalization, and state transitions shared by both targets. Clipboard payloads
remain in memory and are never logged or stored in the runtime state files.

## Validation

```bash
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo check -p clipboard-agent-win --target x86_64-pc-windows-msvc
```

## Rebuilding the checked-in payload

The repository carries both release binaries under
`.local/libexec/arch-sway-wslg/`; users never build them. After changing
anything in this workspace, install the stable Rust toolchain and `cargo-xwin`
(`cargo install cargo-xwin --locked`), then run from the repository root:

```bash
cd clipboard
cargo build --release --locked -p clipboard-broker
cargo xwin build --release --locked -p clipboard-agent-win --target x86_64-pc-windows-msvc
cd ..
install -m 0755 clipboard/target/release/arch-sway-wslg-clipboard \
  .local/libexec/arch-sway-wslg/arch-sway-wslg-clipboard
install -m 0755 clipboard/target/x86_64-pc-windows-msvc/release/arch-sway-wslg-clipboard-agent.exe \
  .local/libexec/arch-sway-wslg/arch-sway-wslg-clipboard-agent.exe
(cd .local/libexec/arch-sway-wslg && \
  sha256sum arch-sway-wslg-clipboard arch-sway-wslg-clipboard-agent.exe > clipboard.sha256 && \
  sha256sum -c clipboard.sha256)
```

The GitHub Actions workflow runs the checks, builds both targets, and verifies
the checked-in payload against `clipboard.sha256`, but it does not upload
artifacts or write back to the repository. Commit the rebuilt binaries and the
checksum together.
