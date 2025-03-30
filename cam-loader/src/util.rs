pub fn log_recv_err(err: &kanal::ReceiveError) {
    match err {
        kanal::ReceiveError::SendClosed => {
            tracing::warn!("loader exiting because all senders have dropped")
        }
        kanal::ReceiveError::Closed => {
            tracing::warn!("loader exiting bacause it was closed")
        }
    }
}
