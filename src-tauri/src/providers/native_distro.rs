use super::{valid_package_id, PackageProvider};
use crate::{
    models::{DistroFamily, InstallSpec, InstalledPackage, PackageCandidate, ProviderId},
    runner::{capture, run_command, which_exists},
};
use std::{fs, path::Path};
use tauri::AppHandle;

#[derive(Debug, Clone)]
pub struct NativeProvider {
    family: DistroFamily,
}

impl NativeProvider {
    pub fn detect() -> Self {
        Self {
            family: distro_from_os_release(Path::new("/etc/os-release")),
        }
    }

    pub fn family(&self) -> DistroFamily {
        self.family
    }

    fn tool(&self) -> Option<(&'static str, ProviderId)> {
        match self.family {
            DistroFamily::Debian => {
                if which_exists("apt-get") {
                    Some(("apt-get", ProviderId::Apt))
                } else {
                    None
                }
            }
            DistroFamily::Fedora => {
                if which_exists("dnf") {
                    Some(("dnf", ProviderId::Dnf))
                } else {
                    None
                }
            }
            DistroFamily::Suse => {
                if which_exists("zypper") {
                    Some(("zypper", ProviderId::Zypper))
                } else {
                    None
                }
            }
            DistroFamily::Arch | DistroFamily::Unknown => None,
        }
    }
}

impl PackageProvider for NativeProvider {
    fn id(&self) -> ProviderId {
        self.tool().map(|(_, id)| id).unwrap_or(ProviderId::Apt)
    }
    fn display_name(&self) -> &'static str {
        "Native distribution"
    }
    fn is_available(&self) -> bool {
        self.tool().is_some()
    }

    fn search(&self, query: &str) -> Result<Vec<PackageCandidate>, String> {
        let Some((tool, provider)) = self.tool() else {
            return Ok(Vec::new());
        };
        let query = query.trim();
        if query.is_empty() {
            return Ok(Vec::new());
        }
        let args = match tool {
            "apt-get" => vec!["search".into(), query.into()],
            "dnf" => vec!["search".into(), query.into()],
            _ => vec!["search".into(), "--details".into(), query.into()],
        };
        let output = if tool == "apt-get" {
            capture("apt-cache", &args)?
        } else {
            capture(tool, &args)?
        };
        Ok(parse_search_output(&output, provider)
            .into_iter()
            .map(|(name, description)| PackageCandidate {
                provider,
                command_preview: install_preview(tool, provider, "install", &name),
                package_id: name.clone(),
                name,
                version: None,
                description,
                available: true,
            })
            .collect())
    }

    fn list_installed(&self) -> Result<Vec<InstalledPackage>, String> {
        let Some((tool, provider)) = self.tool() else {
            return Ok(Vec::new());
        };
        let output = match tool {
            "apt-get" => capture(
                "dpkg-query",
                &vec!["-W".into(), "-f=${binary:Package}\\t${Version}\\n".into()],
            )?,
            "dnf" => capture(
                "rpm",
                &vec![
                    "-qa".into(),
                    "--qf".into(),
                    "%{NAME}\\t%{VERSION}-%{RELEASE}\\n".into(),
                ],
            )?,
            _ => capture(
                "zypper",
                &vec![
                    "search".into(),
                    "--installed-only".into(),
                    "--details".into(),
                ],
            )?,
        };
        Ok(parse_installed_output(&output, provider))
    }

    fn install(&self, spec: InstallSpec, app: &AppHandle, job_id: &str) -> Result<(), String> {
        let Some((tool, _)) = self.tool() else {
            return Err("No supported native package manager was detected".into());
        };
        let id = ensure_id(&spec.package_id)?;
        let args = match tool {
            "apt-get" => vec!["install".into(), "-y".into(), id.into()],
            "dnf" => vec!["install".into(), "-y".into(), id.into()],
            _ => vec!["install".into(), "-y".into(), id.into()],
        };
        run_command(app, job_id, tool, &args, true)
    }

    fn uninstall(
        &self,
        pkg: &InstalledPackage,
        app: &AppHandle,
        job_id: &str,
    ) -> Result<(), String> {
        let Some((tool, _)) = self.tool() else {
            return Err("No supported native package manager was detected".into());
        };
        let id = ensure_id(&pkg.package_id)?;
        let args = match tool {
            "apt-get" => vec!["remove".into(), "-y".into(), id.into()],
            "dnf" => vec!["remove".into(), "-y".into(), id.into()],
            _ => vec!["remove".into(), "-y".into(), id.into()],
        };
        run_command(app, job_id, tool, &args, true)
    }

    fn update(&self, pkg: &InstalledPackage, app: &AppHandle, job_id: &str) -> Result<(), String> {
        let Some((tool, _)) = self.tool() else {
            return Err("No supported native package manager was detected".into());
        };
        let id = ensure_id(&pkg.package_id)?;
        let args = match tool {
            "apt-get" => vec![
                "install".into(),
                "--only-upgrade".into(),
                "-y".into(),
                id.into(),
            ],
            "dnf" => vec!["upgrade".into(), "-y".into(), id.into()],
            _ => vec!["update".into(), "-y".into(), id.into()],
        };
        run_command(app, job_id, tool, &args, true)
    }
}

