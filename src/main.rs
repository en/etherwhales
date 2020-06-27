use std::collections::HashMap;
use std::env;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use futures::{FutureExt, StreamExt};
use tokio::sync::{mpsc, RwLock};
use warp::{
    ws::{Message, WebSocket},
    Filter,
};

mod eth;

static NEXT_USER_ID: AtomicUsize = AtomicUsize::new(1);
type Users = Arc<RwLock<HashMap<usize, mpsc::UnboundedSender<Result<Message, warp::Error>>>>>;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let infura_endpoint = env::var("INFURA_ENDPOINT").expect("failed to get INFURA_ENDPOINT");

    let users = Users::default();

    tokio::task::spawn(eth::subscribe(infura_endpoint.to_string(), users.clone()));

    let users = warp::any().map(move || users.clone());

    // POST /login
    let login = warp::path("login")
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
        .and(users)
        .map(|ws: warp::ws::Ws, users| {
            // And then our closure will be called when it completes...
            ws.on_upgrade(move |socket| user_connected(socket, users))
        });

    // GET /healthz => 200 OK
    let healthz = warp::path("healthz").map(warp::reply);
    let api = login.or(ws);

    let routes = healthz.or(api.with(warp::log("api")));
    warp::serve(routes).run(([0, 0, 0, 0], 3030)).await;
}

async fn user_connected(ws: WebSocket, users: Users) {
    let my_id = NEXT_USER_ID.fetch_add(1, Ordering::Relaxed);

    eprintln!("new user: {}", my_id);

    let (user_ws_tx, _user_ws_rx) = ws.split();

    let (tx, rx) = mpsc::unbounded_channel();
    tokio::task::spawn(rx.forward(user_ws_tx).map(|result| {
        if let Err(e) = result {
            eprintln!("websocket send error: {}", e);
        }
    }));

    users.write().await.insert(my_id, tx);
}
