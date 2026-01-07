use crate::error::{Error, Result};
use crate::types::SearchEvent;
use serde_json::Value;
use std::collections::HashMap;

#[allow(clippy::collapsible_if)]
pub(crate) fn parse_sse_event(json_str: &str) -> Result<SearchEvent> {
    let mut content_json: HashMap<String, Value> =
        serde_json::from_str(json_str).map_err(Error::Json)?;

    let mut answer: Option<String> = None;
    let mut chunks: Vec<Value> = Vec::new();

    if let Some(text_value) = content_json.get("text").cloned() {
        if let Some(text_str) = text_value.as_str() {
            if let Ok(text_parsed) = serde_json::from_str::<Value>(text_str) {
                if let Some(steps) = text_parsed.as_array() {
                    for step in steps {
                        if step.get("step_type").and_then(|v| v.as_str()) == Some("FINAL") {
                            if let Some(content) = step.get("content") {
                                if let Some(answer_str) =
                                    content.get("answer").and_then(|v| v.as_str())
                                {
                                    if let Ok(answer_data) =
                                        serde_json::from_str::<Value>(answer_str)
                                    {
                                        answer = answer_data
                                            .get("answer")
                                            .and_then(|v| v.as_str())
                                            .map(|s| s.to_string());
                                        chunks = answer_data
                                            .get("chunks")
                                            .and_then(|v| v.as_array())
                                            .cloned()
                                            .unwrap_or_default();
                                    }
                                }
                            }
                            break;
                        }
                    }
                }
                content_json.insert("text".to_string(), text_parsed);
            }
        }
    }

    let backend_uuid = content_json
        .get("backend_uuid")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let attachments = content_json
        .get("attachments")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    if answer.is_none() {
        answer = content_json
            .get("answer")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
    }

    if chunks.is_empty() {
        chunks = content_json
            .get("chunks")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
    }

    let mut raw = HashMap::new();
    for (k, v) in content_json {
        if k != "answer" && k != "chunks" && k != "backend_uuid" && k != "attachments" {
            raw.insert(k, v);
        }
    }

    Ok(SearchEvent {
        answer,
        chunks,
        backend_uuid,
        attachments,
        raw,
    })
}
