//! The protocol-agnostic driver exercised end-to-end over the mock transport,
//! never naming a vendor type. Backends reuse this exact driver code.
#![cfg(feature = "bookoo")]

use brewscale::mock::{block_on, MockTransport};
use brewscale::*;

const SERVICE: u16 = 0x0FFE;

fn packet(ty: u8, weight_cg: u8) -> [u8; 20] {
    let mut p = [0u8; 20];
    p[..2].copy_from_slice(&[3, ty]);
    p[6] = b'+';
    p[9] = weight_cg;
    p[19] = p[..19].iter().fold(0, |a, b| a ^ b);
    p
}

fn named(name: &str) -> Advertisement {
    let mut a = Advertisement::empty();
    a.set_name(name.as_bytes());
    a
}

#[test]
fn finds_connects_streams_and_sends_commands() {
    let ok1 = packet(0x0B, 10);
    let unknown = packet(0x77, 0);
    let mut corrupt = packet(0x0B, 20);
    corrupt[19] ^= 0xFF;
    let ok2 = packet(0x0B, 30);
    let script: [&[u8]; 4] = [&ok1, &unknown, &corrupt, &ok2];
    let t = MockTransport::new(named("BOOKOO_SC U 0001"), SERVICE, &script);

    block_on(async {
        let mut transport = &t;
        let (peer, protocol) = find::<AnyProtocol, _>(&mut transport).await.unwrap();
        assert_eq!(protocol.name(), "BOOKOO Themis Ultra");
        let link = transport.connect(&peer).await.unwrap();
        let mut scale: AnyScale<_> = Scale::attach(&link, protocol).await.unwrap();

        let weight = |e: Event| match e {
            Event::Measurement(m) => m.weight_cg,
            e => panic!("{e:?}"),
        };
        assert_eq!(weight(scale.next_event().await.unwrap()), 10);
        // the unknown packet is skipped; the corrupt one is reported once...
        assert!(matches!(scale.next_event().await, Err(Error::Decode(DecodeError::Checksum { .. }))));
        // ...and the stream keeps working afterwards.
        assert_eq!(weight(scale.next_event().await.unwrap()), 30);

        scale.tare().await.unwrap();
        scale.send(Command::SetPowderWeight { deci_g: 150 }).await.unwrap();
        assert!(matches!(scale.send(Command::SetBeepLevel(9)).await, Err(Error::Encode(EncodeError::OutOfRange))));

        // script exhausted == link dropped
        assert!(matches!(scale.next_event().await, Err(Error::Disconnected)));
    });

    assert_eq!(t.write_count(), 2, "rejected command must not reach the device");
    assert_eq!(&t.write(0)[..6], &[3, 0x0A, 1, 0, 0, 8]);
    assert_eq!(&t.write(1)[..6], &[3, 0x0A, 0x0D, 0, 150, 3 ^ 0x0A ^ 0x0D ^ 150]);
}

#[test]
fn unrelated_devices_are_not_detected() {
    assert_eq!(AnyProtocol::detect(&named("ACAIA")), None);
    assert!(AnyProtocol::detect(&named("BOOKOO_SC 1234")).is_some());
}

#[test]
fn attach_fails_when_the_device_lacks_the_protocols_service() {
    let t = MockTransport::new(named("BOOKOO_SC 1"), 0x1234, &[]);
    block_on(async {
        let mut transport = &t;
        let link = transport.connect(&()).await.unwrap();
        let protocol = AnyProtocol::detect(&named("BOOKOO_SC 1")).unwrap();
        assert!(matches!(Scale::attach(&link, protocol).await, Err(Error::Transport(_))));
    });
}
