use crate::rabbitmq::RmqMessage;
use std::{future::Future, marker::Send};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::mpsc::UnboundedSender,
    task::JoinHandle,
};

/// The buffer size to be used when reading tracker connections.
///
/// This is more than enough to handle all packets from all trackers,
/// if a connection sends a packet through TCP/UDP with more bytes than
/// this then its very unlikely to be a tracking device
pub const BUFFER_SIZE: usize = 512;

/// The maximum amount of undecodable packets a tracker can send
/// before its connection should be dropped
pub const INVALID_PACKET_LIMIT: usize = 10;

/// A TCP handle receives the tcp stream to handle and a unbounded sender
/// to send the decoded tracker events sent over the TCP connection (such
/// as a new position or tracker command response)
type TcpHandler<R> = fn(TcpStream, UnboundedSender<(RmqMessage, tracing::Span)>) -> R;

/// Start a new tokio task that binds a TcpListener to addr and pass all
/// incoming connections to the the handler on another task.
pub fn start_tcp_listener(
    addr: &str,
    sender: UnboundedSender<(RmqMessage, tracing::Span)>,
    handler: TcpHandler<impl Future<Output = ()> + 'static + Send>,
) -> JoinHandle<()> {
    let addr = addr.to_string();

    tokio::spawn(async move {
        let listener = TcpListener::bind(addr.clone())
            .await
            .expect("failed to start TCP listener");

        println!("[TCP] listener started at: {}", addr);

        while let Ok((stream, _)) = listener.accept().await {
            tokio::spawn(handler(stream, sender.clone()));
        }

        println!("[TCP] listener at: {} stopped", addr);
    })
}
