//! Bounded byte seams around the pinned SDK. No payloads enter diagnostics.
use serde_json::Value;
use std::{
    collections::VecDeque,
    io,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::io::{AsyncRead, ReadBuf};

pub const MAX_MESSAGE_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_TOOL_BYTES: usize = 256 * 1024;
pub const MAX_TOOLS_PER_MCP: usize = 2048;

#[derive(Clone, Copy)]
pub enum Direction {
    Agent,
    Upstream,
}

fn invalid() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        "Unsupported or invalid bounded MCP message",
    )
}

/// The SDK does not retain execution.taskSupport. Inspect it before typed decoding.
pub fn sanitize(bytes: &[u8], direction: Direction) -> io::Result<Vec<u8>> {
    if bytes.len() > MAX_MESSAGE_BYTES {
        return Err(invalid());
    }
    let mut value: Value = serde_json::from_slice(bytes).map_err(|_| invalid())?;
    if !value.is_object() {
        return Err(invalid());
    }
    match direction {
        Direction::Agent => {
            if value
                .get("params")
                .and_then(Value::as_object)
                .is_some_and(|p| p.contains_key("task"))
            {
                return Err(invalid());
            }
        }
        Direction::Upstream => {
            if let Some(tools) = value.get_mut("result").and_then(|r| r.get_mut("tools")) {
                let tools = tools.as_array_mut().ok_or_else(invalid)?;
                if tools.len() > MAX_TOOLS_PER_MCP {
                    return Err(invalid());
                }
                let mut supported = Vec::with_capacity(tools.len());
                for mut tool in tools.drain(..) {
                    let name = tool
                        .get("name")
                        .and_then(Value::as_str)
                        .ok_or_else(invalid)?;
                    if name.is_empty() || name.len() > 512 || name.contains(['\0', '\r', '\n']) {
                        return Err(invalid());
                    }
                    if serde_json::to_vec(&tool).map_err(|_| invalid())?.len() > MAX_TOOL_BYTES {
                        return Err(invalid());
                    }
                    let support = match tool.get("execution") {
                        None => "forbidden",
                        Some(execution) if execution.is_object() => {
                            match execution.get("taskSupport") {
                                None => "forbidden",
                                Some(v) => v.as_str().ok_or_else(invalid)?,
                            }
                        }
                        _ => return Err(invalid()),
                    };
                    match support {
                        "required" => continue,
                        "forbidden" | "optional" => (),
                        _ => return Err(invalid()),
                    }
                    if let Some(object) = tool.as_object_mut() {
                        object.remove("execution");
                    }
                    supported.push(tool);
                }
                *tools = supported;
            }
        }
    }
    let bytes = serde_json::to_vec(&value).map_err(|_| invalid())?;
    if bytes.len() > MAX_MESSAGE_BYTES {
        return Err(invalid());
    }
    Ok(bytes)
}

/// Cancellation-safe complete-line buffer; the SDK never sees a partial transformed frame.
pub struct BoundedRead<R> {
    inner: R,
    direction: Direction,
    partial: Vec<u8>,
    tail: VecDeque<u8>,
    ready: Vec<u8>,
    offset: usize,
    failed: bool,
}
impl<R> BoundedRead<R> {
    pub fn new(inner: R, direction: Direction) -> Self {
        Self {
            inner,
            direction,
            partial: Vec::new(),
            tail: VecDeque::new(),
            ready: Vec::new(),
            offset: 0,
            failed: false,
        }
    }
}
impl<R: AsyncRead + Unpin> AsyncRead for BoundedRead<R> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        out: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        if out.remaining() == 0 || this.failed {
            return Poll::Ready(Ok(()));
        }
        loop {
            if this.offset < this.ready.len() {
                let end = (this.offset + out.remaining()).min(this.ready.len());
                out.put_slice(&this.ready[this.offset..end]);
                this.offset = end;
                if end == this.ready.len() {
                    this.ready.clear();
                    this.offset = 0;
                }
                return Poll::Ready(Ok(()));
            }
            while let Some(byte) = this.tail.pop_front() {
                if byte == b'\n' {
                    if this.partial.last() == Some(&b'\r') {
                        this.partial.pop();
                    }
                    if this.partial.is_empty() {
                        continue;
                    }
                    match sanitize(&this.partial, this.direction) {
                        Ok(mut frame) => {
                            frame.push(b'\n');
                            this.ready = frame;
                            this.partial.clear();
                            break;
                        }
                        Err(error) => {
                            this.failed = true;
                            return Poll::Ready(Err(error));
                        }
                    }
                }
                if this.partial.len() >= MAX_MESSAGE_BYTES {
                    this.failed = true;
                    return Poll::Ready(Err(invalid()));
                }
                this.partial.push(byte);
            }
            if !this.ready.is_empty() {
                continue;
            }
            let mut chunk = [0u8; 8192];
            let mut input = ReadBuf::new(&mut chunk);
            match Pin::new(&mut this.inner).poll_read(cx, &mut input) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(Err(_)) => {
                    this.failed = true;
                    return Poll::Ready(Err(io::Error::new(
                        io::ErrorKind::BrokenPipe,
                        "MCP transport closed",
                    )));
                }
                Poll::Ready(Ok(())) if input.filled().is_empty() => {
                    this.failed = true;
                    return Poll::Ready(Ok(()));
                }
                Poll::Ready(Ok(())) => this.tail.extend(input.filled()),
            }
        }
    }
}

struct Count(usize);
impl io::Write for Count {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > MAX_MESSAGE_BYTES.saturating_sub(self.0) {
            return Err(invalid());
        }
        self.0 += bytes.len();
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
/// Check the entire outgoing SDK message before any protocol bytes are written.
pub struct BoundedTransport<T> {
    inner: T,
}
impl<T> BoundedTransport<T> {
    pub fn new(inner: T) -> Self {
        Self { inner }
    }
}
impl<Role, T> rmcp::transport::Transport<Role> for BoundedTransport<T>
where
    Role: rmcp::service::ServiceRole,
    T: rmcp::transport::Transport<Role, Error = io::Error>,
    rmcp::service::TxJsonRpcMessage<Role>: serde::Serialize,
{
    type Error = io::Error;
    fn send(
        &mut self,
        message: rmcp::service::TxJsonRpcMessage<Role>,
    ) -> impl Future<Output = io::Result<()>> + Send + 'static {
        let valid = serde_json::to_writer(Count(0), &message).is_ok();
        let future = if valid {
            Some(self.inner.send(message))
        } else {
            None
        };
        async move {
            match future {
                Some(future) => future.await,
                None => Err(invalid()),
            }
        }
    }
    async fn receive(&mut self) -> Option<rmcp::service::RxJsonRpcMessage<Role>> {
        self.inner.receive().await
    }
    async fn close(&mut self) -> io::Result<()> {
        self.inner.close().await
    }
}
