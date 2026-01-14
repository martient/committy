use serial_test::serial;

use committy::tui::state::{AppState, CommitFormField};
use git2::{Repository, Signature};
use tempfile::TempDir;

/// Helper to create a test repository with initial commit
fn setup_test_repo() -> (TempDir, Repository) {
    let dir = TempDir::new().unwrap();
    let repo = Repository::init(dir.path()).unwrap();

    // Create initial commit
    let sig = Signature::now("Test User", "test@example.com").unwrap();
    let tree_id = {
        let mut index = repo.index().unwrap();
        index.write_tree().unwrap()
    };
    {
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            "Initial commit",
            &tree,
            &[],
        )
        .unwrap();
    }

    (dir, repo)
}

#[test]
#[serial]
fn test_commit_form_initial_state() {
    let (dir, _repo) = setup_test_repo();
    std::env::set_current_dir(dir.path()).unwrap();

    let state = AppState::new().unwrap();

    assert_eq!(state.commit_type, "feat");
    assert_eq!(state.commit_scope, "");
    assert_eq!(state.commit_message, "");
    assert_eq!(state.commit_body, "");
    assert!(!state.breaking_change);
    assert_eq!(state.current_field, CommitFormField::Type);
}

#[test]
#[serial]
fn test_commit_type_cycling_all_types() {
    let (dir, _repo) = setup_test_repo();
    std::env::set_current_dir(dir.path()).unwrap();

    let mut state = AppState::new().unwrap();

    let expected_types = vec![
        "feat", "fix", "build", "chore", "ci", "cd", "docs",
        "perf", "refactor", "revert", "style", "test", "security", "config"
    ];

    for (i, expected_type) in expected_types.iter().enumerate() {
        assert_eq!(state.commit_type, *expected_type);
        assert_eq!(state.commit_type_index, i);
        state.cycle_commit_type();
    }

    // Should wrap back to first
    assert_eq!(state.commit_type, "feat");
    assert_eq!(state.commit_type_index, 0);
}

#[test]
#[serial]
fn test_field_navigation_forward() {
    let (dir, _repo) = setup_test_repo();
    std::env::set_current_dir(dir.path()).unwrap();

    let mut state = AppState::new().unwrap();

    let fields = vec![
        CommitFormField::Type,
        CommitFormField::Scope,
        CommitFormField::ShortMessage,
        CommitFormField::LongMessage,
        CommitFormField::BreakingChange,
    ];

    for (i, expected_field) in fields.iter().enumerate() {
        assert_eq!(state.current_field, *expected_field, "Failed at step {}", i);
        state.next_field();
    }

    // Should wrap to beginning
    assert_eq!(state.current_field, CommitFormField::Type);
}

#[test]
#[serial]
fn test_field_navigation_backward() {
    let (dir, _repo) = setup_test_repo();
    std::env::set_current_dir(dir.path()).unwrap();

    let mut state = AppState::new().unwrap();

    // Start at Type, go backwards
    assert_eq!(state.current_field, CommitFormField::Type);

    state.prev_field();
    assert_eq!(state.current_field, CommitFormField::BreakingChange);

    state.prev_field();
    assert_eq!(state.current_field, CommitFormField::LongMessage);

    state.prev_field();
    assert_eq!(state.current_field, CommitFormField::ShortMessage);

    state.prev_field();
    assert_eq!(state.current_field, CommitFormField::Scope);

    state.prev_field();
    assert_eq!(state.current_field, CommitFormField::Type);
}

#[test]
#[serial]
fn test_field_navigation_round_trip() {
    let (dir, _repo) = setup_test_repo();
    std::env::set_current_dir(dir.path()).unwrap();

    let mut state = AppState::new().unwrap();

    let start_field = state.current_field;

    // Go forward 5 times (full cycle)
    for _ in 0..5 {
        state.next_field();
    }

    assert_eq!(state.current_field, start_field);

    // Go backward 5 times (full cycle)
    for _ in 0..5 {
        state.prev_field();
    }

    assert_eq!(state.current_field, start_field);
}

