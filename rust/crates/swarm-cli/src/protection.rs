//! Offline PAEA contract utility. No Root, configuration, network or effect IO.
use crate::{write_output, Arguments, CliError};
use std::{
    io::{self, Write},
    path::Path,
};
use swarm_protection_contract::{
    canonicalize, check_receipt_documents, check_receipt_transition, compare_documents, inspect_document,
    decode_document, decode_receipt, resolve_payment_documents, State, SCHEMA_VERSION,
};

/// Read bounded documents through the existing CLI input mechanism and emit
/// canonical bytes/digests, exact action inspection, a comparison, payment resolution, proposed receipt transition or exact-input report. All
/// supplied context/receipt/state/sink data remain untrusted, explicitly marked
/// in output. Reject incompatible runtime/root/server/output options; no Root is opened,
/// no credential is read and no policy is saved. Input/parse/encode/output/flush
/// errors propagate with their causes and cannot imply a protected effect.
pub(super) fn execute(a: &Arguments, out: &mut dyn Write) -> Result<u8, CliError> {
    if a.root != "."
        || a.server
        || !a.output.is_empty()
        || !a.task.is_empty()
        || a.phase != "delivery"
    {
        return Err(CliError::Usage);
    }
    if a.pos.len() == 6
        && matches!(a.pos[1].as_str(), "check-receipt" | "receipt-report")
        && a.input.is_empty()
    {
        return check_receipt(a, out);
    }
    if a.pos.len() == 4 && a.pos[1] == "resolve-payment" && a.input.is_empty() {
        return resolve_payment(a, out);
    }
    if a.pos.len() == 4 && a.pos[1] == "compare" && a.input.is_empty() {
        // Two named files avoid ambiguous ownership/framing of a shared stdin.
        // No project is opened; the existing regular-file/transport rules apply.
        if a.pos[2] == "-" || a.pos[3] == "-" {
            return Err(CliError::Usage);
        }
        let before = swarm_engine::read_command_file(Path::new(&a.pos[2]))
            .map_err(CliError::CommandInput)?;
        let after = swarm_engine::read_command_file(Path::new(&a.pos[3]))
            .map_err(CliError::CommandInput)?;
        let comparison = compare_documents(&before, &after).map_err(CliError::Protection)?;
        let value = serde_json::json!({
            "schema_version": SCHEMA_VERSION, "scope": "offline-data-only", "comparison": comparison
        });
        let bytes = serde_json::to_vec(&value)
            .map_err(|e| CliError::Protection(swarm_protection_contract::Error::Json(e)))?;
        write_output(out, &bytes)?;
        return Ok(0);
    }
    if a.pos.len() != 2 || !matches!(a.pos[1].as_str(), "canonicalize" | "inspect") || a.input.is_empty() {
        return Err(CliError::Usage);
    }
    let raw = (if a.input == "-" {
        swarm_engine::read_command_document(&mut io::stdin().lock())
    } else {
        swarm_engine::read_command_file(Path::new(&a.input))
    })
    .map_err(CliError::CommandInput)?;
    if a.pos[1] == "inspect" {
        let report = inspect_document(&raw).map_err(CliError::Protection)?;
        let bytes = serde_json::to_vec(&report)
            .map_err(|e| CliError::Protection(swarm_protection_contract::Error::Json(e)))?;
        write_output(out, &bytes)?;
        return Ok(0);
    }
    let document = decode_document(&raw).map_err(CliError::Protection)?;
    let action =
        canonicalize(&document.proposal, &document.context).map_err(CliError::Protection)?;
    let canonical = std::str::from_utf8(action.bytes())
        .map_err(|e| CliError::Input(io::Error::new(io::ErrorKind::InvalidData, e)))?;
    let value = serde_json::json!({
        "schema_version":SCHEMA_VERSION, "canonicalization":"paea-json-v1", "canonical":canonical,
        "action_digest":action.digest(), "intent_id":action.intent_id(),
        "authority":"not-established", "scope":"offline-data-only"
    });
    let bytes = serde_json::to_vec(&value)
        .map_err(|e| CliError::Protection(swarm_protection_contract::Error::Json(e)))?;
    write_output(out, &bytes)?;
    Ok(0)
}

