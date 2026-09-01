# Third-party notices

The clipboard binaries include Rust crates distributed under permissive
licenses. Their exact resolved versions are recorded in `Cargo.lock`; the
corresponding license texts are available from each crate's source package and
upstream repository.

- `calloop`, `calloop-wayland-source`, `wayland-backend`, `wayland-client`,
  `wayland-protocols`, `wayland-scanner`, and `wayland-sys`: MIT
- `sha2` and its RustCrypto dependencies: MIT OR Apache-2.0
- `rustix` and its dependencies: Apache-2.0 WITH LLVM-exception OR Apache-2.0
  OR MIT, depending on the crate
- `thiserror`: MIT OR Apache-2.0
- `windows-link` and `windows-sys`: MIT OR Apache-2.0

The package names above identify the dependency groups and their applicable
license choices. Copyright notices and any additional notices shipped by a
dependency remain recorded in the package source and the locked metadata.
