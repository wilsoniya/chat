#[macro_use]
extern crate log;
extern crate env_logger;
extern crate clap;

extern crate futures;
extern crate tokio_core;

use std::io::Result;
use std::net::SocketAddr;
use std::str::from_utf8;

use futures::Future;
use futures::Sink;
use futures::stream::Stream;
use tokio_core::net::{UdpCodec, UdpSocket};
use tokio_core::reactor::Core;


struct MyCodec;

impl UdpCodec for MyCodec {
    type In = (SocketAddr, String);
    type Out = (SocketAddr, String);

    fn decode(&mut self, src: &SocketAddr, buf: &[u8]) -> Result<Self::In> {
        let incoming = from_utf8(buf).expect("could not decode incoming datagram");
        Ok((*src, incoming.to_owned()))
    }
    fn encode(&mut self, (addr, msg): Self::Out, buf: &mut Vec<u8>) -> SocketAddr {
        let new_msg = format!("received: {}", msg);
        buf.extend_from_slice(new_msg.as_bytes());
        addr
    }
}


fn main() {
    println!("hello, world");

    let mut core = Core::new().expect("Could not create reactor core");
    let handle = core.handle();

    let addr = "0.0.0.0:6900".parse().expect("Could not parse bind address");
    let socket = UdpSocket::bind(&addr, &handle).expect("Could not create UDP socket");
    let codec = MyCodec{};
    let framed: tokio_core::net::UdpFramed<MyCodec> = socket.framed(codec);

    let (sink, stream) = framed.split();

    let out_stream = stream.map(|(addr, msg)| {
        println!("received: {} from {}", msg, addr);
        (addr, msg)
    });

    let sender = sink.send_all(out_stream);

    core.run(sender);
}
