//! Desktop entry point: everything from [`brewscale_core`] plus, with the
//! `btleplug` feature, a [`BtleplugTransport`] for Linux, Windows and macOS.
//! Embedded targets use `brewscale-core` + `brewscale-trouble` instead; the
//! drivers and the [`Scale`] API are identical.

pub use brewscale_core::*;

#[cfg(feature = "btleplug")]
mod btleplug_transport;
#[cfg(feature = "btleplug")]
pub use btleplug_transport::{BtleplugLink, BtleplugNotifications, BtleplugTransport, Error as BtleplugError};
