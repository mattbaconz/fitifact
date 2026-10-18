//! `fitifact mcp`: Model Context Protocol tools over stdio.
//!
//! Each line on stdin is one JSON-RPC 2.0 message, and each reply is one line on stdout;
//! nothing else is ever written there. The tools call the same inspect, check, plan, and adapt
//! functions as the CLI, so destination policy stays in the profiles. The only files written
//! are validated outputs beside the original, never over an existing file.

use std::io::{BufRead, Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use fitifact::error::ErrorEnvelope;
use fitifact::inspect::DefaultInspector;
use fitifact::{
    Artifact, ConstraintSet, Error, ErrorCode, Family, ImageAdaptOptions, ImageAdaptPlan,
    ImageFormat, NeverCancelled, PlanOutcome, check, explain_check, explain_plan, inspect, plan,
};
use serde_json::{Map, Value, json};

/// Newest first. A client asking for any other version is offered the newest.
const PROTOCOL_VERSIONS: [&str; 4] = ["2025-11-25", "2025-06-18", "2025-03-26", "2024-11-05"];
const MAX_MESSAGE_BYTES: usize = 4 * 1024 * 1024;
const FIT_TIMEOUT: Duration = Duration::from_secs(1800);

const PARSE_ERROR: i64 = -32700;
const INVALID_REQUEST: i64 = -32600;
const METHOD_NOT_FOUND: i64 = -32601;
const INVALID_PARAMS: i64 = -32602;

const INSTRUCTIONS: &str = "Fitifact makes a local file fit a destination by changing as little as possible. Call list_destinations for a destination id, check to see what fails, then fit to save a validated copy beside the original. Files never leave this computer, and the original is never changed.";

pub fn serve(mut input: impl BufRead, mut output: impl Write) -> std::io::Result<()> {
    let mut line = Vec::new();
    loop {
        line.clear();
        let read = input
            .by_ref()
            .take(MAX_MESSAGE_BYTES as u64 + 1)
            .read_until(b'\n', &mut line)?;
        if read == 0 {
            return Ok(());
        }
        let ended = line.ends_with(b"\n");
        let reply = if line.len() - usize::from(ended) > MAX_MESSAGE_BYTES {
            if !ended {
                input.skip_until(b'\n')?;
            }
            Some(error_reply(
                Value::Null,
                INVALID_REQUEST,
                "the message exceeds 4 MiB",
            ))
        } else {
            let message = line.trim_ascii();
            if message.is_empty() {
                None
            } else {
                handle(message)
            }
        };
        if let Some(reply) = reply {
            serde_json::to_writer(&mut output, &reply)?;
            output.write_all(b"\n")?;
            output.flush()?;
        }
    }
}

fn handle(message: &[u8]) -> Option<Value> {
    let Ok(message) = serde_json::from_slice::<Value>(message) else {
        return Some(error_reply(
            Value::Null,
            PARSE_ERROR,
            "the message is not JSON",
        ));
    };
    let Value::Object(message) = message else {
        return Some(error_reply(
            Value::Null,
            INVALID_REQUEST,
            "expected one JSON-RPC message, not a batch",
        ));
    };
    let id = message.get("id").cloned();
    let id_ok = matches!(id, None | Some(Value::String(_)) | Some(Value::Number(_)));
    if message.get("jsonrpc").and_then(Value::as_str) != Some("2.0") || !id_ok {
        let id = id.filter(|_| id_ok).unwrap_or(Value::Null);
        return Some(error_reply(id, INVALID_REQUEST, "expected JSON-RPC 2.0"));
    }
    // Notifications, and replies to requests this server never sends, get no answer.
    let (Some(method), Some(id)) = (message.get("method").and_then(Value::as_str), id) else {
        return None;
    };
    let null = Value::Null;
    let params = message.get("params").unwrap_or(&null);
    Some(match dispatch(method, params) {
        Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
        Err((code, text)) => error_reply(id, code, &text),
    })
}

fn error_reply(id: Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

fn dispatch(method: &str, params: &Value) -> Result<Value, (i64, String)> {
    match method {
        "initialize" => Ok(initialize(params)),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({ "tools": tools() })),
        "tools/call" => call_tool(params),
        _ => Err((METHOD_NOT_FOUND, format!("method not found: {method}"))),
    }
}

fn initialize(params: &Value) -> Value {
    let version = params
        .get("protocolVersion")
        .and_then(Value::as_str)
        .filter(|version| PROTOCOL_VERSIONS.contains(version))
        .unwrap_or(PROTOCOL_VERSIONS[0]);
    json!({
        "protocolVersion": version,
        "capabilities": { "tools": { "listChanged": false } },
        "serverInfo": {
            "name": "fitifact",
            "title": "Fitifact",
            "version": env!("CARGO_PKG_VERSION"),
        },
        "instructions": INSTRUCTIONS,
    })
}

fn tools() -> Value {
    let path = json!({ "type": "string", "description": "Absolute path of a local file." });
    let destination = json!({
        "type": "string",
        "description": "A destination id from list_destinations, such as discord/video-upload or anthropic/vision.",
    });
    let constraints = json!({
        "type": "object",
        "description": "A fitifact.constraints/v1 document, used instead of destination.",
    });
    let read_only = json!({ "readOnlyHint": true, "openWorldHint": false });
    json!([
        {
            "name": "list_destinations",
            "title": "List destinations",
            "description": "List the destinations Fitifact knows, each with its requirements. Pass id to get one destination's sources and notes.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "One destination id." },
                },
                "additionalProperties": false,
            },
            "annotations": read_only,
        },
        {
            "name": "inspect",
            "title": "Inspect a file",
            "description": "Read a local file's real format, codecs, dimensions, and size, not its extension.",
            "inputSchema": {
                "type": "object",
                "properties": { "path": path },
                "required": ["path"],
                "additionalProperties": false,
            },
            "annotations": read_only,
        },
        {
            "name": "check",
            "title": "Check a file",
            "description": "Check a local file against a destination's requirements and say what fails. Changes nothing. Give destination or constraints.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": path,
                    "destination": destination,
                    "constraints": constraints,
                },
                "required": ["path"],
                "additionalProperties": false,
            },
            "annotations": read_only,
        },
        {
            "name": "fit",
            "title": "Fit a file",
            "description": "Make a local file fit a destination with the smallest change, validate the result against the same requirements, and save it beside the original as name.fitifact.ext. A file that already fits is left alone, and no existing file is ever overwritten. Give destination or constraints; use dry_run to see the plan first.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": path,
                    "destination": destination,
                    "constraints": constraints,
                    "dry_run": { "type": "boolean", "description": "Plan only and write nothing." },
                    "first_frame_consent": {
                        "type": "boolean",
                        "description": "The user agreed to keep only the first frame or page of an animated or multi-page image.",
                    },
                    "crop_consent": {
                        "type": "boolean",
                        "description": "The user agreed to the crop given in crop.",
                    },
                    "crop": {
                        "type": "object",
                        "description": "The part of the image to keep, as fractions of its width and height.",
                        "properties": {
                            "x": { "type": "number", "minimum": 0, "maximum": 1 },
                            "y": { "type": "number", "minimum": 0, "maximum": 1 },
                            "width": { "type": "number", "exclusiveMinimum": 0, "maximum": 1 },
                            "height": { "type": "number", "exclusiveMinimum": 0, "maximum": 1 },
                        },
                        "required": ["x", "y", "width", "height"],
                        "additionalProperties": false,
                    },
                },
                "required": ["path"],
                "additionalProperties": false,
            },
            "annotations": {
                "readOnlyHint": false,
                "destructiveHint": false,
                "idempotentHint": false,
                "openWorldHint": false,
            },
        },
    ])
}

