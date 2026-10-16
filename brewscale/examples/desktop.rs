//! Find a BOOKOO Themis Mini/Ultra, log weights, tare once, reconnect on drop.
//! `cargo run -p brewscale --features btleplug --example desktop`
use std::time::Duration;

use brewscale::bookoo::{Bookoo, Model};
use brewscale::{find, BtleplugTransport, Error, Event, Link, Scale, Transport, TransportError};
use tokio::time::{sleep, timeout};

/// A scale streams ~10 packets/s; silence this long means a dead link.
const SILENCE_TIMEOUT: Duration = Duration::from_secs(10);

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut transport = BtleplugTransport::new().await?;
    loop {
        if let Err(e) = session(&mut transport).await {
            eprintln!("session ended: {e}");
        }
        sleep(Duration::from_secs(2)).await;
    }
}

async fn session<T: Transport>(transport: &mut T) -> Result<(), Error<T::Error>> {
    println!("scanning for a BOOKOO scale...");
    let (peer, model) = find(transport).await?;
    println!("found BOOKOO Themis {model:?}, connecting");
    let link = transport.connect(&peer).await?;
    let result = stream(&link, model).await;
    let _ = link.disconnect().await;
    result
}

async fn stream<L: Link>(link: &L, model: Model) -> Result<(), Error<L::Error>>
where
    L::Error: TransportError,
{
    let mut scale = Scale::attach(link, Bookoo(model)).await?;
    println!("subscribed to weight notifications");
    let mut tared = false;
    loop {
        match timeout(SILENCE_TIMEOUT, scale.next_event()).await {
            Ok(Ok(Event::Measurement(m))) => {
                println!(
                    "{:.2} g  flow {:.2} g/s  {} ms  battery {}%",
                    m.weight_g(),
                    m.flow_g_per_s(),
                    m.elapsed_ms,
                    m.battery_percent
                );
                if !tared {
                    scale.tare().await?;
                    tared = true;
                    println!("tare sent");
                }
            }
            Ok(Ok(other)) => println!("event: {other:?}"),
            Ok(Err(Error::Decode(e))) => eprintln!("bad packet: {e}"),
            Ok(Err(e)) => return Err(e),
            Err(_) => return Err(Error::Disconnected),
        }
    }
}
