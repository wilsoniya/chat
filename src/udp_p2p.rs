use std::io::Result;
use std::io::{Error, ErrorKind};
use std::net::SocketAddr;
use std::str::from_utf8;
use std::thread;
use std::sync::mpsc::sync_channel;

use futures::Sink;
use futures::stream::Stream;
use tokio_core::net::{UdpCodec, UdpSocket, UdpFramed};
use tokio_core::reactor::{Core, Remote};

pub struct MyCodec;

impl UdpCodec for MyCodec {
    type In = (SocketAddr, String);
    type Out = (SocketAddr, String);

    fn decode(&mut self, src: &SocketAddr, buf: &[u8]) -> Result<Self::In> {
        from_utf8(buf).and_then(|incoming| {
            Ok((*src, incoming.to_owned()))
        }).or_else(|err| {
            Err(Error::new(ErrorKind::InvalidData, err))
        })
    }
    fn encode(&mut self, (addr, msg): Self::Out, buf: &mut Vec<u8>) -> SocketAddr {
        buf.extend_from_slice(msg.as_bytes());
        addr
    }
}


pub struct ChatMedium {
    remote: Remote,
}

impl ChatMedium {
    pub fn new() -> Self {
        let remote = start_message_listener();
        ChatMedium{remote: remote}
    }

    fn send(&self, addr: SocketAddr, msg: String) {
    }
}


fn start_message_listener() -> Remote {
    let (tx, rx) = sync_channel(0);

    thread::spawn(move || {
        let mut core = Core::new().expect("Could not create reactor core");
        let handle = core.handle();
        let remote = core.remote();

        let addr = "0.0.0.0:6900".parse().expect("Could not parse bind address");
        let socket = UdpSocket::bind(&addr, &handle).expect("Could not create UDP socket");
        let codec = MyCodec{};
        let framed: UdpFramed<MyCodec> = socket.framed(codec);

        let receiver = framed.for_each(|(addr, msg)| {
            info!("Received: {}", msg);
            Ok(())
        });

        tx.send(remote);

        info!("Entering chat listener event loop...");
        core.run(receiver);
        info!("Chat listener event loop exited.");
    });

    rx.recv().expect("Could not recieve remote event loop handle.")
}
