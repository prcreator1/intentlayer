use intentlayer::compiler::{CompileInput, Compiler};
use intentlayer::from_rules_file;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process;

const VERSION: &str = concat!(env!("CARGO_PKG_NAME"), " ", env!("CARGO_PKG_VERSION"));

const HELP_TEXT: &str = "\
Usage:
  intentlayer --prompt \"fix this repo\"
  intentlayer --input request.json
  intentlayer --rules-path path/to/rules.json [--prompt ... | --input ...]
  intentlayer --pretty
  intentlayer --json
  intentlayer --version
  intentlayer --help

Options:
  --prompt <text>       Compile the given prompt text directly.
  --input <path>        Read JSON input from a file.
                        File must contain {\"prompt\": \"...\"}.
  --rules-path <path>   Load transformation rules from a JSON file.
                        Default: research/transformation_rules.json
  --pretty              Output pretty-printed JSON (default).
  --json                Output compact JSON.
  --compiled-only       Print only compiled_prompt as plain text (no JSON).
                        Intended for direct handoff to downstream agents.
  --version             Print version and exit.
  --help                Show this help and exit.

When neither --prompt nor --input is provided, JSON is read from stdin.
";

struct Args {
    prompt: Option<String>,
    input: Option<PathBuf>,
    rules_path: PathBuf,
    json: bool,
    compiled_only: bool,
}

/// Manual CLI parser.  Avoids adding a dependency for v0.1.
fn parse_args() -> Result<Args, String> {
    let args: Vec<String> = std::env::args().collect();
    let mut prompt: Option<String> = None;
    let mut input: Option<PathBuf> = None;
    let mut rules_path = PathBuf::from("research/transformation_rules.json");
    let mut json = false;
    let mut compiled_only = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--help" | "-h" => {
                print!("{}", HELP_TEXT);
                process::exit(0);
            }
            "--version" => {
                println!("{}", VERSION);
                process::exit(0);
            }
            "--pretty" => {
                json = false;
            }
            "--json" => {
                json = true;
            }
            "--compiled-only" => {
                compiled_only = true;
            }
            "--prompt" => {
                i += 1;
                if i >= args.len() || args[i].starts_with('-') {
                    return Err(
                        "Missing value for --prompt. Expected: --prompt \"your text\"".into(),
                    );
                }
                prompt = Some(args[i].clone());
            }
            "--input" => {
                i += 1;
                if i >= args.len() || args[i].starts_with('-') {
                    return Err("Missing file path for --input. Expected: --input file.json".into());
                }
                input = Some(PathBuf::from(args[i].clone()));
            }
            "--rules-path" => {
                i += 1;
                if i >= args.len() || args[i].starts_with('-') {
                    return Err(
                        "Missing file path for --rules-path. Expected: --rules-path path/to/rules.json"
                            .into(),
                    );
                }
                rules_path = PathBuf::from(args[i].clone());
            }
            other => {
                return Err(format!(
                    "Unknown argument: '{}'. Use --help for usage.",
                    other
                ));
            }
        }
        i += 1;
    }

    // Validate conflicts
    if prompt.is_some() && input.is_some() {
        return Err(
            "Conflicting input sources: provide either --prompt OR --input, not both.".into(),
        );
    }

    Ok(Args {
        prompt,
        input,
        rules_path,
        json,
        compiled_only,
    })
}

fn load_compiler(rules_path: &Path) -> Result<Compiler, String> {
    from_rules_file(rules_path).map_err(|e| format!("Error loading rules: {}", e))
}

fn resolve_prompt(args: &Args) -> Result<String, String> {
    if let Some(ref text) = args.prompt {
        if text.trim().is_empty() {
            return Err("Prompt text is empty".into());
        }
        return Ok(text.clone());
    }

    if let Some(ref path) = args.input {
        let json_str = fs::read_to_string(path)
            .map_err(|e| format!("Cannot read input file '{}': {}", path.display(), e))?;
        let input: CompileInput = serde_json::from_str(&json_str)
            .map_err(|e| format!("Invalid JSON in input file '{}': {}", path.display(), e))?;
        if input.prompt.trim().is_empty() {
            return Err("Input file missing 'prompt' field or value is empty".into());
        }
        return Ok(input.prompt);
    }

    // Fallback: stdin JSON
    let mut stdin_bytes = Vec::new();
    std::io::stdin()
        .read_to_end(&mut stdin_bytes)
        .map_err(|e| format!("Error reading stdin: {}", e))?;
    let json_str =
        String::from_utf8(stdin_bytes).map_err(|e| format!("Invalid UTF-8 from stdin: {}", e))?;
    if json_str.trim().is_empty() {
        return Err(
            "No input provided. Use --prompt, --input, or pipe JSON to stdin. See --help.".into(),
        );
    }
    let input: CompileInput =
        serde_json::from_str(&json_str).map_err(|e| format!("Invalid JSON from stdin: {}", e))?;
    if input.prompt.trim().is_empty() {
        return Err("Stdin JSON missing 'prompt' field or value is empty".into());
    }
    Ok(input.prompt)
}

fn main() {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    };

    let compiler = match load_compiler(&args.rules_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{}", e);
            process::exit(1);
        }
    };

    let prompt_text = match resolve_prompt(&args) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    };

    let output = compiler.compile_prompt(&prompt_text);

    // compiled-only: plain text handoff to downstream agents
    if args.compiled_only {
        if !output.warnings.is_empty() {
            for w in &output.warnings {
                eprintln!("{}", w);
            }
            process::exit(1);
        }
        println!("{}", output.compiled_prompt);
        return;
    }

    let json_out = if args.json {
        serde_json::to_string(&output).unwrap()
    } else {
        serde_json::to_string_pretty(&output).unwrap()
    };

    println!("{}", json_out);
}
