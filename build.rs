use std::process::Command;

fn main() {
    let branch = git_output(&["rev-parse", "--abbrev-ref", "HEAD"]).unwrap_or_else(|| "unknown".into());
    let short_hash = git_output(&["rev-parse", "--short", "HEAD"]).unwrap_or_else(|| "unknown".into());

    let out = std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join("git_metadata.rs");
    std::fs::write(
        &out,
        format!(
            r#"pub const BRANCH: &str = "{branch}";
pub const SHORT_HASH: &str = "{short_hash}";
"#
        ),
    )
    .expect("write git metadata");
}

fn git_output(args: &[&str]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}
