pub mod appimage;
pub mod arch;
pub mod flatpak;
pub mod native_distro;

use crate::models::{InstallSpec, InstalledPackage, PackageCandidate, ProviderId};
use tauri::AppHandle;

pub trait PackageProvider: Send + Sync {
    fn id(&self) -> ProviderId;
    fn display_name(&self) -> &'static str;
    fn is_available(&self) -> bool;
    fn search(&self, query: &str) -> Result<Vec<PackageCandidate>, String>;
    fn list_installed(&self) -> Result<Vec<InstalledPackage>, String>;
    fn install(&self, spec: InstallSpec, app: &AppHandle, job_id: &str) -> Result<(), String>;
    fn uninstall(
        &self,
        pkg: &InstalledPackage,
        app: &AppHandle,
        job_id: &str,
    ) -> Result<(), String>;
    fn update(&self, pkg: &InstalledPackage, app: &AppHandle, job_id: &str) -> Result<(), String>;
}

pub(crate) fn valid_package_id(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('-')
        && value.len() <= 256
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '+' | '-'))
}
