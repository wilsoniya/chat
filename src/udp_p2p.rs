use std::io::Result;
use std::io::{Error, ErrorKind};
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::str::from_utf8;
use std::sync::mpsc::{Receiver, Sender, channel, sync_channel};
use std::thread;
use std::time::Duration;

use stun::{Attribute, Client, IpVersion, Message as StunMessage, XorMappedAddress};

use log;

type Message = (SocketAddr, String);

pub struct ChatConnection {
    inbound: Receiver<Message>,
    outbound: Sender<Message>,
    bind_addr: SocketAddr,
}

impl ChatConnection {
    pub fn new() -> ChatConnection {
        let (tx, inbound): (Sender<Message>, Receiver<Message>) = channel();
        let (outbound, rx): (Sender<Message>, Receiver<Message>) = channel();
        let (bind_addr_tx, bind_addr_rx) = sync_channel(1);

        let mut pub_addr = get_public_listen_address(0)
            .expect("could not get public listen address");

        if let SocketAddr::V4(addr) = pub_addr {
            let mut _addr = addr.clone();
            _addr.set_ip("0.0.0.0".parse().expect("ASDF"));
            pub_addr = SocketAddr::V4(_addr);
        }

        info!("Public chat listen address: {}", pub_addr);

        // inbound socket
        thread::spawn(move || {
            let socket = UdpSocket::bind(pub_addr)
                .expect("Could not open UDP socket");

            if let Ok(addr) = socket.local_addr() {
                bind_addr_tx.send(addr);
            }

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
                info!("Exiting chat socket listener loop")
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
                info!("Exiting chat socket sender loop")
            }
        });

        let timeout = Duration::new(1, 0);
        let bind_addr = bind_addr_rx.recv_timeout(timeout)
            .expect("Could not receive chat bind address");

        info!("Listening for chats on {}", bind_addr);

        ChatConnection {
            inbound: inbound,
            outbound: outbound,
            bind_addr: bind_addr,
        }
    }

    pub fn send(&self, dest: SocketAddr, msg: String) {
        self.outbound.send((dest, msg));
    }

    pub fn listen_addr(&self) -> SocketAddr {
        self.bind_addr
    }
}

fn get_public_listen_address(port: u16) -> Option<SocketAddr> {
   // stun server list: https://gist.github.com/zziuni/3741933
    let server = "173.194.203.127:19302";
    let ipv4 = IpVersion::V4;
    let client = Client::new(server, port, ipv4);
    let mesage = StunMessage::request();
    let encoded = mesage.encode();
    let response_bytes = client.send(encoded.clone());
    let response: StunMessage = StunMessage::decode(response_bytes);

    info!("STUN response: {:?}", response);

    response.attributes.iter().filter_map(|attr| {
        if let Attribute::XorMappedAddress(XorMappedAddress(addr)) = *attr {
            Some(addr)
        } else {
            None
        }
    }).nth(0)
}
