use anyhow::Result;
use lazy_static::lazy_static;
use tokio::sync::mpsc::{self, Receiver, Sender};

use crate::{
    config,
    parsers::{
        parser::{Block, Parse},
        routes::PrefixGroup,
    },
};

pub struct RoutesWorker {
    jobs: RouteBlockSender,
}

impl RoutesWorker {
    /// Create new routes parser worker
    pub fn spawn() -> Self {
        let (jobs_tx, mut jobs_rx) = mpsc::channel::<RouteBlockParseJob>(64);

        // Parsing is quite CPU bound, so this is spawned
        // on a thread.
        tokio::task::spawn_blocking(move || loop {
            match jobs_rx.blocking_recv() {
                None => break, // channel closed
                Some(job) => {
                    let RouteBlockParseJob { block, results } = job;
                    if results.is_closed() {
                        continue; // next job
                    }
                    // Do heavy lifting.
                    let routes = PrefixGroup::parse(block);
                    if results.blocking_send(routes).is_err() {
                        tracing::warn!(
                            "routes parse job results receiver dropped"
                        );
                        continue;
                    }
                }
            }
        });

        Self { jobs: jobs_tx }
    }

    pub fn accept(&self, job: RouteBlockParseJob) -> Result<()> {
        self.jobs.blocking_send(job)?;
        Ok(())
    }
}

/// Publish Routes parsing results here
pub type RoutesResultsSender = Sender<Result<PrefixGroup>>;

/// Receive parsing results
pub type RoutesResultsReceiver = Receiver<Result<PrefixGroup>>;

/// A routes block parsing job
pub struct RouteBlockParseJob {
    pub block: Block,
    pub results: RoutesResultsSender,
}

/// Parsing Job Sender
pub type RouteBlockSender = Sender<RouteBlockParseJob>;

/// Parsing Job Receiver
pub type RouteBlockReceiver = Receiver<RouteBlockParseJob>;

/// A routes worker pool has a collection of workers
/// and a queue of blocks to be parsed.
pub struct RoutesWorkerPool {
    jobs: RouteBlockSender,
}

impl RoutesWorkerPool {
    /// Start a new workerpool
    pub fn start() -> Self {
        let (jobs_tx, mut jobs_rx) = mpsc::channel::<RouteBlockParseJob>(64);

        // Determine the number of workers
        let num_workers = config::get_routes_worker_pool_size();
        tracing::info!(
            "starting global routes worker pool with {} workers.",
            num_workers
        );

        // Start workers
        let mut workers = vec![];
        for _ in 0..num_workers {
            let w = RoutesWorker::spawn();
            workers.push(w);
        }

        // Feed workers
        let mut next_worker: usize = 0;
        tokio::task::spawn_blocking(move || loop {
            match jobs_rx.blocking_recv() {
                None => break,
                Some(job) => {
                    // round robin fanout
                    next_worker = (next_worker + 1) % num_workers;
                    if let Err(e) = workers[next_worker].accept(job) {
                        tracing::error!("worker stopped: {}", e);
                        panic!();
                    }
                }
            }
        });

        Self { jobs: jobs_tx }
    }

    pub async fn accept(&self, job: RouteBlockParseJob) -> Result<()> {
        self.jobs.send(job).await?;
        Ok(())
    }
}

lazy_static! {
    static ref ROUTES_WORKER_POOL: RoutesWorkerPool =
        RoutesWorkerPool::start();
}

/// Accept a block for parsing. Creates a job and submits
/// it to the the worker pool.
pub async fn accept_block(
    block: Block,
    results: RoutesResultsSender,
) -> Result<()> {
    let job = RouteBlockParseJob { block, results };
    ROUTES_WORKER_POOL.accept(job).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{bird::Route, parsers::parser::BlockIterator};
    use regex::Regex;
    use std::{fs::File, io::BufReader};

    #[tokio::test]
    async fn test_routes_worker() {
        let file =
            File::open("tests/birdc/show-route-all-protocol-R1").unwrap();
        /* let file: File =
        File::open("tests/birdc/show-route-all-table-master4").unwrap(); */
        let reader = BufReader::new(file);
        let re_routes_start = Regex::new(r"1007-\S").unwrap();

        let blocks = BlockIterator::new(reader, &re_routes_start);

        let mut routes: Vec<Route> = vec![];

        let pool = RoutesWorkerPool::start();

        // Spawn workers
        let (results_tx, mut results) = mpsc::channel(64);
        tokio::spawn(async move {
            for block in blocks {
                let job = RouteBlockParseJob {
                    block,
                    results: results_tx.clone(),
                };
                pool.accept(job).await.unwrap();
            }
            println!("done feeding");
        });

        while let Some(result) = results.recv().await {
            let result = result.unwrap();
            routes.extend(result);
        }

        println!("collected routes: {}", routes.len());
        assert_eq!(routes.len(), 194);
    }
}
