use anyhow::Result;
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};
use std::thread;
use tokio::sync::mpsc::error::TryRecvError;
use tokio::sync::mpsc::{
    unbounded_channel, UnboundedReceiver, UnboundedSender,
};

use crate::parsers::{
    parser::{Block, Parse},
    routes::PrefixGroup,
};

type BlockQueue = Arc<Mutex<UnboundedReceiver<Block>>>;
type ResultsQueue = UnboundedSender<Result<PrefixGroup>>;

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
        println!("routes worker {} started.", self.id);

        thread::spawn(move || loop {
            let block = {
                let mut queue = block_queue.lock().unwrap();
                queue.try_recv()
            };
            match block {
                Ok(block) => {
                    let routes = PrefixGroup::parse(block);
                    results_queue.send(routes).unwrap();
                }
                Err(TryRecvError::Empty) => {
                    continue;
                }
                Err(TryRecvError::Disconnected) => {
                    break;
                }
            }
        });
    }
}

/// A routes worker pool has a collection of workers
/// and a queue of blocks to be parsed.
pub struct RoutesWorkerPool {}

impl RoutesWorkerPool {
    /// Create new worker pool and spawn workers
    pub fn spawn() -> (
        UnboundedSender<Block>,
        UnboundedReceiver<Result<PrefixGroup>>,
    ) {
        let (blocks_tx, blocks_rx) = unbounded_channel::<Block>();
        let (results_tx, results_rx) =
            unbounded_channel::<Result<PrefixGroup>>();

        let blocks_queue = Arc::new(Mutex::new(blocks_rx));

        // Determine the number of workers
        let parallelism = std::thread::available_parallelism()
            .unwrap_or(NonZeroUsize::new(1).unwrap());
        let num_workers = parallelism.get();
        println!("Starting routes worker pool with {} workers.", num_workers);

        // Start workers
        for id in 0..num_workers {
            let worker = RoutesWorker::new(id);
            worker.spawn(blocks_queue.clone(), results_tx.clone());
        }

        (blocks_tx, results_rx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsers::parser::BlockIterator;
    use crate::state::Route;
    use std::fs::File;
    use std::io::BufReader;

    use regex::Regex;

    #[tokio::test]
    async fn test_routes_worker() {
        let file =
            File::open("tests/birdc/show-route-all-protocol-R1").unwrap();
        // let file: File = File::open("tests/birdc/show-route-all-table-master4").unwrap();
        let reader = BufReader::new(file);
        let re_routes_start = Regex::new(r"1007-\S").unwrap();

        let blocks = BlockIterator::new(reader, &re_routes_start);

        let mut routes: Vec<Route> = vec![];

        // Spawn workers
        let (blocks_tx, mut results_rx) = RoutesWorkerPool::spawn();

        thread::spawn(move || {
            for block in blocks {
                blocks_tx.send(block).unwrap();
            }
        });

        while let Some(result) = results_rx.recv().await {
            let result = result.unwrap();
            routes.extend(result);
        }

        println!("collected routes: {}", routes.len());
    }
}
