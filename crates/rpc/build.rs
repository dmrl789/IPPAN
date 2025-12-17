use std::process::Command;

fn main() {
    // Re-run if CI provides a SHA, or if git HEAD changes locally.
    println!("cargo:rerun-if-env-changed=GITHUB_SHA");
    println!("cargo:rerun-if-env-changed=GIT_COMMIT_HASH");
    println!("cargo:rerun-if-changed=../../.git/HEAD");

    if let Ok(sha) = std::env::var("GIT_COMMIT_HASH") {
        let sha = sha.trim().to_string();
        if !sha.is_empty() {
            println!("cargo:rustc-env=GIT_COMMIT_HASH={sha}");
            return;
        }
    }

    if let Ok(sha) = std::env::var("GITHUB_SHA") {
        let sha = sha.trim().to_string();
        if !sha.is_empty() {
            println!("cargo:rustc-env=GIT_COMMIT_HASH={sha}");
            return;
        }
    }

    // Best-effort: derive from local git checkout (works for repo builds on servers).
    if let Ok(output) = Command::new("git").args(["rev-parse", "HEAD"]).output() {
        if output.status.success() {
            let sha = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !sha.is_empty() {
                println!("cargo:rustc-env=GIT_COMMIT_HASH={sha}");
            }
        }
    }
}


