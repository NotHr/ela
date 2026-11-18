//! Short-lived model process groups. Parent cancellation/timeout kills the group;
//! a pipe watchdog also covers the parent being killed without Rust destructors.
use serde::{Deserialize, Serialize};
use std::{
    io,
    process::{Output, Stdio},
    time::Duration,
};

fn terminate_group_if_leader() {
    #[cfg(unix)]
    unsafe {
        if libc::getpgrp() == libc::getpid() {
            libc::killpg(0, libc::SIGKILL);
        }
    }
    std::process::exit(1);
}
pub fn watch_parent_pipe() {
    std::thread::spawn(|| {
        use std::io::Read;
        let mut byte = [0_u8; 1];
        loop {
            match std::io::stdin().read(&mut byte) {
                Ok(0) | Err(_) => terminate_group_if_leader(),
                Ok(_) => {}
            }
        }
    });
}
#[derive(Serialize, Deserialize)]
struct Spec {
    program: String,
    args: Vec<String>,
    env: Vec<(String, Option<String>)>,
    cwd: Option<String>,
}
fn utf8(value: &std::ffi::OsStr) -> io::Result<String> {
    value.to_str().map(str::to_owned).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "Model command path is not UTF-8",
        )
    })
}
struct GroupGuard {
    id: u32,
    armed: bool,
}
impl Drop for GroupGuard {
    fn drop(&mut self) {
        #[cfg(unix)]
        if self.armed {
            unsafe {
                libc::killpg(self.id as libc::pid_t, libc::SIGKILL);
            }
        }
    }
}
/// Execute a trusted local program, never a shell. The desktop's main executable
/// implements --model-worker; no model weights are loaded into the supervisor.
pub async fn run_model(command: &tokio::process::Command, timeout: Duration) -> io::Result<Output> {
    use std::io::Write;
    let command = command.as_std();
    let spec = Spec {
        program: utf8(command.get_program())?,
        args: command.get_args().map(utf8).collect::<io::Result<_>>()?,
        env: command
            .get_envs()
            .map(|(key, value)| Ok((utf8(key)?, value.map(utf8).transpose()?)))
            .collect::<io::Result<_>>()?,
        cwd: command
            .get_current_dir()
            .map(|path| utf8(path.as_os_str()))
            .transpose()?,
    };
    let mut file = tempfile::NamedTempFile::new()?;
    serde_json::to_writer(&mut file, &spec)?;
    file.flush()?;
    let mut supervisor = tokio::process::Command::new(std::env::current_exe()?);
    supervisor
        .arg("--model-worker")
        .arg(file.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    #[cfg(unix)]
    supervisor.process_group(0);
    let mut child = supervisor.spawn()?;
    let mut guard = GroupGuard {
        id: child
            .id()
            .ok_or_else(|| io::Error::other("No model supervisor PID"))?,
        armed: true,
    };
    let keepalive = child.stdin.take();
    let output = tokio::time::timeout(timeout, child.wait_with_output())
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "Local model timed out"))??;
    guard.armed = false;
    drop(keepalive);
    Ok(output)
}
/// Internal CLI dispatcher. EOF kills its dedicated group, including the model
/// program and its descendants, if the owning desktop process disappears.
pub fn run_spec(path: &std::path::Path) -> io::Result<()> {
    watch_parent_pipe();
    let bytes = std::fs::read(path)?;
    if bytes.len() > 128 * 1024 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Model request too large",
        ));
    }
    let spec: Spec = serde_json::from_slice(&bytes)?;
    let mut command = std::process::Command::new(spec.program);
    command
        .args(spec.args)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    if let Some(path) = spec.cwd {
        command.current_dir(path);
    }
    for (key, value) in spec.env {
        if let Some(value) = value {
            command.env(key, value);
        } else {
            command.env_remove(key);
        }
    }
    let status = command.status()?;
    if !status.success() {
        return Err(io::Error::other("Local model process failed"));
    }
    Ok(())
}
