use crate::error::{Error, Result};
use crate::types::{SearchEvent, SearchWebResult};
use serde_json::{Map, Value};
use std::collections::HashMap;

/// Keys that are extracted from the raw JSON and stored in dedicated fields.
const EXTRACTED_KEYS: &[&str] = &["answer", "backend_uuid", "attachments"];

/// Parses an SSE event JSON string into a SearchEvent.
pub(crate) fn parse_sse_event(json_str: &str) -> Result<SearchEvent> {
    let mut content: Map<String, Value> =
        serde_json::from_str(json_str).map_err(Error::Json)?;

    // Try to parse the "text" field if it contains nested JSON
    parse_nested_text_field(&mut content);

    // Extract answer and web_results from the FINAL step or fall back to top-level
    let (answer, web_results) = extract_answer_and_web_results(&content);

    // Extract other known fields
    let backend_uuid = extract_string(&content, "backend_uuid");
    let attachments = extract_string_array(&content, "attachments");

    // Build raw map excluding extracted keys
    let raw = build_raw_map(content);

    Ok(SearchEvent { answer, web_results, backend_uuid, attachments, raw })
}

/// If the "text" field is a JSON string, parse it and replace the field with the parsed value.
fn parse_nested_text_field(content: &mut Map<String, Value>) {
    let Some(text_value) = content.get("text") else {
        return;
    };

    let Some(text_str) = text_value.as_str() else {
        return;
    };

    if let Ok(parsed) = serde_json::from_str::<Value>(text_str) {
        content.insert("text".to_string(), parsed);
    }
}

/// Extracts answer and web_results from the event content.
///
/// First tries to find them in a FINAL step within the "text" field,
/// then falls back to top-level "answer" field with empty web_results.
fn extract_answer_and_web_results(
    content: &Map<String, Value>,
) -> (Option<String>, Vec<SearchWebResult>) {
    // Try to extract from FINAL step in text field
    if let Some((answer, web_results)) = extract_from_final_step(content) {
        return (answer, web_results);
    }

    // Fall back to top-level answer field (no web_results available at top level)
    let answer = extract_string(content, "answer");

    (answer, Vec::new())
}

/// Extracts answer and web_results from a FINAL step in the text field.
fn extract_from_final_step(
    content: &Map<String, Value>,
) -> Option<(Option<String>, Vec<SearchWebResult>)> {
    let text = content.get("text")?;
    let steps = text.as_array()?;

    let final_step = steps
        .iter()
        .find(|step| step.get("step_type").and_then(|v| v.as_str()) == Some("FINAL"))?;

    let step_content = final_step.get("content")?;
    let answer_str = step_content.get("answer")?.as_str()?;

    let answer_data: Value = serde_json::from_str(answer_str).ok()?;

    let answer = answer_data.get("answer").and_then(|v| v.as_str()).map(|s| s.to_string());
    let web_results = answer_data
        .get("web_results")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|v| extract_web_result(&v))
        .collect();

    Some((answer, web_results))
}

fn extract_web_result(value: &Value) -> Option<SearchWebResult> {
    let name = value.get("name").and_then(|v| v.as_str()).map(|s| s.to_string())?;
    let url = value.get("url").and_then(|v| v.as_str()).map(|s| s.to_string())?;
    let snippet = value.get("snippet").and_then(|v| v.as_str()).map(|s| s.to_string())?;
    Some(SearchWebResult { name, url, snippet })
}

/// Extracts a string value from the content map.
fn extract_string(content: &Map<String, Value>, key: &str) -> Option<String> {
    content.get(key).and_then(|v| v.as_str()).map(|s| s.to_string())
}

/// Extracts an array of strings from the content map.
fn extract_string_array(content: &Map<String, Value>, key: &str) -> Vec<String> {
    content
        .get(key)
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
        .unwrap_or_default()
}

