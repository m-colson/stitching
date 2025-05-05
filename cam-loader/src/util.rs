//! This module contains functions that could be useful when create custom frame loaders.

/// Logs with [`tracing`] why a loader is exiting based on the kind of receive error.
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
