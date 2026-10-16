//! Compile-time check: a whole session can be spawned on a multi-threaded runtime.
#![cfg(all(feature = "btleplug", feature = "bookoo"))]

use brewscale::*;

async fn session(mut t: BtleplugTransport) -> Result<(), Error<BtleplugError>> {
    let (peer, protocol) = find::<AnyProtocol, _>(&mut t).await?;
    let link = t.connect(&peer).await?;
    let mut scale: AnyScale<_> = Scale::attach(&link, protocol).await?;
    scale.tare().await?;
    scale.next_event().await?;
    Ok(())
}

#[allow(dead_code)]
fn session_future_is_send(t: BtleplugTransport) {
    fn assert_send<T: Send>(_: T) {}
    assert_send(session(t));
}
