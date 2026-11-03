# Verify a Mac archive

Use the checksum supplied through your trusted distribution channel for the exact archive you received. Different local builds can have the same filename and different contents.

The Apple Silicon archive built on November 3, 2026 with these setup guides has SHA-256 `5450c6aaffab666458d0d2ce8f146f912103923c84021bd549e476b227c9bf8f`.

For the earlier Apple Silicon archive built from revision `41d8c65`, the SHA-256 is `e2388fdf4af9371195a2307cdf4e654bb90fd604f62101c739606cdd4900ddc4`.

For a build made with `bash scripts/package-macos.sh`, the command prints the archive and its matching checksum file. From their directory run:

```sh
shasum -a 256 -c dhivon-mcp-gateway-0.1.0-macos-arm64.tar.gz.sha256
```

Expect **OK**. Continue with [Mac setup](mac-install.md).
