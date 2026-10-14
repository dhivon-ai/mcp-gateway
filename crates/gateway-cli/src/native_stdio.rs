//! Nonblocking Mac STDIO, including the Unix socket pairs used by Node child_process.
use std::{
    fs::File,
    io,
    os::{
        fd::{AsFd, OwnedFd},
        unix::{fs::FileTypeExt, net::UnixStream},
    },
};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::unix::pipe::{Receiver, Sender},
};

type Reader = Box<dyn AsyncRead + Send + Unpin>;
type Writer = Box<dyn AsyncWrite + Send + Unpin>;

/// Own duplicated, checked nonblocking descriptors. Never use Tokio's blocking stdin thread.
/// Regular files, terminals, datagrams and non-Unix sockets are not agent transports.
pub fn streams() -> io::Result<(Reader, Writer)> {
    Ok((
        input(rustix::io::dup(std::io::stdin().as_fd())?)?,
        output(rustix::io::dup(std::io::stdout().as_fd())?)?,
    ))
}
fn socket(fd: OwnedFd) -> io::Result<tokio::net::UnixStream> {
    if rustix::net::sockopt::socket_type(&fd)? != rustix::net::SocketType::STREAM {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "STDIO requires a stream socket",
        ));
    }
    let stream = UnixStream::from(fd);
    stream.peer_addr()?; // proves connected AF_UNIX, including unnamed socket pairs
    stream.set_nonblocking(true)?;
    tokio::net::UnixStream::from_std(stream)
}
fn input(fd: OwnedFd) -> io::Result<Reader> {
    let file = File::from(fd);
    let kind = file.metadata()?.file_type();
    if kind.is_fifo() {
        Ok(Box::new(Receiver::from_owned_fd(file.into())?))
    } else if kind.is_socket() {
        Ok(Box::new(socket(file.into())?))
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "STDIO requires a pipe or Unix stream",
        ))
    }
}
fn output(fd: OwnedFd) -> io::Result<Writer> {
    let file = File::from(fd);
    let kind = file.metadata()?.file_type();
    if kind.is_fifo() {
        Ok(Box::new(Sender::from_owned_fd(file.into())?))
    } else if kind.is_socket() {
        Ok(Box::new(socket(file.into())?))
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "STDIO requires a pipe or Unix stream",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{os::unix::net::UnixDatagram, time::Duration};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[tokio::test]
    async fn socketpair_stdio_is_nonblocking_cancellable_and_preserves_bytes() {
        let (read, peer_write) = UnixStream::pair().expect("owned socket pair");
        let (write, peer_read) = UnixStream::pair().expect("owned socket pair");
        peer_write.set_nonblocking(true).expect("peer mode");
        peer_read.set_nonblocking(true).expect("peer mode");
        let mut reader = input(read.into()).expect("socket stdin");
        let mut writer = output(write.into()).expect("socket stdout");
        let mut peer_write = tokio::net::UnixStream::from_std(peer_write).expect("peer");
        let mut peer_read = tokio::net::UnixStream::from_std(peer_read).expect("peer");
        let mut bytes = [0; 4];
        assert!(
            tokio::time::timeout(Duration::from_millis(20), reader.read_exact(&mut bytes))
                .await
                .is_err()
        );
        peer_write.write_all(b"read").await.expect("peer write");
        reader
            .read_exact(&mut bytes)
            .await
            .expect("read after cancelled wait");
        assert_eq!(&bytes, b"read");
        writer.write_all(b"send").await.expect("stdout write");
        peer_read.read_exact(&mut bytes).await.expect("peer read");
        assert_eq!(&bytes, b"send");
        // A blocked agent cannot keep an uncancellable worker thread alive on shutdown.
        let large = vec![0; 8 * 1024 * 1024];
        assert!(
            tokio::time::timeout(Duration::from_millis(20), writer.write_all(&large))
                .await
                .is_err()
        );
        drop(writer);
        drop(peer_write);
        assert_eq!(reader.read(&mut bytes).await.expect("peer EOF"), 0);
    }

    #[tokio::test]
    async fn non_protocol_descriptors_are_rejected() {
        assert!(input(File::open("/dev/null").expect("character device").into()).is_err());
        assert!(
            input(
                File::open(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"))
                    .expect("regular file")
                    .into()
            )
            .is_err()
        );
        let (datagram, _) = UnixDatagram::pair().expect("owned datagram");
        assert!(input(datagram.into()).is_err());
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("owned TCP");
        let tcp = std::net::TcpStream::connect(listener.local_addr().expect("address"))
            .expect("owned connection");
        let (_peer, _) = listener.accept().expect("owned accept");
        assert!(output(tcp.into()).is_err());
    }
}
