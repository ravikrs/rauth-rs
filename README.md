# rauth-rs

An OAuth 2.0 / OpenID Connect provider in Rust, built by a software factory: AI agents write the specs,
acceptance tests and implementation; the author approves every spec and merges every PR.

No product code yet: the workspace holds the factory tooling. The architecture is
`docs/adr-001-architecture.md`, the factory loop `docs/adr-002-factory-loop.md`; work and open
decisions are GitHub Issues; the reference survey is `reference/README.md`.

## Development

Install Rust via rustup (the version is pinned in `rust-toolchain.toml`), then the gate's tools:
`just`, `cargo-nextest`, `cargo-deny`, `gitleaks`, and `pip3 install -r requirements-docs.txt`.

```sh
just check   # the merge gate, exactly as CI runs it
```

Docs: <https://ravikrs.github.io/rauth-rs/>. Locally: `pip3 install -r requirements-docs.txt && mkdocs serve`.

## Licence

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <https://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <https://opensource.org/licenses/MIT>)

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
