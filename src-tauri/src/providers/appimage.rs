use super::PackageProvider;
use crate::{
    desktop_entry::remove_desktop_entry,
    models::{AppImageEntry, InstallSpec, InstalledPackage, PackageCandidate, ProviderId},
    runner::{run_command, which_exists},
};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
use tauri::AppHandle;

pub struct AppImageProvider;

impl PackageProvider for AppImageProvider {
    fn id(&self) -> ProviderId {
        ProviderId::AppImage
    }
    fn display_name(&self) -> &'static str {
        "AppImage"
    }
    fn is_available(&self) -> bool {
        true
    }
    fn search(&self, _query: &str) -> Result<Vec<PackageCandidate>, String> {
        Ok(Vec::new())
    }

    fn list_installed(&self) -> Result<Vec<InstalledPackage>, String> {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        let folders = [
            PathBuf::from(&home).join("Applications"),
            PathBuf::from(&home).join("Downloads"),
        ];
        Ok(scan_directories(&folders)
            .into_iter()
            .map(|entry| InstalledPackage {
                provider: ProviderId::AppImage,
                package_id: entry.id,
                name: entry.name,
                version: None,
                description: entry.source,
                desktop_entry: entry.desktop_entry,
                appimage_path: Some(entry.path),
                update_available: false,
            })
            .collect())
    }

    fn install(&self, _spec: InstallSpec, _app: &AppHandle, _job_id: &str) -> Result<(), String> {
        Err("Use the AppImage URL or import action to add an AppImage".into())
    }
    fn uninstall(
        &self,
        pkg: &InstalledPackage,
        _app: &AppHandle,
        _job_id: &str,
    ) -> Result<(), String> {
        let path = pkg
            .appimage_path
            .as_deref()
            .ok_or("AppImage path is missing")?;
        if Path::new(path)
            .extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| extension.eq_ignore_ascii_case("appimage"))
            != Some(true)
        {
            return Err("Refusing to remove a file that is not an AppImage".into());
        }
        fs::remove_file(path).map_err(|error| format!("Cannot remove AppImage file: {error}"))?;
        remove_desktop_entry(&pkg.package_id)
    }
    fn update(&self, pkg: &InstalledPackage, app: &AppHandle, job_id: &str) -> Result<(), String> {
        let url = pkg
            .description
            .as_deref()
            .filter(|url| url.starts_with("https://") || url.starts_with("http://"))
            .ok_or("This AppImage has no source URL for an update")?;
        let path = pkg
            .appimage_path
            .as_deref()
            .ok_or("AppImage path is missing")?;
        download_url_to_target(app, job_id, url, Path::new(path))
    }
}

pub fn import_file(path: &Path, destination: &Path) -> Result<AppImageEntry, String> {
    if path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.eq_ignore_ascii_case("appimage"))
        != Some(true)
    {
        return Err("Select a file with the .AppImage extension".into());
    }
    let source = path
        .canonicalize()
        .map_err(|error| format!("Cannot access AppImage: {error}"))?;
    if !source.is_file() {
        return Err("AppImage source is not a regular file".into());
    }
    fs::create_dir_all(destination)
        .map_err(|error| format!("Cannot create AppImage directory: {error}"))?;
    let name = source
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Application");
    let target = destination.join(format!("{}.AppImage", safe_filename(name)));
    if source != target {
        fs::copy(&source, &target).map_err(|error| format!("Cannot copy AppImage: {error}"))?;
    }
    make_executable(&target)?;
    Ok(entry_for_path(&target, None))
}

pub fn download_url(
    app: &AppHandle,
    job_id: &str,
    url: &str,
    destination: &Path,
) -> Result<AppImageEntry, String> {
    let filename = validate_appimage_url(url)?;
    let target = destination.join(safe_filename(filename));
    download_url_to_target(app, job_id, url, &target)?;
    Ok(entry_for_path(&target, Some(url.to_string())))
}