fn call_tool(params: &Value) -> Result<Value, (i64, String)> {
    let Some(name) = params.get("name").and_then(Value::as_str) else {
        return Err((INVALID_PARAMS, "tools/call needs a tool name".into()));
    };
    let empty = Map::new();
    let arguments = match params.get("arguments") {
        None | Some(Value::Null) => &empty,
        Some(Value::Object(arguments)) => arguments,
        Some(_) => return Ok(failure(input_error("arguments must be an object"))),
    };
    let outcome = match name {
        "list_destinations" => list_destinations(arguments),
        "inspect" => inspect_tool(arguments),
        "check" => check_tool(arguments),
        "fit" => fit_tool(arguments),
        _ => return Err((INVALID_PARAMS, format!("unknown tool: {name}"))),
    };
    Ok(match outcome {
        Ok(value) => {
            let failed = value.get("status").and_then(Value::as_str) == Some("failed");
            tool_result(value, failed)
        }
        Err(error) => failure(error),
    })
}

fn tool_result(value: Value, is_error: bool) -> Value {
    let text = value.to_string();
    json!({
        "content": [{ "type": "text", "text": text }],
        "structuredContent": value,
        "isError": is_error,
    })
}

fn failure(error: Error) -> Value {
    let envelope = serde_json::to_value(ErrorEnvelope::from(error)).unwrap_or(Value::Null);
    tool_result(envelope, true)
}

