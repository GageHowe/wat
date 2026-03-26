use std::process::{Child, Command};

pub fn run(commands: &[String], label: &str) -> Result<(), String> {
    println!("\n==> {label}");
    for cmd in commands {
        println!("$ {cmd}");
    }
    let mut child = spawn_shell(commands)?;
    let status = child.wait().map_err(|e| format!("failed to wait: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("command exited with {status}"))
    }
}

pub fn start(commands: &[String], label: &str) -> Result<Child, String> {
    println!("\n==> {label}");
    for cmd in commands {
        println!("$ {cmd}");
    }
    spawn_shell(commands)
}

pub fn kill(child: &mut Child) {
    child.kill().ok();
    child.wait().ok();
}

fn spawn_shell(commands: &[String]) -> Result<Child, String> {
    let joined = commands.join(" && ");
    shell(&joined)
        .spawn()
        .map_err(|e| format!("failed to spawn `{joined}`: {e}"))
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