#[test]
#[serial]
fn test_text_input_scope() {
    let (dir, _repo) = setup_test_repo();
    std::env::set_current_dir(dir.path()).unwrap();

    let mut state = AppState::new().unwrap();

    // Navigate to Scope field
    state.next_field();
    assert_eq!(state.current_field, CommitFormField::Scope);

    // Simulate typing
    state.commit_scope.push_str("api");

    assert_eq!(state.commit_scope, "api");
}

#[test]
#[serial]
fn test_text_input_short_message() {
    let (dir, _repo) = setup_test_repo();
    std::env::set_current_dir(dir.path()).unwrap();

    let mut state = AppState::new().unwrap();

    // Navigate to ShortMessage field
    state.next_field(); // Scope
    state.next_field(); // ShortMessage
    assert_eq!(state.current_field, CommitFormField::ShortMessage);

    // Simulate typing
    state.commit_message.push_str("add user authentication");

    assert_eq!(state.commit_message, "add user authentication");
}

#[test]
#[serial]
fn test_text_input_long_message() {
    let (dir, _repo) = setup_test_repo();
    std::env::set_current_dir(dir.path()).unwrap();

    let mut state = AppState::new().unwrap();

    // Navigate to LongMessage field
    for _ in 0..3 {
        state.next_field();
    }
    assert_eq!(state.current_field, CommitFormField::LongMessage);

    // Simulate typing with newlines
    state.commit_body.push_str("This is a longer description.\n");
    state.commit_body.push_str("It can span multiple lines.\n");
    state.commit_body.push_str("Details about the implementation.");

    assert!(state.commit_body.contains("longer description"));
    assert!(state.commit_body.contains("\n"));
    assert!(state.commit_body.contains("implementation"));
}

#[test]
#[serial]
fn test_breaking_change_toggle() {
    let (dir, _repo) = setup_test_repo();
    std::env::set_current_dir(dir.path()).unwrap();

    let mut state = AppState::new().unwrap();

    assert!(!state.breaking_change);

    // Navigate to BreakingChange field
    for _ in 0..4 {
        state.next_field();
    }
    assert_eq!(state.current_field, CommitFormField::BreakingChange);

    // Toggle it
    state.breaking_change = !state.breaking_change;
    assert!(state.breaking_change);

    // Toggle again
    state.breaking_change = !state.breaking_change;
    assert!(!state.breaking_change);
}

#[test]
#[serial]
fn test_complete_commit_form_filling() {
    let (dir, _repo) = setup_test_repo();
    std::env::set_current_dir(dir.path()).unwrap();

    let mut state = AppState::new().unwrap();

    // Set commit type
    assert_eq!(state.current_field, CommitFormField::Type);
    state.cycle_commit_type(); // Change to "fix"
    assert_eq!(state.commit_type, "fix");

    // Set scope
    state.next_field();
    assert_eq!(state.current_field, CommitFormField::Scope);
    state.commit_scope.push_str("auth");

    // Set short message
    state.next_field();
    assert_eq!(state.current_field, CommitFormField::ShortMessage);
    state.commit_message.push_str("resolve token expiration bug");

    // Set long message
    state.next_field();
    assert_eq!(state.current_field, CommitFormField::LongMessage);
    state.commit_body.push_str("Fixed an issue where tokens were not being refreshed properly.\n");
    state.commit_body.push_str("Added additional validation for token expiration.");

    // Enable breaking change
    state.next_field();
    assert_eq!(state.current_field, CommitFormField::BreakingChange);
    state.breaking_change = true;

    // Verify all fields
    assert_eq!(state.commit_type, "fix");
    assert_eq!(state.commit_scope, "auth");
    assert_eq!(state.commit_message, "resolve token expiration bug");
    assert!(state.commit_body.contains("tokens were not being refreshed"));
    assert!(state.breaking_change);
}

