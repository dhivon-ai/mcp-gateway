//! Fixed endpoint telemetry boundary: SDK/network payload events are never enabled by RUST_LOG.
use tracing_subscriber::{Layer, layer::SubscriberExt};

pub fn subscriber<W>(writer: W) -> impl tracing::Subscriber + Send + Sync
where
    W: for<'a> tracing_subscriber::fmt::MakeWriter<'a> + Send + Sync + 'static,
{
    tracing_subscriber::registry().with(
        tracing_subscriber::fmt::layer()
            .with_writer(writer)
            .with_ansi(false)
            .with_filter(tracing_subscriber::filter::filter_fn(|metadata| {
                *metadata.level() <= tracing::Level::INFO
                    && (metadata.target().starts_with("gateway_cli::")
                        || metadata.target() == "gateway_cli"
                        || metadata.target() == "mcp_gateway")
            })),
    )
}
