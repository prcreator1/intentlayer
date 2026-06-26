//! Live OpenRouter smoke test — manual only, never runs in normal CI.
//!
//! Usage:
//!   export OPENROUTER_API_KEY="..."
//!   INTENTLAYER_RUN_LIVE_SMOKE=1 cargo test --features openrouter-http -- --ignored
//!
//! No API key is committed. No live call is made in normal CI.

use std::env;
use std::process::Command;

/// Returns Ok(()) if live smoke can run, or a reason string if skipped.
fn can_run_smoke() -> Result<(), String> {
    if env::var("INTENTLAYER_RUN_LIVE_SMOKE").unwrap_or_default() != "1" {
        return Err("INTENTLAYER_RUN_LIVE_SMOKE=1 not set".into());
    }
    if cfg!(not(feature = "openrouter-http")) {
        return Err("openrouter-http feature not enabled".into());
    }
    if env::var("OPENROUTER_API_KEY")
        .unwrap_or_default()
        .is_empty()
    {
        return Err("OPENROUTER_API_KEY not set or empty".into());
    }
    Ok(())
}

fn can_run_groq_smoke() -> Result<(), String> {
    if env::var("INTENTLAYER_RUN_LIVE_GROQ_SMOKE").unwrap_or_default() != "1" {
        return Err("INTENTLAYER_RUN_LIVE_GROQ_SMOKE=1 not set".into());
    }
    if cfg!(not(feature = "groq-http")) {
        return Err("groq-http feature not enabled".into());
    }
    if env::var("GROQ_API_KEY").unwrap_or_default().is_empty() {
        return Err("GROQ_API_KEY not set or empty".into());
    }
    Ok(())
}

