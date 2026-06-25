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
  --llm                 Enable LLM-assisted compilation (opt-in).
                        Requires --provider openrouter + --api-key-env.
  --provider <name>     LLM provider: openrouter
  --model <name>        Model name override (default: from config)
  --api-key-env <ENV>   Env var name holding API key. Never a raw key.
  --base-url <url>      Optional base URL override
  --timeout-seconds <n> Optional timeout (default: 30)
  --max-tokens <n>      Optional max tokens (default: 800)
  --temperature <n>     Optional temperature (default: 0.1)
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
    llm: bool,
    provider: Option<String>,
    #[allow(dead_code)]
    model: Option<String>,
    api_key_env: Option<String>,
    #[allow(dead_code)]
    base_url: Option<String>,
    #[allow(dead_code)]
    timeout_seconds: Option<u64>,
    #[allow(dead_code)]
    max_tokens: Option<u32>,
    #[allow(dead_code)]
    temperature: Option<f32>,
}

/// Manual CLI parser.  Avoids adding a dependency for v0.1.
fn parse_args() -> Result<Args, String> {
    let args: Vec<String> = std::env::args().collect();
    let mut prompt: Option<String> = None;
    let mut input: Option<PathBuf> = None;
    let mut rules_path = PathBuf::from("research/transformation_rules.json");
    let mut json = false;
    let mut compiled_only = false;
    let mut llm = false;
    let mut provider: Option<String> = None;
    let mut model: Option<String> = None;
    let mut api_key_env: Option<String> = None;
    let mut base_url: Option<String> = None;
    let mut timeout_seconds: Option<u64> = None;
    let mut max_tokens: Option<u32> = None;
    let mut temperature: Option<f32> = None;

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
            "--llm" => {
                llm = true;
            }
            "--provider" => {
                i += 1;
                if i >= args.len() || args[i].starts_with('-') {
                    return Err(
                        "Missing value for --provider. Expected: --provider openrouter".into(),
                    );
                }
                provider = Some(args[i].clone());
            }
            "--model" => {
                i += 1;
                if i >= args.len() || args[i].starts_with('-') {
                    return Err("Missing value for --model. Expected: --model gpt-4.1-mini".into());
                }
                model = Some(args[i].clone());
            }
            "--api-key-env" => {
                i += 1;
                if i >= args.len() || args[i].starts_with('-') {
                    return Err(
                        "Missing value for --api-key-env. Expected: --api-key-env OPENAI_API_KEY"
                            .into(),
                    );
                }
                api_key_env = Some(args[i].clone());
            }
            "--base-url" => {
                i += 1;
                if i >= args.len() || args[i].starts_with('-') {
                    return Err(
                        "Missing value for --base-url. Expected: --base-url https://...".into(),
                    );
                }
                base_url = Some(args[i].clone());
            }
            "--timeout-seconds" => {
                i += 1;
                if i >= args.len() || args[i].starts_with('-') {
                    return Err("Missing value for --timeout-seconds".into());
                }
                timeout_seconds =
                    Some(args[i].parse().map_err(|_| {
                        format!("Invalid number for --timeout-seconds: '{}'", args[i])
                    })?);
            }
            "--max-tokens" => {
                i += 1;
                if i >= args.len() || args[i].starts_with('-') {
                    return Err("Missing value for --max-tokens".into());
                }
                max_tokens = Some(
                    args[i]
                        .parse()
                        .map_err(|_| format!("Invalid number for --max-tokens: '{}'", args[i]))?,
                );
            }
            "--temperature" => {
                i += 1;
                if i >= args.len() || args[i].starts_with('-') {
                    return Err("Missing value for --temperature".into());
                }
                temperature = Some(
                    args[i]
                        .parse()
                        .map_err(|_| format!("Invalid number for --temperature: '{}'", args[i]))?,
                );
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
        llm,
        provider,
        model,
        api_key_env,
        base_url,
        timeout_seconds,
        max_tokens,
        temperature,
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

    // Validate LLM args
    if args.llm {
        if args.provider.is_none() {
            eprintln!("Error: --llm requires --provider openrouter");
            process::exit(1);
        }
        let p = args.provider.as_deref().unwrap();
        if p != "openrouter" && p != "groq" {
            eprintln!(
                "Error: unsupported LLM provider '{}'. Supported providers: openrouter, groq",
                p
            );
            process::exit(1);
        }
    }

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

    // Classification — must run before deciding LLM path
    let classification = {
        let rules = &compiler.rules;
        intentlayer::classifier::classify(&prompt_text, rules)
    };

    let llm_eligible = args.llm
        && (args.provider.as_deref() == Some("openrouter")
            || args.provider.as_deref() == Some("groq"))
        && classification.mode == intentlayer::classifier::Mode::LlmCompile;

    // Only require API key when an actual LLM call will be made
    if llm_eligible && args.api_key_env.is_none() {
        eprintln!("Error: --llm requires --api-key-env <ENV_VAR_NAME>");
        process::exit(1);
    }

    let output = if llm_eligible {
        match args.provider.as_deref() {
            Some("openrouter") => run_llm_openrouter(&prompt_text, &classification.category, &args),
            Some("groq") => run_llm_groq(&prompt_text, &classification.category, &args),
            _ => compiler.compile_prompt(&prompt_text),
        }
    } else {
        compiler.compile_prompt(&prompt_text)
    };

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

#[cfg(not(feature = "openrouter-http"))]
fn run_llm_openrouter(
    _prompt: &str,
    _category: &str,
    #[allow(unused_variables)] _args: &Args,
) -> intentlayer::compiler::CompileOutput {
    eprintln!("Error: OpenRouter HTTP transport is not enabled.");
    eprintln!("Rebuild with --features openrouter-http.");
    process::exit(1);
}

#[cfg(feature = "openrouter-http")]
fn run_llm_openrouter(
    prompt: &str,
    category: &str,
    args: &Args,
) -> intentlayer::compiler::CompileOutput {
    use intentlayer::llm::LlmEnvelopeOptions;
    use intentlayer::llm_config::{resolve_from_env, LlmProviderConfig};
    use intentlayer::llm_orchestrate::compile_with_llm_orchestration;
    use intentlayer::openrouter::{OpenRouterProvider, ReqwestOpenRouterTransport};

    let api_key_env = args.api_key_env.clone().unwrap_or_default();
    let config = LlmProviderConfig {
        provider: "openai-compatible".into(),
        base_url: args
            .base_url
            .clone()
            .or(Some("https://openrouter.ai/api/v1".into())),
        model: args.model.clone().unwrap_or_else(|| "gpt-4.1-mini".into()),
        api_key_env: Some(api_key_env),
        timeout_seconds: args.timeout_seconds.unwrap_or(30),
        max_tokens: args.max_tokens.unwrap_or(800),
        temperature: args.temperature.unwrap_or(0.1),
    };

    let resolved = match resolve_from_env(&config) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    };

    let transport = match ReqwestOpenRouterTransport::new(&resolved) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    };

    let provider = OpenRouterProvider::new(resolved, transport);
    let opts = LlmEnvelopeOptions::default();

    compile_with_llm_orchestration(prompt, category, &provider, &opts)
}

