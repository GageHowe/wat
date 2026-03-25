use std::process::{Child, Command};

/// Run a list of commands sequentially, waiting for each to finish.
/// Returns an error if any command fails.
pub fn run(commands: &[String], label: &str) -> Result<(), String> {
    println!("\n==> {label}");
    for cmd in commands {
        println!("$ {cmd}");
        let status = shell(cmd)
            .status()
            .map_err(|e| format!("failed to spawn `{cmd}`: {e}"))?;
        if !status.success() {
            return Err(format!("`{cmd}` exited with {status}"));
        }
    }
    Ok(())
}

/// Spawn all commands joined with `&&` as a single background shell process.
/// Returns the child handle so the caller can kill/wait it.
/// Used for `interrupt: true` targets where we need to kill mid-run.
pub fn spawn(commands: &[String], label: &str) -> Result<Child, String> {
    let joined = commands.join(" && ");
    println!("\n==> {label}");
    println!("$ {joined}");
    shell(&joined)
        .spawn()
        .map_err(|e| format!("failed to spawn `{joined}`: {e}"))
}

/// Kill a running child process and wait for it to exit fully.
pub fn kill(child: &mut Child) {
    child.kill().ok();
    child.wait().ok();
}

#[cfg(windows)]
fn shell(cmd: &str) -> Command {
    let mut c = Command::new("cmd");
    c.arg("/C").arg(cmd);
    c
}

#[cfg(not(windows))]
fn shell(cmd: &str) -> Command {
    let mut c = Command::new("sh");
    c.arg("-c").arg(cmd);
    c
}
