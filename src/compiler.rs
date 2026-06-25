//! Core compiler: routes a raw prompt through classification → mode → output.

use crate::classifier::{classify, Mode};
use crate::guard::check_invention;
use crate::rules::RuleSet;

use serde::{Deserialize, Serialize};

/// Input JSON: `{"prompt": "fix this repo"}`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompileInput {
    pub prompt: String,
}

/// Output JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompileOutput {
    pub original_prompt: String,
    pub compiled_prompt: String,
    pub mode: String,
    pub category: String,
    pub changed: bool,
    pub warnings: Vec<String>,
}

/// The compiler holds a loaded [`RuleSet`] for pattern matching and templates.
#[derive(Debug, Clone)]
pub struct Compiler {
    pub rules: RuleSet,
}

impl Compiler {
    pub fn new(rules: RuleSet) -> Self {
        Compiler { rules }
    }

    /// Compile a single prompt string.
    pub fn compile_prompt(&self, prompt: &str) -> CompileOutput {
        let classification = classify(prompt, &self.rules);

        let (compiled_prompt, changed) = match classification.mode {
            Mode::PassThrough => (prompt.to_string(), false),

            Mode::MinimalCompile => {
                let compiled = self.apply_minimal_compile(prompt, &classification);
                let changed = compiled != prompt;
                (compiled, changed)
            }

            Mode::LocalCompile => {
                let compiled = self.apply_local_compile(prompt, &classification);
                let changed = compiled != prompt;
                (compiled, changed)
            }

            Mode::LlmCompile => {
                // v0.1 stub: no real model call — use local template as fallback
                let compiled = self.apply_llm_compile_stub(prompt, &classification);
                let changed = compiled != prompt;
                (compiled, changed)
            }
        };

        // Invention guard
        let warnings = check_invention(prompt, &compiled_prompt);

        // Check for forbidden clarification questions
        let clarification_warning = self.check_clarification(&compiled_prompt, &classification);
        if let Some(w) = clarification_warning {
            let mut all_warnings = warnings;
            all_warnings.push(w);
            return CompileOutput {
                original_prompt: prompt.to_string(),
                compiled_prompt,
                mode: classification.mode.to_string(),
                category: classification.category,
                changed,
                warnings: all_warnings,
            };
        }

        CompileOutput {
            original_prompt: prompt.to_string(),
            compiled_prompt,
            mode: classification.mode.to_string(),
            category: classification.category,
            changed,
            warnings,
        }
    }

    /// minimal_compile: small 1-15 token expansion.
    fn apply_minimal_compile(
        &self,
        prompt: &str,
        classification: &crate::classifier::Classification,
    ) -> String {
        let lower = prompt.to_lowercase();

        // Check for specific tiny prompts that need minimal expansion
        let compiled: String = match lower.as_str() {
            "continue" => "Continue from current state.".to_string(),
            "resume" => "Resume previous work.".to_string(),
            "next step" => "Proceed to next step.".to_string(),
            "try again" => "Retry previous action.".to_string(),
            "proceed" => "Proceed with current context.".to_string(),
            "do what we discussed" => "Proceed with discussed plan.".to_string(),
            "same plan continue" => "Continue existing plan.".to_string(),
            "i think i have broken you" => "Continue normally.".to_string(),
            "same issue as before" => {
                "Re-apply previous fix. Adjust if context changed.".to_string()
            }
            _ => {
                if classification.category == "continuation_previous_plan" {
                    format!("Continue from current context: {}", prompt)
                } else {
                    format!("Proceed with current context: {}", prompt)
                }
            }
        };

        enforce_token_cap(compiled, 15)
    }

    /// local_compile: category-based rewrite using rule templates.
    fn apply_local_compile(
        &self,
        _prompt: &str,
        classification: &crate::classifier::Classification,
    ) -> String {
        // Find the matching rule
        if let Some(rule_id) = &classification.rule_id {
            if let Some(rule) = self.rules.rules.iter().find(|r| &r.rule_id == rule_id) {
                if let Some(template) = &rule.compact_rewrite_template {
                    return enforce_token_cap(template.clone(), 90);
                }
            }
        }

        // Fallback: find a rule by category
        if let Some(rule) = self
            .rules
            .find_by_category_and_mode(&classification.category, "local_compile")
        {
            if let Some(template) = &rule.compact_rewrite_template {
                return enforce_token_cap(template.clone(), 90);
            }
        }

        // Generic fallback
        "Using the current project context, implement the requested change. Make the smallest safe change, verify where practical, and report files changed.".to_string()
    }

    /// llm_compile stub: no real model call yet.
    fn apply_llm_compile_stub(
        &self,
        _prompt: &str,
        classification: &crate::classifier::Classification,
    ) -> String {
        // Use the first matching rule template for this category
        if let Some(rule) = self
            .rules
            .find_by_category_and_mode(&classification.category, "llm_compile")
        {
            if let Some(template) = &rule.compact_rewrite_template {
                return enforce_token_cap(template.clone(), 120);
            }
        }

        // Fallback for architecture/planning
        "Propose a minimal viable architecture. Do not assume a stack. State tradeoffs but do not overbuild. Request confirmation before implementing.".into()
    }

    /// Check if the compiled prompt asks a forbidden clarification question.
    fn check_clarification(
        &self,
        compiled: &str,
        _classification: &crate::classifier::Classification,
    ) -> Option<String> {
        let lower = compiled.to_lowercase();

        // Forbidden clarification patterns
        let forbidden = [
            "which repo",
            "which error",
            "which file",
            "what do you mean",
            "can you clarify",
            "which test",
            "please specify",
        ];

        for pattern in &forbidden {
            if lower.contains(pattern) {
                return Some(format!(
                    "Forbidden clarification question detected: contains '{}'",
                    pattern
                ));
            }
        }

        None
    }
}

/// Functional entry point: compile a prompt string directly.
pub fn compile(input: &CompileInput, compiler: &Compiler) -> CompileOutput {
    compiler.compile_prompt(&input.prompt)
}

/// Roughly enforce a token cap by truncating at the given number of words.
/// This is a v0.1 approximation — a real tokenizer would be more precise.
fn enforce_token_cap(text: String, max_tokens: usize) -> String {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.len() <= max_tokens {
        return text;
    }
    words[..max_tokens].join(" ") + "..."
}
