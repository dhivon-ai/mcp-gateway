# Supported platforms

| Component | Supported or tested setup |
| --- | --- |
| Mac gateway | Apple Silicon, tested on macOS 27.0.1. The native CLI includes its browser console. |
| Server | Self-hosted Linux arm64 container and PostgreSQL 18.6. The root Compose stack was exercised with Docker 29.8.2 and Compose 5.5.0. |
| Agent connection | MCP tools over STDIO, protocol 2025-11-25. The official rmcp client is tested. Adapt the [agent example](agent-configuration.md) to your client. |
| Tool servers | Local STDIO commands and remote Streamable HTTP. Remote authentication supports no credentials, per-user bearer keys or the supported public-client OAuth flow. |
| Archive | Unsigned Apple Silicon archive built locally. Use an artifact supplied through your organization’s trusted channel or [build from source](mac-install.md#build-your-own-archive). |

Resources, prompts, legacy SSE transport, Intel Macs and Windows/Linux endpoints are outside the supported profile. Compatibility with a named agent or OAuth provider depends on its implementation and registration.

## Fresh installation

Start the server with a fresh database. Existing Mac users should read the [upgrade and recovery guide](endpoint-identity.md#upgrade-and-recovery) and retain the executable that created their credentials. Check a distributed archive using its matching checksum.
