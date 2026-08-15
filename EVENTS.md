# breadshot — bread event integration

breadshot is a standalone Wayland screenshot orchestrator: it works
exactly the same with or without `breadd` running. When breadd *is*
present, a successful capture publishes into the shared bread automation
fabric. See the parent `bread` repo's `Documentation.md` — specifically
its "Namespaces" and "Integrating a bread\* app" sections — for the
general convention this follows.

This is a different job from `bread-screenshots` (the crate in
`bread-ecosystem`): that one is a capture harness for screenshotting
sibling apps in CI. Do not merge the two.

App id: **`shot`**. Transport: `bread-utils`'s `bread_client` module
(feature `bread-client`). One-shot CLI invocations (`breadshot region`,
…) each `emit` on their own fire-and-forget connection (the same stance
`bread-emit` takes for occasional callers). Command verbs are only
received while `breadshot listen` is running — that process holds the
`bread.command.shot.**` subscription open.

## Events published (`bread.shot.*`)

| Event | Data | When |
|-------|------|------|
| `bread.shot.captured` | `{ "mode": "region" \| "window" \| "output" \| "active-window" \| "active-output", "clipboard": bool, "path": <string or null> }` | A capture completed successfully (grim + clipboard write both returned), whether triggered by the CLI or by `bread.command.shot.region`. Not emitted on a cancelled slurp selection, a missing dependency, or a grim/wl-copy failure. |
| `bread.shot.region.done` | `{ "clipboard": true, "path": null }` | `bread.command.shot.region` was received and the region capture succeeded. |
| `bread.shot.region.failed` | `{ "error": "<message>" }` | `bread.command.shot.region` was received but the capture failed (cancelled slurp, missing dependency, grim/wl-copy error). |

`mode` is the CLI mode name (same strings `breadshot <mode>` accepts).
`clipboard` is whether the PNG was written to the clipboard — both current
capture paths do this (`save_and_copy` and `--clipboard-only`). `path` is
the saved file, or `null` when `--clipboard-only` was used (no file on
disk). The listen-triggered region path is clipboard-only, so `path` is
always `null` on `bread.shot.region.done`.

The image bytes themselves are never included in the payload. The event
bus is a notification that a capture happened, not a channel for the
screenshot.

## Commands honored (`bread.command.shot.*`)

These are only received while `breadshot listen` is running. Publishing a
command with no subscriber is a silent no-op — that is the documented
bread convention, not a breadshot bug.

| Verb | Data | Effect |
|------|------|--------|
| `region` | none | Same interactive region capture as `breadshot region --clipboard-only`. Emits `bread.shot.region.done`/`.failed`. A successful capture also publishes `bread.shot.captured` the same way the CLI path does. |

```lua
bread.spawn(function()
    bread.emit("bread.command.shot.region")
    bread.wait("bread.shot.region.done", { timeout = 30000 })
end)
```

A workflow that wants a file on disk (not just the clipboard) should
still shell out:

```lua
bread.exec("breadshot region")
bread.exec("breadshot active-output")
```

### Not implemented: extra verbs

There is no `window`, `output`, `active-window`, `active-output`, `pin`,
`select`, or `edit` command verb. The CLI already covers the other
capture modes as synchronous one-shots, and breadshot has no editor, no
history, and no concept those other verbs could hang on. If/when that
changes, the corresponding `bread.command.shot.*` verb should be added
at the same time, not stubbed as a no-op ahead of it.

## Fail-safe behavior

- If breadd isn't installed or isn't running, `emit` is a silent no-op
  (`BreadClient::emit` never blocks or errors the caller) and the
  command subscription simply never receives anything — breadshot's
  actual grim/slurp/wl-copy path is entirely unaffected either way.
- If breadd restarts, the command subscription reconnects automatically
  (`BreadClient::subscribe`'s background thread has its own backoff
  loop); no restart of `breadshot listen` is needed.
- If `breadshot listen` is not running, commands are a graceful no-op at
  the bus (no subscriber). The CLI still works, and one-shot invocations
  still emit `bread.shot.captured` on their own short-lived connection.
