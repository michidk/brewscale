# brewscale

Protocol-agnostic BLE coffee-scale library for Rust, on desktop **and**
microcontrollers. Write your application once against one API; pick which scale
protocols and which Bluetooth backend get compiled in.

Currently supported scales: **BOOKOO Themis Mini** and **Themis Ultra** (wire
format from the vendor's [open protocol docs](https://github.com/BooKooCode/OpenSource)).

## Crates

| Crate | Role | `no_std` | Heap |
|---|---|---|---|
| `brewscale-core` | Vendor-neutral types, `Protocol`/`Detect` traits, `Transport`/`Link` traits, `Scale` driver | yes | none |
| `brewscale-bookoo` | BOOKOO protocol (`Mini`, `Ultra`, runtime-selected `Bookoo`) | yes | none |
| `brewscale-trouble` | Embedded BLE backend on [TrouBLE](https://github.com/embassy-rs/trouble), any HCI controller (ESP32, nRF, ...) | yes | none |
| `brewscale` | Umbrella: re-exports core, protocols by feature, `AnyProtocol`, `btleplug` desktop backend | unless `btleplug` | unless `btleplug` |
| `examples/esp32c3` | ESP32-C3 firmware | yes | radio driver only |

Each protocol is its own crate, and `brewscale-core` knows none of them.

```toml
# Desktop: all protocols (default) + btleplug backend
brewscale = { version = "0.1", features = ["btleplug"] }

# Embedded: only the protocols you want, no std, no allocator
brewscale = { version = "0.1", default-features = false, features = ["bookoo"] }
brewscale-trouble = "0.1"

# Just the traits and driver, for writing your own protocol or backend
brewscale-core = { version = "0.1", default-features = false }
```

> Not yet published to crates.io; use `path`/`git` dependencies for now.

## Protocol-agnostic applications

`AnyProtocol` is generated from the enabled protocol features. The application
never names a vendor; enabling another protocol feature makes its scales work
with no code change.

```rust
use brewscale::*;

let (peer, protocol) = find::<AnyProtocol, _>(&mut transport).await?; // first scale of any enabled protocol
println!("found {}", protocol.name());
let link = transport.connect(&peer).await?;
let mut scale: AnyScale<_> = Scale::attach(&link, protocol).await?;   // discover + subscribe

scale.tare().await?;
loop {
    match scale.next_event().await? {
        Event::Measurement(m) => println!("{:.2} g", m.weight_g()),
        _ => {} // protocol-specific events; Event is #[non_exhaustive]
    }
}
```

`next_event` is cancel-safe: `select` it against a timer or a command channel.
`Error::Disconnected` means the link dropped; start over from `find` to
reconnect. `protocol.capabilities()` tells you what the connected scale
supports (timer, dose weight, ...); unsupported commands return
`EncodeError::Unsupported`.

Full programs: [`brewscale/examples/desktop.rs`](brewscale/examples/desktop.rs)
and [`examples/esp32c3/src/bin/scale.rs`](examples/esp32c3/src/bin/scale.rs).

## Design

* **Protocols are pure.** A protocol implements `Protocol` (GATT layout,
  `decode`, `encode`, `capabilities`) and `Detect` (recognise its devices from
  an advertisement). No I/O, no allocation, no BLE stack.
* **Transport is a trait.** `Transport` (scan, connect), `Link` (discover,
  read, write, subscribe, disconnect) and `Notifications` use native async fn
  in traits: no boxed futures, no runtime dependency. Backends implement them.
* **One driver.** `Scale` is generic over protocol and link, so protocol tests
  cover every backend and every backend works with every protocol.
* **Stable surface.** `Event`, `Command` and the error enums are
  `#[non_exhaustive]`, so new protocols can add variants without breaking you.

## Adding a protocol

1. Create `brewscale-<vendor>` depending on `brewscale-core` (`no_std`).
2. Implement `Protocol` and `Detect` for your type. Reuse the shared `Event`
   and `Command` types; add a variant to them in core if your device needs one.
3. In `brewscale/Cargo.toml` add an optional dependency and a feature
   (`<vendor> = ["dep:brewscale-<vendor>"]`, plus the `std` forwarding line).
4. Add one line to the `protocols!` list in `brewscale/src/any.rs`.
5. Add the crate to the workspace and to `manifest_paths` in
   `.github/workflows/bump-version.yml` and `publish-cratesio.yml` (list
   dependencies first).

[`brewscale-core/src/scale.rs`](brewscale-core/src/scale.rs) contains a ten-line
toy protocol test showing the minimum.

## Adding a backend

Implement `Transport`, `Link` and `Notifications` from `brewscale-core`
(`brewscale-trouble` and `brewscale/src/btleplug_transport.rs` are the
references). `Notifications::next` must be cancel-safe.

## ESP32

See [`examples/esp32c3`](examples/esp32c3/README.md) for hardware, flashing and
expected output. Target: **ESP32-C3** (stable Rust, `riscv32imc-unknown-none-elf`).
ESP32-S3 needs the Xtensa toolchain (`espup`) and is not built here, but
`brewscale-trouble` is chip-independent: any `trouble-host` controller works.

## Features

* `brewscale-core`: `std` (impl `std::error::Error`), `mock` (in-memory transport
  for tests; also `no_std`).
* `brewscale-bookoo`: `std`.
* `brewscale`: `bookoo` (default), `std`, `btleplug` (implies `std`).

## Testing

```sh
cargo test --workspace                                    # parsers + driver over a mock transport
cargo build -p brewscale --no-default-features --target thumbv7em-none-eabihf   # no std, no alloc
(cd examples/esp32c3 && cargo build --release)            # real firmware
```

CI runs these plus the desktop backend on Linux, Windows and macOS.

## License

MIT
