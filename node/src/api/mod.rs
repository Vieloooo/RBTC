use serde::Serialize;
use crate::blockchain::Blockchain;
use crate::miner::Handle as MinerHandle;
use crate::network::server::Handle as NetworkServerHandle;
use crate::network::message::Message;
use crate::types::hash::{Hashable, H256};
use crate::types::mempool::UTXO;
use log::info;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use tiny_http::Header;
use tiny_http::Response;
use tiny_http::Server as HTTPServer;
use url::Url;
use crate::types::block::{Block, Body};
use crate::types::mempool::{Mempool, self};
use crate::types::transaction::SignedTransaction;
pub struct Server {
    handle: HTTPServer,
    miner: MinerHandle,
    network: NetworkServerHandle,
    blockchain: Arc<Mutex<Blockchain>>,
    mempool: Arc<Mutex<Mempool>>,
}

#[derive(Serialize)]
struct ApiResponse {
    success: bool,
    message: String,
}

macro_rules! respond_result {
    ( $req:expr, $success:expr, $message:expr ) => {{
        let content_type = "Content-Type: application/json".parse::<Header>().unwrap();
        let payload = ApiResponse {
            success: $success,
            message: $message.to_string(),
        };
        let resp = Response::from_string(serde_json::to_string_pretty(&payload).unwrap())
            .with_header(content_type);
        $req.respond(resp).unwrap();
    }};
}
macro_rules! respond_json {
    ( $req:expr, $message:expr ) => {{
        let content_type = "Content-Type: application/json".parse::<Header>().unwrap();
        let resp = Response::from_string(serde_json::to_string(&$message).unwrap())
            .with_header(content_type);
        $req.respond(resp).unwrap();
    }};
}

