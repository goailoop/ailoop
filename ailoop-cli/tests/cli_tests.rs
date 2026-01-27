use std::process::Command;

pub fn run_ailoop(args: &[&str]) -> Result<String, String> {
    let output = Command::new("cargo")
        .args(["run", "--bin", "ailoop", "--"])
        .args(args)
        .output()
        .map_err(|e| format!("Failed to run ailoop: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Command failed: {}", stderr));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn get_help_text() -> Result<String, String> {
    run_ailoop(&["--help", ""])
}

pub fn get_version_text() -> Result<String, String> {
    run_ailoop(&["--version", ""])
}