/// Read registry/request from two named regular files after the common runtime
/// option gate. Stdin is excluded to avoid ambiguous framing. All strict parsing
/// and resolution precede output; reads/serialization/write/flush retain causes.
/// Sequential reads establish no atomic snapshot or freshness. No Root, protected
/// registry or effect is opened; partial output after IO failure grants no rights.
fn resolve_payment(a: &Arguments, out: &mut dyn Write) -> Result<u8, CliError> {
    if a.pos[2] == "-" || a.pos[3] == "-" {
        return Err(CliError::Usage);
    }
    let registry =
        swarm_engine::read_command_file(Path::new(&a.pos[2])).map_err(CliError::CommandInput)?;
    let request =
        swarm_engine::read_command_file(Path::new(&a.pos[3])).map_err(CliError::CommandInput)?;
    let report = resolve_payment_documents(&registry, &request).map_err(CliError::Protection)?;
    let bytes = serde_json::to_vec(&report)
        .map_err(|e| CliError::Protection(swarm_protection_contract::Error::Json(e)))?;
    write_output(out, &bytes)?;
    Ok(0)
}

/// Consume two named regular files and explicit expected sink/prior-state data
/// through the existing input and pure receipt checker contracts. Invalid framing
/// or state refuses before file reads; stdin is deliberately excluded. A valid
/// State label is case-sensitive and parsed with the shared serde representation.
/// The caller must first check six positional arguments and incompatible runtime
/// options, including an empty --input; this private helper relies on that dispatch.
/// Decode/validation/canonicalization/serialization/output/flush errors retain
/// typed causes. All JSON is prepared before writing; failed write/flush may have
/// exposed partial or complete output, never a project mutation or protected effect.
/// Sequential file reads provide no atomic snapshot, freshness or authenticity.
/// A successful report proposes an untrusted state only and grants no authority.
/// receipt-report emits explicit schema v2 with raw input hashes; check-receipt
/// keeps its existing nine-field schema-v1 output without additional properties.
fn check_receipt(a: &Arguments, out: &mut dyn Write) -> Result<u8, CliError> {
    if a.pos[2] == "-" || a.pos[3] == "-" {
        return Err(CliError::Usage);
    }
    let previous: State = serde_json::from_value(serde_json::Value::String(a.pos[5].clone()))
        .map_err(|e| CliError::Protection(swarm_protection_contract::Error::Json(e)))?;
    let raw_action =
        swarm_engine::read_command_file(Path::new(&a.pos[2])).map_err(CliError::CommandInput)?;
    let raw_receipt =
        swarm_engine::read_command_file(Path::new(&a.pos[3])).map_err(CliError::CommandInput)?;
    if a.pos[1] == "receipt-report" {
        let report = check_receipt_documents(&raw_action, &raw_receipt, &a.pos[4], previous)
            .map_err(CliError::Protection)?;
        let bytes = serde_json::to_vec(&report)
            .map_err(|e| CliError::Protection(swarm_protection_contract::Error::Json(e)))?;
        write_output(out, &bytes)?;
        return Ok(0);
    }
    let document = decode_document(&raw_action).map_err(CliError::Protection)?;
    let action =
        canonicalize(&document.proposal, &document.context).map_err(CliError::Protection)?;
    let receipt = decode_receipt(&raw_receipt).map_err(CliError::Protection)?;
    let proposed = check_receipt_transition(&action, &receipt, &a.pos[4], previous)
        .map_err(CliError::Protection)?;
    let value = serde_json::json!({
        "schema_version": SCHEMA_VERSION, "scope": "offline-data-only",
        "action_digest": action.digest(), "intent_id": action.intent_id(),
        "previous_state": previous, "proposed_state": proposed,
        "authority": "not-established", "receipt_authenticity": "not-established",
        "effect": "not-established"
    });
    let bytes = serde_json::to_vec(&value)
        .map_err(|e| CliError::Protection(swarm_protection_contract::Error::Json(e)))?;
    write_output(out, &bytes)?;
    Ok(0)
}
