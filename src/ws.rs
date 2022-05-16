use futures::{SinkExt, StreamExt};
use log::{error, info};
use std::net::{SocketAddr, TcpListener as StdTcpListener};
use std::thread;
use std::time::Duration;
use crossbeam_channel::Receiver;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{accept_async, tungstenite::Error};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::Result;

async fn accept_connection(peer: SocketAddr, stream: TcpStream, rx: Receiver<i32>) {
    if let Err(e) = handle_connection(peer, stream, rx).await {
        match e {
            Error::ConnectionClosed | Error::Protocol(_) | Error::Utf8 => (),
            err => error!("Error processing connection: {:?}", err),
        }
    }
}

async fn handle_connection(peer: SocketAddr, stream: TcpStream, rx: Receiver<i32>) -> Result<()> {
    let mut ws_stream = accept_async(stream).await.expect("Failed to accept");

    info!("New WebSocket connection: {}", peer);

    while let Some(msg) = ws_stream.next().await {
        let r = rx.try_recv().unwrap();
        if r > 0 {
            ws_stream.send(Message::Text(String::from("NEW_MESSAGE")));
        }
    }

    Ok(())
}

pub async fn run_ws_server(
    tcp_listener: StdTcpListener,
    rx: Receiver<i32>
) -> Result<()> {
    info!(
        "Starting ws server on {}",
        tcp_listener.local_addr().unwrap()
    );

    tcp_listener.set_nonblocking(true).unwrap();
    let listener = TcpListener::from_std(tcp_listener)?;

    while let Ok((stream, _)) = listener.accept().await {
        let peer = stream.peer_addr().expect("connected streams should have a peer address");
        info!("Peer address: {}", peer);

        tokio::spawn(accept_connection(peer, stream, rx.clone()));
    }

    Ok(())
}