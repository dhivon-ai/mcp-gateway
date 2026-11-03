# Connect your Mac to your team

Ask your administrator for the gateway’s HTTPS address, your account, and a trusted Apple Silicon CLI archive with its checksum. The CLI is the Mac application. It includes the browser console and the connection your coding agent uses.

## Install the CLI

On a supported Apple Silicon Mac, place the archive and its matching checksum in the same folder. From that folder, run:

```sh
shasum -a 256 -c dhivon-mcp-gateway-0.1.0-macos-arm64.tar.gz.sha256
```

Continue only if the result is **OK**. For a fresh installation:

```sh
tar -xzf dhivon-mcp-gateway-0.1.0-macos-arm64.tar.gz
mkdir -p "$HOME/.local/bin"
cp dhivon-mcp-gateway-0.1.0-macos-arm64/dhivon-mcp-gateway "$HOME/.local/bin/"
"$HOME/.local/bin/dhivon-mcp-gateway" --version
```

Expect `dhivon-mcp-gateway 0.1.0`. The archive is unsigned. Follow your organization’s software policy. If you already have the CLI, retain the old executable until you’ve completed the [upgrade procedure](endpoint-identity.md#upgrade-and-recovery).

## Open the console

Replace the example URL with the address from your administrator:

```sh
"$HOME/.local/bin/dhivon-mcp-gateway" ui --server https://gateway.example.com
```

Keep the command running. It opens a local page in your browser. Enter your work email and gateway password. Under **MCPs**, open **Manage** for an assigned tool server and enable it. If it needs an installation, review and confirm the command first. For a bearer key or OAuth sign-in, follow the instructions in **Manage**.

When you see **Enabled**, [connect your coding agent](agent-configuration.md). Each time you launch the console, supply the team URL. Your agent uses the saved account and needs just the path to the CLI and the argument `mcp`.

## Build your own archive

If your administrator has supplied the archive, you can skip this step. To build from a source checkout, use an Apple Silicon Mac with Xcode command-line tools, Rust 1.95.0, Node 24.19.0 and pnpm 11.4.0. From the repository root run:

```sh
bash scripts/package-macos.sh
```

The command builds the CLI and its console, then prints the full **Archive** and **Checksum** paths. By default each build gets a private directory under `dist/`. Verify and install the archive using the steps above. Node and Rust are only required on the build machine.

## Sign out

Stop agent connections, then run `"$HOME/.local/bin/dhivon-mcp-gateway" logout`. The gateway clears the signed-in generation’s local credentials and requests session revocation when online.
