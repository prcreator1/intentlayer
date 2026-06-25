use intentlayer::compiler::{CompileInput, Compiler};
use intentlayer::rules::RuleSet;
use std::io::Read;
use std::path::Path;

fn main() {
    let rules_path = Path::new("research/transformation_rules.json");
    if !rules_path.exists() {
        eprintln!("Error: research/transformation_rules.json not found");
        std::process::exit(1);
    }

    let rules = match RuleSet::load(rules_path) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Error loading rules: {}", e);
            std::process::exit(1);
        }
    };

    let compiler = Compiler::new(rules);

    let mut input_str = String::new();
    if std::io::stdin().read_to_string(&mut input_str).is_err() {
        eprintln!("Error reading stdin");
        std::process::exit(1);
    }

    let input: CompileInput = match serde_json::from_str(&input_str) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("Error parsing input JSON: {}", e);
            std::process::exit(1);
        }
    };

    let output = compiler.compile_prompt(&input.prompt);
    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}