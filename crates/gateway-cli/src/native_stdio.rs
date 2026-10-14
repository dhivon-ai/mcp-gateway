//! Native Mac protocol pipes avoid Tokio stdin's uncancellable blocking-thread shutdown.
use std::{io, os::fd::AsFd};
use tokio::net::unix::pipe::{Receiver, Sender};

/// Safe pipe adapters verify access/type and own duplicated descriptors in nonblocking mode.
/// Agent STDIO must be pipes; terminal/file input is not an interactive login surface.
pub fn pipes() -> io::Result<(Receiver, Sender)> {
    Ok((
        Receiver::from_owned_fd(rustix::io::dup(std::io::stdin().as_fd())?)?,
        Sender::from_owned_fd(rustix::io::dup(std::io::stdout().as_fd())?)?,
    ))
}
