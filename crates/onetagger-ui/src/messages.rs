//! Presentation hints for the shared desktop/browser client.
//!
//! Wire actions, status values and existing English diagnostics remain stable.
//! The UI formats complete messages with named arguments in its own locale; the
//! server never changes a process-global locale or formats music/library data.
use std::fmt;
use anyhow::Error;
use serde_json::{json, Value};

/// Errors owned by the UI boundary have stable identities, independently of
/// their English Display text. Never infer a translation key by parsing errors.
#[derive(Debug, Clone, Copy)]
pub(crate) enum UserError {
    TaggingAlreadyActive,
    ReportsRequireAutoTagger,
    SpotifyUnauthorized,
}

impl fmt::Display for UserError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::TaggingAlreadyActive => "A tagging run is already active; wait for it to finish",
            Self::ReportsRequireAutoTagger => "Results reports require AutoTagger",
            Self::SpotifyUnauthorized => "Spotify unauthorized!",
        })
    }
}

impl std::error::Error for UserError {}

/// Only application-owned semantic identifiers can be emitted. Provider/OS
/// errors and paths are ordinary string arguments, never translation source.
pub(crate) enum UiMessage {
    RequestFailed { detail: String },
    TaggingAlreadyActive,
    ReportsRequireAutoTagger,
    SpotifyUnauthorized,
    PlaylistSaved { path: String },
    TagEditorSaveFailed { detail: String },
    ManualTagFailed { detail: String },
    ManualTagApplyFailed { detail: String },
    PlatformInstallFailed { detail: String },
}

impl UiMessage {
    fn parts(self) -> (&'static str, Value) {
        match self {
            Self::RequestFailed { detail } => ("backend.requestFailed", json!({ "detail": detail })),
            Self::TaggingAlreadyActive => ("backend.taggingAlreadyActive", json!({})),
            Self::ReportsRequireAutoTagger => ("backend.reportsRequireAutoTagger", json!({})),
            Self::SpotifyUnauthorized => ("backend.spotifyUnauthorized", json!({})),
            Self::PlaylistSaved { path } => ("backend.playlistSaved", json!({ "path": path })),
            Self::TagEditorSaveFailed { detail } => ("backend.tagEditorSaveFailed", json!({ "detail": detail })),
            Self::ManualTagFailed { detail } => ("backend.manualTagFailed", json!({ "detail": detail })),
            Self::ManualTagApplyFailed { detail } => ("backend.manualTagApplyFailed", json!({ "detail": detail })),
            Self::PlatformInstallFailed { detail } => ("backend.platformInstallFailed", json!({ "detail": detail })),
        }
    }
}

impl From<UserError> for UiMessage {
    fn from(error: UserError) -> Self {
        match error {
            UserError::TaggingAlreadyActive => Self::TaggingAlreadyActive,
            UserError::ReportsRequireAutoTagger => Self::ReportsRequireAutoTagger,
            UserError::SpotifyUnauthorized => Self::SpotifyUnauthorized,
        }
    }
}

/// Add optional presentation fields to an application-created event object.
/// Legacy clients ignore them and keep consuming `message` / `error` unchanged.
pub(crate) fn localized(mut event: Value, message: UiMessage) -> Value {
    let (key, arguments) = message.parts();
    let fields = event.as_object_mut().expect("socket event must be an object");
    fields.insert("messageKey".into(), Value::String(key.into()));
    fields.insert("messageArgs".into(), arguments);
    event
}

pub(crate) fn request_error(error: &Error) -> Value {
    let message = error.downcast_ref::<UserError>().map(|error| UiMessage::from(*error))
        .unwrap_or_else(|| UiMessage::RequestFailed { detail: error.to_string() });
    localized(json!({ "action": "error", "message": error.to_string() }), message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translation_hints_preserve_legacy_fields_and_literal_unicode_arguments() {
        let path = "/Музыка/Ελληνικά/été '<img onerror=alert(1)>.mp3";
        let legacy = json!({ "action": "notify", "message": format!("Playlist saved to: {path}"),
            "status": "ok", "requestId": "request-7", "path": path });
        let mut event = localized(legacy.clone(), UiMessage::PlaylistSaved { path: path.into() });
        assert_eq!(event["messageKey"], "backend.playlistSaved");
        assert_eq!(event["messageArgs"], json!({ "path": path }));
        event.as_object_mut().unwrap().remove("messageKey");
        event.as_object_mut().unwrap().remove("messageArgs");
        assert_eq!(event, legacy, "localization must not change protocol or user data");
    }

    #[test]
    fn typed_errors_localize_without_english_string_matching() {
        let typed: Error = UserError::TaggingAlreadyActive.into();
        let event = request_error(&typed);
        assert_eq!(event["action"], "error");
        assert_eq!(event["message"], "A tagging run is already active; wait for it to finish");
        assert_eq!(event["messageKey"], "backend.taggingAlreadyActive");
        assert_eq!(event["messageArgs"], json!({}));
        // A foreign error can happen to have identical text: it is still detail,
        // not an application-owned error identity or executable message source.
        let foreign = Error::msg(typed.to_string());
        let event = request_error(&foreign);
        assert_eq!(event["messageKey"], "backend.requestFailed");
        assert_eq!(event["messageArgs"]["detail"], foreign.to_string());
    }

    #[test]
    fn typed_error_identity_survives_anyhow_context() {
        let error = Error::new(UserError::SpotifyUnauthorized).context("Audio features could not start");
        let event = request_error(&error);
        assert_eq!(event["message"], "Audio features could not start");
        assert_eq!(event["messageKey"], "backend.spotifyUnauthorized");
    }

    #[test]
    fn backend_message_contract_exists_in_every_shipped_catalog() {
        let catalogs = [
            include_str!("../../../client/src/locales/en/backend.json"),
            include_str!("../../../client/src/locales/it/backend.json"),
            include_str!("../../../client/src/locales/fr/backend.json"),
            include_str!("../../../client/src/locales/nl/backend.json"),
            include_str!("../../../client/src/locales/de/backend.json"),
            include_str!("../../../client/src/locales/es/backend.json"),
            include_str!("../../../client/src/locales/el/backend.json"),
            include_str!("../../../client/src/locales/ru/backend.json"),
        ];
        let messages = [
            UiMessage::RequestFailed { detail: String::new() }, UiMessage::TaggingAlreadyActive,
            UiMessage::ReportsRequireAutoTagger, UiMessage::SpotifyUnauthorized,
            UiMessage::PlaylistSaved { path: String::new() },
            UiMessage::TagEditorSaveFailed { detail: String::new() },
            UiMessage::ManualTagFailed { detail: String::new() },
            UiMessage::ManualTagApplyFailed { detail: String::new() },
            UiMessage::PlatformInstallFailed { detail: String::new() },
        ];
        for message in messages {
            let (key, arguments) = message.parts();
            for catalog in catalogs {
                let catalog: Value = serde_json::from_str(catalog).unwrap();
                let text = catalog[key.strip_prefix("backend.").unwrap()].as_str().unwrap();
                assert!(!text.trim().is_empty(), "empty translation: {key}");
                for argument in arguments.as_object().unwrap().keys() {
                    assert!(text.contains(format!("{{{argument}}}").as_str()), "missing {argument} in {key}");
                }
            }
        }
    }
}
