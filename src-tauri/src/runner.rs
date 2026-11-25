use crate::models::JobEvent;
use std::{
    collections::HashMap,
    io::{BufRead, BufReader, Write},
    process::{Child, Command, Output, Stdio},
    sync::{Arc, Mutex, OnceLock},
    thread,
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Emitter};

type ChildRegistry = Mutex<HashMap<String, Arc<Mutex<Child>>>>;
static ACTIVE_JOBS: OnceLock<ChildRegistry> = OnceLock::new();

fn registry() -> &'static ChildRegistry {
    ACTIVE_JOBS.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn which_exists(binary: &str) -> bool {
    if binary.contains('/') {
        return std::path::Path::new(binary).is_file();
    }
    std::env::var_os("PATH")
        .map(|paths| std::env::split_paths(&paths).any(|dir| dir.join(binary).is_file()))
        .unwrap_or(false)
}

pub fn is_root() -> bool {
    unsafe { libc_getuid() == 0 }
}

pub fn command_preview(program: &str, args: &[String]) -> Vec<String> {
    std::iter::once(program.to_string())
        .chain(args.iter().cloned())
        .collect()
}

pub fn capture(program: &str, args: &[String]) -> Result<String, String> {
    let output = capture_output(program, args)?;
    if !output.status.success() {
        let message = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if message.is_empty() {
            format!("{program} exited with {}", output.status)
        } else {
            message
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

pub fn capture_output(program: &str, args: &[String]) -> Result<Output, String> {
    if !which_exists(program) {
        return Err(format!(
            "Required command `{program}` was not found on PATH"
        ));
    }
    Command::new(program)
        .args(args)
        .output()
        .map_err(|error| format!("Cannot start {program}: {error}"))
}

pub fn run_command(
    app: &AppHandle,
    job_id: &str,
    program: &str,
    args: &[String],
    privilege: bool,
) -> Result<(), String> {
    let elevated = privilege && !is_root();
    let (binary, full_args) = if elevated {
        if !which_exists("pkexec") {
            return Err(
                "This system action needs administrator privileges, but `pkexec` is unavailable"
                    .into(),
            );
        }
        let mut full = vec![program.to_string()];
        full.extend(args.iter().cloned());
        ("pkexec", full)
    } else {
        (program, args.to_vec())
    };
    if !which_exists(binary) {
        return Err(format!("Required command `{binary}` was not found on PATH"));
    }

    let mut child = Command::new(binary)
        .args(&full_args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("Cannot start {binary}: {error}"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or("Could not capture command stdout")?;
    let stderr = child
        .stderr
        .take()
        .ok_or("Could not capture command stderr")?;
    let child = Arc::new(Mutex::new(child));
    registry()
        .lock()
        .map_err(|_| "Job registry lock was poisoned".to_string())?
        .insert(job_id.to_string(), child.clone());
    emit(
        app,
        job_id,
        "system",
        &format!("Started: {}", command_preview(binary, &full_args).join(" ")),
    );
    let out_app = app.clone();
    let out_id = job_id.to_string();
    let out_reader =
        thread::spawn(move || stream_lines(BufReader::new(stdout), &out_app, &out_id, "stdout"));
    let err_app = app.clone();
    let err_id = job_id.to_string();
    let err_reader =
        thread::spawn(move || stream_lines(BufReader::new(stderr), &err_app, &err_id, "stderr"));

    let status = loop {
        let result = child
            .lock()
            .map_err(|_| "Child process lock was poisoned".to_string())?
            .try_wait()
            .map_err(|error| format!("Could not wait for command: {error}"))?;
        if let Some(status) = result {
            break status;
        }
        thread::sleep(std::time::Duration::from_millis(80));
    };
    let _ = out_reader.join();
    let _ = err_reader.join();
    if let Ok(mut jobs) = registry().lock() {
        jobs.remove(job_id);
    }
    if status.success() {
        emit(app, job_id, "system", "Command completed successfully");
        Ok(())
    } else {
        let message = format!("Command exited with {status}");
        emit(app, job_id, "system", &message);
        Err(message)
    }
}

fn stream_lines<R: BufRead>(reader: R, app: &AppHandle, job_id: &str, stream: &str) {
    for line in reader.lines() {
        match line {
            Ok(line) => emit(app, job_id, stream, &line),
            Err(error) => {
                emit(
                    app,
                    job_id,
                    "system",
                    &format!("Output stream error: {error}"),
                );
                break;
            }
        }
    }
}

fn emit(app: &AppHandle, job_id: &str, stream: &str, line: &str) {
    let timestamp_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    let _ = app.emit(
        "job-output",
        JobEvent {
            job_id: job_id.to_string(),
            timestamp_ms,
            stream: stream.to_string(),
            line: line.to_string(),
        },
    );
}

pub fn cancel_job(job_id: &str) -> Result<(), String> {
    let jobs = registry()
        .lock()
        .map_err(|_| "Job registry lock was poisoned".to_string())?;
    let child = jobs
        .get(job_id)
        .cloned()
        .ok_or_else(|| format!("No active job `{job_id}`"))?;
    drop(jobs);
    let result = child
        .lock()
        .map_err(|_| "Child process lock was poisoned".to_string())?
        .kill()
        .map_err(|error| format!("Could not cancel job: {error}"));
    result
}

pub fn write_job_input(job_id: &str, input: &str) -> Result<(), String> {
    let jobs = registry()
        .lock()
        .map_err(|_| "Job registry lock was poisoned".to_string())?;
    let child = jobs
        .get(job_id)
        .cloned()
        .ok_or_else(|| format!("No active job `{job_id}`"))?;
    drop(jobs);
    let mut child = child
        .lock()
        .map_err(|_| "Child process lock was poisoned".to_string())?;
    let stdin = child
        .stdin
        .as_mut()
        .ok_or("Job input is no longer available")?;
    stdin
        .write_all(input.as_bytes())
        .and_then(|_| {
            if input.ends_with('\n') {
                Ok(())
            } else {
                stdin.write_all(b"\n")
            }
        })
        .and_then(|_| stdin.flush())
        .map_err(|error| format!("Cannot send input to job: {error}"))
}

#[cfg(target_os = "linux")]
unsafe fn libc_getuid() -> u32 {
    unsafe extern "C" {
        fn getuid() -> u32;
    }
    getuid()
}

#[cfg(not(target_os = "linux"))]
unsafe fn libc_getuid() -> u32 {
    1000
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_preserves_arguments_as_separate_values() {
        let args = vec!["install".into(), "name with spaces".into(), "--yes".into()];
        assert_eq!(
            command_preview("apt-get", &args),
            vec!["apt-get", "install", "name with spaces", "--yes"]
        );
    }

    #[test]
    fn lookup_handles_absolute_and_missing_paths() {
        assert!(which_exists("/bin/sh"));
        assert!(!which_exists("quelldock-binary-that-does-not-exist"));
    }
}
