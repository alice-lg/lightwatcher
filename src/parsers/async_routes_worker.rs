use std::sync::{Arc, Mutex};

use anyhow::Result;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::task;

use crate::parsers::{
    parser::{Block, Parse},
    routes::PrefixGroup,
};
use crate::state::Route;

type BlockQueue = Arc<Mutex<Receiver<Block>>>;
type ResultsQueue = Sender<Route>;

/// Main worker function. This will be blocking.
fn worker_main(id: usize, block_queue: BlockQueue, results_queue: ResultsQueue) {
    println!("Routes worker {} started", id);

    loop {
        // Recevie block from block queue
        let block = {
            let queue = &mut block_queue.lock().unwrap();
            queue.blocking_recv()
        };
        if let Some(block) = block {
            let routes = PrefixGroup::parse(block);
            let routes = match routes {
                Ok(routes) => routes,
                Err(err) => {
                    println!("Routes worker {} failed to parse block: {}", id, err);
                    continue;
                }
            };

            // Send results to results queue
            for route in routes {
                results_queue.blocking_send(route).unwrap();
            }
        } else {
            break;
        }
    }

    println!("Routes worker {} stopped", id);
}

/// Spawn workers and return a block and results queue.
pub async fn spawn(num: usize) -> (Sender<Block>, Receiver<Route>) {
    let (block_tx, block_rx) = channel::<Block>(100);
    let (results_tx, results_rx) = channel::<Route>(100);

    let blocks_queue = Arc::new(Mutex::new(block_rx));

    // Start workers
    for id in 0..num {
        task::spawn_blocking({
            let blocks_queue = blocks_queue.clone();
            let results_tx = results_tx.clone();
            move || worker_main(id, blocks_queue, results_tx)
        });
    }
    (block_tx, results_rx)
}