/// Builds the raw map by excluding extracted keys.
fn build_raw_map(content: Map<String, Value>) -> HashMap<String, Value> {
    content.into_iter().filter(|(k, _)| !EXTRACTED_KEYS.contains(&k.as_str())).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_event() {
        let json = r#"{"answer": "Hello world"}"#;
        let event = parse_sse_event(json).unwrap();

        assert_eq!(event.answer, Some("Hello world".to_string()));
        assert!(event.web_results.is_empty());
        assert!(event.backend_uuid.is_none());
        assert!(event.attachments.is_empty());
    }

    #[test]
    fn test_parse_event_with_backend_uuid() {
        let json = r#"{"answer": "Test", "backend_uuid": "abc-123"}"#;
        let event = parse_sse_event(json).unwrap();

        assert_eq!(event.answer, Some("Test".to_string()));
        assert_eq!(event.backend_uuid, Some("abc-123".to_string()));
    }

    #[test]
    fn test_parse_event_with_attachments() {
        let json = r#"{"answer": "Test", "attachments": ["url1", "url2"]}"#;
        let event = parse_sse_event(json).unwrap();

        assert_eq!(event.attachments, vec!["url1", "url2"]);
    }

    #[test]
    fn test_parse_event_with_nested_text_json() {
        // Simulates the "text" field containing JSON string with steps
        let inner_answer = r#"{"answer": "Nested answer", "web_results": [{"name": "Source", "url": "https://example.com", "snippet": "Example"}]}"#;
        let text_content = serde_json::json!([
            {
                "step_type": "SEARCH",
                "content": {}
            },
            {
                "step_type": "FINAL",
                "content": {
                    "answer": inner_answer
                }
            }
        ]);
        let text_str = serde_json::to_string(&text_content).unwrap();

        let json = serde_json::json!({
            "text": text_str,
            "some_field": "value"
        });

        let event = parse_sse_event(&json.to_string()).unwrap();

        assert_eq!(event.answer, Some("Nested answer".to_string()));
        assert_eq!(event.web_results.len(), 1);
        assert_eq!(event.web_results[0].name, "Source");
        assert_eq!(event.web_results[0].url, "https://example.com");
        assert_eq!(event.web_results[0].snippet, "Example");
        // The "text" field should be parsed and stored in raw
        assert!(event.raw.contains_key("text"));
        assert!(event.raw.contains_key("some_field"));
    }

    #[test]
    fn test_parse_event_fallback_to_top_level() {
        // When text doesn't contain FINAL step, fall back to top-level
        let text_content = serde_json::json!([
            {
                "step_type": "SEARCH",
                "content": {}
            }
        ]);
        let text_str = serde_json::to_string(&text_content).unwrap();

        let json = serde_json::json!({
            "text": text_str,
            "answer": "Top level answer"
        });

        let event = parse_sse_event(&json.to_string()).unwrap();

        assert_eq!(event.answer, Some("Top level answer".to_string()));
        assert!(event.web_results.is_empty());
    }

    #[test]
    fn test_parse_event_raw_excludes_extracted_keys() {
        let json = r#"{
            "answer": "Test",
            "backend_uuid": "uuid",
            "attachments": [],
            "extra_field": "should be in raw",
            "another": 123
        }"#;
        let event = parse_sse_event(json).unwrap();

        // Extracted keys should not be in raw
        assert!(!event.raw.contains_key("answer"));
        assert!(!event.raw.contains_key("backend_uuid"));
        assert!(!event.raw.contains_key("attachments"));

        // Other fields should be in raw
        assert!(event.raw.contains_key("extra_field"));
        assert!(event.raw.contains_key("another"));
    }

    #[test]
    fn test_parse_event_empty_fields() {
        let json = r#"{}"#;
        let event = parse_sse_event(json).unwrap();

        assert!(event.answer.is_none());
        assert!(event.web_results.is_empty());
        assert!(event.backend_uuid.is_none());
        assert!(event.attachments.is_empty());
    }

    #[test]
    fn test_parse_invalid_json() {
        let result = parse_sse_event("not json");
        assert!(result.is_err());
    }
}
