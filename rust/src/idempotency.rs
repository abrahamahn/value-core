use serde::Serialize;
use serde_json::Value;

use crate::canonical::domain_separated_digest;
use crate::{ValueError, ValueResult, required};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ValueCommand {
    pub command_id: String,
    pub contract_version: String,
    pub payload: Value,
}

fn validate_command(command: &ValueCommand) -> ValueResult<()> {
    required(&command.command_id, "Value command identity is required")?;
    required(
        &command.contract_version,
        "Value command contract version is required",
    )
}

/// Project a command payload onto caller-selected semantic fields.
#[must_use]
pub fn project_value_command_payload(value: &Value, excluded_keys: &[&str]) -> Value {
    match value {
        Value::Array(items) => Value::Array(
            items
                .iter()
                .map(|item| project_value_command_payload(item, excluded_keys))
                .collect(),
        ),
        Value::Object(object) => Value::Object(
            object
                .iter()
                .filter(|(key, _)| !excluded_keys.contains(&key.as_str()))
                .map(|(key, item)| {
                    (
                        key.clone(),
                        project_value_command_payload(item, excluded_keys),
                    )
                })
                .collect(),
        ),
        _ => value.clone(),
    }
}

/// # Errors
/// Returns [`ValueError`] when the command cannot be canonicalized.
pub fn create_value_command_digest(command: &ValueCommand) -> ValueResult<String> {
    validate_command(command)?;
    domain_separated_digest(
        "value-core/command",
        &command.contract_version,
        &serde_json::json!({"commandId": command.command_id, "payload": command.payload}),
    )
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandReplay {
    pub status: &'static str,
    pub digest: String,
}

/// # Errors
/// Returns [`ValueError`] when a command identity is reused with changed intent.
pub fn resolve_value_command_replay(
    existing: &ValueCommand,
    incoming: &ValueCommand,
) -> ValueResult<CommandReplay> {
    validate_command(existing)?;
    validate_command(incoming)?;
    if existing.command_id != incoming.command_id {
        return Err(ValueError::new("Value command identity changed"));
    }
    let existing_digest = create_value_command_digest(existing)?;
    if existing_digest != create_value_command_digest(incoming)? {
        return Err(ValueError::new(
            "Value command identity was reused with changed semantic intent",
        ));
    }
    Ok(CommandReplay {
        status: "replayed",
        digest: existing_digest,
    })
}
