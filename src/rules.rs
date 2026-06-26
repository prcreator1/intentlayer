//! Loader and data structures for `transformation_rules.json`.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub rule_id: String,
    pub category: String,
    pub raw_patterns: Vec<String>,
    pub risk: String,
    pub transformation_principle: String,
    pub compact_rewrite_template: Option<String>,
    pub must_preserve: Vec<String>,
    pub must_not_invent: Vec<String>,
    pub max_expansion_guidance: String,
    pub mode_recommendation: String,
}

#[derive(Debug, Clone)]
pub struct RuleSet {
    pub rules: Vec<Rule>,
}

/// Strip bracketed placeholders from a pattern for matching.
/// E.g. "add [feature]" → "add ", "fix this" → "fix this".
fn strip_brackets(pattern: &str) -> String {
    // Remove everything inside square brackets and trim trailing whitespace.
    let mut result = String::new();
    let mut in_brackets = false;
    for ch in pattern.chars() {
        match ch {
            '[' => in_brackets = true,
            ']' => in_brackets = false,
            _ if !in_brackets => result.push(ch),
            _ => {}
        }
    }
    result.trim().to_string()
}

/// Check if `word` appears at a word boundary in `text`.
/// Matches: standalone word, word at start/end, word preceded/followed by non-alphanumeric.
fn matches_word_boundary(text: &str, word: &str) -> bool {
    if text == word {
        return true;
    }
    // Check if the word appears as a whole word (not part of a larger word like "builds" containing "build").
    // Split on non-alphanumeric boundaries and check exact match.
    for token in text.split(|c: char| !c.is_alphanumeric()) {
        if token == word {
            return true;
        }
    }
    false
}

impl RuleSet {
    /// Load rules from a JSON file.
    pub fn load(path: &Path) -> Result<Self, String> {
        let data = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read rules file {:?}: {}", path, e))?;
        let rules: Vec<Rule> = serde_json::from_str(&data)
            .map_err(|e| format!("Failed to parse rules JSON: {}", e))?;
        Ok(RuleSet { rules })
    }

    /// Parse rules from a JSON string (useful for tests).
    pub fn from_json(json: &str) -> Result<Self, String> {
        let rules: Vec<Rule> =
            serde_json::from_str(json).map_err(|e| format!("Failed to parse rules JSON: {}", e))?;
        Ok(RuleSet { rules })
    }

    /// Find the best matching rule for a given prompt by checking raw_patterns.
    ///
    /// Matching strategy:
    /// - Patterns without brackets: case-insensitive substring check.
    ///   E.g. "fix this" matches "fix this repo".
    ///
    /// - Patterns with brackets (templates like "add [feature]"):
    ///   strip brackets, then check if prompt contains the stripped part.
    ///   E.g. "add [feature]" → stripped "add" → matches "add payment".
    ///
    /// For short stripped patterns (single words ≤7 chars), require word
    /// boundary match to avoid false positives (e.g. "build" matching "builds").
    ///
    /// Longer stripped patterns are preferred to avoid overly generic matches.
    pub fn match_prompt(&self, prompt: &str) -> Option<&Rule> {
        let lower = prompt.to_lowercase();

        // Sort rules by length of stripped pattern (longest first) for specificity.
        let mut indexed: Vec<(&Rule, Vec<String>)> = self
            .rules
            .iter()
            .map(|r| {
                let stripped: Vec<String> =
                    r.raw_patterns.iter().map(|p| strip_brackets(p)).collect();
                (r, stripped)
            })
            .collect();

        // Sort by max stripped pattern length (descending) — longer patterns first.
        indexed.sort_by(|a, b| {
            let a_max = a.1.iter().map(|s| s.len()).max().unwrap_or(0);
            let b_max = b.1.iter().map(|s| s.len()).max().unwrap_or(0);
            b_max.cmp(&a_max)
        });

        for (rule, stripped_patterns) in &indexed {
            for sp in stripped_patterns {
                if sp.is_empty() {
                    continue;
                }
                let sp_lower = sp.to_lowercase();
                let sp_words: Vec<&str> = sp_lower.split_whitespace().collect();
                let single_word = sp_words.len() == 1;

                // For short single-word patterns (≤7 chars), require word boundary match.
                if single_word && sp_lower.len() <= 7 {
                    if matches_word_boundary(&lower, &sp_lower) {
                        return Some(rule);
                    }
                } else if lower.contains(&sp_lower) {
                    return Some(rule);
                }
            }
        }
        None
    }

    /// Find a rule by its category name (not pattern matching).
    /// Returns the first rule matching the given category.
    pub fn find_by_category(&self, category: &str) -> Option<&Rule> {
        self.rules.iter().find(|r| r.category == category)
    }

    /// Find a rule by its mode recommendation for a given category.
    pub fn find_by_category_and_mode(&self, category: &str, mode: &str) -> Option<&Rule> {
        self.rules
            .iter()
            .find(|r| r.category == category && r.mode_recommendation == mode)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_brackets() {
        assert_eq!(strip_brackets("add [feature]"), "add");
        assert_eq!(strip_brackets("fix this"), "fix this");
        assert_eq!(strip_brackets("/help"), "/help");
        assert_eq!(strip_brackets("add [feature] "), "add");
    }
}
