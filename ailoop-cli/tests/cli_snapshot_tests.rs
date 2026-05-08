mod cli_tests;

use cli_tests::{get_help_text, get_version_text};

#[test]
fn test_help_includes_version() {
    let help_text = get_help_text().expect("Failed to get help text");
    let expected = format!("ailoop - {}", env!("CARGO_PKG_VERSION"));
    assert!(
        help_text.contains(&expected),
        "Help text should include version number '{}'\nActual: {}",
        expected,
        help_text
    );
}

#[test]
fn test_version_output() {
    let version_text = get_version_text().expect("Failed to get version text");
    let expected_version = env!("CARGO_PKG_VERSION");
    assert!(
        version_text.contains(expected_version),
        "Version output should contain '{}'\nActual: {}",
        expected_version,
        version_text
    );
}

#[test]
fn test_help_shows_commands() {
    let help_text = get_help_text().expect("Failed to get help text");

    let expected_commands = vec![
        "ask",
        "authorize",
        "say",
        "serve",
        "config",
        "image",
        "navigate",
        "forward",
        "task",
        "provider",
        "help",
    ];

    for cmd in expected_commands {
        assert!(
            help_text.contains(cmd),
            "Help text should contain command '{}'\nActual: {}",
            cmd,
            help_text
        );
    }
}

#[test]
fn test_task_help_shows_subcommands() {
    let help_text =
        cli_tests::run_ailoop(&["task", "--help"]).expect("Failed to get task help text");

    let expected_subcommands = vec![
        "create", "list", "show", "update", "dep", "ready", "blocked",
    ];

    for cmd in expected_subcommands {
        assert!(
            help_text.contains(cmd),
            "Task help text should contain subcommand '{}'\nActual: {}",
            cmd,
            help_text
        );
    }
}
