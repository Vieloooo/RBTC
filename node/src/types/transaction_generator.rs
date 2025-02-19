use std::{thread, time};
use crate::network::server::Handle as NetworkServerHandle;
use std::sync::{Arc, Mutex};
use crate::types::mempool::Mempool;
use crate::types::transaction::*; 
use crate::types::key_pair; 
pub struct TransactionGenerator{}

impl TransactionGenerator{
    pub fn start(theta: u32, network: NetworkServerHandle, mempool : Arc<Mutex<Mempool>>){
        thread::spawn(move || {
            let mut i = 0; 
          loop{
                if theta != 0{
                    //gen tx 
                    // if i = 0, A => B 10 btc 
                    // if i = 1, B => C 10 btc
                    // if i = 2, C => A 10 btc
                    i = (i + 1) % 3; 
                    let interval = time::Duration::from_micros(theta as u64);
                    thread::sleep(interval);
                }  
            }
        });
    }
}