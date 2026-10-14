//! Replace a file without truncating it in place.
//!
//! `std::fs::write` opens the destination with truncate, so a failed save can
//! destroy the original. This module writes a sibling temp file and swaps it
//! into place. Access denied keeps a staged copy so the UI can retry through
//! an elevated process. A sharing lock is reported as busy: elevation cannot
//! unlock a file another program has open.

use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static TEMP_SEQ: AtomicU64 = AtomicU64::new(1);

#[derive(Debug)]
pub enum SaveWriteError {
    /// Destination is open in another process (sharing / lock / mapped view).
    Busy { path: String, os: String },
    /// Read-only attribute or ACL denied the write. `staged` holds the full new bytes.
    Denied {
        path: String,
        os: String,
        staged: PathBuf,
    },
    Other { detail: String },
}

impl SaveWriteError {
    pub fn other(detail: impl Into<String>) -> Self {
        Self::Other {
            detail: detail.into(),
        }
    }
}

impl std::fmt::Display for SaveWriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Busy { path, os } => write!(f, "file is in use: {path} ({os})"),
            Self::Denied { path, os, staged } => write!(
                f,
                "access denied: {path} ({os}); staged copy at {}",
                staged.display()
            ),
            Self::Other { detail } => write!(f, "{detail}"),
        }
    }
}

impl std::error::Error for SaveWriteError {}

#[derive(Debug)]
pub enum ElevateError {
    Cancelled,
    Failed(String),
    /// Non-Windows builds have no UAC prompt.
    #[allow(dead_code)]
    Unsupported,
}

impl std::fmt::Display for ElevateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cancelled => write!(f, "administrator permission was not granted"),
            Self::Failed(msg) => write!(f, "{msg}"),
            Self::Unsupported => write!(f, "administrator elevation is not available"),
        }
    }
}

/// Write `bytes` over `dest`. The previous file stays intact if the swap fails.
pub fn write_replacing(dest: &Path, bytes: &[u8]) -> Result<(), SaveWriteError> {
    let tmp = sibling_temp(dest, "tmp");
    if let Err(e) = fs::write(&tmp, bytes) {
        let _ = fs::remove_file(&tmp);
        return Err(classify(dest, &e, bytes));
    }
    replace_with_temp(dest, &tmp, bytes)
}

/// Copy `staged` over `dest` with administrator rights (Windows UAC).
///
/// Blocks until the user answers the prompt. A sharing lock is not solved by
/// this; callers should not offer it for [`SaveWriteError::Busy`].
pub fn elevate_replace(dest: &Path, staged: &Path) -> Result<(), ElevateError> {
    #[cfg(windows)]
    {
        elevate_replace_windows(dest, staged)
    }
    #[cfg(not(windows))]
    {
        let _ = (dest, staged);
        Err(ElevateError::Unsupported)
    }
}

fn replace_with_temp(dest: &Path, tmp: &Path, bytes: &[u8]) -> Result<(), SaveWriteError> {
    if !dest.exists() {
        return fs::rename(tmp, dest).map_err(|e| {
            let _ = fs::remove_file(tmp);
            classify(dest, &e, bytes)
        });
    }

    clear_readonly(dest);
    let displaced = sibling_temp(dest, "old");
    if let Err(e) = fs::rename(dest, &displaced) {
        let _ = fs::remove_file(tmp);
        return Err(classify(dest, &e, bytes));
    }

    if let Err(e) = fs::rename(tmp, dest) {
        return match fs::rename(&displaced, dest) {
            Ok(()) => {
                let _ = fs::remove_file(tmp);
                Err(classify(dest, &e, bytes))
            }
            Err(restore_err) => Err(SaveWriteError::other(format!(
                "Could not replace {} ({e}). The previous contents are still at {}. Restoring them failed: {restore_err}.",
                dest.display(),
                displaced.display()
            ))),
        };
    }

    let _ = fs::remove_file(&displaced);
    Ok(())
}

fn classify(dest: &Path, err: &io::Error, bytes: &[u8]) -> SaveWriteError {
    let path = dest.display().to_string();
    let os = err.to_string();
    if is_busy(err) {
        return SaveWriteError::Busy { path, os };
    }
    if is_denied(err) {
        return match stage_bytes(dest, bytes) {
            Ok(staged) => SaveWriteError::Denied { path, os, staged },
            Err(stage_err) => SaveWriteError::other(format!(
                "Could not save {path}: {os}. A retry copy could not be stored either: {stage_err}."
            )),
        };
    }
    SaveWriteError::other(format!("Could not save {path}: {os}"))
}

fn is_busy(err: &io::Error) -> bool {
    // 32 sharing violation, 33 lock violation, 1224 user-mapped section.
    matches!(err.raw_os_error(), Some(32 | 33 | 1224)) || err.kind() == io::ErrorKind::ResourceBusy
}

fn is_denied(err: &io::Error) -> bool {
    err.kind() == io::ErrorKind::PermissionDenied || matches!(err.raw_os_error(), Some(5))
}

fn clear_readonly(path: &Path) {
    let Ok(meta) = fs::metadata(path) else {
        return;
    };
    let mut perms = meta.permissions();
    if perms.readonly() {
        perms.set_readonly(false);
        let _ = fs::set_permissions(path, perms);
    }
}

fn stage_bytes(dest: &Path, bytes: &[u8]) -> io::Result<PathBuf> {
    let mut staged = std::env::temp_dir();
    let leaf = dest
        .file_name()
        .unwrap_or(std::ffi::OsStr::new("nus3"))
        .to_string_lossy();
    let n = TEMP_SEQ.fetch_add(1, Ordering::Relaxed);
    staged.push(format!(
        "nus3-elevate-{}-{n}-{leaf}",
        std::process::id()
    ));
    fs::write(&staged, bytes)?;
    Ok(staged)
}

