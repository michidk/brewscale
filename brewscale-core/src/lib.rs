//! Vendor-neutral coffee-scale API plus the BOOKOO Themis Mini/Ultra drivers.
//!
//! `no_std`, no heap, no executor, no BLE stack. Protocol decoding
//! ([`Protocol`]) is pure functions over byte slices; Bluetooth is reached only
//! through the [`Transport`]/[`Link`] traits, implemented by backend crates
//! (`brewscale` for desktop, `brewscale-trouble` for embedded).
#![no_std]
#![forbid(unsafe_code)]

#[cfg(feature = "std")]
extern crate std;

pub mod bookoo;
#[cfg(feature = "mock")]
pub mod mock;
mod protocol;
mod scale;
mod transport;

pub use protocol::{
    AutoBrewEvent, AutoBrewState, Capabilities, Command, DecodeError, EncodeError, Event, Frame, Measurement, Protocol,
    WeightUnit,
};
pub use scale::{find, Error, Scale};
pub use transport::{Advertisement, ErrorKind, Link, Notifications, Transport, TransportError};
