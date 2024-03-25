use crate::protocols::common::Decoded;
use crate::protocols::h02;
use crate::protocols::h02::decoder::Message;
use crate::rabbitmq::RmqMessage;
use crate::server::listeners::{BUFFER_SIZE, INVALID_PACKET_LIMIT};
use serde::Serialize;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc::UnboundedSender;
use tracing::{error, info_span, span, Level};

type RmqMsgSender = UnboundedSender<(RmqMessage, tracing::Span)>;

/// sends the decoded event to the rust channel, once
/// recieved it will be sent to the tracker events exchange
fn send_event<T>(event: Decoded<T>, sender: &RmqMsgSender) -> Result<(), String>
where
    T: Serialize,
{
    let span = info_span!("send_event");

    let rmq_msg: RmqMessage = event.try_into()?;

    sender
        .send((rmq_msg, span))
        .or(Err("rmq msg channel closed"))?;

    Ok(())
}

#[tracing::instrument(skip_all)]
fn handle_decoded_message(message: Message, sender: &RmqMsgSender) -> Option<Box<[u8]>> {
    match message {
        Message::Heartbeat(decoded) => {
            let response = decoded.response.clone();
            let _ = send_event(decoded, sender);

            response
        }
        Message::Location(decoded) => {
            let response = decoded.response.clone();
            let _ = send_event(decoded, sender);

            response
        }
    }
}

pub async fn stream_handler(stream: TcpStream, sender: RmqMsgSender) {
    let mut buffer = vec![0; BUFFER_SIZE];

    let (mut reader, mut writer) = io::split(stream);

    let mut invalid_packets_cnt: usize = 0;

    while let Ok(n) = reader.read(&mut buffer).await {
        if n == 0 {
            // EOF
            break;
        }

        let packets = &buffer[..n];
        let packets_len = packets.len();

        let span = span!(
            Level::ERROR,
            "stream_handler",
            invalid_packets_cnt,
            packets_len
        );
        let _enter = span.enter();

        let decode_result = h02::decoder::decode(packets);

        match decode_result {
            Ok(msg) => {
                if let Some(response_to_tracker) = handle_decoded_message(msg, &sender) {
                    // We intentionally block on write here because because writes rarely happen (so blocking should not be much of a problem)
                    // and because some tracker models should receive the response to their commands in order, so if a tracker sends a command
                    // A and B responses A1 and B1 should be in that order.
                    if let Err(err) = writer.write_all(&response_to_tracker).await {
                        // writes to the tracker happen when responding to commands and failures
                        // are a really bad state, so for now assume the connection is unrecoverable
                        // and end it.
                        error!("IO error writing response to tracker: {}", err);
                        break;
                    }
                }
            }
            Err(err_msg) => {
                error!("error parsing h02 packets: {}", err_msg);

                invalid_packets_cnt += 1;

                if invalid_packets_cnt >= INVALID_PACKET_LIMIT {
                    break;
                }
            }
        }
    }
}
