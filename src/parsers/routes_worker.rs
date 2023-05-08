use anyhow::Result;
use std::sync::{
    mpsc::{channel, Receiver, Sender},
    Arc, Mutex,
};
use std::thread;

use crate::parsers::{
    parser::{Block, Parse},
    routes::PrefixGroup,
};

type BlockQueue = Arc<Mutex<Receiver<Block>>>;
type ResultsQueue = Sender<Result<PrefixGroup>>;

pub struct RoutesWorker {
    id: usize,
}

impl RoutesWorker {
    /// Create new routes parser worker
    pub fn new(id: usize) -> Self {
        Self { id }
    }

    /// Spawn a new routes worker and create a response and
    /// request channel.
    pub fn spawn(&self, block_queue: BlockQueue, results_queue: ResultsQueue) {
        let id = self.id;

        thread::spawn(move || {
            println!("Routes worker {} started.", id);
            loop {
                let block = {
                    let queue = block_queue.lock().unwrap();
                    queue.recv()
                };
                if let Ok(block) = block {
                    let routes = PrefixGroup::parse(block);
                    results_queue.send(routes).unwrap();
                } else {
                    break;
                }
            }
            println!("Routes worker {} stopped.", id);
        });
    }
}

/// A routes worker pool has a collection of workers
/// and a queue of blocks to be parsed.
pub struct RoutesWorkerPool {}

impl RoutesWorkerPool {
    /// Create new worker pool and spawn workers
    pub fn spawn(num: usize) -> (Sender<Block>, Receiver<Result<PrefixGroup>>) {
        let (blocks_tx, blocks_rx) = channel::<Block>();
        let (results_tx, results_rx) = channel::<Result<PrefixGroup>>();

        let blocks_queue = Arc::new(Mutex::new(blocks_rx));

        // Start workers
        for id in 0..num {
            let worker = RoutesWorker::new(id);
            worker.spawn(blocks_queue.clone(), results_tx.clone());
        }

        (blocks_tx, results_rx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsers::parser::{Block, BlockIterator};
    use crate::state::Route;
    use std::fs::File;
    use std::io::BufReader;

    use regex::Regex;

    #[test]
    fn test_routes_worker() {
        // let file = File::open("tests/birdc/show-route-all-protocol-R192_175").unwrap();
        let file: File = File::open("tests/birdc/show-route-all-table-master4").unwrap();
        let reader = BufReader::new(file);
        let re_routes_start = Regex::new(r"1007-\S").unwrap();

        let blocks = BlockIterator::new(reader, &re_routes_start);

        let mut routes: Vec<Route> = vec![];

        // Spawn workers
        let (blocks_tx, results_rx) = RoutesWorkerPool::spawn(4);

        thread::spawn(move || {
            for block in blocks {
                blocks_tx.send(block).unwrap();
            }
        });

        for result in results_rx {
            let result = result.unwrap();
            routes.extend(result);
        }

        println!("collected routes: {}", routes.len());
    }
}
