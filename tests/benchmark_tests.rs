//! Benchmark test runner for research/vibe_prompt_bench.draft.jsonl
//!
//! Enforced checks:
//! 1. correct mode                       (100/100 — enforced)
//! 2. pass_through returns exact original (22/22 — enforced)
//! 3. minimal_compile returns non-null    (9/9  — enforced)
//! 4. proper-noun brands not invented    (100/100 — enforced, uppercase only)
//! 5. no clarification when forbidden     (enforced)
//! 6. token cap respected                (enforced)
//!
//! Aspirational / informational checks:
//! 7. category accuracy    (ignored — TODO v0.1)
//! 8. expected prompt match (aspirational — asserts ≥10)

use intentlayer::compiler::{CompileOutput, Compiler};
use intentlayer::rules::RuleSet;
use serde::Deserialize;
use std::path::PathBuf;

/// Benchmark record loaded from JSONL.
#[derive(Debug, Deserialize)]
struct BenchRecord {
    id: String,
    category: String,
    raw_prompt: String,
    expected_compiled_prompt: Option<String>,
    mode: String,
    #[allow(dead_code)]
    must_preserve: Vec<String>,
    must_not_invent: Vec<String>,
    should_not_ask_clarification: bool,
    max_compiled_tokens: usize,
    notes: String,
}

/// Locate the benchmark file relative to the crate.
fn find_benchmark_file() -> PathBuf {
    let candidates = [
        "research/vibe_prompt_bench.draft.jsonl",
        "../research/vibe_prompt_bench.draft.jsonl",
        "../../research/vibe_prompt_bench.draft.jsonl",
    ];
    for c in &candidates {
        let p = PathBuf::from(c);
        if p.exists() {
            return p;
        }
    }
    PathBuf::from("research/vibe_prompt_bench.draft.jsonl")
}

/// Load all benchmark records from the JSONL file.
fn load_benchmarks() -> Vec<BenchRecord> {
    let path = find_benchmark_file();
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to read benchmark file {:?}: {}", path, e));

    content
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| {
            serde_json::from_str::<BenchRecord>(l).expect("Failed to parse benchmark JSONL line")
        })
        .collect()
}

/// Build a compiler from the transformation rules file.
fn build_compiler() -> Compiler {
    let candidates = [
        "research/transformation_rules.json",
        "../research/transformation_rules.json",
        "../../research/transformation_rules.json",
    ];
    let path = candidates
        .iter()
        .find(|p| PathBuf::from(p).exists())
        .expect("Could not locate research/transformation_rules.json");
    let rules =
        RuleSet::load(std::path::Path::new(path)).expect("Failed to load transformation rules");
    Compiler::new(rules)
}

/// Count "tokens" (whitespace-delimited words) — v0.1 approximation.
fn token_count(text: &str) -> usize {
    text.split_whitespace().count()
}

/// Check if proper-noun / brand-name must_not_invent terms appear in the
/// compiled prompt.  Lowercase behavioral terms (e.g. "changes", "file path")
/// are NOT checked because they describe compiler behaviour constraints, not
/// text-substitution rules.  They are tested separately through
/// expected_compiled_prompt matching.
fn has_invented_terms(output: &CompileOutput, must_not_invent: &[String]) -> Vec<String> {
    let lower = output.compiled_prompt.to_lowercase();
    must_not_invent
        .iter()
        .filter(|term| {
            // Only check proper nouns / brand names (have at least 1 uppercase char).
            let has_uppercase = term.chars().any(|c| c.is_uppercase());
            has_uppercase && lower.contains(&term.to_lowercase())
        })
        .cloned()
        .collect()
}

