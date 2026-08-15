//! Long-running command subscription for `bread.command.shot.*`.
//!
//! `breadshot` is still a one-shot CLI by default. `breadshot listen` is the
//! optional persistent process that can honor bus commands. See `EVENTS.md`.

use anyhow::Result;
use bread_utils::bread_client::{BreadClient, BreadEvent};

use crate::capture::{self, CaptureOutcome, Mode, Overrides, APP_ID};
use crate::config::Config;

/// Subscribe to `bread.command.shot.**` and block until the process is killed.
///
/// breadd being absent is not an error: [`BreadClient::subscribe`] reconnects
/// with backoff, and `on_event` simply isn't called until the daemon is up.
pub fn run(config: &Config) -> Result<()> {
    let client = BreadClient::connect(APP_ID);
    if client.health().is_none() {
        tracing::warn!("breadd unreachable; command subscription will connect when it comes back");
    }

    let config = config.clone();
    let _commands = client.subscribe("bread.command.shot.**", move |event| {
        handle_command(&event, &config);
    });

    tracing::info!("listening for bread.command.shot.**");
    loop {
        std::thread::park();
    }
}

/// Reacts to `bread.command.shot.*` verbs. Only `region` and `annotate` are
/// honored — other verbs are ignored, not stubbed as no-ops that pretend
/// to succeed.
///
/// Emits `bread.shot.<verb>.done` / `.failed` per the confirmation
/// convention in bread's Documentation.md.
fn handle_command(event: &BreadEvent, config: &Config) {
    let Some(verb) = command_verb(&event.event) else {
        return;
    };
    match verb {
        "region" => handle_region(config),
        "annotate" => handle_annotate(config),
        other => {
            tracing::debug!("ignoring unrecognized command verb '{other}'");
        }
    }
}

fn handle_region(config: &Config) {
    // Clipboard-only matches the default region *bus* path: a Lua workflow
    // that wants a file on disk can still `bread.exec("breadshot region")`.
    let result = capture::run(
        &Mode::Region,
        config,
        Overrides {
            clipboard_only: true,
            silent: false,
            freeze: false,
            annotate: false,
            output_dir: None,
            filename: None,
        },
    );
    let client = BreadClient::connect(APP_ID);
    match result {
        Ok(_) => client.emit("bread.shot.region.done", region_done_payload()),
        Err(e) => {
            tracing::warn!("bread.command.shot.region failed: {e}");
            client.emit("bread.shot.region.failed", command_failed_payload(&e));
        }
    }
}

fn handle_annotate(config: &Config) {
    // Interactive: satty/swappy get a default save path so the user can
    // write the annotated frame. If the annotator is missing, this falls
    // back to a normal region save (with a warning on stderr).
    let result = capture::run(
        &Mode::Region,
        config,
        Overrides {
            clipboard_only: false,
            silent: false,
            freeze: false,
            annotate: true,
            output_dir: None,
            filename: None,
        },
    );
    let client = BreadClient::connect(APP_ID);
    match result {
        Ok(outcome) => client.emit("bread.shot.annotate.done", annotate_done_payload(&outcome)),
        Err(e) => {
            tracing::warn!("bread.command.shot.annotate failed: {e}");
            client.emit("bread.shot.annotate.failed", command_failed_payload(&e));
        }
    }
}

fn command_verb(event_name: &str) -> Option<&str> {
    event_name.strip_prefix("bread.command.shot.")
}

fn region_done_payload() -> serde_json::Value {
    serde_json::json!({ "clipboard": true, "path": serde_json::Value::Null })
}

fn command_failed_payload(error: &impl ToString) -> serde_json::Value {
    serde_json::json!({ "error": error.to_string() })
}

fn annotate_done_payload(outcome: &CaptureOutcome) -> serde_json::Value {
    serde_json::json!({
        "clipboard": outcome.clipboard,
        "path": outcome.path.as_ref().map(|p| p.to_string_lossy().into_owned()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_verb_strips_shot_prefix() {
        assert_eq!(command_verb("bread.command.shot.region"), Some("region"));
        assert_eq!(
            command_verb("bread.command.shot.annotate"),
            Some("annotate")
        );
        assert_eq!(command_verb("bread.command.shot.window"), Some("window"));
        assert_eq!(command_verb("bread.command.clip.clear"), None);
        assert_eq!(command_verb("bread.shot.captured"), None);
    }

    #[test]
    fn region_done_payload_is_clipboard_only() {
        let v = region_done_payload();
        assert_eq!(v["clipboard"], true);
        assert!(v["path"].is_null());
    }

    #[test]
    fn command_failed_payload_includes_error() {
        let v = command_failed_payload(&"selection cancelled");
        assert_eq!(v["error"], "selection cancelled");
    }

    #[test]
    fn annotate_done_payload_includes_saved_path() {
        let v = annotate_done_payload(&CaptureOutcome {
            clipboard: true,
            path: Some(std::path::PathBuf::from("/tmp/shot.png")),
        });
        assert_eq!(v["clipboard"], true);
        assert_eq!(v["path"], "/tmp/shot.png");
    }

    #[test]
    fn annotate_done_payload_null_path_when_unsaved() {
        let v = annotate_done_payload(&CaptureOutcome {
            clipboard: true,
            path: None,
        });
        assert_eq!(v["clipboard"], true);
        assert!(v["path"].is_null());
    }
}
