use std::sync::Once;

static INIT: Once = Once::new();

pub fn setup_test_env() {
    INIT.call_once(|| {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("off"))
            .is_test(true)
            .init();
    });
}