fn run_intentlayer(args: &[&str]) -> (bool, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_intentlayer"))
        .args(args)
        .output()
        .expect("Failed to run intentlayer");
    (
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

// ── Live smoke tests (ignored, manual) ──────────────────────────────

fn assert_no_secret_leak(stdout: &str, stderr: &str, api_key: &str) {
    if !api_key.is_empty() {
        assert!(!stdout.contains(api_key), "stdout must not contain API key");
        assert!(!stderr.contains(api_key), "stderr must not contain API key");
    }
    let lower_stdout = stdout.to_lowercase();
    let lower_stderr = stderr.to_lowercase();
    assert!(
        !lower_stdout.contains("authorization"),
        "stdout must not contain Authorization"
    );
    assert!(
        !lower_stdout.contains("bearer"),
        "stdout must not contain Bearer"
    );
    assert!(
        !lower_stderr.contains("authorization"),
        "stderr must not contain Authorization"
    );
    assert!(
        !lower_stderr.contains("bearer"),
        "stderr must not contain Bearer"
    );
}

fn assert_no_provider_fallback_in_warnings(stdout: &str) {
    let val: serde_json::Value =
        serde_json::from_str(stdout).expect("Live smoke stdout must be valid JSON");
    let compiled = val["compiled_prompt"].as_str().unwrap_or("");
    assert!(!compiled.is_empty(), "compiled_prompt must be non-empty");
    let warnings = val["warnings"]
        .as_array()
        .expect("warnings must be an array");
    assert!(
        warnings.is_empty(),
        "live provider smoke must have empty warnings: {:?}",
        warnings
    );
}

#[test]
#[ignore = "live smoke test — requires INTENTLAYER_RUN_LIVE_SMOKE=1 + openrouter-http + OPENROUTER_API_KEY"]
fn smoke_deterministic_bypass() {
    if let Err(r) = can_run_smoke() {
        println!("SKIPPED: {}", r);
        return;
    }
    let (ok, stdout, _stderr) = run_intentlayer(&[
        "--prompt",
        "/help",
        "--llm",
        "--provider",
        "openrouter",
        "--api-key-env",
        "OPENROUTER_API_KEY",
        "--compiled-only",
    ]);
    assert!(ok, "Deterministic bypass must succeed");
    assert_eq!(
        stdout.trim(),
        "/help",
        "Slash command must return unchanged"
    );
    assert_no_secret_leak(&stdout, &_stderr, "");
}

#[test]
#[ignore = "live smoke test — requires INTENTLAYER_RUN_LIVE_GROQ_SMOKE=1 + groq-http + GROQ_API_KEY"]
fn smoke_groq_deterministic_bypass() {
    if let Err(r) = can_run_groq_smoke() {
        println!("SKIPPED: {}", r);
        return;
    }
    let (ok, stdout, _stderr) = run_intentlayer(&[
        "--prompt",
        "/help",
        "--llm",
        "--provider",
        "groq",
        "--api-key-env",
        "GROQ_API_KEY",
        "--compiled-only",
    ]);
    assert!(ok);
    assert_eq!(stdout.trim(), "/help");
    assert_no_secret_leak(&stdout, &_stderr, "");
}

#[test]
#[ignore = "live smoke test — requires INTENTLAYER_RUN_LIVE_SMOKE=1 + openrouter-http + OPENROUTER_API_KEY"]
fn smoke_real_llm_compile_call() {
    match can_run_smoke() {
        Ok(()) => {}
        Err(r) => {
            println!("SKIPPED: {}", r);
            return;
        }
    }
    let prompt =
        "Design a concise implementation plan for adding a retry wrapper around a failing parser.";
    let (ok, stdout, stderr) = run_intentlayer(&[
        "--prompt",
        prompt,
        "--llm",
        "--provider",
        "openrouter",
        "--api-key-env",
        "OPENROUTER_API_KEY",
        "--json",
    ]);
    assert!(ok, "Live LLM call must exit 0; stderr: {}", stderr);
    assert!(
        stdout.contains("compiled_prompt"),
        "Must have compiled_prompt field"
    );
    assert!(stdout.contains("warnings"), "Must have warnings field");
    assert_no_provider_fallback_in_warnings(&stdout);
    let api_key = env::var("OPENROUTER_API_KEY").unwrap_or_default();
    assert_no_secret_leak(&stdout, &stderr, &api_key);
}

// ── Groq live smoke test ──────────────────────────────────────────

#[test]
#[ignore = "live smoke test — requires INTENTLAYER_RUN_LIVE_GROQ_SMOKE=1 + groq-http + GROQ_API_KEY"]
fn smoke_real_groq_compile_call() {
    if let Err(r) = can_run_groq_smoke() {
        println!("SKIPPED: {}", r);
        return;
    }
    let prompt = "Design a retry wrapper for failed HTTP requests. Keep it provider-agnostic.";
    let (ok, stdout, stderr) = run_intentlayer(&[
        "--prompt",
        prompt,
        "--llm",
        "--provider",
        "groq",
        "--api-key-env",
        "GROQ_API_KEY",
        "--json",
    ]);
    assert!(ok, "Live Groq call must exit 0; stderr: {}", stderr);
    assert!(
        stdout.contains("compiled_prompt"),
        "Must have compiled_prompt"
    );
    assert!(stdout.contains("warnings"), "Must have warnings");
    assert_no_provider_fallback_in_warnings(&stdout);
    let api_key = env::var("GROQ_API_KEY").unwrap_or_default();
    assert_no_secret_leak(&stdout, &stderr, &api_key);
}

// ── Smoke gating tests (always run, no network) ────────────────────

#[test]
fn test_compiled_prompt_may_contain_fallback_word_if_warnings_empty() {
    let json = r#"{"compiled_prompt":"Design a fallback strategy","warnings":[]}"#;
    // Must not panic
    assert_no_provider_fallback_in_warnings(json);
}

#[test]
fn test_warnings_with_fallback_text_fail_validation() {
    let json = r#"{"compiled_prompt":"ok","warnings":["LLM provider failed"]}"#;
    let result = std::panic::catch_unwind(|| {
        assert_no_provider_fallback_in_warnings(json);
    });
    assert!(result.is_err(), "Fallback warnings must fail validation");
}

#[test]
fn test_warnings_with_repair_text_fail_validation() {
    let json = r#"{"compiled_prompt":"ok","warnings":["LLM response format was repaired"]}"#;
    let result = std::panic::catch_unwind(|| {
        assert_no_provider_fallback_in_warnings(json);
    });
    assert!(result.is_err(), "Repair warnings must fail validation");
}

#[test]
fn test_smoke_skipped_without_env() {
    let key_was_set = env::var("OPENROUTER_API_KEY").is_ok();
    env::remove_var("INTENTLAYER_RUN_LIVE_SMOKE");
    let result = can_run_smoke();
    assert!(result.is_err(), "Should skip when env var not set");
    if key_was_set {
        env::set_var(
            "OPENROUTER_API_KEY",
            env::var("OPENROUTER_API_KEY").unwrap_or_default(),
        );
    }
}

#[test]
fn test_smoke_requires_key() {
    let had_smoke = env::var("INTENTLAYER_RUN_LIVE_SMOKE").unwrap_or_default() == "1";
    let had_key = env::var("OPENROUTER_API_KEY").ok();
    env::set_var("INTENTLAYER_RUN_LIVE_SMOKE", "1");
    env::remove_var("OPENROUTER_API_KEY");
    let result = can_run_smoke();
    assert!(result.is_err(), "Should fail without API key");
    if had_smoke {
        env::set_var("INTENTLAYER_RUN_LIVE_SMOKE", "1");
    }
    if let Some(k) = had_key {
        env::set_var("OPENROUTER_API_KEY", &k);
    }
}

#[test]
fn test_smoke_error_never_contains_api_key() {
    // Prove that errors from missing key don't expose the key
    let had_smoke = env::var("INTENTLAYER_RUN_LIVE_SMOKE").unwrap_or_default() == "1";
    let had_key = env::var("OPENROUTER_API_KEY").ok();
    env::remove_var("INTENTLAYER_RUN_LIVE_SMOKE");
    let result = can_run_smoke();
    let msg = result.unwrap_err();
    assert!(
        !msg.contains("sk-"),
        "Error must not contain key-like patterns"
    );
    assert!(
        !msg.to_lowercase().contains("bearer"),
        "Error must not mention bearer"
    );
    if had_smoke {
        env::set_var("INTENTLAYER_RUN_LIVE_SMOKE", "1");
    }
    if let Some(k) = had_key {
        env::set_var("OPENROUTER_API_KEY", &k);
    }
}

#[test]
fn test_smoke_deterministic_bypass_in_smoke_test_itself() {
    // The deterministic bypass logic is tested via the CLI test:
    // /help with --llm should still return /help unchanged.
    // This test only exercises the code path that runs /help through
    // the local compiler, not OpenRouter.
    let (ok, stdout, _) = run_intentlayer(&[
        "--prompt",
        "/help",
        "--llm",
        "--provider",
        "openrouter",
        "--compiled-only",
    ]);
    assert!(ok, "/help should succeed without API key");
    assert_eq!(stdout.trim(), "/help");
}
