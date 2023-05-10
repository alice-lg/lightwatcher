use actix_web::{middleware::Compress, web, App, Error, HttpResponse, HttpServer, Responder};
use anyhow::Result;
use tokio::task;

use std::fs::File;
use std::io::BufReader;
use std::num::NonZeroUsize;
use std::thread;

use crate::parsers::parser::BlockIterator;
use crate::parsers::routes::RE_ROUTES_START;
use crate::parsers::routes_worker::RoutesWorkerPool;
use crate::state::Route;

// Get number of available cores
fn get_cpu_count() -> usize {
    thread::available_parallelism()
        .unwrap_or(NonZeroUsize::new(1).unwrap())
        .get()
}

async fn index() -> impl Responder {
    // let file = File::open("tests/birdc/show-route-all-protocol-R192_175").unwrap();
    let file = File::open("tests/birdc/show-route-all-protocol-R192_175").unwrap();

    let reader = BufReader::new(file);

    let blocks = BlockIterator::new(reader, &RE_ROUTES_START);
    let (blocks_tx, results_rx) = RoutesWorkerPool::spawn(get_cpu_count());

    task::spawn_blocking(move || {
        for block in blocks {
            blocks_tx.send(block).unwrap();
        }
    })
    .await
    .unwrap();

    // let routes_stream = RoutesStream::new(results_rx);
    let mut routes: Vec<Route> = vec![];
    for result in results_rx {
        let result = result.unwrap();
        routes.extend(result);
    }

    HttpResponse::Ok().json(routes) // .streaming(routes_stream)
}

/// Start the server on a given listening port
pub async fn start_server(port: u16) -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .wrap(Compress::default())
            .route("/", web::get().to(index))
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}
