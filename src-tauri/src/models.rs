use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderId {
    AppImage,
    Flatpak,
    Aur,
    Pacman,
    Apt,
    Dnf,
    Zypper,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DistroFamily {
    Arch,
    Debian,
    Fedora,
    Suse,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageCandidate {
    pub provider: ProviderId,
    pub package_id: String,
    pub name: String,
    pub version: Option<String>,
    pub description: Option<String>,
    pub available: bool,
    pub command_preview: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledPackage {
    pub provider: ProviderId,
    pub package_id: String,
    pub name: String,
    pub version: Option<String>,
    pub description: Option<String>,
    pub desktop_entry: bool,
    pub appimage_path: Option<String>,
    pub update_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallSpec {
    pub provider: ProviderId,
    pub package_id: String,
    pub name: String,
    pub remote: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobEvent {
    pub job_id: String,
    pub timestamp_ms: u128,
    pub stream: String,
    pub line: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppImageEntry {
    pub id: String,
    pub name: String,
    pub path: String,
    pub source: Option<String>,
    pub desktop_entry: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub appimage_directory: String,
    pub scan_directories: Vec<String>,
    pub aur_no_confirm: bool,
    pub aur_helper_order: Vec<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        Self {
            appimage_directory: format!("{home}/Applications"),
            scan_directories: vec![format!("{home}/Applications"), format!("{home}/Downloads")],
            aur_no_confirm: false,
            aur_helper_order: vec!["yay".into(), "paru".into()],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppState {
    pub settings: AppSettings,
    pub appimages: Vec<AppImageEntry>,
    pub tracked_packages: Vec<InstalledPackage>,
}
