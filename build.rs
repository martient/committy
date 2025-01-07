use std::env;
use std::fs;

fn main() {
    // Read the SENTRY_DSN environment variable if exists take it otherwise put undefined
    let sentry_dsn = env::var("SENTRY_DSN").unwrap_or_else(|_| String::from("undefined"));
    // Write it to an environment file or compile-time constant
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = std::path::Path::new(&out_dir).join("sentry_dsn.rs");
    fs::write(
        &dest_path,
        format!(r#"pub const SENTRY_DSN: &str = "{}";"#, sentry_dsn),
    )
    .unwrap();
}
