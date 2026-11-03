# Why use a gateway?

Connect your coding agent once. When your team adds or removes a tool, your administrator updates access in one place and you choose which assigned tools to enable on your Mac.

## Who does what?

| Where | What happens there |
| --- | --- |
| Team server | An administrator creates accounts, assigns tool servers and chooses which tools people may call. PostgreSQL stores those settings. |
| Your Mac | The console lets you sign in, select credentials and enable tools. The gateway checks your access whenever your agent lists or calls a tool. |
| Tool server | A local command runs on your Mac with your permissions. A remote tool runs at the configured HTTPS service. |

## An example

An administrator gives the Engineering group access to an echo tool. You sign in and enable it. Your coding agent can now call it. After you disable it, the gateway rejects a call using its old name.

[Try the echo walkthrough](quickstart.md#try-an-echo-tool), read [how access works](configuration.md), or check [supported platforms](compatibility.md).
