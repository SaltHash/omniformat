use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crate::tools::Tools;

/// Everything a conversion pipeline needs: binaries, cancellation, scratch
/// space and a progress sink.
pub struct TaskCtx {
    pub tools: Tools,
    pub cancel: Arc<AtomicBool>,
    pub temp: PathBuf,
    pub progress: Arc<dyn Fn(f32, &str) + Send + Sync>,
}

impl TaskCtx {
    pub fn report(&self, pct: f32, stage: &str) {
        (self.progress)(pct.clamp(0.0, 100.0), stage);
    }

    pub fn cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }

    pub fn check(&self) -> Result<(), String> {
        if self.cancelled() {
            Err("Cancelled".to_string())
        } else {
            Ok(())
        }
    }

    pub fn temp_file(&self, name: &str) -> PathBuf {
        let _ = std::fs::create_dir_all(&self.temp);
        self.temp.join(name)
    }
}

/// Run a command, capturing stdout/stderr, while feeding stdout lines to a
/// callback (FFmpeg's -progress stream). Cancellation kills the child.
pub fn run_streaming<F>(
    ctx: &TaskCtx,
    program: &str,
    args: &[String],
    cwd: Option<&Path>,
    mut on_line: F,
) -> Result<String, String>
where
    F: FnMut(&str),
{
    ctx.check()?;

    let mut cmd = Command::new(program);
    cmd.args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("LC_ALL", "C")
        .env("LANG", "C");
    if let Some(cwd) = cwd {
        cmd.current_dir(cwd);
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Could not start {}: {}", program, e))?;

    let stdout = child.stdout.take().ok_or("No stdout pipe")?;
    let stderr = child.stderr.take().ok_or("No stderr pipe")?;

    let (tx, rx) = std::sync::mpsc::channel::<String>();
    let stdout_thread = std::thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            match line {
                Ok(line) => {
                    if tx.send(line).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    let stderr_thread = std::thread::spawn(move || {
        let mut text = String::new();
        let mut reader = BufReader::new(stderr);
        let _ = reader.read_to_string(&mut text);
        text
    });

    let mut collected = String::new();
    let mut status = None;

    loop {
        if ctx.cancelled() {
            let _ = child.kill();
            let _ = child.wait();
            let _ = stdout_thread.join();
            let _ = stderr_thread.join();
            return Err("Cancelled".to_string());
        }

        match rx.recv_timeout(Duration::from_millis(80)) {
            Ok(line) => {
                on_line(&line);
                collected.push_str(&line);
                collected.push('\n');
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {}
        }

        match child.try_wait() {
            Ok(Some(done)) => {
                status = Some(done);
                break;
            }
            Ok(None) => {}
            Err(e) => {
                let _ = child.kill();
                return Err(format!("Process error: {}", e));
            }
        }
    }

    let _ = stdout_thread.join();
    while let Ok(line) = rx.try_recv() {
        on_line(&line);
        collected.push_str(&line);
        collected.push('\n');
    }
    let stderr_text = stderr_thread.join().unwrap_or_default();
    let status = status.unwrap_or_else(|| child.wait().ok().unwrap_or_default());

    if status.success() {
        Ok(collected)
    } else {
        let mut message = stderr_text.trim().to_string();
        if message.is_empty() {
            message = collected.trim().to_string();
        }
        if message.chars().count() > 1400 {
            message = message.chars().rev().take(1400).collect::<String>().chars().rev().collect();
        }
        if message.is_empty() {
            Err(format!("{} exited with {}", program, status))
        } else {
            Err(message)
        }
    }
}

pub fn run(
    ctx: &TaskCtx,
    program: &str,
    args: &[String],
    cwd: Option<&Path>,
) -> Result<String, String> {
    run_streaming(ctx, program, args, cwd, |_| {})
}

pub fn arg(value: impl Into<String>) -> String {
    value.into()
}

pub fn path_arg(path: &Path) -> String {
    path.to_string_lossy().to_string()
}
