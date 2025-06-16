use std::process::Command;

fn main() {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .expect("failed to execute git");

    let git_hash = String::from_utf8_lossy(&output.stdout).trim().to_string();

    println!("cargo:rustc-env=GIT_COMMIT_HASH={}", git_hash);
}
