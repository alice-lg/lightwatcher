use actix_web::{
    middleware::Compress, web, web::Bytes, App, Error, HttpResponse, HttpServer, Responder,
};
use anyhow::Result;
use bytes::{BufMut, BytesMut};
use futures::Stream;
use regex::Regex;
use std::fs::File;
use std::io::BufReader;
use std::io::Write;
use std::pin::Pin;
use std::sync::mpsc::Receiver;
use std::task::{Context, Poll};
use std::thread;

use crate::parsers::parser::BlockIterator;
use crate::parsers::routes::PrefixGroup;
use crate::parsers::routes_worker::RoutesWorkerPool;
use crate::state::Route;

/// Routes response stream
pub struct RoutesStream {
    receiver: Receiver<Result<PrefixGroup>>,
    count: usize,
    buf: BytesMut,
}

impl RoutesStream {
    /// Create a new stream from a receiver
    pub fn new(receiver: Receiver<Result<PrefixGroup>>) -> Self {
        let buf = BytesMut::new();
        Self {
            receiver,
            buf,
            count: 0,
        }
    }
}

impl Stream for RoutesStream {
    type Item = Result<Bytes, Error>;

    fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        let buf = BytesMut::new();
        let mut writer = buf.writer();

        if this.count == 0 {
            writer.write(b"[\n").unwrap();
        }

        match this.receiver.recv() {
            Ok(prefix_group) => {
                // Encode the routes using json and send them to the client
                for route in prefix_group.unwrap() {
                    serde_json::to_writer(&mut writer, &route).unwrap();
                    writer.write(b",\n").unwrap();
                    this.count += 1;
                }

                Poll::Ready(Some(Ok(Bytes::copy_from_slice(&this.buf))))
            }
            Err(_) => Poll::Ready(None),
        }
    }
}

async fn index() -> impl Responder {
    // let file = File::open("tests/birdc/show-route-all-protocol-R192_175").unwrap();
    let file = File::open("tests/birdc/show-route-all-table-master4").unwrap();

    let reader = BufReader::new(file);
    let re_routes_start = Regex::new(r"1007-\S").unwrap();

    let blocks = BlockIterator::new(reader, &re_routes_start);
    let (blocks_tx, results_rx) = RoutesWorkerPool::spawn(4);

    thread::spawn(move || {
        for block in blocks {
            blocks_tx.send(block).unwrap();
        }
    });

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
