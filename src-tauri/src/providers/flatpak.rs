use super::{valid_package_id, PackageProvider};
use crate::{
    models::{InstallSpec, InstalledPackage, PackageCandidate, ProviderId},
    runner::{capture, run_command},
};
use tauri::AppHandle;

pub struct FlatpakProvider;

impl PackageProvider for FlatpakProvider {
    fn id(&self) -> ProviderId {
        ProviderId::Flatpak
    }
    fn display_name(&self) -> &'static str {
        "Flatpak"
    }
    fn is_available(&self) -> bool {
        crate::runner::which_exists("flatpak")
    }

    fn search(&self, query: &str) -> Result<Vec<PackageCandidate>, String> {
        let query = query.trim();
        if query.is_empty() {
            return Ok(Vec::new());
        }
        let args = vec![
            "search".into(),
            "--columns=application,name,version,branch,remotes,description".into(),
            query.into(),
        ];
        let output = capture("flatpak", &args)?;
        Ok(output
            .lines()
            .filter_map(|line| {
                let columns: Vec<_> = line.split('\t').collect();
                if columns.len() < 2 {
                    return None;
                }
                let id = columns[0].trim();
                if !valid_package_id(id) {
                    return None;
                }
                Some(PackageCandidate {
                    provider: ProviderId::Flatpak,
                    package_id: id.into(),
                    name: columns.get(1).unwrap_or(&columns[0]).trim().into(),
                    version: columns
                        .get(2)
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty()),
                    description: columns
                        .get(5)
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty()),
                    available: true,
                    command_preview: vec![
                        "flatpak".into(),
                        "install".into(),
                        "-y".into(),
                        id.into(),
                    ],
                })
            })
            .collect())
    }

    fn list_installed(&self) -> Result<Vec<InstalledPackage>, String> {
        let args = vec![
            "list".into(),
            "--columns=application,name,version,branch,origin".into(),
        ];
        let output = capture("flatpak", &args)?;
        Ok(output
            .lines()
            .filter_map(|line| {
                let columns: Vec<_> = line.split('\t').collect();
                let id = columns.first()?.trim();
                if !valid_package_id(id) {
                    return None;
                }
                Some(InstalledPackage {
                    provider: ProviderId::Flatpak,
                    package_id: id.into(),
                    name: columns.get(1).unwrap_or(&columns[0]).trim().into(),
                    version: columns.get(2).map(|s| s.trim().to_string()),
                    description: None,
                    desktop_entry: false,
                    appimage_path: None,
                    update_available: false,
                })
            })
            .collect())
    }

    fn install(&self, spec: InstallSpec, app: &AppHandle, job_id: &str) -> Result<(), String> {
        ensure_id(&spec.package_id)?;
        let mut args = vec!["install".into(), "-y".into()];
        if let Some(remote) = spec.remote.as_deref().filter(|v| valid_package_id(v)) {
            args.push(remote.into());
        }
        args.push(spec.package_id);
        run_command(app, job_id, "flatpak", &args, false)
    }

    fn uninstall(
        &self,
        pkg: &InstalledPackage,
        app: &AppHandle,
        job_id: &str,
    ) -> Result<(), String> {
        let args = vec![
            "uninstall".into(),
            "-y".into(),
            ensure_id(&pkg.package_id)?.to_string(),
        ];
        run_command(app, job_id, "flatpak", &args, false)
    }

    fn update(&self, pkg: &InstalledPackage, app: &AppHandle, job_id: &str) -> Result<(), String> {
        let args = vec![
            "update".into(),
            "-y".into(),
            ensure_id(&pkg.package_id)?.to_string(),
        ];
        run_command(app, job_id, "flatpak", &args, false)
    }
}

fn ensure_id(id: &str) -> Result<&str, String> {
    if valid_package_id(id) {
        Ok(id)
    } else {
        Err("Invalid Flatpak application ID".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_options_as_application_ids() {
        assert!(ensure_id("--user").is_err());
        assert!(ensure_id("org.example/App").is_err());
        assert_eq!(
            ensure_id("org.example.Editor").unwrap(),
            "org.example.Editor"
        );
    }
}
