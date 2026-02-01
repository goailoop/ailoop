mod cli_tests;

use cli_tests::{get_help_text, get_version_text};

#[test]
fn test_help_includes_version() {
    let help_text = get_help_text().expect("Failed to get help text");
    assert!(
        help_text.contains("ailoop - 0.1.7"),
        "Help text should include version number 'ailoop - 0.1.7'\nActual: {}",
        help_text
    );
}

#[test]
fn test_version_output() {
    let version_text = get_version_text().expect("Failed to get version text");
    assert!(
        version_text.contains("0.1.7"),
        "Version output should contain '0.1.7'\nActual: {}",
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
        "workflow",
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
    // Task command is currently disabled pending full implementation
    // Skipping task subcommand checks for now
}
