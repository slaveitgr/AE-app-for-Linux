# Contributing to aeapp

Report bugs and propose changes through the [project issue tracker](https://github.com/slaveitgr/AE-app-for-Linux/issues). Include the Linux distribution, graphics session, relevant error, and steps to reproduce. Remove account tokens and personal data from logs.

## Project structure

- [aeapp-lib](aeapp-lib) provides account, library, downloads, cloud saves, and game-launch services.
- [aeapp-ui](aeapp-ui) contains the native egui interface and `aeapp` binary.
- [aeapp-cli](aeapp-cli) contains the command-line client.
- [aeapp-tui](aeapp-tui) contains the terminal interface.
- [aeapp-bootstrap](aeapp-bootstrap) handles game launching and browser callbacks.
- [aeapp-service](aeapp-service) contains the Windows background service.
- [aeapp-resources](aeapp-resources) contains shared build assets and metadata.

## Development setup

Install Rust through [rustup](https://rustup.rs/) and the Linux prerequisites in [README-LINUX.md](README-LINUX.md). The repository pins **Rust 1.93.0 stable** in `rust-toolchain.toml`; dependencies are recorded in `Cargo.lock`.

```sh
cargo test --locked -p aeapp-lib --lib
cargo test --locked -p aeapp-ui --bin aeapp
cargo test --locked -p aeapp-bootstrap --bin aeapp-bootstrap
cargo build --locked --release -p aeapp-ui -p aeapp-bootstrap -p aeapp-cli
./target/release/aeapp --preview --preview-page downloads
```

Keep the UI, bootstrap, and CLI binaries together when testing normal mode. The ignored live-account tests require explicit account access and may write cloud saves; leave them ignored in unattended checks. Use local previews and synthetic fixtures for UI tests.

## Code and licensing

Keep changes focused and use `cargo fmt` for Rust formatting. Add tests when they verify meaningful behavior or a regression. Preserve the GPL license and existing author/copyright notices; contributor attribution is recorded in [CREDITS](CREDITS.md).