pub fn distro_from_os_release(path: &Path) -> DistroFamily {
    let contents = fs::read_to_string(path)
        .unwrap_or_default()
        .to_ascii_lowercase();
    let id = contents
        .lines()
        .find_map(|line| line.strip_prefix("id="))
        .unwrap_or("")
        .trim_matches('"');
    let id_like = contents
        .lines()
        .find_map(|line| line.strip_prefix("id_like="))
        .unwrap_or("")
        .trim_matches('"');
    let value = format!("{id} {id_like}");
    if ["arch", "manjaro", "endeavouros"]
        .iter()
        .any(|name| value.split_whitespace().any(|part| part == *name))
    {
        DistroFamily::Arch
    } else if ["debian", "ubuntu", "linuxmint", "pop"]
        .iter()
        .any(|name| value.split_whitespace().any(|part| part == *name))
    {
        DistroFamily::Debian
    } else if ["fedora", "rhel", "centos"]
        .iter()
        .any(|name| value.split_whitespace().any(|part| part == *name))
    {
        DistroFamily::Fedora
    } else if ["opensuse", "suse"]
        .iter()
        .any(|name| value.split_whitespace().any(|part| part == *name))
    {
        DistroFamily::Suse
    } else {
        DistroFamily::Unknown
    }
}

fn parse_search_output(output: &str, provider: ProviderId) -> Vec<(String, Option<String>)> {
    output
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            let (name, description) = if provider == ProviderId::Apt {
                let (left, right) = trimmed.split_once(" - ")?;
                (
                    left.split('/').next().unwrap_or(left),
                    Some(right.to_string()),
                )
            } else if provider == ProviderId::Dnf {
                let name = trimmed.split_whitespace().next()?;
                (name, Some(trimmed.to_string()))
            } else {
                let fields: Vec<_> = trimmed.split('|').map(str::trim).collect();
                if fields.len() < 3
                    || !fields[0]
                        .chars()
                        .next()
                        .is_some_and(|c| c.is_ascii_alphanumeric())
                {
                    return None;
                }
                (fields[1], Some(fields.last()?.to_string()))
            };
            if valid_package_id(name) {
                Some((name.into(), description))
            } else {
                None
            }
        })
        .collect()
}

fn parse_installed_output(output: &str, provider: ProviderId) -> Vec<InstalledPackage> {
    output
        .lines()
        .filter_map(|line| {
            let mut columns = line.split('\t');
            let name = columns.next()?.trim();
            if !valid_package_id(name) {
                return None;
            }
            Some(InstalledPackage {
                provider,
                package_id: name.into(),
                name: name.into(),
                version: columns
                    .next()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string),
                description: None,
                desktop_entry: false,
                appimage_path: None,
                update_available: false,
            })
        })
        .collect()
}

fn install_preview(tool: &str, provider: ProviderId, _action: &str, package: &str) -> Vec<String> {
    let args = if tool == "apt-get" {
        vec!["apt-get", "install", "-y", package]
    } else {
        vec![tool, "install", "-y", package]
    };
    let _ = provider;
    let mut command: Vec<String> = if crate::runner::is_root() {
        Vec::new()
    } else {
        vec!["pkexec".into()]
    };
    command.extend(args.into_iter().map(str::to_string));
    command
}

fn ensure_id(id: &str) -> Result<&str, String> {
    if valid_package_id(id) {
        Ok(id)
    } else {
        Err("Invalid package name".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_supported_os_release_families() {
        let path = std::env::temp_dir().join("quelldock-os-release-test");
        fs::write(&path, "ID=linuxmint\nID_LIKE=ubuntu debian\n").unwrap();
        assert_eq!(distro_from_os_release(&path), DistroFamily::Debian);
        fs::remove_file(path).unwrap();
        assert_eq!(
            distro_from_os_release(Path::new("/definitely/missing")),
            DistroFamily::Unknown
        );
    }

    #[test]
    fn parses_package_search_and_rejects_options() {
        let rows =
            parse_search_output("editor - useful editor\n--bad - invalid\n", ProviderId::Apt);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].0, "editor");
    }
}