impl Server {
    pub fn start(
        addr: std::net::SocketAddr,
        miner: &MinerHandle,
        network: &NetworkServerHandle,
        blockchain: &Arc<Mutex<Blockchain>>,
        mempool: &Arc<Mutex<Mempool>>,
    ) {
        let handle = HTTPServer::http(&addr).unwrap();
        let server = Self {
            handle,
            miner: miner.clone(),
            network: network.clone(),
            blockchain: Arc::clone(blockchain),
            mempool: Arc::clone(mempool),
        };
        thread::spawn(move || {
            for mut req in server.handle.incoming_requests() {
                let miner = server.miner.clone();
                let network = server.network.clone();
                let blockchain = Arc::clone(&server.blockchain);
                let mempool = Arc::clone(&server.mempool);
                thread::spawn(move || {
                    // a valid url requires a base
                    let base_url = Url::parse(&format!("http://{}/", &addr)).unwrap();
                    let url = match base_url.join(req.url()) {
                        Ok(u) => u,
                        Err(e) => {
                            respond_result!(req, false, format!("error parsing url: {}", e));
                            return;
                        }
                    };
                    match url.path() {
                        "/miner/start" => {
                            info!("Received request to start mining");
                            let params = url.query_pairs();
                            let params: HashMap<_, _> = params.into_owned().collect();
                            let lambda = match params.get("lambda") {
                                Some(v) => v,
                                None => {
                                    respond_result!(req, false, "missing lambda");
                                    return;
                                }
                            };
                            let lambda = match lambda.parse::<u64>() {
                                Ok(v) => v,
                                Err(e) => {
                                    respond_result!(
                                        req,
                                        false,
                                        format!("error parsing lambda: {}", e)
                                    );
                                    return;
                                }
                            };
                            miner.start(lambda);
                            respond_result!(req, true, "ok");
                        }
                        "/tx-generator/start" => {
                            // unimplemented!()
                            respond_result!(req, false, "unimplemented!");
                        }
                        "/network/ping" => {
                            network.broadcast(Message::Ping(String::from("Test ping")));
                            respond_result!(req, true, "ok");
                        }
                        "/blockchain/longest-chain" => {
                            let blockchain = blockchain.lock().unwrap();
                            let v = blockchain.all_blocks_in_longest_chain();
                            let v_string: Vec<String> = v.into_iter().map(|h|h.to_string()).collect();
                            respond_json!(req, v_string);
                        }
                        "/blockchain/f-chain" => {
                            let blockchain = blockchain.lock().unwrap();
                            let v = blockchain.all_blocks_in_longest_chain();
                           
                            // remove the last 6 block hash if len > 6 
                            let mut v_string: Vec<String> = v.into_iter().map(|h|h.to_string()).collect();
                            if v_string.len() > 6{
                               v_string = v_string[0..v_string.len()-6].to_vec();
                            }
                            

                            respond_json!(req, v_string);
                        }
                        "/blockchain/height" => {
                            let height = blockchain.lock().unwrap().height; 
                            respond_json!(req, height);
                        }
                        "/blockchain/total_block" => {
                            let n = blockchain.lock().unwrap().blocks.len(); 
                            respond_json!(req, n); 
                        }
                        "/blockchain/longest-chain-tx" => {
                            let blockchain = blockchain.lock().unwrap();
                            let all_blocks_hash = blockchain.all_blocks_in_longest_chain();
                            let mut tx_list = Vec::new();
                            for block_hash in all_blocks_hash{
                                let block_with_height = blockchain.blocks.get(&block_hash).unwrap();
                                let block = &block_with_height.block;
                                // append all txs in the block.body.txs to tx_list 
                                for tx in &block.body.txs{
                                    tx_list.push(tx.get_tx_hash().to_string()); 
                                } 
                                //send in json format 
                            }
                            respond_json!(req, tx_list);
                        }
                        "/blockchain/longest-chain-tx-count" => {
                            let blockchain = blockchain.lock().unwrap();
                            let all_blocks_hash = blockchain.all_blocks_in_longest_chain();
                            let mut tx_count: usize = 0; 
                            for block_hash in all_blocks_hash{
                                let block_with_height = blockchain.blocks.get(&block_hash).unwrap();
                                tx_count += block_with_height.block.body.tx_count; 
                            }
                            respond_json!(req, tx_count); 
                              
                        }
                        "/utxo" => {
                            let mempool = mempool.lock().unwrap();
                            let mut utxo_list = Vec::new(); 
                            for (k, v) in mempool.utxo.clone(){
                                let otxo_string = format!("{:.8}-{:.2} => {:.8}:{}:{}", k.0, k.1, v.output.pk_hash, v.output.value, v.used_in_mempool);

                                utxo_list.push(otxo_string);
                            }
                            respond_json!(req, utxo_list);
                        }
                        "/mempool/txs" => {
                            // return all tx in the mempool 
                            let mempool = mempool.lock().unwrap();
                            let mut tx_list = Vec::new();
                            for tx in mempool.txs.clone(){
                                tx_list.push(tx);
                            }
                            respond_json!(req, tx_list);
                        }
                        "/utxo-count" => {
                            let mempool = mempool.lock().unwrap();
                            let utxo_count = mempool.utxo.len(); 
                            respond_json!(req, utxo_count);
                        }
                        "/mempool/query_utxo_by_pk" => {
                            let params = url.query_pairs();
                            let params: HashMap<_, _> = params.into_owned().collect();
                            let pk_hash = match params.get("pkh") {
                                Some(v) => v,
                                None => {
                                    respond_result!(req, false, "pkh");
                                    return;
                                }
                            };
                            let pk_hash = match pk_hash.parse::<H256>() {
                                Ok(v) => v,
                                Err(e) => {
                                    respond_result!(
                                        req,
                                        false,
                                        format!("error parsing pkh: {}", e)
                                    );
                                    return;
                                }
                            };
                            let mempool = mempool.lock().unwrap();
                            let mut utxo_list = Vec::new();
                            for (k, v) in mempool.utxo.clone(){
                                if v.output.pk_hash == pk_hash && v.used_in_mempool == false{
                                    utxo_list.push((k.0, k.1, v.output));
                                }
                            }
                            //serialize utxo_list into json and return 
                            respond_json!(req, utxo_list);
                        }
                        "/mempool/submit_tx" => {
                            // get signed tx from url 
                            if req.method().as_str() != "POST"{
                                respond_result!(req, false, "not a post request");
                                return;
                            }
                            // read the body of the request as Signed tx 
                            let mut content = String::new(); 
                            req.as_reader().read_to_string(& mut content).unwrap(); 
                            let signed_tx = serde_json::from_str::<SignedTransaction>(&content).unwrap();
                           
                            let mut mempool = mempool.lock().unwrap();
                            let res = mempool.add_tx(&signed_tx);
                            match res {
                                Ok(_) => {
                                    // broadcast the tx to the network 
                                    network.broadcast(Message::NewTransactionHashes(vec![signed_tx.get_tx_hash()]));
                                    respond_result!(req, true, "ok");
                                }
                                Err(e) => {
                                    respond_result!(req, false, format!("error adding tx: {}", e));
                                }
                            }
                        }
                        _ => {
                            let content_type =
                                "Content-Type: application/json".parse::<Header>().unwrap();
                            let payload = ApiResponse {
                                success: false,
                                message: "endpoint not found".to_string(),
                            };
                            let resp = Response::from_string(
                                serde_json::to_string_pretty(&payload).unwrap(),
                            )
                            .with_header(content_type)
                            .with_status_code(404);
                            req.respond(resp).unwrap();
                        }
                    }
                });
            }
        });
        info!("API server listening at {}", &addr);
    }
}
