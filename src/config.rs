pub const SENTRY_DSN: &str = "https://994880aa262e93af0026738f64a5576c@o4508086241394688.ingest.de.sentry.io/4508126163107920";

pub const COMMIT_TYPES: &[&str] = &[
    "feat", "fix", "build", "chore", "ci", "cd", "docs", "perf", "refactor", "revert", "style",
    "test",
];

pub const MAX_SHORT_DESCRIPTION_LENGTH: usize = 150;

pub const MAJOR_REGEX: &str = r"(?im)^(breaking change:|feat(?:\s*\([^)]*\))?!:)";
pub const MINOR_REGEX: &str = r"(?im)^feat(?:\s*\([^)]*\))?:";
pub const PATCH_REGEX: &str = r"(?im)^(fix|docs|style|refactor|perf|test|chore|ci|cd|build|revert)(?:\s*\([^)]*\))?:";