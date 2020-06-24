use std::collections::HashMap;

use futures::{FutureExt, StreamExt};
use warp::Filter;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    // POST /login
    let login = warp::path!("login")
        .and(warp::post())
        .and(warp::body::content_length_limit(1024 * 16))
        .and(warp::body::form())
        .map(|form: HashMap<String, String>| {
            println!("form: {:?}", form);
            ""
        });

    // GET /ws
    let ws = warp::path("ws")
        // The `ws()` filter will prepare the Websocket handshake.
        .and(warp::ws())
        .map(|ws: warp::ws::Ws| {
            // And then our closure will be called when it completes...
            ws.on_upgrade(|websocket| {
                // Just echo all messages back...
                let (tx, rx) = websocket.split();
                rx.forward(tx).map(|result| {
                    if let Err(e) = result {
                        eprintln!("websocket error: {:?}", e);
                    }
                })
            })
        });

    // GET /healthz => 200 OK
    let healthz = warp::path("healthz").map(warp::reply);
    let api = login.or(ws);

    let routes = healthz.or(api.with(warp::log("api")));
    warp::serve(routes).run(([0, 0, 0, 0], 3030)).await;
}
