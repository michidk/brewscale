//! Protocol-agnostic entry point.
//!
//! Re-exports [`brewscale_core`] and, per Cargo feature, the protocol crates
//! (`bookoo` -> [`bookoo`]). [`AnyProtocol`] combines whichever protocols are
//! enabled, so an application can discover and drive any supported scale
//! without naming a vendor:
//!
//! ```ignore
//! let (peer, protocol) = find::<AnyProtocol, _>(&mut transport).await?;
//! let link = transport.connect(&peer).await?;
//! let mut scale: AnyScale<_> = Scale::attach(&link, protocol).await?;
//! ```
//!
//! `no_std` and heap-free unless a backend feature such as `btleplug` is on.
//! Embedded builds use `default-features = false`, pick protocols, and pair
//! this crate with an embedded backend crate such as `brewscale-trouble`.
#![no_std]
#![forbid(unsafe_code)]

#[cfg(feature = "std")]
extern crate std;

mod any;
#[cfg(feature = "btleplug")]
mod btleplug_transport;

#[cfg(feature = "bookoo")]
pub use brewscale_bookoo as bookoo;
pub use brewscale_core::*;

pub use any::{AnyProtocol, AnyScale};
#[cfg(feature = "btleplug")]
pub use btleplug_transport::{BtleplugLink, BtleplugNotifications, BtleplugTransport, Error as BtleplugError};