fn download_url_to_target(
    app: &AppHandle,
    job_id: &str,
    url: &str,
    target: &Path,
) -> Result<(), String> {
    validate_appimage_url(url)?;
    let temporary = target.with_extension("AppImage.part");
    let parent = target
        .parent()
        .ok_or("AppImage path has no parent directory")?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("Cannot create AppImage directory: {error}"))?;
    let args = vec![
        "--location".into(),
        "--fail".into(),
        "--show-error".into(),
        "--output".into(),
        temporary.to_string_lossy().to_string(),
        "--".into(),
        url.into(),
    ];
    if let Err(error) = run_command(app, job_id, "curl", &args, false) {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    if let Err(error) = make_executable(&temporary) {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    fs::rename(&temporary, target).map_err(|error| format!("Cannot replace AppImage: {error}"))
}

fn validate_appimage_url(url: &str) -> Result<&str, String> {
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err("AppImage URL must use HTTP or HTTPS".into());
    }
    let filename = url
        .split('?')
        .next()
        .unwrap_or(url)
        .rsplit('/')
        .next()
        .unwrap_or("")
        .trim();
    if !filename.to_ascii_lowercase().ends_with(".appimage") {
        return Err("The URL must point directly to a .AppImage file".into());
    }
    Ok(filename)
}

pub fn scan_directories(directories: &[PathBuf]) -> Vec<AppImageEntry> {
    let mut found = Vec::new();
    for directory in directories {
        let Ok(entries) = fs::read_dir(directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file()
                && path
                    .extension()
                    .and_then(|s| s.to_str())
                    .map(|s| s.eq_ignore_ascii_case("appimage"))
                    == Some(true)
            {
                found.push(entry_for_path(&path, None));
            }
        }
    }
    found.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    found.dedup_by(|a, b| a.path == b.path);
    found
}

pub fn launch(path: &Path) -> Result<(), String> {
    let path = path
        .canonicalize()
        .map_err(|error| format!("Cannot access application: {error}"))?;
    if !path.is_file() {
        return Err("Application is not a regular file".into());
    }
    make_executable(&path)?;
    let mut command = if which_exists("setsid") {
        let mut cmd = Command::new("setsid");
        cmd.arg(path);
        cmd
    } else {
        Command::new(path)
    };
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("Cannot launch AppImage: {error}"))?;
    Ok(())
}

fn entry_for_path(path: &Path, source: Option<String>) -> AppImageEntry {
    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Application")
        .to_string();
    let id = safe_filename(&name)
        .trim_end_matches(".AppImage")
        .to_lowercase();
    AppImageEntry {
        id,
        name,
        path: path.to_string_lossy().into_owned(),
        source,
        desktop_entry: false,
    }
}

fn make_executable(path: &Path) -> Result<(), String> {
    let mut permissions = fs::metadata(path)
        .map_err(|error| error.to_string())?
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions)
        .map_err(|error| format!("Cannot set executable permission: {error}"))
}

fn safe_filename(name: &str) -> String {
    let safe: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') {
                c
            } else {
                '-'
            }
        })
        .collect();
    let safe = safe.trim_matches('.');
    if safe.is_empty() {
        "Application.AppImage".into()
    } else {
        safe.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filename_is_safe_and_scan_ignores_non_appimages() {
        assert_eq!(safe_filename("../../Some App"), "-..-Some-App");
        let root = std::env::temp_dir().join(format!("quelldock-scan-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("Editor.AppImage"), b"binary").unwrap();
        fs::write(root.join("notes.txt"), b"text").unwrap();
        let entries = scan_directories(std::slice::from_ref(&root));
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "Editor");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn validates_direct_appimage_urls() {
        assert!(validate_appimage_url("file:///tmp/evil.AppImage").is_err());
        assert!(validate_appimage_url("https://example.test/file.tar.gz").is_err());
        assert_eq!(
            validate_appimage_url("https://example.test/release/App.APPIMAGE?download=1").unwrap(),
            "App.APPIMAGE"
        );
    }
}
