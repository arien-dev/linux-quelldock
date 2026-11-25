use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

fn safe_id(value: &str) -> String {
    let normalized: String = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' {
                character
            } else {
                '-'
            }
        })
        .collect();
    let trimmed = normalized.trim_matches('-');
    if trimmed.is_empty() {
        "app".into()
    } else {
        trimmed.to_ascii_lowercase()
    }
}

fn desktop_escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace('\r', "")
}

fn exec_escape(value: &str) -> String {
    value
        .replace('%', "%%")
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
}

pub fn desktop_path(id: &str) -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME").ok_or("HOME is not set")?;
    Ok(PathBuf::from(home)
        .join(".local/share/applications")
        .join(format!("quelldock-{}.desktop", safe_id(id))))
}

pub fn create_desktop_entry(id: &str, name: &str, executable: &Path) -> Result<PathBuf, String> {
    if executable
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.eq_ignore_ascii_case("appimage"))
        != Some(true)
    {
        return Err("Desktop integration is only supported for AppImage files".into());
    }
    let executable = executable
        .canonicalize()
        .unwrap_or_else(|_| executable.to_path_buf());
    let metadata =
        fs::metadata(&executable).map_err(|error| format!("Cannot access AppImage: {error}"))?;
    if !metadata.is_file() {
        return Err("AppImage path is not a regular file".into());
    }
    let mut permissions = metadata.permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&executable, permissions)
        .map_err(|error| format!("Cannot make AppImage executable: {error}"))?;

    let target = desktop_path(id)?;
    let parent = target.parent().ok_or("Desktop entry path has no parent")?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("Cannot create applications directory: {error}"))?;
    let data = format!(
        "[Desktop Entry]\nType=Application\nName={}\nExec=\"{}\" %U\nTerminal=false\nCategories=Utility;\n",
        desktop_escape(name),
        exec_escape(&executable.to_string_lossy())
    );
    let temporary = target.with_extension("desktop.tmp");
    fs::write(&temporary, data).map_err(|error| format!("Cannot write desktop entry: {error}"))?;
    fs::set_permissions(&temporary, fs::Permissions::from_mode(0o644))
        .map_err(|error| format!("Cannot set desktop entry permissions: {error}"))?;
    fs::rename(&temporary, &target)
        .map_err(|error| format!("Cannot install desktop entry: {error}"))?;
    Ok(target)
}

pub fn remove_desktop_entry(id: &str) -> Result<(), String> {
    let path = desktop_path(id)?;
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("Cannot remove desktop entry: {error}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifiers_and_exec_values_are_safely_escaped() {
        assert_eq!(safe_id("../Odd App"), "odd-app");
        assert_eq!(
            exec_escape("/tmp/a b%\".AppImage"),
            "/tmp/a b%%\\\".AppImage"
        );
        assert_eq!(desktop_escape("Name\nNext"), "Name\\nNext");
    }
}
