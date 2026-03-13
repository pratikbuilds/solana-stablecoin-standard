use anyhow::{bail, Context, Result};
use serde_json::Value;

pub(crate) fn payload_string(payload: &Value, key: &str) -> Result<String> {
    payload
        .get(key)
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .context(format!("missing string payload field {key}"))
}

pub(crate) fn payload_optional_string(payload: &Value, key: &str) -> Option<String> {
    payload.get(key).and_then(Value::as_str).map(ToString::to_string)
}

pub(crate) fn payload_bool(payload: &Value, key: &str) -> Result<bool> {
    payload
        .get(key)
        .and_then(Value::as_bool)
        .context(format!("missing bool payload field {key}"))
}

pub(crate) fn payload_i64(payload: &Value, key: &str) -> Result<i64> {
    if let Some(value) = payload.get(key).and_then(Value::as_i64) {
        return Ok(value);
    }
    if let Some(value) = payload.get(key).and_then(Value::as_str) {
        return value.parse::<i64>().context(format!("invalid i64 payload field {key}"));
    }
    bail!("missing i64 payload field {key}")
}

pub(crate) fn payload_i128(payload: &Value, key: &str) -> Result<i128> {
    if let Some(value) = payload.get(key).and_then(Value::as_i64) {
        return Ok(value as i128);
    }
    if let Some(value) = payload.get(key).and_then(Value::as_u64) {
        return Ok(value as i128);
    }
    if let Some(value) = payload.get(key).and_then(Value::as_str) {
        return value.parse::<i128>().context(format!("invalid i128 payload field {key}"));
    }
    bail!("missing i128 payload field {key}")
}

pub(crate) fn payload_optional_i128(payload: &Value, key: &str) -> Option<i128> {
    payload_i128(payload, key).ok()
}