#[cfg(not(feature = "groq-http"))]
fn run_llm_groq(
    _prompt: &str,
    _category: &str,
    #[allow(unused_variables)] _args: &Args,
) -> intentlayer::compiler::CompileOutput {
    eprintln!("Error: Groq HTTP transport is not enabled.");
    eprintln!("Rebuild with --features groq-http.");
    process::exit(1);
}

#[cfg(feature = "groq-http")]
fn run_llm_groq(prompt: &str, category: &str, args: &Args) -> intentlayer::compiler::CompileOutput {
    use intentlayer::groq::{GroqProvider, ReqwestGroqTransport};
    use intentlayer::llm::LlmEnvelopeOptions;
    use intentlayer::llm_config::{resolve_from_env, LlmProviderConfig};
    use intentlayer::llm_orchestrate::compile_with_llm_orchestration;

    let api_key_env = args.api_key_env.clone().unwrap_or_default();
    let config = LlmProviderConfig {
        provider: "groq".into(),
        base_url: args
            .base_url
            .clone()
            .or(Some("https://api.groq.com/openai/v1".into())),
        model: args
            .model
            .clone()
            .unwrap_or_else(|| "llama-3.3-70b-versatile".into()),
        api_key_env: Some(api_key_env),
        timeout_seconds: args.timeout_seconds.unwrap_or(30),
        max_tokens: args.max_tokens.unwrap_or(800),
        temperature: args.temperature.unwrap_or(0.1),
    };

    let resolved = match resolve_from_env(&config) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    };

    let transport = match ReqwestGroqTransport::new(&resolved) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    };

    let provider = GroqProvider::new(resolved, transport);
    let opts = LlmEnvelopeOptions::default();

    compile_with_llm_orchestration(prompt, category, &provider, &opts)
}
