#[macro_use]
extern crate log;
extern crate env_logger;
extern crate rand;
extern crate clap;
extern crate igd;
extern crate stun;
extern crate kademlia;
extern crate futures;
extern crate tokio_core;

mod udp_p2p;

use clap::{Arg, App};
use futures::stream::Stream;
use tokio_core::net::UdpSocket;
use tokio_core::reactor::Core;

use udp_p2p::ChatMedium;

const NET_ID: &'static str = "chat_net";


fn main() {
    env_logger::init();

    let matches = App::new("Chat.")
        .arg(Arg::with_name("username")
             .index(1)
             .help("Your desired username")
             .required(true))
        .arg(Arg::with_name("address")
             .index(2)
             .help("Bind address hostname:port")
             .required(true))
        .arg(Arg::with_name("bootstrap")
             .short("b")
             .long("bootstrap")
             .help("Address of bootstrap node like hostname:port")
             .requires("bootstrap_key")
             .takes_value(true))
        .arg(Arg::with_name("bootstrap_key")
             .short("k")
             .long("bootstrap_key")
             .help("Key of bootstrap node")
             .requires("bootstrap")
             .takes_value(true))
        .get_matches();

    let username = matches.value_of("username")
        .expect("Username must be supplied");
    let bind_address = matches.value_of("address")
        .expect("Address must be supplied");
    let maybe_bootstrap_address = matches.value_of("bootstrap");
    let maybe_bootstrap_key = matches.value_of("bootstrap_key");

    let bootstrap = maybe_bootstrap_address.and_then(|bootstrap_address| {
        maybe_bootstrap_key.map(|bootstrap_key| {
            kademlia::NodeInfo {
                id: kademlia::Key::hash(bootstrap_key.to_owned()),
                addr: bootstrap_address.to_owned(),
                net_id: NET_ID.to_owned(),
            }
        })
    });

    debug!("Selected username={}", username);
    debug!("Bootstrap node: {:?}", bootstrap);

    let network_name = NET_ID.to_owned();
    let node_id = kademlia::Key::hash(username.to_owned());

    let dht = kademlia::Kademlia::start(
        network_name.clone(), node_id, bind_address, bootstrap);

    info!("Kademlia network: {}; node_id: {:?}", network_name, node_id);


    let chat = ChatMedium::new();


    dht.put(username.to_owned(), bind_address.to_owned());
    info!("Stored chat peer in DHT: username={} node_address={}",
          username, bind_address);

    let stdin = std::io::stdin();
    let mut line = String::new();
    loop {
        line = String::new();
        stdin.read_line(&mut line);
        debug!("Got line: {}", line.trim());
        if line.to_lowercase().trim() == "exit" {
            info!("Exiting.");
            break;
        }

        if let Some(peer_address) = dht.get(line.trim().to_owned()) {
            info!("Resolved peer address username={} peer_address={}",
                  line.trim(), peer_address);
        } else {
            info!("Could not resolve peer address for username={}",
                  line.trim());
        }
    }

}
