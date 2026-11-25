use super::{valid_package_id, PackageProvider};
use crate::{
    models::{AppSettings, InstallSpec, InstalledPackage, PackageCandidate, ProviderId},
    runner::{capture, run_command, which_exists},
};
use tauri::AppHandle;

pub struct ArchProvider {
    settings: AppSettings,
}

impl ArchProvider {
    pub fn new(settings: AppSettings) -> Self {
        Self { settings }
    }

    fn helper(&self) -> Option<&'static str> {
        self.settings
            .aur_helper_order
            .iter()
            .find_map(|name| match name.as_str() {
                "yay" if which_exists("yay") => Some("yay"),
                "paru" if which_exists("paru") => Some("paru"),
                _ => None,
            })
    }

    fn install_args(&self, operation: &str, package: &str, helper: &str) -> Vec<String> {
        let mut args = vec![operation.into(), package.into()];
        if helper != "pacman" && self.settings.aur_no_confirm {
            args.push("--noconfirm".into());
        }
        args
    }
}

impl PackageProvider for ArchProvider {
    fn id(&self) -> ProviderId {
        ProviderId::Aur
    }
    fn display_name(&self) -> &'static str {
        "AUR / Arch"
    }
    fn is_available(&self) -> bool {
        self.helper().is_some() || which_exists("pacman")
    }

    fn search(&self, query: &str) -> Result<Vec<PackageCandidate>, String> {
        let query = query.trim();
        if query.is_empty() {
            return Ok(Vec::new());
        }
        let helper = self.helper().unwrap_or("pacman");
        let args = if helper == "pacman" {
            vec!["-Ss".into(), query.into()]
        } else {
            vec!["-Ss".into(), query.into()]
        };
        let output = capture(helper, &args)?;
        let mut results = Vec::new();
        let mut lines = output.lines();
        while let Some(header) = lines.next() {
            let mut fields = header.split_whitespace();
            let Some(name_version) = fields.next() else {
                continue;
            };
            let Some((repo, package_version)) = name_version.split_once('/') else {
                continue;
            };
            let Some((name, version)) = package_version.rsplit_once('-') else {
                continue;
            };
            if !valid_package_id(name) {
                continue;
            }
            let description = lines
                .next()
                .map(|line| line.trim().to_string())
                .filter(|s| !s.is_empty());
            results.push(PackageCandidate {
                provider: if repo.eq_ignore_ascii_case("aur") {
                    ProviderId::Aur
                } else {
                    ProviderId::Pacman
                },
                package_id: name.into(),
                name: name.into(),
                version: Some(version.into()),
                description,
                available: true,
                command_preview: command_preview_for(helper, name, &self.settings),
            });
        }
        Ok(results)
    }

    fn list_installed(&self) -> Result<Vec<InstalledPackage>, String> {
        let args = vec!["-Q".into()];
        let output = capture("pacman", &args)?;
        Ok(output
            .lines()
            .filter_map(|line| {
                let mut fields = line.split_whitespace();
                let name = fields.next()?;
                let version = fields.next()?;
                Some(InstalledPackage {
                    provider: ProviderId::Pacman,
                    package_id: name.into(),
                    name: name.into(),
                    version: Some(version.into()),
                    description: None,
                    desktop_entry: false,
                    appimage_path: None,
                    update_available: false,
                })
            })
            .collect())
    }

    fn install(&self, spec: InstallSpec, app: &AppHandle, job_id: &str) -> Result<(), String> {
        let package = ensure_id(&spec.package_id)?;
        let helper = self.helper().unwrap_or("pacman");
        let args = self.install_args("-S", package, helper);
        run_command(app, job_id, helper, &args, helper == "pacman")
    }

    fn uninstall(
        &self,
        pkg: &InstalledPackage,
        app: &AppHandle,
        job_id: &str,
    ) -> Result<(), String> {
        let package = ensure_id(&pkg.package_id)?;
        let helper = if pkg.provider == ProviderId::Aur {
            self.helper().unwrap_or("pacman")
        } else {
            "pacman"
        };
        let args = self.install_args("-Rns", package, helper);
        run_command(app, job_id, helper, &args, helper == "pacman")
    }

    fn update(&self, pkg: &InstalledPackage, app: &AppHandle, job_id: &str) -> Result<(), String> {
        let helper = if pkg.provider == ProviderId::Aur {
            self.helper().unwrap_or("pacman")
        } else {
            "pacman"
        };
        let package = ensure_id(&pkg.package_id)?;
        let args = self.install_args("-S", package, helper);
        run_command(app, job_id, helper, &args, helper == "pacman")
    }
}

fn ensure_id(id: &str) -> Result<&str, String> {
    if valid_package_id(id) {
        Ok(id)
    } else {
        Err("Invalid package name".into())
    }
}

fn command_preview_for(helper: &str, package: &str, settings: &AppSettings) -> Vec<String> {
    let mut args = vec!["-S".into(), package.into()];
    if helper != "pacman" && settings.aur_no_confirm {
        args.push("--noconfirm".into());
    }
    let mut command = if helper == "pacman" && !crate::runner::is_root() {
        vec!["pkexec".into()]
    } else {
        Vec::new()
    };
    command.push(helper.into());
    command.extend(args);
    command
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aur_confirmation_is_opt_in() {
        let mut settings = AppSettings::default();
        assert_eq!(
            command_preview_for("yay", "neovim", &settings),
            vec!["yay", "-S", "neovim"]
        );
        settings.aur_no_confirm = true;
        assert_eq!(
            command_preview_for("paru", "neovim", &settings),
            vec!["paru", "-S", "neovim", "--noconfirm"]
        );
    }

    #[test]
    fn package_names_cannot_be_options() {
        assert!(ensure_id("--overwrite").is_err());
    }
}
