FROM node:24.19.0-bookworm-slim AS web
WORKDIR /source
RUN npm install --global pnpm@11.4.0
COPY package.json pnpm-lock.yaml pnpm-workspace.yaml ./
COPY apps/web/package.json apps/web/package.json
RUN pnpm install --frozen-lockfile
COPY apps/web apps/web
ENV GATEWAY_BUILD_CACHE=/build
RUN pnpm build

FROM rust:1.95.0-bookworm AS rust
WORKDIR /source
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY crates crates
COPY --from=web /build/web /build/web
ENV GATEWAY_WEB_ASSETS=/build/web
RUN cargo build --locked --release -p gateway-server

FROM debian:bookworm-slim AS service
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --gid 10001 gateway && useradd --uid 10001 --gid gateway --no-create-home gateway
COPY --from=rust /source/target/release/mcp-gateway-server /usr/local/bin/mcp-gateway-server
COPY deploy/healthcheck.sh /usr/local/bin/gateway-healthcheck
USER 10001:10001
ENV GATEWAY_BIND=0.0.0.0:8080
EXPOSE 8080
STOPSIGNAL SIGINT
HEALTHCHECK --interval=10s --timeout=6s --start-period=20s --retries=6 CMD ["/bin/sh", "/usr/local/bin/gateway-healthcheck"]
ENTRYPOINT ["/usr/local/bin/mcp-gateway-server"]
