# brewscale

BLE coffee-scale drivers in Rust for desktop **and** microcontrollers. Supports
the **BOOKOO Themis Mini** and **Themis Ultra** (wire format from the vendor's
[open protocol docs](https://github.com/BooKooCode/OpenSource)).

| Crate | What | `no_std` | Heap |
|---|---|---|---|
| `brewscale-core` | Protocol drivers, common types, `Scale` driver, `Transport`/`Link` traits | yes | none |
| `brewscale` | Re-exports core; `btleplug` feature adds Linux/Windows/macOS backend | no | yes |
| `brewscale-trouble` | [TrouBLE](https://github.com/embassy-rs/trouble) central backend for any HCI controller | yes | none |
| `examples/esp32c3` | ESP32-C3 firmware using the above | yes | radio driver only |

```toml
# Protocol + driver only: no std, no allocator, no BLE stack, no dependencies
brewscale-core = { version = "0.1", default-features = false }
# Desktop
brewscale = { version = "0.1", features = ["btleplug"] }
# Embedded: core + a backend
brewscale-trouble = "0.1"
```

> Not yet published to crates.io; use `path`/`git` dependencies for now.

## Usage

The same code runs on desktop and embedded; only the transport differs
(see `brewscale/examples/desktop.rs` and `examples/esp32c3/src/bin/scale.rs`).

```rust
let (peer, model) = find(&mut transport).await?;        // scan until a BOOKOO scale shows up
let link = transport.connect(&peer).await?;             // GATT connection
let mut scale = Scale::attach(&link, Bookoo(model)).await?; // discover + subscribe
scale.tare().await?;
while let Event::Measurement(m) = scale.next_event().await? {
    println!("{:.2} g", m.weight_g());
}
```

`next_event` is cancel-safe: `select` it against a timer or a command channel.
`Error::Disconnected` means the link dropped; start over from `find` to reconnect.

## Design

* **Protocol is pure.** `Protocol::decode`/`encode` are functions over byte
  slices (`bookoo::Mini`, `bookoo::Ultra`, or `bookoo::Bookoo(Model)` when the
  model is identified at runtime from the advertised name). No I/O, no allocation.
* **Transport is a trait.** `Transport` (scan, connect), `Link` (discover,
  read, write, subscribe, disconnect) and `Notifications` use native async
  fn in traits: no boxed futures, no runtime dependency. Backends implement
  them; the driver (`Scale`) is shared, so the protocol tests cover every backend.
* **Backends isolate platform code.** `btleplug`, Tokio and TrouBLE never appear
  in core's API.

### Features

`brewscale-core`: `std` (impl `std::error::Error`), `mock` (in-memory transport
used by the tests; also `no_std`). `brewscale`: `btleplug`. `brewscale-trouble`:
`log`, `defmt` (forwarded to `trouble-host`).

## ESP32

See [`examples/esp32c3`](examples/esp32c3/README.md) for hardware, flashing and
expected output. Target: **ESP32-C3** (stable Rust; `riscv32imc-unknown-none-elf`).
ESP32-S3 needs the Xtensa toolchain (`espup`) and is not built here, but
`brewscale-trouble` is chip-independent: any `trouble-host` controller works.

## Testing

```sh
cargo test --workspace --features brewscale-core/mock     # parsers + driver, host
cargo build -p brewscale-core --no-default-features --target thumbv7em-none-eabihf  # no std, no alloc
(cd examples/esp32c3 && cargo build --release)            # real firmware
```

CI (`.github/workflows/ci.yml`) runs these plus desktop builds on Linux, Windows
and macOS.

## Validation status: read before relying on this

**No physical hardware was available. Nothing here has been run against a real
scale or a real ESP32.** What was verified:

* Decoders/encoders: unit tests with hand-built packets and checksums (host).
* `brewscale-core` builds for `thumbv7em-none-eabihf` and
  `riscv32imc-unknown-none-elf` (no std, no allocator, zero dependencies).
* The ESP32-C3 firmware compiles and links to a RISC-V ELF.
* The btleplug backend builds and lints clean on Linux, and type-checks for
  Windows and macOS. It was never run against a Bluetooth adapter.

Still to do on hardware, and known risks:

1. **Embedded backend is untested at runtime.** `brewscale-trouble` polls
   TrouBLE's `GattClient::task()` inside each `Link` operation, because TrouBLE
   only routes responses/notifications while it is polled. This is the most
   likely place for a real-world bug (e.g. notifications delayed while nothing
   awaits the link). Test connect, subscribe, tare, and unplug/reconnect.
2. **Checksums.** Frames follow the vendor spec (XOR of all preceding bytes).
   Tare and beep frames match third-party ESP32 code byte for byte, but that
   code sends different checksums for start/stop/reset timer, so the scale may
   not verify them. Confirm each command has an effect.
3. **Sign byte.** The spec says only "+/-". The driver treats `0x2D` (`-`) as
   negative and anything else as positive (matches third-party Mini code);
   confirm on the Ultra.
4. **Advertised names.** Matches `BOOKOO_SC`/`BOOKOO_SC U`, tolerates a missing
   leading `B` (reported for Mini firmware 1.4.1), and falls back to the `0FFE`
   service (assumed Mini, the safe command subset). Ultra detection relies on the
   `U` in the name.
5. **Ultra firmware gating.** Some Ultra commands/packets need firmware
   V3.2.4/V4.0.0+ (powder weight, auto-brew events, shutdown, keep-awake). The
   driver cannot read the firmware version and will send them regardless.
6. **Not implemented:** calibration and the automatic-mode stop-condition
   commands (risky to send blind); pairing/bonding; multiple simultaneous scales.
7. **Dependencies are young.** `esp-radio` is `1.0.0-beta`; `esp-hal`/`esp-rtos`
   0.x/1.x APIs move quickly. The example pins compatible versions.
8. `ScanInbox` uses a no-op mutex: run the TrouBLE runner and your app on the
   same executor (as the example does), not on separate cores/threads.

## License

MIT
