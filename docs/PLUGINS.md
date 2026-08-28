# Plugins

`unified-agent-rs` has a plugin system for adding new tool providers,
auth backends, or transport adapters without forking the core. Plugins
are loaded at startup from the configured plugin directory.

## Plugin layout

A plugin is a directory (or a single `.wasm` file) with a manifest:

```
my-plugin/
  plugin.toml      # manifest
  module.wasm      # compiled plugin body
```

The manifest declares what kind of plugin this is and what extension
points it hooks:

```toml
[plugin]
name = "my-tool-provider"
version = "0.1.0"
kind = "tool_provider"
api_version = 1

[tool_provider]
tool = "my_tool"
schema = "schema/my_tool.json"
```

## Loading plugins

Point the agent at a plugin directory:

```bash
unified-agent-rs run --plugin-dir ./plugins task.yaml
```

Or configure it permanently:

```toml
[plugins]
dirs = ["./plugins", "/usr/lib/unified-agent/plugins"]
allow_unsigned = false   # refuse plugins without a valid signature
```

At startup the agent scans the directories, validates manifests, checks
signatures (if `allow_unsigned = false`), and registers each plugin at
its declared extension point. A failed plugin logs an error and is
skipped; it does not abort startup.

## Extension points

The currently supported `kind` values are:

| kind | Hooks |
|------|-------|
| `tool_provider` | Registers a new tool the agent can call |
| `auth_backend` | Provides credentials for a named provider |
| `transport_adapter` | Adds a new wire format for agent↔host comms |
| `policy_hook` | Runs before/after tool calls for gating or audit |

A plugin can declare multiple `[plugin]` blocks to hook more than one
extension point from the same module.

## Signing

Plugins intended for shared distribution should be signed. The agent
verifies the signature against the public key in the manifest and
refuses to load unsigned plugins when `allow_unsigned = false`.

```bash
unified-agent-rs plugin sign ./my-plugin --key ./signing.key
unified-agent-rs plugin verify ./my-plugin
```

## Verification

```bash
unified-agent-rs doctor --check plugins
```

Lists every loaded plugin, its kind, version, and signature status.
Run this after adding or upgrading a plugin to confirm it registered
cleanly.