fn input_error(message: impl Into<String>) -> Error {
    Error::new(ErrorCode::InputInvalid, message)
}

fn optional_string(arguments: &Map<String, Value>, key: &str) -> Result<Option<String>, Error> {
    match arguments.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(text)) if !text.trim().is_empty() => Ok(Some(text.clone())),
        Some(_) => Err(input_error(format!("{key} must be a non-empty string"))),
    }
}

fn optional_bool(arguments: &Map<String, Value>, key: &str) -> Result<bool, Error> {
    match arguments.get(key) {
        None | Some(Value::Null) => Ok(false),
        Some(Value::Bool(value)) => Ok(*value),
        Some(_) => Err(input_error(format!("{key} must be true or false"))),
    }
}

fn path_argument(arguments: &Map<String, Value>) -> Result<PathBuf, Error> {
    let path = PathBuf::from(
        optional_string(arguments, "path")?.ok_or_else(|| input_error("path is required"))?,
    );
    if !path.is_absolute() {
        return Err(input_error(
            "path must be absolute, because this server's working directory is not the client's",
        ));
    }
    Ok(path)
}

fn target_argument(arguments: &Map<String, Value>) -> Result<ConstraintSet, Error> {
    match (
        optional_string(arguments, "destination")?,
        arguments.get("constraints"),
    ) {
        (Some(id), None | Some(Value::Null)) => {
            if fitifact::shipped_profile_ids().contains(&id.as_str()) {
                fitifact::compile_shipped_profile(&id)
            } else {
                fitifact::load_profile(&id)
            }
        }
        (None, Some(document @ Value::Object(_))) => {
            fitifact::compile_from_json(&document.to_string())
        }
        _ => Err(input_error(
            "give exactly one of destination (an id from list_destinations) or constraints (a fitifact.constraints/v1 object)",
        )),
    }
}

fn list_destinations(arguments: &Map<String, Value>) -> Result<Value, Error> {
    let registry: Value = serde_json::from_str(&fitifact::profile::shipped_registry_json()?)
        .map_err(|error| input_error(error.to_string()))?;
    let profiles = registry["profiles"].as_array().cloned().unwrap_or_default();
    if let Some(id) = optional_string(arguments, "id")? {
        return profiles
            .into_iter()
            .find(|profile| profile["id"] == id)
            .map(|profile| json!({ "destination": profile }))
            .ok_or_else(|| input_error(format!("no destination has the id {id}")));
    }
    let destinations: Vec<Value> = profiles
        .iter()
        .map(|profile| {
            let requirements: Vec<Value> = profile["rules"]
                .as_array()
                .map(Vec::as_slice)
                .unwrap_or_default()
                .iter()
                .map(|rule| json!({ "field": rule["field"], "op": rule["op"], "value": rule["value"] }))
                .collect();
            json!({
                "id": profile["id"],
                "name": profile["name"],
                "trust": profile["trust"],
                "last_verified": profile["last_verified"],
                "requirements": requirements,
            })
        })
        .collect();
    Ok(json!({ "destinations": destinations }))
}

fn inspect_tool(arguments: &Map<String, Value>) -> Result<Value, Error> {
    let path = path_argument(arguments)?;
    let artifact = inspect(&path, &DefaultInspector::default())?;
    serde_json::to_value(&artifact).map_err(|error| input_error(error.to_string()))
}

fn check_tool(arguments: &Map<String, Value>) -> Result<Value, Error> {
    let path = path_argument(arguments)?;
    let constraints = target_argument(arguments)?;
    let artifact = inspect(&path, &DefaultInspector::default())?;
    let report = check(&artifact, &constraints);
    let explanation = explain_check(&artifact, &report);
    Ok(json!({
        "compatible": report.compatible,
        "summary": explanation.summary,
        "details": explanation.details,
        "report": report,
    }))
}

