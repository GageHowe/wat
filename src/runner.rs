use std::process::{Child, Command};

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

pub fn spawn(commands: &[String], label: &str) -> Result<Child, String> {
    let joined = commands.join(" && ");
    println!("\n==> {label}");
    println!("$ {joined}");
    shell(&joined)
        .spawn()
        .map_err(|e| format!("failed to spawn `{joined}`: {e}"))
}

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
