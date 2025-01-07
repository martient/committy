pub const COMMIT_TYPES: &[&str] = &[
    "feat", "fix", "build", "chore", "ci", "cd", "docs", "perf", "refactor", "revert", "style",
    "test", "security",
];

pub const MAX_SHORT_DESCRIPTION_LENGTH: usize = 150;

pub const MAJOR_REGEX: &str = r"(?im)^(breaking change:|feat(?:\s*\([^)]*\))?!:)";
pub const MINOR_REGEX: &str = r"(?im)^feat(?:\s*\([^)]*\))?:";
pub const PATCH_REGEX: &str = r"(?im)^(fix|docs|style|refactor|perf|test|chore|ci|cd|build|revert|security)(?:\s*\([^)]*\))?:";