fn fit_tool(arguments: &Map<String, Value>) -> Result<Value, Error> {
    let path = path_argument(arguments)?;
    let constraints = target_argument(arguments)?;
    let dry_run = optional_bool(arguments, "dry_run")?;
    let crop = match arguments.get("crop") {
        None | Some(Value::Null) => None,
        Some(crop) => Some(serde_json::from_value(crop.clone()).map_err(|error| {
            input_error(format!(
                "crop must be {{x, y, width, height}} as fractions of the image: {error}"
            ))
        })?),
    };
    let options = ImageAdaptOptions {
        crop,
        crop_consent: optional_bool(arguments, "crop_consent")?,
        first_frame_consent: optional_bool(arguments, "first_frame_consent")?,
    };
    let artifact = inspect(&path, &DefaultInspector::default())?;
    if artifact.family == Family::Image {
        fit_image(&path, &constraints, &options, dry_run)
    } else {
        fit_media(&path, &artifact, constraints, dry_run)
    }
}

fn fit_media(
    path: &Path,
    artifact: &Artifact,
    constraints: ConstraintSet,
    dry_run: bool,
) -> Result<Value, Error> {
    let report = check(artifact, &constraints);
    let outcome = plan(
        artifact,
        &constraints,
        &fitifact::ffmpeg::catalog_for(artifact),
    );
    let status = match &outcome {
        PlanOutcome::Compatible { .. } => "compatible",
        PlanOutcome::CannotSatisfy { .. } => "cannot_satisfy",
        PlanOutcome::Planned { .. } if dry_run => "planned",
        PlanOutcome::Planned { .. } => {
            let result = crate::adapt_file(path, constraints, None, FIT_TIMEOUT)?;
            return Ok(json!({
                "status": result.status,
                "output": result.output,
                "summary": result.explanation.summary,
                "details": result.explanation.details,
                "result": result,
            }));
        }
    };
    let explanation = explain_plan(artifact, &report, &outcome);
    Ok(json!({
        "status": status,
        "output": null,
        "summary": explanation.summary,
        "details": explanation.details,
        "report": report,
        "plan": outcome,
    }))
}

/// Still images take the same in-process path as the web app: resize, re-encode, and byte
/// fitting, each validated before anything is written.
fn fit_image(
    path: &Path,
    constraints: &ConstraintSet,
    options: &ImageAdaptOptions,
    dry_run: bool,
) -> Result<Value, Error> {
    let bytes = std::fs::read(path)
        .map_err(|error| input_error(format!("cannot read the image: {error}")))?;
    let source = fitifact::image_artifact_from_bytes(Some(path), &bytes)?;
    let report = check(&source, constraints);
    let plan = match fitifact::plan_image_adaptation(&source, constraints) {
        Ok(plan) => plan,
        Err(error) if error.code == ErrorCode::NoValidPlan => {
            return Ok(json!({
                "status": "cannot_satisfy",
                "output": null,
                "summary": "Cannot satisfy these requirements.",
                "details": [error.message],
                "report": report,
            }));
        }
        Err(error) => return Err(error),
    };
    if plan.noop {
        return Ok(json!({
            "status": "compatible",
            "output": null,
            "summary": "Already compatible. Nothing changed.",
            "details": [],
            "report": report,
            "plan": plan,
        }));
    }
    let consent = missing_consent(&plan, options);
    if dry_run || !consent.is_empty() {
        let mut details = vec![format!("Target: {}.", describe_target(&plan))];
        details.extend(plan.warnings.iter().cloned());
        details.extend(consent.iter().cloned());
        return Ok(json!({
            "status": if consent.is_empty() { "planned" } else { "consent_required" },
            "output": null,
            "summary": if consent.is_empty() {
                "Fitifact can adapt this image. Nothing has been written."
            } else {
                "The user must approve a change before Fitifact adapts this image."
            },
            "details": details,
            "report": report,
            "plan": plan,
        }));
    }
    let execution =
        fitifact::execute_image_adaptation(&bytes, constraints, &plan, options, &NeverCancelled)?;
    let Some(output) = execution.output.as_deref() else {
        return Err(Error::new(
            ErrorCode::ValidationFailed,
            "the adapted image was not produced",
        ));
    };
    let written = fitifact::write_sibling_output(path, extension(&plan.target.format), output)?;
    Ok(json!({
        "status": "adapted",
        "output": written,
        "summary": format!("Adapted and verified: {}.", describe(&execution.output_artifact)),
        "details": execution.disclosures,
        "report": execution.report,
        "plan": plan,
        "output_artifact": execution.output_artifact,
    }))
}

