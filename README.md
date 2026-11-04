# swissqrust

[![swiss_qrust](https://github.com/thomi137/swissqrust/actions/workflows/ci.yml/badge.svg)](https://github.com/thomi137/swissqrust/actions/workflows/ci.yml)
[![Deploy Documentation](https://github.com/thomi137/swissqrust/actions/workflows/docs.yml/badge.svg)](https://github.com/thomi137/swissqrust/actions/workflows/docs.yml)
[![codecov](https://codecov.io/gh/thomi137/swissqrust/branch/master/graph/badge.svg)](https://codecov.io/gh/thomi137/swissqrust)
[![Benchmarks](https://github.com/thomi137/swissqrust/actions/workflows/bench.yml/badge.svg)](https://thomi137.github.io/swissqrust/dev/bench/)
[![Rust 1.90.0+](https://img.shields.io/badge/rust-1.87.0+-orange.svg)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

# Swiss QR Payment Slips in Rust

This is for the moment an experimental project to generate Swiss payment slips using Rust. The main target was for
me to learn Rust, but since I started Software Development, I was obsessed with speed.
So Java an Python versions, while more easily integratable, to my mind are just slow.  Feel free to fork and send PRs.

Currently, the core (everyting in src) is producing valid Swiss QR payment slips.
There are some rough edges and test cases that are not yet implemented.

## Workspace layout

`swiss_qrust` (this crate, at the repo root) is the publishable library - it has no CLI/GUI/wasm
dependencies of its own. `crates/cli/`, `crates/gui/`, and `crates/web/` are separate crates in the
same Cargo workspace that consume it via a path dependency, showing three different ways to use it:

- `crates/cli/` is a simple binary that generates a payment slip and saves it to a file. You can select
input and output files. It works with both .toml and .json \
How these need to be formatted is shown in the data directory.
- `crates/gui/` is a fully functional native GUI application. At the moment it is only configured for macOS and does not support cross compilation for all mac processors (that would be a nice PR, if you would like to help)
There are icons for macOS, Windows and Linux. so using packager on your machine should give you a styled application. The developer got carried away so he built a styled gui compiling down into a .dmg for mac, completely
sidetracking the actual showcase. But it was fun to learn (iced)[https://docs.rs/iced/0.14.0/iced/index.html]  🤷‍♂️
- `crates/web/` - see [Web (WASM)](#web-wasm) below.

### Executing
however, you do not need to run packager. To see how this works, run

```
cargo run -p swiss-qrust-cli -- --input crates/cli/data/robert_schneider.json --output crates/cli/output/bill.pdf --lang fr # or de, it, rm or en, default is en
```

And for the GUI version:

```
cargo run -p swiss-qrust-gui # GUI version
```

If you want to see how to package that into a working application,
run

```
cd crates/gui
cargo packager --release
```

It will compile to a release version and package it into a .dmg file for macOS.
Currently, the developer does not have access to a windows or linux machine, so if you would like
to help, please let him know. Or better yet, send a PR. Icons and assets are there and hooked up in `crates/gui/Cargo.toml`.

## Web (WASM)

The `crates/web/` directory is a small [Trunk](https://github.com/trunk-rs/trunk)-based single-page app that compiles
`swiss_qrust` to WebAssembly and renders the QR-bill live in the browser as you fill in the form.

Prerequisites (one-time):

```
cargo install trunk
rustup target add wasm32-unknown-unknown
```

### Start (dev server)

```
cd crates/web
trunk serve --open
```

This builds the wasm bundle, serves it at `http://127.0.0.1:8080`, and opens it in your browser.
`crates/web/Trunk.toml` tells Trunk to watch both `crates/web/` and the core `../../src` library, so
it rebuilds automatically when either changes - without it, Trunk only watches `crates/web/` and edits
to the library silently go stale. Add `--no-autoreload` if you don't want the browser tab to
force-refresh on every rebuild (it still rebuilds in the background; just refresh manually when you
want the update).

To run it detached from your terminal (so it survives closing the shell), e.g.:

```
cd crates/web
nohup trunk serve --port 8080 --no-autoreload true > /tmp/trunk_serve.log &
```

### Stop

If running in the foreground, `Ctrl+C`. If started detached as above, find and kill the process:

```
lsof -i :8080 -sTCP:LISTEN   # note the PID
kill <PID>
```

### Build (static output, no server)

```
cd crates/web
trunk build --release
```

Produces the static bundle in `crates/web/dist/` with no dev server involved.
