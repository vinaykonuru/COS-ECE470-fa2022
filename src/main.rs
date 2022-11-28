#[cfg(test)]
#[macro_use]
extern crate hex_literal;

pub mod api;
pub mod blockchain;
pub mod miner;
pub mod network;
pub mod types;
pub mod transaction_generator;
use std::collections::HashMap;
use crate::types::key_pair;
use crate::types::address::Address;
use types::transaction::{SignedTransaction, Transaction};
use types::hash::{Hashable, H256};
use api::Server as ApiServer;
use blockchain::Blockchain;
use clap::clap_app;
use log::{error, info};
use ring::signature::KeyPair;
use smol::channel;
use std::net;
use std::process;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time;

fn main() {
    // parse command line arguments
    let matches = clap_app!(Bitcoin =>
     (version: "0.1")
     (about: "Bitcoin client")
     (@arg verbose: -v ... "Increases the verbosity of logging")
     (@arg peer_addr: --p2p [ADDR] default_value("127.0.0.1:6000") "Sets the IP address and the port of the P2P server")
     (@arg api_addr: --api [ADDR] default_value("127.0.0.1:7000") "Sets the IP address and the port of the API server")
     (@arg known_peer: -c --connect ... [PEER] "Sets the peers to connect to at start")
     (@arg p2p_workers: --("p2p-workers") [INT] default_value("4") "Sets the number of worker threads for P2P server")
    )
    .get_matches();
    // create random account addresses for the tx_generator to use to make valid transactions
    // remember that on different processes, we want different key pairs. Therefore, randomly generate the key pairs
    // let key_pair_1 = key_pair::random();
    // let key_pair_2 = key_pair::random();
    // let key_pair_3 = key_pair::random();
    // init logger
    let verbosity = matches.occurrences_of("verbose") as usize;
    stderrlog::new().verbosity(verbosity).init().unwrap();
    let blockchain = Arc::new(Mutex::new(Blockchain::new()));
    let transactions: HashMap<H256,SignedTransaction> = HashMap::new();
    let mempool = Arc::new(Mutex::new(transactions));
    // parse p2p server address
    let p2p_addr = matches
        .value_of("peer_addr")
        .unwrap()
        .parse::<net::SocketAddr>()
        .unwrap_or_else(|e| {
            error!("Error parsing P2P server address: {}", e);
            process::exit(1);
        });
     let seed : [u8;32] = {
         if p2p_addr == "127.0.0.1:6000".parse::<net::SocketAddr>().unwrap(){
             [0;32]
         }
         else if p2p_addr == "127.0.0.1:6001".parse::<net::SocketAddr>().unwrap(){
             [1;32]
         }
         else {
             [2;32]
         }
     };
     let key_pair = key_pair::from_seed(seed);
     println!("Key Pair: {:?}", key_pair);
     println!("Seed: {:?}", seed);
    // parse api server address
    let api_addr = matches
        .value_of("api_addr")
        .unwrap()
        .parse::<net::SocketAddr>()
        .unwrap_or_else(|e| {
            error!("Error parsing API server address: {}", e);
            process::exit(1);
        });

    // create channels between server and worker
    let (msg_tx, msg_rx) = channel::bounded(10000);

    // start the p2p server
    let (server_ctx, server) = network::server::new(p2p_addr, msg_tx).unwrap();
    server_ctx.start().unwrap();

    // start the worker
    let p2p_workers = matches
        .value_of("p2p_workers")
        .unwrap()
        .parse::<usize>()
        .unwrap_or_else(|e| {
            error!("Error parsing P2P workers: {}", e);
            process::exit(1);
        });
    
    let worker_ctx = network::worker::Worker::new(&blockchain, &mempool, p2p_workers, msg_rx, &server);
    worker_ctx.start();

    // start the miner
    let (miner_ctx, miner, finished_block_chan) = miner::new(&blockchain, &mempool);
    let miner_worker_ctx = miner::worker::Worker::new(&blockchain,&mempool, &server, finished_block_chan);
    let (tx_generator_ctx, tx_generator) = transaction_generator::new(&blockchain, &mempool, &server, key_pair);
 
    miner_ctx.start();
    miner_worker_ctx.start();
    tx_generator_ctx.start();
    // connect to known peers
    if let Some(known_peers) = matches.values_of("known_peer") {
        let known_peers: Vec<String> = known_peers.map(|x| x.to_owned()).collect();
        let server = server.clone();
        thread::spawn(move || {
            for peer in known_peers {
                loop {
                    let addr = match peer.parse::<net::SocketAddr>() {
                        Ok(x) => x,
                        Err(e) => {
                            error!("Error parsing peer address {}: {}", &peer, e);
                            break;
                        }
                    };
                    match server.connect(addr) {
                        Ok(_) => {
                            info!("Connected to outgoing peer {}", &addr);
                            break;
                        }
                        Err(e) => {
                            error!(
                                "Error connecting to peer {}, retrying in one second: {}",
                                addr, e
                            );
                            thread::sleep(time::Duration::from_millis(1000));
                            continue;
                        }
                    }
                }
            }
        });
    }

    // start the API server
    ApiServer::start(api_addr, &miner, &tx_generator, &server, &blockchain);

    loop {
        std::thread::park();
    }
}
