use once_cell::sync::Lazy;
use std::env;
use std::sync::Once;
use tempfile::TempDir;

#[allow(dead_code)]
static INIT: Once = Once::new();

#[allow(dead_code)]
pub fn setup_test_env() {
    INIT.call_once(|| {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("off"))
            .is_test(true)
            .init();

        // Force non-interactive behavior across all tests and skip update prompts
        env::set_var("COMMITTY_NONINTERACTIVE", "1");
        env::set_var("CI", "1");
        // Silence noisy network-related logs
        env::set_var("RUST_LOG", "off");

        // Create and use a persistent temporary HOME for the whole test process
        static TEST_HOME: Lazy<TempDir> =
            Lazy::new(|| tempfile::tempdir().expect("Failed to create temp HOME for tests"));
        env::set_var("HOME", TEST_HOME.path());
    });
}
