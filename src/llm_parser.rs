//! LLM output parser — parse, repair, and fallback for future LLM responses.
//!
//! Real models may not follow the expected JSON contract.  This module
//! provides deterministic local-only parsing that never makes network
//! calls, never executes code, and never trusts upstream output blindly.

use crate::llm::LlmResponseContract;

/// Outcome of parsing an LLM response.
#[derive(Debug, Clone)]
pub enum LlmParseOutcome {
    /// Strict valid JSON matching the contract.
    Parsed(LlmResponseContract),
    /// JSON was repaired (e.g. missing warnings, wrong key names, fenced).
    Repaired {
        response: LlmResponseContract,
        warnings: Vec<String>,
    },
    /// Bare text accepted as best-effort compiled_prompt with warnings.
    BestEffort {
        compiled_prompt: String,
        warnings: Vec<String>,
    },
    /// Could not parse; fallback to a safe local prompt.
    Fallback {
        compiled_prompt: String,
        warnings: Vec<String>,
    },
}

/// Parse a raw LLM response string into a safe outcome.
///
/// - Never makes network calls
/// - Never executes code or commands
/// - Never exposes secrets in warnings
/// - Falls back to a safe local prompt on failure
pub fn parse_llm_response(raw: &str, original_prompt: &str) -> LlmParseOutcome {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return fallback("LLM response was empty", original_prompt);
    }

    // 1. Try strict JSON
    if let Some(outcome) = try_strict_json(trimmed, original_prompt) {
        return outcome;
    }

    // 2. Try fenced JSON (```json ... ```)
    if let Some(outcome) = try_fenced_json(trimmed, original_prompt) {
        return outcome;
    }

    // 3. Try first JSON object in prose
    if let Some(outcome) = try_first_json_object(trimmed, original_prompt) {
        return outcome;
    }

    // 4. Try bare text as best-effort compiled_prompt
    if let Some(outcome) = try_bare_text(trimmed) {
        return outcome;
    }

    // 5. Fallback
    fallback(
        "LLM response could not be parsed as valid output",
        original_prompt,
    )
}

fn fallback(reason: &str, original_prompt: &str) -> LlmParseOutcome {
    LlmParseOutcome::Fallback {
        compiled_prompt: format!(
            "Use the original user prompt as the compiled prompt. Preserve stated intent and constraints. Do not invent missing context.\n\nOriginal prompt:\n{}",
            original_prompt
        ),
        warnings: vec![reason.to_string()],
    }
}

fn try_strict_json(text: &str, original_prompt: &str) -> Option<LlmParseOutcome> {
    let parsed: Result<LlmResponseContract, _> = serde_json::from_str(text);
    match parsed {
        Ok(resp) => {
            if resp.compiled_prompt.trim().is_empty() {
                return Some(fallback(
                    "LLM response had empty compiled_prompt after repair",
                    original_prompt,
                ));
            }
            if looks_like_meta_commentary(&resp.compiled_prompt) {
                return Some(LlmParseOutcome::Repaired {
                    response: resp,
                    warnings: vec!["LLM response appears to contain model meta-commentary".into()],
                });
            }
            Some(LlmParseOutcome::Parsed(resp))
        }
        Err(_) => None,
    }
}

fn try_fenced_json(text: &str, original_prompt: &str) -> Option<LlmParseOutcome> {
    let lower = text.to_lowercase();
    let fence = "```json";
    if let Some(start) = lower.find(fence) {
        let after = &text[start + fence.len()..];
        if let Some(end) = after.find("```") {
            let inner = after[..end].trim();
            return try_strict_json(inner, original_prompt);
        }
    }
    // Try plain ``` fence
    if let Some(start) = lower.find("```") {
        let after = &text[start + 3..];
        // Skip optional language label like "json"
        let after = after.trim_start_matches(|c: char| c.is_alphanumeric());
        if let Some(end) = after.find("```") {
            let inner = after[..end].trim();
            return try_strict_json(inner, original_prompt);
        }
    }
    None
}

fn try_first_json_object(text: &str, original_prompt: &str) -> Option<LlmParseOutcome> {
    if let Some(start) = text.find('{') {
        let mut depth = 0i32;
        let mut in_string = false;
        let mut escaped = false;
        let mut end = None;
        for (i, ch) in text[start..].char_indices() {
            if escaped {
                escaped = false;
                continue;
            }
            match ch {
                '\\' if in_string => escaped = true,
                '"' => in_string = !in_string,
                '{' if !in_string => depth += 1,
                '}' if !in_string => {
                    depth -= 1;
                    if depth == 0 {
                        end = Some(start + i + 1);
                        break;
                    }
                }
                _ => {}
            }
        }
        if let Some(e) = end {
            let inner = text[start..e].trim();
            return try_repair_json(inner, original_prompt);
        }
    }
    None
}