#[test]
#[serial]
fn test_commit_form_with_empty_optional_fields() {
    let (dir, _repo) = setup_test_repo();
    std::env::set_current_dir(dir.path()).unwrap();

    let mut state = AppState::new().unwrap();

    // Only set type and short message (minimum required)
    assert_eq!(state.commit_type, "feat");

    // Navigate to short message
    state.next_field(); // Scope (skip)
    state.next_field(); // ShortMessage
    state.commit_message.push_str("add new feature");

    // Verify optional fields are empty
    assert_eq!(state.commit_scope, "");
    assert_eq!(state.commit_body, "");
    assert!(!state.breaking_change);

    // But required fields are filled
    assert_eq!(state.commit_type, "feat");
    assert_eq!(state.commit_message, "add new feature");
}

#[test]
#[serial]
fn test_commit_message_backspace_simulation() {
    let (dir, _repo) = setup_test_repo();
    std::env::set_current_dir(dir.path()).unwrap();

    let mut state = AppState::new().unwrap();

    // Navigate to short message
    state.next_field();
    state.next_field();
    assert_eq!(state.current_field, CommitFormField::ShortMessage);

    // Type a message
    state.commit_message.push_str("add feature");

    assert_eq!(state.commit_message, "add feature");

    // Simulate backspace (remove last char)
    state.commit_message.pop();
    assert_eq!(state.commit_message, "add featur");

    state.commit_message.pop();
    assert_eq!(state.commit_message, "add featu");
}

#[test]
#[serial]
fn test_scope_backspace_simulation() {
    let (dir, _repo) = setup_test_repo();
    std::env::set_current_dir(dir.path()).unwrap();

    let mut state = AppState::new().unwrap();

    // Navigate to scope
    state.next_field();
    assert_eq!(state.current_field, CommitFormField::Scope);

    // Type scope
    state.commit_scope.push_str("api");
    assert_eq!(state.commit_scope, "api");

    // Simulate backspace
    state.commit_scope.pop();
    assert_eq!(state.commit_scope, "ap");

    state.commit_scope.pop();
    state.commit_scope.pop();
    assert_eq!(state.commit_scope, "");
}

#[test]
#[serial]
fn test_multiple_commit_type_cycles() {
    let (dir, _repo) = setup_test_repo();
    std::env::set_current_dir(dir.path()).unwrap();

    let mut state = AppState::new().unwrap();

    // Cycle many times
    for _ in 0..50 {
        state.cycle_commit_type();
    }

    // Should still be valid and consistent (14 types total)
    assert_eq!(state.commit_type_index, 50 % 14);

    // Verify type matches index
    let expected_types = vec![
        "feat", "fix", "build", "chore", "ci", "cd", "docs",
        "perf", "refactor", "revert", "style", "test", "security", "config"
    ];
    assert_eq!(state.commit_type, expected_types[50 % 14]);
}

#[test]
#[serial]
fn test_commit_form_reset_simulation() {
    let (dir, _repo) = setup_test_repo();
    std::env::set_current_dir(dir.path()).unwrap();

    let mut state = AppState::new().unwrap();

    // Fill out the form
    state.cycle_commit_type();
    state.commit_scope.push_str("test");
    state.commit_message.push_str("test message");
    state.commit_body.push_str("test body");
    state.breaking_change = true;

    // Verify it's filled
    assert_eq!(state.commit_type, "fix");
    assert_eq!(state.commit_scope, "test");
    assert_eq!(state.commit_message, "test message");
    assert_eq!(state.commit_body, "test body");
    assert!(state.breaking_change);

    // Simulate reset by creating new state
    state = AppState::new().unwrap();

    // Verify it's reset
    assert_eq!(state.commit_type, "feat");
    assert_eq!(state.commit_scope, "");
    assert_eq!(state.commit_message, "");
    assert_eq!(state.commit_body, "");
    assert!(!state.breaking_change);
}