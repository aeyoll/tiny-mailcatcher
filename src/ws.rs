use event_listener::Event;
use futures::{sink::SinkExt, stream::StreamExt};
use hyper::Server;
use hyper::{Body, Request, Response};
use hyper_tungstenite::{tungstenite, HyperWebsocket};
use log::info;
use std::convert::Infallible;
use std::net::TcpListener;
use std::sync::Arc;
use tokio::try_join;
use tungstenite::Message;

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

/// Handle a HTTP or WebSocket request.
pub async fn handle_request(request: Request<Body>, event: Arc<Event>) -> Result<Response<Body>, Error> {
    // Check if the request is a websocket upgrade request.
    if hyper_tungstenite::is_upgrade_request(&request) {
        let (response, mut websocket) = hyper_tungstenite::upgrade(request, None)?;

        let a = tokio::spawn(async move {
            if let Err(e) = serve_websocket(websocket).await {
                eprintln!("Error in websocket connection: {}", e);
            }
        });

        let b = tokio::spawn(async move {
            // Start listening for events.
            let listener = event.listen();

            websocket.send(Message::text("NEW_MESSAGE")).await?;

            // Wait for a notification and continue the loop.
            listener.wait();

        });

        let (a_res, b_res) = try_join!(a, b)?;
        // Return the response so the spawned future can continue.
        Ok(response)
    } else {
        // Handle regular HTTP requests here.
        Ok(Response::new(Body::from("Hello HTTP!")))
    }
}

/// Handle a websocket connection.
async fn serve_websocket(websocket: HyperWebsocket) -> Result<(), Error> {
    let mut websocket = websocket.await?;

    while let Some(message) = websocket.next().await {
        match message? {
            Message::Text(msg) => {
                println!("Received text message: {}", msg);
            }
            Message::Binary(msg) => {
                println!("Received binary message: {:02X?}", msg);
            }
            Message::Ping(msg) => {
                println!("Received ping message: {:02X?}", msg);
            }
            Message::Pong(msg) => {
                println!("Received pong message: {:02X?}", msg);
            }
            Message::Close(msg) => {
                if let Some(msg) = &msg {
                    println!(
                        "Received close message with code {} and message: {}",
                        msg.code, msg.reason
                    );
                } else {
                    println!("Received close message");
                }
            }
            Message::Frame(msg) => {
                println!("Received frame message: {:02X?}", msg);
            }
        }
    }

    Ok(())
}

pub async fn run_ws_server(
    tcp_listener: TcpListener,
    event: Arc<Event>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!(
        "Starting WS server on {}",
        tcp_listener.local_addr().unwrap()
    );

    let service = hyper::service::make_service_fn(|_connection| {
        let event = event.clone();

        async {
            Ok::<_, Infallible>(hyper::service::service_fn(move |req| {
                let event = event.clone();
                handle_request(req, event)
            }))
        }
    });

    let server = Server::from_tcp(tcp_listener).unwrap().serve(service);

    server.await?;

    Ok(())
}
