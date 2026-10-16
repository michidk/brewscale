//! Driver tying a [`Protocol`] to a [`Link`].

use core::fmt;

use crate::bookoo::{Bookoo, Model};
use crate::{
    Command, DecodeError, EncodeError, ErrorKind, Event, Link, Notifications, Protocol, Transport, TransportError,
};

#[derive(Debug)]
pub enum Error<E> {
    /// The link dropped; reconnect from [`find`].
    Disconnected,
    Transport(E),
    /// A notification failed validation (e.g. checksum). Safe to keep reading.
    Decode(DecodeError),
    Encode(EncodeError),
}

impl<E: TransportError> From<E> for Error<E> {
    fn from(e: E) -> Self {
        if e.kind() == ErrorKind::Disconnected {
            Self::Disconnected
        } else {
            Self::Transport(e)
        }
    }
}

impl<E: fmt::Debug> fmt::Display for Error<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Disconnected => f.write_str("disconnected"),
            Self::Transport(e) => write!(f, "transport error: {e:?}"),
            Self::Decode(e) => write!(f, "decode error: {e}"),
            Self::Encode(e) => write!(f, "encode error: {e}"),
        }
    }
}

#[cfg(feature = "std")]
impl<E: fmt::Debug> std::error::Error for Error<E> {}

/// Scan until a BOOKOO scale is seen. Returns its peer handle and model; pass
/// the peer to [`Transport::connect`].
pub async fn find<T: Transport>(transport: &mut T) -> Result<(T::Peer, Model), T::Error> {
    loop {
        let (peer, adv) = transport.scan().await?;
        if let Some(model) = Model::identify(&adv) {
            return Ok((peer, model));
        }
    }
}

/// A connected scale: decoded events out, [`Command`]s in.
pub struct Scale<'l, L: Link, P: Protocol = Bookoo> {
    link: &'l L,
    protocol: P,
    command: L::Characteristic,
    notifications: L::Notifications<'l>,
}

impl<'l, L: Link, P: Protocol> Scale<'l, L, P> {
    /// Discover the protocol's characteristics and subscribe to measurements.
    pub async fn attach(link: &'l L, protocol: P) -> Result<Self, Error<L::Error>> {
        let notify = link.characteristic(protocol.service(), protocol.notify_characteristic()).await?;
        let command = link.characteristic(protocol.service(), protocol.command_characteristic()).await?;
        let notifications = link.subscribe(&notify).await?;
        Ok(Self { link, protocol, command, notifications })
    }

    pub fn protocol(&self) -> &P {
        &self.protocol
    }

    /// Next decoded event. Cancel-safe, so it can be `select`ed against timers
    /// or commands. Packets of unknown type are skipped; a corrupt packet
    /// yields [`Error::Decode`] and the stream stays usable.
    pub async fn next_event(&mut self) -> Result<Event, Error<L::Error>> {
        let mut buf = [0u8; 32];
        loop {
            let n = self.notifications.next(&mut buf).await?;
            match self.protocol.decode(&buf[..n]) {
                Ok(event) => return Ok(event),
                Err(DecodeError::UnknownType(_)) => {}
                Err(e) => return Err(Error::Decode(e)),
            }
        }
    }

    pub async fn send(&self, command: Command) -> Result<(), Error<L::Error>> {
        let frame = self.protocol.encode(command).map_err(Error::Encode)?;
        Ok(self.link.write(&self.command, &frame).await?)
    }

    pub async fn tare(&self) -> Result<(), Error<L::Error>> {
        self.send(Command::Tare).await
    }
}
