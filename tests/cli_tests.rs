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
    assert!(
        !output.status.success(),
        "Invalid stdin JSON must exit non-zero"
    );
}

#[test]
fn test_contract_exit_missing_input_file() {
    let output = run(&["--input", "/nonexistent/path.json"]);
    assert!(
        !output.status.success(),
        "Missing input file must exit non-zero"
    );
}

#[test]
fn test_contract_exit_bad_rules_path() {
    let output = run(&["--rules-path", "/bad/rules.json", "--prompt", "hi"]);
    assert!(
        !output.status.success(),
        "Bad rules path must exit non-zero"
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
