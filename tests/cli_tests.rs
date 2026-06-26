//! CLI integration tests for the intentlayer binary.

use std::io::Write;
use std::process::{Command, Output};

/// Run the binary with given args and return captured output.
fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_intentlayer"))
        .args(args)
        .output()
        .expect("Failed to run binary")
}

/// Run the binary with given args, providing a JSON string on stdin.
fn run_with_stdin(args: &[&str], stdin_text: &str) -> Output {
    use std::process::Stdio;
    let mut child = Command::new(env!("CARGO_BIN_EXE_intentlayer"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn binary");
    {
        let mut stdin = child.stdin.take().unwrap();
        stdin
            .write_all(stdin_text.as_bytes())
            .expect("Failed to write to stdin");
    }
    child.wait_with_output().expect("Failed to read output")
}

#[test]
fn test_help_exits_successfully() {
    let output = run(&["--help"]);
    assert!(output.status.success(), "--help should exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Usage:"),
        "Help should contain Usage: section"
    );
    assert!(stdout.contains("--prompt"), "Help should mention --prompt");
    assert!(stdout.contains("--input"), "Help should mention --input");
    assert!(
        stdout.contains("--rules-path"),
        "Help should mention --rules-path"
    );
    assert!(stdout.contains("--pretty"), "Help should mention --pretty");
    assert!(stdout.contains("--json"), "Help should mention --json");
    assert!(stdout.contains("--help"), "Help should mention --help");
}

#[test]
fn test_prompt_via_direct_argument() {
    let output = run(&["--prompt", "fix this repo", "--json"]);
    assert!(
        output.status.success(),
        "--prompt 'fix this repo' should exit 0"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("fix this repo"),
        "Output should contain the original prompt"
    );
    assert!(
        stdout.contains("compiled_prompt"),
        "Output should be valid JSON with compiled_prompt"
    );
    assert!(
        stdout.contains("local_compile"),
        "Mode should be local_compile for 'fix this repo'"
    );
}

#[test]
fn test_json_input_file() {
    let dir = std::env::temp_dir();
    let path = dir.join("test_intentlayer_input.json");
    std::fs::write(&path, r#"{"prompt":"fix this repo"}"#).unwrap();

    let output = run(&[
        "--rules-path",
        "research/transformation_rules.json",
        "--input",
        path.to_str().unwrap(),
        "--json",
    ]);
    // Clean up
    let _ = std::fs::remove_file(&path);

    assert!(output.status.success(), "--input file.json should exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("compiled_prompt"),
        "Output should be valid JSON"
    );
}

#[test]
fn test_stdin_json_fallback() {
    let output = run_with_stdin(&["--json"], r#"{"prompt":"fix this repo"}"#);
    assert!(output.status.success(), "stdin JSON should exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("compiled_prompt"),
        "Output should be valid JSON"
    );
}

#[test]
fn test_invalid_json_gives_error() {
    let output = run_with_stdin(&[], "not valid json");
    assert!(
        !output.status.success(),
        "Invalid JSON should exit non-zero"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.to_lowercase().contains("invalid"),
        "stderr should say 'invalid': {}",
        stderr
    );
}

#[test]
fn test_missing_prompt_field_gives_error() {
    let output = run_with_stdin(&[], r#"{"not_prompt":"value"}"#);
    assert!(
        !output.status.success(),
        "Missing prompt field should exit non-zero"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.to_lowercase().contains("prompt"),
        "stderr should mention 'prompt': {}",
        stderr
    );
}

#[test]
fn test_missing_prompt_value_after_flag() {
    let output = run(&["--prompt"]);
    assert!(
        !output.status.success(),
        "Missing value for --prompt should exit non-zero"
    );
}

#[test]
fn test_missing_input_value_after_flag() {
    let output = run(&["--input"]);
    assert!(
        !output.status.success(),
        "Missing value for --input should exit non-zero"
    );
}

#[test]
fn test_missing_rules_path_value_after_flag() {
    let output = run(&["--rules-path"]);
    assert!(
        !output.status.success(),
        "Missing value for --rules-path should exit non-zero"
    );
}

#[test]
fn test_unreadable_input_file_gives_error() {
    let output = run(&["--input", "/nonexistent/path/input.json"]);
    assert!(
        !output.status.success(),
        "Unreadable input file should exit non-zero"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.to_lowercase().contains("cannot read"),
        "stderr should say 'cannot read': {}",
        stderr
    );
}

#[test]
fn test_unreadable_rules_file_gives_error() {
    let output = run(&[
        "--rules-path",
        "/nonexistent/path/rules.json",
        "--prompt",
        "hello",
    ]);
    assert!(
        !output.status.success(),
        "Unreadable rules file should exit non-zero"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.to_lowercase().contains("error loading rules"),
        "stderr should say 'error loading rules': {}",
        stderr
    );
}

#[test]
fn test_conflicting_input_sources_give_error() {
    let output = run(&["--prompt", "hello", "--input", "file.json"]);
    assert!(
        !output.status.success(),
        "Conflicting --prompt and --input should exit non-zero"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.to_lowercase().contains("conflicting"),
        "stderr should say 'conflicting': {}",
        stderr
    );
}

#[test]
fn test_pretty_json_is_default() {
    let output = run(&["--prompt", "hello"]);
    assert!(output.status.success(), "Should exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Pretty JSON has newlines and indentation
    assert!(
        stdout.contains('\n'),
        "Default output should be pretty (multi-line) JSON"
    );
}

#[test]
fn test_json_flag_produces_compact() {
    let output = run(&["--prompt", "hello", "--json"]);
    assert!(output.status.success(), "Should exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Compact JSON is a single line (excluding trailing newline from println!)
    let body = stdout.trim_end();
    assert!(
        !body.contains('\n'),
        "--json output should be single-line compact: {}",
        body
    );
}

#[test]
fn test_rules_path_override_works() {
    let output = run(&[
        "--rules-path",
        "research/transformation_rules.json",
        "--prompt",
        "fix this repo",
        "--json",
    ]);
    assert!(output.status.success(), "--rules-path should exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("local_compile"),
        "Should compile successfully"
    );
}

#[test]
fn test_no_input_produces_error() {
    let output = run_with_stdin(&[], "");
    assert!(!output.status.success(), "No input should exit non-zero");
}

#[test]
fn test_version_flag() {
    let output = run(&["--version"]);
    assert!(output.status.success(), "--version should exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let expected = format!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
    assert_eq!(
        stdout.trim(),
        expected,
        "--version should match Cargo metadata"
    );
}

#[test]
fn test_help_mentions_version() {
    let output = run(&["--help"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("--version"),
        "--help should mention --version"
    );
}

#[test]
fn test_help_mentions_stdin() {
    let output = run(&["--help"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.to_lowercase().contains("stdin"),
        "--help should mention stdin JSON usage"
    );
}

#[test]
fn test_release_invocation_with_prompt() {
    let output = run(&["--prompt", "fix this release bug", "--json"]);
    assert!(
        output.status.success(),
        "Release-style invocation should exit 0"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("compiled_prompt"),
        "Release-style invocation should produce valid JSON"
    );
}

// ── Agent integration safety tests ──

#[test]
fn test_contract_exit_valid_prompt() {
    let output = run(&["--prompt", "hello", "--json"]);
    assert!(output.status.success(), "Valid --prompt must exit 0");
}

#[test]
fn test_contract_exit_valid_stdin() {
    let output = run_with_stdin(&["--json"], r#"{"prompt":"hello"}"#);
    assert!(output.status.success(), "Valid stdin JSON must exit 0");
}

#[test]
fn test_contract_exit_invalid_stdin() {
    let output = run_with_stdin(&[], "bad json");
    assert_eq!(
        output.status.code(),
        Some(1),
        "Invalid stdin JSON must exit with code 1"
    );
}

#[test]
fn test_contract_exit_missing_input_file() {
    let output = run(&["--input", "/nonexistent/path.json"]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "Missing input file must exit with code 1"
    );
}

#[test]
fn test_contract_exit_bad_rules_path() {
    let output = run(&["--rules-path", "/bad/rules.json", "--prompt", "hi"]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "Bad rules path must exit with code 1"
    );
}

#[test]
fn test_contract_exit_version_zero() {
    let output = run(&["--version"]);
    assert!(output.status.success(), "--version must exit 0");
}

#[test]
fn test_contract_exit_help_zero() {
    let output = run(&["--help"]);
    assert!(output.status.success(), "--help must exit 0");
}

#[test]
fn test_contract_smoke_output_fields() {
    let output = run(&["--input", "examples/agent_request.json", "--json"]);
    assert!(output.status.success(), "Smoke test must exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let required = [
        "original_prompt",
        "compiled_prompt",
        "mode",
        "category",
        "changed",
        "warnings",
    ];
    for field in &required {
        assert!(
            stdout.contains(field),
            "Output must contain field '{}': {}",
            field,
            stdout
        );
    }
}

// ── Compiled-only mode tests ──

#[test]
fn test_compiled_only_prints_only_compiled_prompt() {
    let output = run(&["--prompt", "fix this bug", "--compiled-only"]);
    assert!(output.status.success(), "compiled-only should exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains('{'),
        "compiled-only should not output JSON"
    );
    assert!(
        !stdout.contains("original_prompt"),
        "compiled-only should not contain metadata"
    );
    assert!(
        stdout.contains("context"),
        "compiled-only should contain compiled prompt"
    );
}

#[test]
fn test_compiled_only_output_is_not_json() {
    let output = run(&["--prompt", "hello", "--compiled-only"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.trim().starts_with('{'),
        "compiled-only output must not start with {{"
    );
}

#[test]
fn test_compiled_only_with_stdin() {
    let output = run_with_stdin(&["--compiled-only"], r#"{"prompt":"fix this bug"}"#);
    assert!(
        output.status.success(),
        "compiled-only with stdin should exit 0"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains('{'),
        "compiled-only stdin should not output JSON"
    );
}

#[test]
fn test_help_mentions_compiled_only() {
    let output = run(&["--help"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("--compiled-only"),
        "--help should mention --compiled-only"
    );
}

#[test]
fn test_llm_unsupported_provider_rejected() {
    let output = run(&[
        "--llm",
        "--provider",
        "typo",
        "--api-key-env",
        "SOME_ENV",
        "--prompt",
        "design the system",
        "--json",
    ]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unsupported"),
        "Must say unsupported: {}",
        stderr
    );
    assert!(stderr.contains("openrouter"), "Must mention openrouter");
    assert!(stderr.contains("groq"), "Must mention groq");
    assert!(
        !stderr.contains("SOME_ENV"),
        "Must not expose env var value"
    );
}

#[test]
fn test_groq_provider_accepted() {
    // Provider name is accepted (actual call fails without key)
    let output = run(&[
        "--prompt",
        "design the system",
        "--llm",
        "--provider",
        "groq",
        "--api-key-env",
        "GROQ_API_KEY",
        "--json",
    ]);
    // Should fail with transport/key error, not "unsupported provider"
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("unsupported"),
        "groq must be accepted provider: {}",
        stderr
    );
}

#[test]
fn test_pretty_still_works_with_compiled_only_absent() {
    let output = run(&["--prompt", "hello"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains('{'), "--pretty default still outputs JSON");
}

// ── CLI LLM opt-in tests ──

#[test]
fn test_default_cli_remains_local() {
    let output = run(&["--prompt", "fix this repo", "--json"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("local_compile"),
        "Default mode should be local_compile"
    );
}

#[test]
fn test_compiled_only_still_works_locally() {
    let output = run(&["--prompt", "hello", "--compiled-only"]);
    assert!(output.status.success());
}

#[test]
fn test_llm_without_provider_returns_error() {
    let output = run(&["--prompt", "test", "--llm"]);
    assert!(
        !output.status.success(),
        "--llm without --provider must fail"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("openrouter"),
        "Error must mention openrouter: {}",
        stderr
    );
    assert!(
        stderr.contains("groq"),
        "Error must mention groq: {}",
        stderr
    );
}

#[test]
fn test_llm_openrouter_without_api_key_env_returns_error() {
    // "design the system" is llm_compile, so it requires api-key-env
    let output = run(&[
        "--prompt",
        "design the system",
        "--llm",
        "--provider",
        "openrouter",
    ]);
    assert!(
        !output.status.success(),
        "llm_compile without --api-key-env must fail"
    );
}

#[test]
fn test_help_mentions_llm_flags() {
    let output = run(&["--help"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--llm"), "--help must mention --llm");
    assert!(
        stdout.contains("--provider"),
        "--help must mention --provider"
    );
    assert!(
        stdout.contains("--api-key-env"),
        "--help must mention --api-key-env"
    );
}

#[test]
fn test_default_compile_unchanged_with_llm_flags() {
    let output = run(&["--prompt", "fix this repo", "--json"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("compiled_prompt"));
}

#[test]
fn test_llm_slash_command_remains_pass_through_and_needs_no_key() {
    // slash commands must bypass LLM entirely
    let output = run(&[
        "--llm",
        "--provider",
        "openrouter",
        "--prompt",
        "/help",
        "--compiled-only",
    ]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Pass-through returns exact original prompt
    assert_eq!(stdout.trim(), "/help");
}

#[test]
fn test_llm_local_compile_still_uses_local_compiler() {
    // "fix this repo" is local_compile — even with --llm, stays local
    let output = run(&[
        "--prompt",
        "fix this repo",
        "--llm",
        "--provider",
        "openrouter",
        "--api-key-env",
        "INTENTLAYER_FAKE_KEY",
        "--json",
    ]);
    // Should succeed because local_compile bypasses LLM, so no real key needed
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("compiled_prompt"),
        "Should produce output: {}",
        stdout
    );
}

#[test]
fn test_llm_llm_compile_without_feature_is_handled() {
    // Without openrouter-http feature, llm_compile prompts get error
    // But the error is about the transport, not crash
    let output = run(&[
        "--prompt",
        "design the system",
        "--llm",
        "--provider",
        "openrouter",
        "--api-key-env",
        "INTENTLAYER_FAKE_KEY",
        "--json",
    ]);
    // This will fail because no key, but it's a feature-gate or config issue, not panic
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("panic"), "Must not panic");
}

// --- Phase 027: Large input tests ---

fn large_prompt(size: usize) -> String {
    let mut s = String::with_capacity(size + 64);
    s.push_str("Fix the following bug: ");
    // Fill with repeated ASCII text to reach target size
    while s.len() < size {
        s.push_str("lorem ipsum dolor sit amet consectetur adipiscing elit sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. ");
    }
    s.truncate(size);
    s
}

fn write_temp_json(filename: &str, prompt: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir();
    let path = dir.join(filename);
    let json = serde_json::json!({"prompt": prompt});
    std::fs::write(&path, serde_json::to_vec(&json).unwrap()).unwrap();
    path
}

#[test]
fn test_large_prompt_via_input_file() {
    let prompt = large_prompt(30_000);
    let path = write_temp_json("large_input.json", &prompt);
    let output = run(&["--input", path.to_str().unwrap(), "--compiled-only"]);
    assert!(
        output.status.success(),
        "Large input via --input should succeed"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.trim().is_empty(),
        "Compiled output should not be empty"
    );
}

#[test]
fn test_large_prompt_via_stdin_json() {
    let prompt = large_prompt(25_000);
    let json = serde_json::json!({"prompt": prompt});
    let json_str = serde_json::to_string(&json).unwrap();
    let output = run_with_stdin(&["--compiled-only"], &json_str);
    assert!(output.status.success(), "Large stdin JSON should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.trim().is_empty(),
        "Compiled output should not be empty"
    );
}

#[test]
fn test_large_prompt_compiled_only_plain_text() {
    let prompt = large_prompt(20_000);
    let path = write_temp_json("large_compiled_only.json", &prompt);
    let output = run(&["--input", path.to_str().unwrap(), "--compiled-only"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    // --compiled-only output should NOT contain JSON braces wrapping metadata
    assert!(
        !stdout.trim().starts_with('{'),
        "Compiled-only output must be plain text, not JSON"
    );
}

#[test]
fn test_large_prompt_slash_pass_through() {
    // A large prompt that starts with / should pass through exactly
    let mut prompt = String::from("/review ");
    prompt.push_str(&large_prompt(20_000));
    let path = write_temp_json("large_slash.json", &prompt);
    let output = run(&["--input", path.to_str().unwrap(), "--compiled-only"]);
    assert!(
        output.status.success(),
        "Large slash command should succeed"
    );
    let stdout = String::from_utf8_lossy(&output.stdout)
        .trim_end_matches('\n')
        .to_string();
    // Because this is a slash command, it should pass through exactly
    assert_eq!(
        stdout, prompt,
        "Large slash command must pass through exactly"
    );
}

// --- Phase 028: Provider failure visibility tests ---

#[test]
fn test_help_lists_both_openrouter_and_groq() {
    let output = run(&["--help"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("openrouter") && stdout.contains("groq"),
        "--help must list both openrouter and groq providers"
    );
}

#[test]
fn test_help_lists_allow_llm_fallback() {
    let output = run(&["--help"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("--allow-llm-fallback"),
        "--help must mention --allow-llm-fallback"
    );
}

#[test]
fn test_llm_without_provider_shows_both_providers() {
    let output = run(&["--prompt", "fix bug", "--llm", "--json"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !output.status.success(),
        "Should exit non-zero without provider"
    );
    assert!(
        stderr.contains("openrouter") && stderr.contains("groq"),
        "Error must list both openrouter and groq as supported"
    );
}

#[test]
fn test_unknown_provider_shows_valid_list() {
    let output = run(&[
        "--prompt",
        "fix bug",
        "--llm",
        "--provider",
        "nonexistent",
        "--json",
    ]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "Unknown provider should fail");
    assert!(
        stderr.contains("openrouter") && stderr.contains("groq"),
        "Error must include supported providers: {}",
        stderr
    );
}

#[test]
fn test_allow_llm_fallback_flag_accepted() {
    // Verifies the flag is parsed without error
    let output = run(&[
        "--prompt",
        "fix bug",
        "--llm",
        "--provider",
        "openrouter",
        "--api-key-env",
        "FAKE_KEY",
        "--allow-llm-fallback",
        "--json",
    ]);
    // With --allow-llm-fallback, a provider failure would still fallback
    // Without the feature, the stub exits 1, but the flag itself shouldn't
    // cause an "unknown argument" error
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("Unknown argument"),
        "Should not reject --allow-llm-fallback: {}",
        stderr
    );
}
