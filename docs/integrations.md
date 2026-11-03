# Add a tool server

In the admin console, open **Catalog** and add the MCP server you want your team to use. Choose the connection type that matches where its tools run.

## Local STDIO

Choose **STDIO** for a program that runs on the developer’s Mac. Enter the absolute executable path, arguments and any required environment values. If installation is required, enter its command separately. Developers review and run it explicitly.

## Remote Streamable HTTP without authentication

Choose **Streamable HTTP** for a remote service. Enter its exact HTTPS MCP endpoint and choose **None** only if the service accepts requests without credentials.

## Remote Streamable HTTP with a bearer key

Select **Bearer**. The developer adds their own key under **Secrets** in the Mac console and selects it in **Manage**. Keep tokens out of catalog URLs, command arguments and agent settings.

## Remote Streamable HTTP with supported OAuth

Enter the registered public client ID and scopes. The developer chooses **Connect**, completes provider consent and enables the MCP. See the [callback and provider instructions](oauth.md).

## Check access end to end

Allow the original tool names that should be available. In **Access**, assign the entry to the intended users or groups. Ask a developer to refresh the console, enable it, and call a permitted tool from their [connected agent](agent-configuration.md). The [echo walkthrough](quickstart.md#try-an-echo-tool) gives a working local example.