fn missing_consent(plan: &ImageAdaptPlan, options: &ImageAdaptOptions) -> Vec<String> {
    let mut missing = Vec::new();
    let crop = &plan.target.crop;
    if crop.required && (!options.crop_consent || options.crop.is_none()) {
        missing.push(format!(
            "This destination needs a {}:{} crop. Once the user approves one, call fit again with crop and crop_consent true.",
            crop.target_aspect_width, crop.target_aspect_height
        ));
    }
    if plan.target.first_frame.required && !options.first_frame_consent {
        missing.push(
            "Only the first frame or page can be kept. Once the user agrees, call fit again with first_frame_consent true."
                .into(),
        );
    }
    missing
}

fn describe_target(plan: &ImageAdaptPlan) -> String {
    let target = &plan.target;
    let size = if target.proportional_reduction_allowed {
        format!("up to {}×{}", target.width, target.height)
    } else {
        format!("{}×{}", target.width, target.height)
    };
    match target.max_bytes {
        Some(max) => format!(
            "{}, {size}, at most {max} bytes",
            target.format.display_label()
        ),
        None => format!("{}, {size}", target.format.display_label()),
    }
}

fn describe(artifact: &Artifact) -> String {
    let facts = artifact.image.as_ref();
    let format = facts
        .and_then(|facts| facts.format.as_ref())
        .map(ImageFormat::display_label)
        .unwrap_or_else(|| "image".into());
    match facts.and_then(|facts| facts.width.zip(facts.height)) {
        Some((width, height)) => {
            format!("{format}, {width}×{height}, {} bytes", artifact.byte_length)
        }
        None => format!("{format}, {} bytes", artifact.byte_length),
    }
}