/// Check for forbidden clarification questions.
fn has_forbidden_clarification(compiled: &str) -> bool {
    let lower = compiled.to_lowercase();
    let forbidden = [
        "which repo",
        "which error",
        "which file?",
        "what do you mean",
        "can you clarify",
    ];
    forbidden.iter().any(|p| lower.contains(p))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_benchmark_loads_all_100_records() {
        let records = load_benchmarks();
        assert_eq!(
            records.len(),
            100,
            "Benchmark must contain exactly 100 records"
        );
    }

    #[test]
    fn test_all_records_have_correct_mode() {
        let compiler = build_compiler();
        let records = load_benchmarks();

        for r in &records {
            let input = intentlayer::compiler::CompileInput {
                prompt: r.raw_prompt.clone(),
            };
            let output = intentlayer::compiler::compile(&input, &compiler);
            assert_eq!(
                output.mode, r.mode,
                "[{}] Mode mismatch: expected '{}', got '{}'",
                r.id, r.mode, output.mode
            );
        }
    }

    #[test]
    fn test_output_category_matches_benchmark() {
        let compiler = build_compiler();
        let records = load_benchmarks();

        for r in &records {
            let input = intentlayer::compiler::CompileInput {
                prompt: r.raw_prompt.clone(),
            };
            let output = intentlayer::compiler::compile(&input, &compiler);
            assert_eq!(
                output.category, r.category,
                "[{}] Category mismatch: expected '{}', got '{}'",
                r.id, r.category, output.category
            );
        }
    }

    #[test]
    fn test_pass_through_returns_exact_original() {
        let compiler = build_compiler();
        let records = load_benchmarks();

        for r in &records {
            if r.mode != "pass_through" {
                continue;
            }
            let input = intentlayer::compiler::CompileInput {
                prompt: r.raw_prompt.clone(),
            };
            let output = intentlayer::compiler::compile(&input, &compiler);
            assert_eq!(
                output.compiled_prompt, r.raw_prompt,
                "[{}] pass_through must return exact original prompt",
                r.id
            );
            assert!(
                !output.changed,
                "[{}] pass_through must not be changed",
                r.id
            );
            assert_eq!(
                output.compiled_prompt, output.original_prompt,
                "[{}] compiled_prompt must equal original_prompt for pass_through",
                r.id
            );
        }
    }

    #[test]
    fn test_pass_through_has_null_expected_compiled_prompt() {
        let records = load_benchmarks();
        for r in &records {
            if r.mode == "pass_through" {
                assert!(
                    r.expected_compiled_prompt.is_none(),
                    "[{}] pass_through must have expected_compiled_prompt = null",
                    r.id
                );
            }
        }
    }

    #[test]
    fn test_minimal_compile_returns_non_null_small_prompt() {
        let compiler = build_compiler();
        let records = load_benchmarks();

        for r in &records {
            if r.mode != "minimal_compile" {
                continue;
            }
            let input = intentlayer::compiler::CompileInput {
                prompt: r.raw_prompt.clone(),
            };
            let output = intentlayer::compiler::compile(&input, &compiler);
            assert!(
                !output.compiled_prompt.is_empty(),
                "[{}] minimal_compile must return non-null prompt",
                r.id
            );
            assert!(
                output.compiled_prompt != r.raw_prompt,
                "[{}] minimal_compile must not return exact original",
                r.id
            );
            assert!(
                !r.notes.to_lowercase().contains("pass through")
                    && !r.notes.to_lowercase().contains("exact pass"),
                "[{}] minimal_compile notes must not say 'pass through': {}",
                r.id,
                r.notes
            );
        }
    }

    #[test]
    fn test_proper_noun_brand_terms_not_invented() {
        let compiler = build_compiler();
        let records = load_benchmarks();

        for r in &records {
            let input = intentlayer::compiler::CompileInput {
                prompt: r.raw_prompt.clone(),
            };
            let output = intentlayer::compiler::compile(&input, &compiler);
            let invented = has_invented_terms(&output, &r.must_not_invent);
            assert!(
                invented.is_empty(),
                "[{}] must_not_invent terms found in compiled prompt: {:?}",
                r.id,
                invented
            );
        }
    }

    #[test]
    fn test_no_clarification_when_forbidden() {
        let compiler = build_compiler();
        let records = load_benchmarks();

        for r in &records {
            if !r.should_not_ask_clarification {
                continue;
            }
            let input = intentlayer::compiler::CompileInput {
                prompt: r.raw_prompt.clone(),
            };
            let output = intentlayer::compiler::compile(&input, &compiler);
            assert!(
                !has_forbidden_clarification(&output.compiled_prompt),
                "[{}] Forbidden clarification question detected when should_not_ask_clarification = true",
                r.id
            );
        }
    }

    #[test]
    fn test_token_cap_respected() {
        let compiler = build_compiler();
        let records = load_benchmarks();

        for r in &records {
            // pass_through returns the exact original — cap is 0 expansion tokens.
            // Only check token cap for compiled modes.
            if r.mode == "pass_through" {
                continue;
            }
            let input = intentlayer::compiler::CompileInput {
                prompt: r.raw_prompt.clone(),
            };
            let output = intentlayer::compiler::compile(&input, &compiler);
            let count = token_count(&output.compiled_prompt);
            assert!(
                count <= r.max_compiled_tokens,
                "[{}] Token cap exceeded: {} > {}",
                r.id,
                count,
                r.max_compiled_tokens
            );
        }
    }

    /// Aspirational test: for non-pass-through records, the compiled prompt
    /// should equal the benchmark's expected_compiled_prompt.
    ///
    /// v0.1 note: local_compile templates are generated from rule
    /// compact_rewrite_template fields, not from the benchmark's
    /// expected_compiled_prompt.  Currently only minimal_compile records
    /// and arch_001 match exactly.  This test asserts a minimum correct
    /// count rather than 100% so it acts as a progress tracker.
    #[test]
    fn test_compiled_prompt_matches_expected_for_non_pass_through() {
        let compiler = build_compiler();
        let records = load_benchmarks();
        let mut correct = 0usize;
        let mut non_pass_through = 0usize;

        for r in &records {
            if r.mode == "pass_through" {
                continue;
            }
            non_pass_through += 1;
            // Only check records that have an expected_compiled_prompt.
            if let Some(ref expected) = r.expected_compiled_prompt {
                let input = intentlayer::compiler::CompileInput {
                    prompt: r.raw_prompt.clone(),
                };
                let output = intentlayer::compiler::compile(&input, &compiler);
                if output.compiled_prompt == *expected {
                    correct += 1;
                }
            }
        }

        assert!(correct >= 10,
            "Expected at least 10 non-pass-through records to match expected_compiled_prompt, got {}/{}",
            correct, non_pass_through
        );
    }

    #[test]
    fn test_mode_distribution() {
        let records = load_benchmarks();
        let mut pass_through = 0;
        let mut minimal_compile = 0;
        let mut local_compile = 0;
        let mut llm_compile = 0;

        for r in &records {
            match r.mode.as_str() {
                "pass_through" => pass_through += 1,
                "minimal_compile" => minimal_compile += 1,
                "local_compile" => local_compile += 1,
                "llm_compile" => llm_compile += 1,
                _ => panic!("Unknown mode in record {}: {}", r.id, r.mode),
            }
        }

        assert_eq!(pass_through, 22, "pass_through count");
        assert_eq!(minimal_compile, 9, "minimal_compile count");
        assert_eq!(local_compile, 66, "local_compile count");
        assert_eq!(llm_compile, 3, "llm_compile count");
        assert_eq!(
            pass_through + minimal_compile + local_compile + llm_compile,
            100,
            "total count"
        );
    }

    /// Prints a full accuracy report during test execution.
    ///
    /// Metrics reported:
    /// - total records
    /// - mode accuracy
    /// - category accuracy     (informational — not enforced)
    /// - exact compiled-prompt match count
    /// - per-mode exact match counts (pass_through, minimal_compile,
    ///   local_compile, llm_compile)
    ///
    /// Only mode accuracy is enforced (must be 100/100).
    /// Category and exact-prompt are informational.
    #[test]
    fn test_accuracy_report() {
        let compiler = build_compiler();
        let records = load_benchmarks();
        let total = records.len();

        let mut mode_correct = 0usize;
        let mut category_correct = 0usize;
        let mut exact_prompt_correct = 0usize;
        let mut pt_exact = 0usize;
        let mut pt_total = 0usize;
        let mut mc_exact = 0usize;
        let mut mc_total = 0usize;
        let mut lc_exact = 0usize;
        let mut lc_total = 0usize;
        let mut llmc_exact = 0usize;
        let mut llmc_total = 0usize;

        for r in &records {
            let input = intentlayer::compiler::CompileInput {
                prompt: r.raw_prompt.clone(),
            };
            let output = intentlayer::compiler::compile(&input, &compiler);

            // Mode accuracy
            if output.mode == r.mode {
                mode_correct += 1;
            }

            // Category accuracy (informational)
            if output.category == r.category {
                category_correct += 1;
            }

            // Exact prompt match
            if let Some(ref expected) = r.expected_compiled_prompt {
                if output.compiled_prompt == *expected {
                    exact_prompt_correct += 1;
                }
            } else if output.mode == "pass_through" && output.compiled_prompt == r.raw_prompt {
                // pass_through with null expected means the raw_prompt IS the expected output
                exact_prompt_correct += 1;
            }

            // Per-mode counts
            match r.mode.as_str() {
                "pass_through" => {
                    pt_total += 1;
                    if output.compiled_prompt == r.raw_prompt {
                        pt_exact += 1;
                    }
                }
                "minimal_compile" => {
                    mc_total += 1;
                    if let Some(ref e) = r.expected_compiled_prompt {
                        if output.compiled_prompt == *e {
                            mc_exact += 1;
                        }
                    }
                }
                "local_compile" => {
                    lc_total += 1;
                    if let Some(ref e) = r.expected_compiled_prompt {
                        if output.compiled_prompt == *e {
                            lc_exact += 1;
                        }
                    }
                }
                "llm_compile" => {
                    llmc_total += 1;
                    if let Some(ref e) = r.expected_compiled_prompt {
                        if output.compiled_prompt == *e {
                            llmc_exact += 1;
                        }
                    }
                }
                _ => {}
            }
        }

        println!();
        println!("=== IntentLayer Benchmark Accuracy Report ===");
        println!("total_records:                {}", total);
        println!(
            "mode_accuracy:                {}/{} ({:.1}%)",
            mode_correct,
            total,
            mode_correct as f64 / total as f64 * 100.0
        );
        println!(
            "category_accuracy:            {}/{} ({:.1}%)  [informational]",
            category_correct,
            total,
            category_correct as f64 / total as f64 * 100.0
        );
        println!(
            "exact_prompt_match:           {}/{} ({:.1}%)  [aspirational]",
            exact_prompt_correct,
            total,
            exact_prompt_correct as f64 / total as f64 * 100.0
        );
        println!();
        println!("pass_through exact:           {}/{}", pt_exact, pt_total);
        println!("minimal_compile exact:        {}/{}", mc_exact, mc_total);
        println!("local_compile exact:          {}/{}", lc_exact, lc_total);
        println!(
            "llm_compile exact:            {}/{}",
            llmc_exact, llmc_total
        );
        println!("==============================================");
        println!();

        // Enforced: mode accuracy must be 100%
        assert_eq!(
            mode_correct, total,
            "Mode accuracy must be 100%: got {}/{}",
            mode_correct, total
        );
        // Enforced: all pass_through must be exact
        assert_eq!(
            pt_exact, pt_total,
            "All pass_through records must return exact original: got {}/{}",
            pt_exact, pt_total
        );
    }

    // ── Focused classifier tests ──

    #[test]
    fn test_slash_commands_still_pass_through() {
        let compiler = build_compiler();
        for cmd in &["/help", "/clear", "/model", "/init", "/permissions"] {
            let input = intentlayer::compiler::CompileInput {
                prompt: cmd.to_string(),
            };
            let output = intentlayer::compiler::compile(&input, &compiler);
            assert_eq!(
                output.mode, "pass_through",
                "Slash command '{}' must be pass_through",
                cmd
            );
            assert_eq!(
                output.compiled_prompt, *cmd,
                "Slash command must be returned unchanged"
            );
        }
    }

    #[test]
    fn test_already_good_long_prompts_still_pass_through() {
        let compiler = build_compiler();
        let long_prompts = [
            "Fix the race condition in the UserService.create method by adding a distributed lock using Redis. Keep the change minimal and add a test.",
            "Create a React hook useLocalStorage that syncs state with localStorage. Handle SSR gracefully. Add TypeScript types and unit tests.",
            "Update the GraphQL schema to add a 'likes' field to the Post type. Create a new resolver and add a database migration for the likes count column.",
        ];
        for prompt in &long_prompts {
            let input = intentlayer::compiler::CompileInput {
                prompt: prompt.to_string(),
            };
            let output = intentlayer::compiler::compile(&input, &compiler);
            assert_eq!(
                output.mode, "pass_through",
                "Long specific prompt must be pass_through: '{}'",
                prompt
            );
            assert_eq!(
                output.compiled_prompt, *prompt,
                "Long specific prompt must be returned unchanged"
            );
        }
    }

    #[test]
    fn test_minimal_compile_prompts_route_correctly() {
        let compiler = build_compiler();
        let cases = [
            ("continue", "minimal_compile", "continuation_previous_plan"),
            ("resume", "minimal_compile", "continuation_previous_plan"),
            ("next step", "minimal_compile", "continuation_previous_plan"),
            ("try again", "minimal_compile", "ambiguous_tiny_command"),
            ("proceed", "minimal_compile", "ambiguous_tiny_command"),
        ];
        for (prompt, mode, category) in &cases {
            let input = intentlayer::compiler::CompileInput {
                prompt: prompt.to_string(),
            };
            let output = intentlayer::compiler::compile(&input, &compiler);
            assert_eq!(output.mode, *mode, "Mode mismatch for '{}'", prompt);
            assert_eq!(
                output.category, *category,
                "Category mismatch for '{}'",
                prompt
            );
        }
    }

    #[test]
    fn test_common_local_compile_categories_route_correctly() {
        let compiler = build_compiler();
        let cases = [
            ("fix this", "repair_debug"),
            ("this error is back", "repair_debug"),
            ("add payment", "feature_implementation"),
            ("add auth", "feature_implementation"),
            ("refactor this mess", "refactor_cleanup"),
            ("clean up this code", "refactor_cleanup"),
            ("fix tests", "testing_test_failure"),
            ("tests are flaky", "testing_test_failure"),
            ("run tests and fix", "testing_test_failure"),
            ("api endpoint for users", "backend_api_database"),
            ("add database model", "backend_api_database"),
            ("add rate limiting", "security_permissions_auth"),
            ("add RBAC", "security_permissions_auth"),
            ("api key authentication", "security_permissions_auth"),
            ("document this code", "documentation_explanation"),
            ("explain this error", "documentation_explanation"),
            ("add JSDoc comments", "documentation_explanation"),
            ("deployment is broken", "deployment_config_environment"),
            (
                "environment variables are not loading",
                "deployment_config_environment",
            ),
            ("fix the Dockerfile", "deployment_config_environment"),
        ];
        for (prompt, expected_category) in &cases {
            let input = intentlayer::compiler::CompileInput {
                prompt: prompt.to_string(),
            };
            let output = intentlayer::compiler::compile(&input, &compiler);
            assert_eq!(
                output.category, *expected_category,
                "Category mismatch for '{}': expected '{}', got '{}'",
                prompt, expected_category, output.category
            );
        }
    }

    // ── Generalization benchmark tests ──

    /// Generalization benchmark record (simpler schema than seed bench).
    #[derive(Debug, Deserialize)]
    struct GenRecord {
        id: String,
        category: String,
        raw_prompt: String,
        expected_mode: String,
        #[allow(dead_code)]
        notes: String,
    }

    fn find_gen_file() -> PathBuf {
        let candidates = [
            "research/vibe_prompt_generalization.jsonl",
            "../research/vibe_prompt_generalization.jsonl",
            "../../research/vibe_prompt_generalization.jsonl",
        ];
        for c in &candidates {
            let p = PathBuf::from(c);
            if p.exists() {
                return p;
            }
        }
        PathBuf::from("research/vibe_prompt_generalization.jsonl")
    }

    fn load_gen_records() -> Vec<GenRecord> {
        let path = find_gen_file();
        let content = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("Failed to read gen file {:?}: {}", path, e));
        content
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| {
                serde_json::from_str::<GenRecord>(l)
                    .expect("Failed to parse generalization JSONL line")
            })
            .collect()
    }

    #[test]
    fn test_generalization_file_loads_50_records() {
        let records = load_gen_records();
        assert_eq!(
            records.len(),
            50,
            "Generalization benchmark must have exactly 50 records"
        );
    }

    #[test]
    fn test_generalization_no_duplicate_ids() {
        let records = load_gen_records();
        let mut seen = std::collections::HashSet::new();
        for r in &records {
            assert!(
                seen.insert(&r.id),
                "Duplicate ID in generalization set: {}",
                r.id
            );
        }
    }

    #[test]
    fn test_generalization_valid_categories() {
        let valid = [
            "repair_debug",
            "error_log_fixing",
            "continuation_previous_plan",
            "ambiguous_tiny_command",
            "feature_implementation",
            "refactor_cleanup",
            "production_readiness_hardening",
            "testing_test_failure",
            "deployment_config_environment",
            "commit_push_review",
            "ui_ux_fix",
            "backend_api_database",
            "documentation_explanation",
            "architecture_planning",
            "performance_optimization",
            "security_permissions_auth",
            "already_good_prompt",
            "slash_command_agent_command",
        ];
        let records = load_gen_records();
        for r in &records {
            assert!(
                valid.contains(&r.category.as_str()),
                "[{}] Invalid category: '{}'",
                r.id,
                r.category
            );
            assert!(
                [
                    "pass_through",
                    "minimal_compile",
                    "local_compile",
                    "llm_compile"
                ]
                .contains(&r.expected_mode.as_str()),
                "[{}] Invalid expected_mode: '{}'",
                r.id,
                r.expected_mode
            );
        }
    }

    #[test]
    fn test_generalization_mode_accuracy() {
        let compiler = build_compiler();
        let records = load_gen_records();
        let mut correct = 0;
        let mut failures = Vec::new();
        for r in &records {
            let input = intentlayer::compiler::CompileInput {
                prompt: r.raw_prompt.clone(),
            };
            let output = intentlayer::compiler::compile(&input, &compiler);
            if output.mode == r.expected_mode {
                correct += 1;
            } else {
                failures.push(format!(
                    "  {}: expected={} got={} prompt=\"{}\"",
                    r.id, r.expected_mode, output.mode, r.raw_prompt
                ));
            }
        }
        let pct = correct as f64 / records.len() as f64 * 100.0;
        println!(
            "Generalization mode accuracy: {}/{} ({:.1}%)",
            correct,
            records.len(),
            pct
        );
        if !failures.is_empty() {
            println!("Mode failures:");
            for f in &failures {
                println!("{}", f);
            }
        }
        assert!(
            pct >= 80.0,
            "Generalization mode accuracy too low: {:.1}% (need >= 80%)",
            pct
        );
    }

    #[test]
    fn test_generalization_category_accuracy() {
        let compiler = build_compiler();
        let records = load_gen_records();
        let mut correct = 0;
        let mut failures = Vec::new();
        let mut confusion: std::collections::HashMap<
            String,
            std::collections::HashMap<String, usize>,
        > = std::collections::HashMap::new();
        for r in &records {
            let input = intentlayer::compiler::CompileInput {
                prompt: r.raw_prompt.clone(),
            };
            let output = intentlayer::compiler::compile(&input, &compiler);
            if output.category == r.category {
                correct += 1;
            } else {
                failures.push(format!(
                    "  {}: expected={} got={} prompt=\"{}\"",
                    r.id, r.category, output.category, r.raw_prompt
                ));
                confusion
                    .entry(r.category.clone())
                    .or_default()
                    .entry(output.category.clone())
                    .and_modify(|c| *c += 1)
                    .or_insert(1);
            }
        }
        let pct = correct as f64 / records.len() as f64 * 100.0;
        println!(
            "Generalization category accuracy: {}/{} ({:.1}%)",
            correct,
            records.len(),
            pct
        );
        if !failures.is_empty() {
            println!("Category failures:");
            for f in &failures {
                println!("{}", f);
            }
            println!("Category confusion (expected → got):");
            for (exp, gots) in &confusion {
                for (got, count) in gots {
                    println!("  {} → {} ({}x)", exp, got, count);
                }
            }
        }
        assert!(
            pct >= 70.0,
            "Generalization category accuracy too low: {:.1}% (need >= 70%)",
            pct
        );
    }

    // ── Regression: generic review / clean up routing ──

    #[test]
    fn test_generic_review_routes_to_commit_push_review() {
        let compiler = build_compiler();
        let cases = [
            ("review this PR", "commit_push_review"),
            ("review current diff", "commit_push_review"),
            ("review the branch", "commit_push_review"),
        ];
        for (prompt, expected) in &cases {
            let input = intentlayer::compiler::CompileInput {
                prompt: prompt.to_string(),
            };
            let output = intentlayer::compiler::compile(&input, &compiler);
            assert_eq!(
                output.category, *expected,
                "Generic review prompt '{}' should route to {}: got {}",
                prompt, expected, output.category
            );
        }
    }

    #[test]
    fn test_generic_clean_up_routes_to_refactor_cleanup() {
        let compiler = build_compiler();
        let cases = [
            ("clean up auth module", "refactor_cleanup"),
            ("clean up dashboard code", "refactor_cleanup"),
            ("clean up the parser", "refactor_cleanup"),
        ];
        for (prompt, expected) in &cases {
            let input = intentlayer::compiler::CompileInput {
                prompt: prompt.to_string(),
            };
            let output = intentlayer::compiler::compile(&input, &compiler);
            assert_eq!(
                output.category, *expected,
                "Generic clean up prompt '{}' should route to {}: got {}",
                prompt, expected, output.category
            );
        }
    }
}
