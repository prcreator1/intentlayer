//! Invention guard — checks compiled prompts for banned provider/framework names.

/// Provider names that must never appear in a compiled prompt unless the user
/// explicitly named them in the raw prompt.
const BANNED_PROVIDERS: &[&str] = &[
    "Stripe",
    "Auth0",
    "Clerk",
    "Supabase",
    "Cognito",
    "Firebase",
    "AWS",
    "GCP",
    "Azure",
    "Vercel",
    "Netlify",
    "Heroku",
    "DigitalOcean",
    "Elasticsearch",
    "Algolia",
    "Meilisearch",
    "SendGrid",
    "Mailgun",
    "Postmark",
    "Cloudinary",
    "ImageKit",
    "Sentry",
    "Datadog",
    "New Relic",
    "Stripe Checkout",
    "PayPal",
    "Braintree",
    "Cloudflare",
];

/// Check a compiled prompt for banned provider names.
///
/// Returns a list of warning strings for any banned names found that are NOT
/// present in the original raw prompt (i.e., were invented).
pub fn check_invention(raw: &str, compiled: &str) -> Vec<String> {
    let raw_lower = raw.to_lowercase();
    let compiled_lower = compiled.to_lowercase();

    let mut warnings = Vec::new();

    for provider in BANNED_PROVIDERS {
        let p_lower = provider.to_lowercase();
        if compiled_lower.contains(&p_lower) && !raw_lower.contains(&p_lower) {
            warnings.push(format!("Invented provider name detected: '{}'", provider));
        }
    }

    warnings
}