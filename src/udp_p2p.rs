use std::io::Result;
use std::io::{Error, ErrorKind};
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::str::from_utf8;
use std::thread;
use std::sync::mpsc::{Receiver, Sender, channel};

use log;

type Message = (SocketAddr, String);

pub struct ChatConnection {
    inbound: Receiver<Message>,
    outbound: Sender<Message>,
}

impl ChatConnection {
    pub fn new(bind_address: SocketAddr) -> ChatConnection {
        let (tx, inbound): (Sender<Message>, Receiver<Message>) = channel();
        let (outbound, rx): (Sender<Message>, Receiver<Message>) = channel();

        // inbound socket
        thread::spawn(move || {
            let socket = UdpSocket::bind(bind_address)
                .expect("Could not open UDP socket");
            let mut buf = [0u8; 8192];

            loop {
                match socket.recv_from(&mut buf) {
                    Ok((num_bytes, remote_addr)) => {
                        match from_utf8(&buf[0..num_bytes]) {
                            Ok(msg) => {
                                info!("Received '{}' from {}", msg.trim(), remote_addr);
                                tx.send((remote_addr, msg.to_owned()));
                            },
                            Err(e) => error!("Error decoding message: {}", e),
                        };
                    },
                    Err(e) => {
                        error!("Error receiving datagram: {}", e);
                        break;
                    },
                }
            }
        });

        // outbound socket
        thread::spawn(move || {
            let socket = UdpSocket::bind("0.0.0.0:0")
                .expect("Could not open UDP socket");

            loop {
                match rx.recv() {
                    Ok((dest_addr, msg)) => {
                        let buf: Vec<u8> = msg.into_bytes();
                        match socket.send_to(&buf, dest_addr) {
                            Ok(num_bytes) => {},
                            Err(e) => {
                                error!("Failed to send outbound message: {}", e);
                            }
                        }
                    },
                    Err(e) => {
                        error!("Error receiving outbound message from receiver: {}", e);
                        break;
                    }
                }
            }
        });

        ChatConnection {
            inbound: inbound,
            outbound: outbound,
        }
    }

    pub fn send(&self, dest: SocketAddr, msg: String) {
        self.outbound.send((dest, msg));
    }
}