fn extension(format: &ImageFormat) -> &str {
    match format {
        ImageFormat::Jpeg => "jpg",
        other => other.as_str(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exchange(input: &[u8]) -> Vec<Value> {
        let mut output = Vec::new();
        serve(input, &mut output).unwrap();
        String::from_utf8(output)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect()
    }

    fn lines(messages: &[Value]) -> Vec<u8> {
        messages
            .iter()
            .flat_map(|message| format!("{message}\n").into_bytes())
            .collect()
    }

    fn call(id: u64, name: &str, arguments: Value) -> Value {
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "tools/call",
            "params": { "name": name, "arguments": arguments },
        })
    }

    fn test_dir(label: &str) -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "fitifact-mcp-{label}-{}-{unique}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn initialize_negotiates_and_notifications_get_no_reply() {
        let replies = exchange(&lines(&[
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2025-06-18",
                    "capabilities": {},
                    "clientInfo": { "name": "test", "version": "0" },
                },
            }),
            json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }),
            json!({ "jsonrpc": "2.0", "id": "p", "method": "ping" }),
            json!({
                "jsonrpc": "2.0",
                "id": 2,
                "method": "initialize",
                "params": { "protocolVersion": "1999-01-01" },
            }),
        ]));
        assert_eq!(replies.len(), 3);
        assert_eq!(replies[0]["id"], 1);
        assert_eq!(replies[0]["result"]["protocolVersion"], "2025-06-18");
        assert_eq!(
            replies[0]["result"]["capabilities"],
            json!({ "tools": { "listChanged": false } })
        );
        assert_eq!(
            replies[1],
            json!({ "jsonrpc": "2.0", "id": "p", "result": {} })
        );
        assert_eq!(
            replies[2]["result"]["protocolVersion"],
            PROTOCOL_VERSIONS[0]
        );
    }

    #[test]
    fn malformed_and_unknown_requests_get_json_rpc_errors() {
        let mut input = b"not json\n[1]\n{\"id\":9,\"method\":\"ping\"}\n".to_vec();
        input.extend(lines(&[
            json!({ "jsonrpc": "2.0", "id": 3, "method": "resources/list" }),
            call(4, "delete_everything", json!({})),
        ]));
        let codes: Vec<Value> = exchange(&input)
            .iter()
            .map(|reply| reply["error"]["code"].clone())
            .collect();
        assert_eq!(
            codes,
            [
                PARSE_ERROR,
                INVALID_REQUEST,
                INVALID_REQUEST,
                METHOD_NOT_FOUND,
                INVALID_PARAMS
            ]
        );
    }

    #[test]
    fn an_oversized_message_is_refused_and_the_stream_continues() {
        let mut input = vec![b'x'; MAX_MESSAGE_BYTES + 10];
        input.extend(b"\n{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}\n");
        let replies = exchange(&input);
        assert_eq!(replies.len(), 2);
        assert_eq!(replies[0]["error"]["code"], INVALID_REQUEST);
        assert_eq!(replies[1]["result"], json!({}));
    }

    #[test]
    fn tools_list_names_the_four_tools() {
        let replies = exchange(&lines(&[json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/list",
        })]));
        let names: Vec<&str> = replies[0]["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|tool| tool["name"].as_str().unwrap())
            .collect();
        assert_eq!(names, ["list_destinations", "inspect", "check", "fit"]);
    }

    #[test]
    fn list_destinations_includes_ai_profiles_with_their_sources() {
        let replies = exchange(&lines(&[
            call(1, "list_destinations", json!({})),
            call(2, "list_destinations", json!({ "id": "anthropic/vision" })),
            call(3, "list_destinations", json!({ "id": "nowhere/else" })),
        ]));
        let list = replies[0]["result"]["structuredContent"]["destinations"]
            .as_array()
            .unwrap();
        assert!(list.iter().any(|item| item["id"] == "openai/transcription"));
        let vision = &replies[1]["result"];
        assert_eq!(vision["isError"], false);
        assert_eq!(
            vision["structuredContent"]["destination"]["sources"][0]["url"],
            "https://platform.claude.com/docs/en/build-with-claude/vision"
        );
        assert_eq!(replies[2]["result"]["isError"], true);
        assert_eq!(
            replies[2]["result"]["structuredContent"]["code"],
            "INPUT_INVALID"
        );
    }

    #[test]
    fn bad_arguments_are_tool_errors_the_model_can_read() {
        let replies = exchange(&lines(&[
            call(
                1,
                "check",
                json!({ "path": "shot.png", "destination": "anthropic/vision" }),
            ),
            call(
                2,
                "check",
                json!({
                    "path": std::env::temp_dir().join("shot.png"),
                    "destination": "anthropic/vision",
                    "constraints": { "schema": "fitifact.constraints/v1", "hard": [] },
                }),
            ),
        ]));
        for reply in &replies {
            assert_eq!(reply["result"]["isError"], true);
            assert_eq!(
                reply["result"]["structuredContent"]["code"],
                "INPUT_INVALID"
            );
        }
        assert!(
            replies[0]["result"]["content"][0]["text"]
                .as_str()
                .unwrap()
                .contains("absolute")
        );
    }

    #[test]
    fn fit_plans_then_writes_a_validated_sibling_and_keeps_the_original() {
        let dir = test_dir("fit");
        let input = dir.join("shot.png");
        let original = fitifact::sample_png_rgb(64, 48);
        std::fs::write(&input, &original).unwrap();
        let jpeg = json!({
            "schema": "fitifact.constraints/v1",
            "hard": [{ "id": "format", "field": "image.format", "op": "in", "value": ["jpeg"] }],
        });

        let planned = exchange(&lines(&[call(
            1,
            "fit",
            json!({ "path": input, "constraints": jpeg, "dry_run": true }),
        )]));
        assert_eq!(
            planned[0]["result"]["structuredContent"]["status"],
            "planned"
        );
        assert!(!dir.join("shot.fitifact.jpg").exists());

        let fitted = exchange(&lines(&[
            call(2, "fit", json!({ "path": input, "constraints": jpeg })),
            call(3, "fit", json!({ "path": input, "constraints": jpeg })),
        ]));
        let first = &fitted[0]["result"]["structuredContent"];
        assert_eq!(first["status"], "adapted");
        assert_eq!(first["report"]["compatible"], true);
        assert_eq!(
            first["output"],
            dir.join("shot.fitifact.jpg").to_str().unwrap()
        );
        assert_eq!(
            fitted[1]["result"]["structuredContent"]["output"],
            dir.join("shot.fitifact.2.jpg").to_str().unwrap()
        );
        assert_eq!(std::fs::read(&input).unwrap(), original);
        let written = std::fs::read(dir.join("shot.fitifact.jpg")).unwrap();
        assert!(written.starts_with(&[0xFF, 0xD8]));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn fit_leaves_a_file_that_already_fits_alone() {
        let dir = test_dir("noop");
        let input = dir.join("photo.jpg");
        std::fs::write(&input, fitifact::sample_jpeg_rgb(32, 32)).unwrap();
        let replies = exchange(&lines(&[call(
            1,
            "fit",
            json!({ "path": input, "destination": "anthropic/vision" }),
        )]));
        let result = &replies[0]["result"]["structuredContent"];
        assert_eq!(result["status"], "compatible");
        assert_eq!(result["output"], Value::Null);
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
