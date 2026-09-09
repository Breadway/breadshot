# breadshot: bread event integration

breadshot is a standalone Wayland screenshot orchestrator: it works
exactly the same with or without `breadd` running. When breadd *is*
present, a successful capture publishes into the shared bread automation
fabric. See the parent `bread` repo's `Documentation.md`, specifically
its "Namespaces" and "Integrating a bread\* app" sections, for the
general convention this follows.

This is a different job from `bread-screenshots` (the crate in
`bread-ecosystem`): that one is a capture harness for screenshotting
sibling apps in CI. Do not merge the two.

App id: **`shot`**. Transport: `bread-utils`'s `bread_client` module
(feature `bread-client`). One-shot CLI invocations (`breadshot region`,
etc) each `emit` on their own fire-and-forget connection (the same stance
`bread-emit` takes for occasional callers). Command verbs are only
received while `breadshot listen` is running: that process holds the
`bread.command.shot.**` subscription open.

## Events published (`bread.shot.*`)

| Event | Data | When |
|-------|------|------|
| `bread.shot.captured` | `{ "mode": "region" \| "window" \| "output" \| "active-window" \| "active-output", "clipboard": bool, "path": <string or null> }` | A capture completed successfully (grim + clipboard write both returned), whether triggered by the CLI or by `bread.command.shot.region` / `bread.command.shot.annotate`. Not emitted on a cancelled slurp selection, a missing dependency, or a grim/wl-copy failure. `breadshot annotate` publishes `mode: "region"`. |
| `bread.shot.region.done` | `{ "clipboard": true, "path": null }` | `bread.command.shot.region` was received and the region capture succeeded. |
| `bread.shot.region.failed` | `{ "error": "<message>" }` | `bread.command.shot.region` was received but the capture failed (cancelled slurp, missing dependency, grim/wl-copy error). |
| `bread.shot.annotate.done` | `{ "clipboard": true, "path": <string or null> }` | `bread.command.shot.annotate` was received and the region capture (plus optional satty/swappy pass) succeeded. `path` is the saved file, or `null` if the annotator exited without writing it. |
| `bread.shot.annotate.failed` | `{ "error": "<message>" }` | `bread.command.shot.annotate` was received but the capture failed (cancelled slurp, missing grim/slurp, annotator error). Missing satty/swappy is not a failure: breadshot warns and falls back to grim+slurp. |

`mode` is the CLI capture-mode name (`region`, `window`, `output`,
`active-window`, `active-output`), not the `annotate` subcommand.
`clipboard` is whether the PNG was written to the clipboard; both current
capture paths do this (`save_and_copy` and `--clipboard-only`). `path` is
the saved file, or `null` when `--clipboard-only` was used (no file on
disk) or the annotator exited without writing one. The listen-triggered
region path is clipboard-only, so `path` is always `null` on
`bread.shot.region.done`.

The image bytes themselves are never included in the payload. The event
bus is a notification that a capture happened, not a channel for the
screenshot.

## Commands honored (`bread.command.shot.*`)

These are only received while `breadshot listen` is running. Publishing a
command with no subscriber is a silent no-op; that is the documented
bread convention, not a breadshot bug.

| Verb | Data | Effect |
|------|------|--------|
| `region` | none | Same interactive region capture as `breadshot region --clipboard-only`. Emits `bread.shot.region.done`/`.failed`. A successful capture also publishes `bread.shot.captured` the same way the CLI path does. |
| `annotate` | none | Same as `breadshot annotate`: region capture, then freeze the frame in `satty` (preferred) or `swappy` for arrows/text/rect. Emits `bread.shot.annotate.done`/`.failed`. A successful capture also publishes `bread.shot.captured` (`mode: "region"`). If neither annotator is installed, breadshot warns and saves the unannotated region shot. |

```lua
bread.spawn(function()
    bread.emit("bread.command.shot.region")
    bread.wait("bread.shot.region.done", { timeout = 30000 })
end)

bread.spawn(function()
    bread.emit("bread.command.shot.annotate")
    bread.wait("bread.shot.annotate.done", { timeout = 120000 })
end)
```

A workflow that wants a file on disk (not just the clipboard) should
still shell out:

```lua
bread.exec("breadshot region")
bread.exec("breadshot active-output")
bread.exec("breadshot annotate")
```

### Not implemented: extra verbs

There is no `window`, `output`, `active-window`, `active-output`, `pin`,
`select`, or `edit` command verb. The CLI already covers the other
capture modes as synchronous one-shots. Annotation is the `annotate`
verb (a thin satty/swappy hand-off), not a built-in editor; do not
merge this with `bread-screenshots`. If/when another verb is needed,
add it at the same time, not stubbed as a no-op ahead of it.

## Fail-safe behavior

- If breadd isn't installed or isn't running, `emit` is a silent no-op
  (`BreadClient::emit` never blocks or errors the caller) and the
  command subscription simply never receives anything; breadshot's
  actual grim/slurp/wl-copy path is entirely unaffected either way.
- If breadd restarts, the command subscription reconnects automatically
  (`BreadClient::subscribe`'s background thread has its own backoff
  loop); no restart of `breadshot listen` is needed.
- If `breadshot listen` is not running, commands are a graceful no-op at
  the bus (no subscriber). The CLI still works, and one-shot invocations
  still emit `bread.shot.captured` on their own short-lived connection.
