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
anything in this workspace, install the toolchain pinned in
`rust-toolchain.toml` (rustup does so on first use) and `cargo-xwin`
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
.local/libexec/arch-sway-wslg/arch-sway-wslg-clipboard --probe
.local/libexec/arch-sway-wslg/arch-sway-wslg-clipboard-agent.exe --probe
```

The two probe lines must be identical apart from the program name. Each build
script (`build-support/source_digest.rs`) embeds a SHA-256 digest of every
`.rs`, `.toml`, and `Cargo.lock` file in this workspace, ignoring carriage
returns, and `--probe` prints it as `source=`. The digest identifies the
source, not the bytes: linker output is not reproducible, so a rebuild may
change the binary checksums without changing the digest.

The GitHub Actions workflow runs on every change. It runs the shell checks and
the installer tests in an Arch container, runs the checks above, builds both
targets, and fails when the checked-in binaries do not match
`clipboard.sha256` or report a different digest than a build of the current
tree. It does not upload artifacts or write back to the repository. Commit the
rebuilt binaries and the checksum together.

A change to the frame layout or the message set, including a new flag such as
`TEXT_SENSITIVE`, requires a `PROTOCOL_VERSION` bump; `arch-sway-wslg doctor`
reports a broker and agent that disagree.