fn try_repair_json(json_str: &str, original_prompt: &str) -> Option<LlmParseOutcome> {
    // Try as-is first
    if let Ok(resp) = serde_json::from_str::<LlmResponseContract>(json_str) {
        if resp.compiled_prompt.trim().is_empty() {
            return Some(fallback(
                "LLM response had empty compiled_prompt",
                original_prompt,
            ));
        }
        return Some(LlmParseOutcome::Parsed(resp));
    }

    // Try generic JSON value and extract fields manually
    let val: serde_json::Value = serde_json::from_str(json_str).ok()?;
    let obj = val.as_object()?;

    let compiled_prompt = obj
        .get("compiled_prompt")
        .or_else(|| obj.get("prompt"))
        .or_else(|| obj.get("output"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let mut warnings: Vec<String> = obj
        .get("warnings")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    let mut repair_warnings = Vec::new();

    if obj.contains_key("prompt") && !obj.contains_key("compiled_prompt") {
        repair_warnings.push("LLM used 'prompt' key instead of 'compiled_prompt'; repaired".into());
    }
    if obj.contains_key("output") && !obj.contains_key("compiled_prompt") {
        repair_warnings.push("LLM used 'output' key instead of 'compiled_prompt'; repaired".into());
    }
    if !obj.contains_key("warnings") {
        repair_warnings.push("LLM response missing 'warnings' field; repaired as empty".into());
    }

    let prompt = compiled_prompt?;
    if prompt.trim().is_empty() {
        return Some(fallback(
            "LLM response had empty compiled_prompt after repair",
            original_prompt,
        ));
    }

    warnings.extend(repair_warnings);

    Some(LlmParseOutcome::Repaired {
        response: LlmResponseContract {
            compiled_prompt: prompt,
            warnings,
        },
        warnings: vec!["LLM response format was repaired by IntentLayer parser".into()],
    })
}

fn try_bare_text(text: &str) -> Option<LlmParseOutcome> {
    let trimmed = text.trim();
    // Must have some content and not look like only markdown fences
    if trimmed.is_empty() || trimmed == "```" || trimmed == "```json" {
        return None;
    }
    // Must not be obviously a code block
    if trimmed.starts_with("```") && trimmed.ends_with("```") {
        return None;
    }
    // Must not be meta-commentary only
    if looks_like_meta_commentary(trimmed) {
        return None;
    }
    Some(LlmParseOutcome::BestEffort {
        compiled_prompt: trimmed.to_string(),
        warnings: vec!["LLM returned bare text; accepted as best-effort compiled_prompt".into()],
    })
}

/// Safety validation: reject responses that appear to be model meta-commentary
/// rather than a prompt.
fn looks_like_meta_commentary(text: &str) -> bool {
    let lower = text.to_lowercase();
    let indicators = [
        "as an ai",
        "as a large language",
        "i hope this",
        "let me know if",
        "feel free to",
        "here is the",
        "here's the",
        "sure!",
        "certainly!",
        "of course!",
    ];
    let mut hits = 0;
    for ind in &indicators {
        if lower.contains(ind) {
            hits += 1;
        }
    }
    hits >= 2
}

/// Validate a compiled prompt for safety issues.
/// Returns warnings if anything looks suspicious; never blocks normal prompts.
pub fn validate_compiled_prompt(text: &str) -> Vec<String> {
    let mut warnings = Vec::new();
    let trimmed = text.trim();

    if trimmed.is_empty() {
        warnings.push("Compiled prompt is empty".into());
    }
    if trimmed == "```" || trimmed == "```json" {
        warnings.push("Compiled prompt is only a markdown fence".into());
    }
    if looks_like_meta_commentary(trimmed) {
        warnings.push("Compiled prompt appears to contain model meta-commentary".into());
    }
    if trimmed.len() > 4096 {
        warnings.push("Compiled prompt exceeds 4096 characters".into());
    }

    warnings
}

#[cfg(test)]
mod tests {
    use super::*;

    fn extract_prompt(outcome: &LlmParseOutcome) -> &str {
        match outcome {
            LlmParseOutcome::Parsed(r) => &r.compiled_prompt,
            LlmParseOutcome::Repaired { response, .. } => &response.compiled_prompt,
            LlmParseOutcome::BestEffort {
                compiled_prompt, ..
            } => compiled_prompt,
            LlmParseOutcome::Fallback {
                compiled_prompt, ..
            } => compiled_prompt,
        }
    }

    #[test]
    fn test_strict_valid_json_parses() {
        let raw = r#"{"compiled_prompt":"safe prompt","warnings":[]}"#;
        let result = parse_llm_response(raw, "original");
        match result {
            LlmParseOutcome::Parsed(resp) => {
                assert_eq!(resp.compiled_prompt, "safe prompt");
                assert!(resp.warnings.is_empty());
            }
            _ => panic!("Expected Parsed"),
        }
    }

    #[test]
    fn test_json_in_fence_parses() {
        let raw = "```json\n{\"compiled_prompt\":\"safe\",\"warnings\":[]}\n```";
        let result = parse_llm_response(raw, "original");
        assert_eq!(extract_prompt(&result), "safe");
    }

    #[test]
    fn test_json_in_plain_fence_parses() {
        let raw = "```\n{\"compiled_prompt\":\"safe\",\"warnings\":[]}\n```";
        let result = parse_llm_response(raw, "original");
        assert_eq!(extract_prompt(&result), "safe");
    }

    #[test]
    fn test_prose_before_json_parses() {
        let raw = "Here is your prompt:\n{\"compiled_prompt\":\"safe\",\"warnings\":[]}";
        let result = parse_llm_response(raw, "original");
        assert_eq!(extract_prompt(&result), "safe");
    }

    #[test]
    fn test_prose_after_json_parses() {
        let raw = "{\"compiled_prompt\":\"safe\",\"warnings\":[]}\nHope that helps!";
        let result = parse_llm_response(raw, "original");
        assert_eq!(extract_prompt(&result), "safe");
    }

    #[test]
    fn test_missing_warnings_repaired() {
        let raw = r#"{"compiled_prompt":"safe"}"#;
        let result = parse_llm_response(raw, "original");
        assert_eq!(extract_prompt(&result), "safe");
        // Should be Repaired, not Parsed
        match result {
            LlmParseOutcome::Repaired { .. } => {}
            _ => panic!("Expected Repaired for missing warnings"),
        }
    }

    #[test]
    fn test_prompt_key_repairs_to_compiled_prompt() {
        let raw = r#"{"prompt":"safe prompt text"}"#;
        let result = parse_llm_response(raw, "original");
        assert_eq!(extract_prompt(&result), "safe prompt text");
    }

    #[test]
    fn test_output_key_repairs_to_compiled_prompt() {
        let raw = r#"{"output":"safe prompt text"}"#;
        let result = parse_llm_response(raw, "original");
        assert_eq!(extract_prompt(&result), "safe prompt text");
    }

    #[test]
    fn test_bare_text_becomes_best_effort() {
        let result = parse_llm_response("A simple prompt rewrite", "original");
        match result {
            LlmParseOutcome::BestEffort {
                compiled_prompt,
                warnings,
            } => {
                assert_eq!(compiled_prompt, "A simple prompt rewrite");
                assert!(!warnings.is_empty());
            }
            _ => panic!("Expected BestEffort for bare text"),
        }
    }

    #[test]
    fn test_empty_string_fallbacks() {
        let result = parse_llm_response("", "original prompt");
        match result {
            LlmParseOutcome::Fallback { warnings, .. } => {
                assert!(warnings.iter().any(|w| w.contains("empty")));
            }
            _ => panic!("Expected Fallback for empty string"),
        }
    }

    #[test]
    fn test_whitespace_only_fallbacks() {
        let result = parse_llm_response("   \n  \t  ", "original prompt");
        match result {
            LlmParseOutcome::Fallback { .. } => {}
            _ => panic!("Expected Fallback for whitespace"),
        }
    }

    #[test]
    fn test_invalid_json_fallbacks() {
        let result = parse_llm_response("not json at all { ", "original prompt");
        match result {
            LlmParseOutcome::Fallback { .. } => {}
            LlmParseOutcome::BestEffort { .. } => {}
            _ => panic!("Expected Fallback or BestEffort for invalid JSON"),
        }
    }

    #[test]
    fn test_empty_compiled_prompt_fallbacks() {
        let raw = r#"{"compiled_prompt":"","warnings":[]}"#;
        let result = parse_llm_response(raw, "original");
        match result {
            LlmParseOutcome::Fallback { .. } => {}
            _ => panic!("Expected Fallback for empty compiled_prompt"),
        }
    }

    #[test]
    fn test_meta_commentary_gets_repaired() {
        let raw = r#"{"compiled_prompt":"Here is the prompt. I hope this helps. Let me know if you need anything.","warnings":[]}"#;
        let result = parse_llm_response(raw, "original");
        match result {
            LlmParseOutcome::Repaired { warnings, .. } => {
                assert!(warnings.iter().any(|w| w.contains("meta-commentary")));
            }
            _ => panic!("Expected Repaired for meta-commentary"),
        }
    }

    #[test]
    fn test_parser_does_not_expose_secrets_in_warnings() {
        let raw = r#"{"compiled_prompt":"use sk-abc for api","warnings":[]}"#;
        let result = parse_llm_response(raw, "original");
        let warnings = match &result {
            LlmParseOutcome::Parsed(r) => &r.warnings,
            LlmParseOutcome::Repaired { warnings, .. } => warnings,
            _ => return,
        };
        for w in warnings {
            assert!(!w.contains("sk-abc"), "Warning must not expose secret");
        }
    }

    #[test]
    fn test_validate_empty_prompt_warns() {
        let w = validate_compiled_prompt("");
        assert!(w.iter().any(|x| x.contains("empty")));
    }

    #[test]
    fn test_validate_normal_prompt_ok() {
        let w = validate_compiled_prompt("Fix this bug using existing code");
        assert!(w.is_empty());
    }

    // ── Issue 1 — fallback preserves original_prompt ─────────────

    #[test]
    fn test_strict_json_empty_prompt_fallback_includes_original() {
        let raw = r#"{"compiled_prompt":"","warnings":[]}"#;
        let result = parse_llm_response(raw, "fix the login bug");
        let text = extract_prompt(&result).to_string();
        assert!(
            text.contains("fix the login bug"),
            "Fallback must include original prompt: {}",
            text
        );
    }

    #[test]
    fn test_repaired_json_empty_prompt_fallback_includes_original() {
        let raw = r#"{"prompt":""}"#;
        let result = parse_llm_response(raw, "add auth feature");
        let text = extract_prompt(&result).to_string();
        assert!(text.contains("add auth feature"));
    }

    #[test]
    fn test_fenced_json_empty_prompt_fallback_includes_original() {
        let raw = "```json\n{\"compiled_prompt\":\"\",\"warnings\":[]}\n```";
        let result = parse_llm_response(raw, "refactor the parser");
        let text = extract_prompt(&result).to_string();
        assert!(text.contains("refactor the parser"));
    }

    #[test]
    fn test_prose_wrapped_empty_json_fallback_includes_original() {
        let raw = "Here you go:\n{\"compiled_prompt\":\"\"}\nDone.";
        let result = parse_llm_response(raw, "optimize query");
        let text = extract_prompt(&result).to_string();
        assert!(text.contains("optimize query"));
    }

    #[test]
    fn test_fallback_never_ends_with_empty_original_prompt_block() {
        let result = parse_llm_response("", "fix the build");
        let text = extract_prompt(&result).to_string();
        assert!(
            !text.ends_with("Original prompt:\n"),
            "Fallback must not end with empty original block"
        );
    }

    // ── Issue 2 — string-aware JSON extraction ──────────────────

    #[test]
    fn test_prose_with_braces_inside_compiled_prompt_parses() {
        let raw = "Response:\n{\"compiled_prompt\":\"Update route /users/{id} and preserve constraints.\",\"warnings\":[]}";
        let result = parse_llm_response(raw, "original");
        assert!(extract_prompt(&result).contains("/users/{id}"));
    }

    #[test]
    fn test_prose_with_escaped_quote_parses() {
        let raw = "{\"compiled_prompt\":\"Use \\\"sync\\\" mode for this\",\"warnings\":[]}";
        let result = parse_llm_response(raw, "original");
        assert!(extract_prompt(&result).contains("sync"));
    }

    #[test]
    fn test_prose_with_multiple_strings_and_braces_parses() {
        let raw = "Here:\n{\"compiled_prompt\":\"Fix {count} items and {total} others.\",\"warnings\":[\"noted\"]}\nDone.";
        let result = parse_llm_response(raw, "original");
        assert!(extract_prompt(&result).contains("{count}"));
        assert!(extract_prompt(&result).contains("{total}"));
    }

    #[test]
    fn test_unmatched_json_object_falls_back_safely() {
        let raw = "{\"compiled_prompt\":\"unclosed";
        let result = parse_llm_response(raw, "original");
        match result {
            LlmParseOutcome::BestEffort { .. } | LlmParseOutcome::Fallback { .. } => {}
            other => panic!(
                "Expected BestEffort or Fallback for unmatched JSON: {:?}",
                other
            ),
        }
    }
}
