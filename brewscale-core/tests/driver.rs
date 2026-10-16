//! The driver exercised end-to-end over the mock transport. Backends reuse
//! this exact driver code, so these tests cover them too.
#![cfg(feature = "mock")]

use brewscale_core::bookoo::{Bookoo, Model};
use brewscale_core::mock::{block_on, MockTransport};
use brewscale_core::*;

fn packet(ty: u8, weight_cg: u8) -> [u8; 20] {
    let mut p = [0u8; 20];
    p[..2].copy_from_slice(&[3, ty]);
    p[6] = b'+';
    p[9] = weight_cg;
    p[19] = p[..19].iter().fold(0, |a, b| a ^ b);
    p
}

fn bookoo_adv(name: &str) -> Advertisement {
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
    let t = MockTransport::new(bookoo_adv("BOOKOO_SC U 0001"), &script);

    block_on(async {
        let mut transport = &t;
        let (peer, model) = find(&mut transport).await.unwrap();
        assert_eq!(model, Model::Ultra);
        let link = transport.connect(&peer).await.unwrap();
        let mut scale = Scale::attach(&link, Bookoo(model)).await.unwrap();

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
fn find_skips_unrelated_devices_and_attach_fails_on_wrong_service() {
    let t = MockTransport::new(bookoo_adv("ACAIA"), &[]);
    // Not a BOOKOO scale: Model::identify refuses it.
    assert_eq!(Model::identify(&t.adv), None);

    struct Odd;
    impl Protocol for Odd {
        fn service(&self) -> u16 {
            0x1234
        }
        fn notify_characteristic(&self) -> u16 {
            1
        }
        fn command_characteristic(&self) -> u16 {
            2
        }
        fn capabilities(&self) -> Capabilities {
            unreachable!()
        }
        fn decode(&self, _: &[u8]) -> Result<Event, DecodeError> {
            unreachable!()
        }
        fn encode(&self, _: Command) -> Result<Frame, EncodeError> {
            unreachable!()
        }
    }
    block_on(async {
        let mut transport = &t;
        let link = transport.connect(&()).await.unwrap();
        assert!(matches!(Scale::attach(&link, Odd).await, Err(Error::Transport(_))));
    });
}
