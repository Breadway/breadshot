# breadshot — bread event integration

breadshot is a standalone, one-shot Wayland screenshot orchestrator: it
works exactly the same with or without `breadd` running. When breadd *is*
present, a successful capture publishes one event into the shared bread
automation fabric. See the parent `bread` repo's `Documentation.md` —
specifically its "Namespaces" and "Integrating a bread\* app" sections —
for the general convention this follows.

This is a different job from `bread-screenshots` (the crate in
`bread-ecosystem`): that one is a capture harness for screenshotting
sibling apps in CI. Do not merge the two.

App id: **`shot`**. Transport: `bread-utils`'s `bread_client` module
(feature `bread-client`). breadshot is a short-lived CLI, not a daemon —
each `emit` is its own fire-and-forget connection (the same stance
`bread-emit` takes for occasional callers). There is no process to hold a
command subscription open.

## Events published (`bread.shot.*`)

| Event | Data | When |
|-------|------|------|
| `bread.shot.captured` | `{ "mode": "region" \| "window" \| "output" \| "active-window" \| "active-output", "clipboard": bool, "path": <string or null> }` | A capture completed successfully (grim + clipboard write both returned). Not emitted on a cancelled slurp selection, a missing dependency, or a grim/wl-copy failure. |

`mode` is the CLI mode name (same strings `breadshot <mode>` accepts).
`clipboard` is whether the PNG was written to the clipboard — both current
capture paths do this (`save_and_copy` and `--clipboard-only`). `path` is
the saved file, or `null` when `--clipboard-only` was used (no file on
disk).

The image bytes themselves are never included in the payload. The event
bus is a notification that a capture happened, not a channel for the
screenshot.

## Commands honored (`bread.command.shot.*`)

None. breadshot is a one-shot CLI with no persistent process to subscribe
to `bread.command.shot.*`. A Lua workflow that wants a screenshot should
shell out:

```lua
bread.exec("breadshot region")
-- or
bread.exec("breadshot region --clipboard-only")
bread.exec("breadshot active-output")
```

The outcome of that exec is the same `bread.shot.captured` event the
keybind path already publishes — `bread.wait("bread.shot.captured")`
inside a spawned coroutine if the workflow needs to react to the file.

There is no `pin`, `select`, `edit`, or other command verb. breadshot has
no editor, no history, and no concept those verbs could hang on.
`bread.exec("breadshot …")` is the whole command surface. If/when a
long-running piece exists, verbs should be added then, not stubbed as
no-ops ahead of it.

## Fail-safe behavior

- If breadd isn't installed or isn't running, `emit` is a silent no-op
  (`BreadClient::emit` never blocks or errors the caller) — breadshot's
  actual grim/slurp/wl-copy path is entirely unaffected either way.
- There is no command subscription, so a breadd restart cannot drop one.
  The next `breadshot` invocation emits (or silently doesn't) on its own
  short-lived connection.