fn sibling_temp(dest: &Path, kind: &str) -> PathBuf {
    let n = TEMP_SEQ.fetch_add(1, Ordering::Relaxed);
    let name = dest
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("nus3");
    let parent = dest.parent().filter(|p| !p.as_os_str().is_empty());
    let file_name = format!(".{name}.nus3save-{}-{n}.{kind}", std::process::id());
    match parent {
        Some(parent) => parent.join(file_name),
        None => PathBuf::from(file_name),
    }
}

#[cfg(windows)]
fn elevate_replace_windows(dest: &Path, staged: &Path) -> Result<(), ElevateError> {
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    let script_path = sibling_in_temp("ps1");
    let result_path = sibling_in_temp("result");
    let script = format!(
        "$ErrorActionPreference = 'Stop'\r\n\
         $dest = {dest}\r\n\
         $src = {src}\r\n\
         $result = {result}\r\n\
         try {{\r\n\
           $parent = Split-Path -Parent -Path $dest\r\n\
           if ([string]::IsNullOrEmpty($parent) -or -not (Test-Path -LiteralPath $parent)) {{\r\n\
             throw 'Destination folder does not exist'\r\n\
           }}\r\n\
           if (Test-Path -LiteralPath $dest) {{\r\n\
             $item = Get-Item -LiteralPath $dest -Force\r\n\
             if (($item.Attributes -band [IO.FileAttributes]::ReadOnly) -ne 0) {{\r\n\
               $item.Attributes = $item.Attributes -band (-bnot [IO.FileAttributes]::ReadOnly)\r\n\
             }}\r\n\
           }}\r\n\
           Copy-Item -LiteralPath $src -Destination $dest -Force\r\n\
           Set-Content -LiteralPath $result -Value 'ok' -Encoding ascii -NoNewline\r\n\
         }} catch {{\r\n\
           Set-Content -LiteralPath $result -Value $_.Exception.Message -Encoding utf8\r\n\
           exit 1\r\n\
         }}\r\n",
        dest = ps_single(&dest.display().to_string()),
        src = ps_single(&staged.display().to_string()),
        result = ps_single(&result_path.display().to_string()),
    );
    let mut body = vec![0xEF, 0xBB, 0xBF];
    body.extend(script.as_bytes());
    if let Err(e) = fs::write(&script_path, body) {
        return Err(ElevateError::Failed(e.to_string()));
    }

    let command = format!(
        "Start-Process -FilePath powershell.exe -Verb RunAs -Wait -WindowStyle Hidden -ArgumentList '-NoProfile','-ExecutionPolicy','Bypass','-File',{}",
        ps_single(&script_path.display().to_string())
    );
    let output = Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &command,
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output();

    let outcome = match output {
        Ok(output) => interpret_elevate_output(&output, dest, staged, &result_path),
        Err(e) => Err(ElevateError::Failed(e.to_string())),
    };
    let _ = fs::remove_file(&script_path);
    let _ = fs::remove_file(&result_path);
    outcome
}

#[cfg(windows)]
fn interpret_elevate_output(
    output: &std::process::Output,
    dest: &Path,
    staged: &Path,
    result_path: &Path,
) -> Result<(), ElevateError> {
    let result_text = fs::read_to_string(result_path).unwrap_or_default();
    let trimmed = result_text.trim();
    if trimmed == "ok" && files_match(dest, staged) {
        return Ok(());
    }

    let captured = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout)
    );
    if trimmed.is_empty() && is_cancel(&captured) {
        return Err(ElevateError::Cancelled);
    }
    if !trimmed.is_empty() && trimmed != "ok" {
        return Err(ElevateError::Failed(trimmed.to_string()));
    }
    let fallback = captured.trim();
    if fallback.is_empty() {
        Err(ElevateError::Failed(format!(
            "elevated copy did not update {}",
            dest.display()
        )))
    } else {
        Err(ElevateError::Failed(fallback.to_string()))
    }
}

fn files_match(a: &Path, b: &Path) -> bool {
    match (file_hash(a), file_hash(b)) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}

fn file_hash(path: &Path) -> io::Result<u64> {
    let mut file = File::open(path)?;
    let mut buf = [0u8; 64 * 1024];
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        for byte in &buf[..n] {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x100_0000_01b3);
        }
    }
    Ok(hash)
}

fn is_cancel(text: &str) -> bool {
    let lower = text.to_lowercase();
    lower.contains("cancel") || text.contains("取消")
}

fn ps_single(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn sibling_in_temp(ext: &str) -> PathBuf {
    let n = TEMP_SEQ.fetch_add(1, Ordering::Relaxed);
    let mut path = std::env::temp_dir();
    path.push(format!(
        "nus3-elevate-{}-{n}.{ext}",
        std::process::id()
    ));
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        let n = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        path.push(format!("exvs2_save_io_{name}_{n}"));
        path
    }

    #[test]
    fn replaces_existing_bytes() {
        let path = scratch("ok.bin");
        fs::write(&path, b"old-contents").unwrap();
        write_replacing(&path, b"new-contents").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"new-contents");
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn clears_readonly_attribute_and_replaces() {
        let path = scratch("ro.bin");
        fs::write(&path, b"old").unwrap();
        let mut perms = fs::metadata(&path).unwrap().permissions();
        perms.set_readonly(true);
        fs::set_permissions(&path, perms).unwrap();
        write_replacing(&path, b"new").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"new");
        assert!(!fs::metadata(&path).unwrap().permissions().readonly());
        let _ = fs::remove_file(&path);
    }
}
