mod desktop_entry;
mod models;
mod providers;
mod runner;
mod state;

use models::{
    AppImageEntry, AppSettings, InstallSpec, InstalledPackage, PackageCandidate, ProviderId,
};
use providers::{
    appimage::AppImageProvider, arch::ArchProvider, flatpak::FlatpakProvider,
    native_distro::NativeProvider, PackageProvider,
};
use state::StateStore;
use std::{
    collections::HashSet,
    path::PathBuf,
    process::{Command, Stdio},
    sync::Mutex,
    thread,
};
use tauri::State;

struct AppData {
    store: StateStore,
}

#[derive(serde::Serialize)]
struct ProviderStatus {
    id: ProviderId,
    name: String,
    available: bool,
    distro: Option<String>,
    is_root: bool,
}

#[tauri::command]
fn get_settings(data: State<'_, AppData>) -> Result<AppSettings, String> {
    Ok(data.store.snapshot()?.settings)
}

#[tauri::command]
fn save_settings(settings: AppSettings, data: State<'_, AppData>) -> Result<AppSettings, String> {
    Ok(data
        .store
        .update(|state| state.settings = settings)?
        .settings)
}

#[tauri::command]
fn get_state_path(data: State<'_, AppData>) -> String {
    data.store.path().to_string_lossy().into_owned()
}

#[tauri::command]
fn reset_state(data: State<'_, AppData>) -> Result<AppSettings, String> {
    Ok(data.store.reset()?.settings)
}

#[tauri::command]
fn pull_search(query: String, data: State<'_, AppData>) -> Result<Vec<PackageCandidate>, String> {
    let query = query.trim().to_string();
    if query.is_empty() {
        return Ok(Vec::new());
    }
    if query.len() > 200 {
        return Err("Search query is too long".into());
    }
    let settings = data.store.snapshot()?.settings;
    let providers: Vec<Box<dyn PackageProvider>> = vec![
        Box::new(FlatpakProvider),
        Box::new(ArchProvider::new(settings)),
        Box::new(NativeProvider::detect()),
    ];
    let results = Mutex::new(Vec::new());
    thread::scope(|scope| {
        for provider in providers {
            let results = &results;
            let query = query.clone();
            scope.spawn(move || {
                if provider.is_available() {
                    if let Ok(found) = provider.search(&query) {
                        if let Ok(mut results) = results.lock() {
                            results.extend(found);
                        }
                    }
                }
            });
        }
    });
    results
        .into_inner()
        .map_err(|_| "Search results lock was poisoned".to_string())
}

#[tauri::command]
fn pull_install(
    spec: InstallSpec,
    job_id: String,
    app: tauri::AppHandle,
    data: State<'_, AppData>,
) -> Result<(), String> {
    let settings = data.store.snapshot()?.settings;
    provider_for(spec.provider, settings).install(spec.clone(), &app, &job_id)?;
    if matches!(spec.provider, ProviderId::Aur | ProviderId::Pacman) {
        data.store.update(|state| {
            state
                .tracked_packages
                .retain(|pkg| pkg.package_id != spec.package_id);
            state.tracked_packages.push(InstalledPackage {
                provider: spec.provider,
                package_id: spec.package_id,
                name: spec.name,
                version: None,
                description: None,
                desktop_entry: false,
                appimage_path: None,
                update_available: false,
            });
        })?;
    }
    Ok(())
}

#[tauri::command]
fn download_appimage_url(
    url: String,
    job_id: String,
    app: tauri::AppHandle,
    data: State<'_, AppData>,
) -> Result<AppImageEntry, String> {
    let settings = data.store.snapshot()?.settings;
    let entry = providers::appimage::download_url(
        &app,
        &job_id,
        &url,
        &PathBuf::from(settings.appimage_directory),
    )?;
    let updated = data
        .store
        .update(|state| upsert_appimage(&mut state.appimages, entry.clone()))?;
    updated
        .appimages
        .into_iter()
        .find(|saved| saved.id == entry.id && saved.path == entry.path)
        .ok_or_else(|| "Downloaded AppImage was not saved to state".into())
}

#[tauri::command]
fn import_appimage_file(path: String, data: State<'_, AppData>) -> Result<AppImageEntry, String> {
    let settings = data.store.snapshot()?.settings;
    let entry = providers::appimage::import_file(
        PathBuf::from(path).as_path(),
        PathBuf::from(settings.appimage_directory).as_path(),
    )?;
    data.store
        .update(|state| upsert_appimage(&mut state.appimages, entry.clone()))?;
    Ok(entry)
}

#[tauri::command]
fn scan_appimages(data: State<'_, AppData>) -> Result<Vec<AppImageEntry>, String> {
    let settings = data.store.snapshot()?.settings;
    let directories: Vec<PathBuf> = settings
        .scan_directories
        .iter()
        .map(PathBuf::from)
        .collect();
    let scanned = providers::appimage::scan_directories(&directories);
    let state = data.store.update(|state| {
        for entry in &scanned {
            let mut merged = entry.clone();
            if let Some(old) = state.appimages.iter().find(|old| old.path == entry.path) {
                merged.source = old.source.clone();
                merged.desktop_entry = old.desktop_entry;
            }
            upsert_appimage(&mut state.appimages, merged);
        }
        state
            .appimages
            .retain(|entry| scanned.iter().any(|found| found.path == entry.path));
    })?;
    Ok(state.appimages)
}

#[tauri::command]
fn list_installed_all(data: State<'_, AppData>) -> Result<Vec<InstalledPackage>, String> {
    list_installed(&data)
}

fn list_installed(data: &AppData) -> Result<Vec<InstalledPackage>, String> {
    let settings = data.store.snapshot()?;
    let mut installed = Vec::new();
    let mut image_directories: Vec<PathBuf> = settings
        .settings
        .scan_directories
        .iter()
        .map(PathBuf::from)
        .collect();
    image_directories.push(PathBuf::from(&settings.settings.appimage_directory));
    for entry in providers::appimage::scan_directories(&image_directories) {
        let was_integrated = settings
            .appimages
            .iter()
            .any(|saved| saved.path == entry.path && saved.desktop_entry);
        installed.push(InstalledPackage {
            provider: ProviderId::AppImage,
            package_id: entry.id,
            name: entry.name,
            version: None,
            description: entry.source,
            desktop_entry: was_integrated,
            appimage_path: Some(entry.path),
            update_available: false,
        });
    }
    let flatpak = FlatpakProvider;
    if flatpak.is_available() {
        if let Ok(packages) = flatpak.list_installed() {
            installed.extend(packages);
        }
    }
    let arch = ArchProvider::new(settings.settings.clone());
    if arch.is_available() {
        if let Ok(packages) = arch.list_installed() {
            installed.extend(packages);
        }
    }
    let native = NativeProvider::detect();
    if native.is_available() {
        if let Ok(packages) = native.list_installed() {
            installed.extend(packages);
        }
    }
    for package in settings.tracked_packages {
        if package.provider == ProviderId::Aur {
            installed.retain(|existing| {
                !(existing.provider == ProviderId::Pacman
                    && existing.package_id == package.package_id)
            });
        }
        if !installed.iter().any(|existing| {
            existing.provider == package.provider && existing.package_id == package.package_id
        }) {
            installed.push(package);
        }
    }
    installed.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(installed)
}

#[tauri::command]
fn check_for_updates(data: State<'_, AppData>) -> Result<Vec<InstalledPackage>, String> {
    let settings = data.store.snapshot()?.settings;
    let mut installed = list_installed(&data)?;

    if installed
        .iter()
        .any(|package| package.provider == ProviderId::Flatpak)
    {
        let output = runner::capture(
            "flatpak",
            &[
                "remote-ls".into(),
                "--updates".into(),
                "--columns=application".into(),
            ],
        )?;
        mark_updates(
            &mut installed,
            ProviderId::Flatpak,
            parse_update_ids(ProviderId::Flatpak, &output),
        );
    }

    if installed
        .iter()
        .any(|package| matches!(package.provider, ProviderId::Aur | ProviderId::Pacman))
    {
        let output = if runner::which_exists("checkupdates") {
            runner::capture_output("checkupdates", &[])?
        } else {
            runner::capture_output("pacman", &["-Qu".into()])?
        };
        if !output.status.success() {
            return Err(output_error("Arch update check", &output));
        }
        mark_updates(
            &mut installed,
            ProviderId::Pacman,
            parse_update_ids(ProviderId::Pacman, &String::from_utf8_lossy(&output.stdout)),
        );

        if installed
            .iter()
            .any(|package| package.provider == ProviderId::Aur)
        {
            let helper = settings
                .aur_helper_order
                .iter()
                .find(|name| matches!(name.as_str(), "yay" | "paru") && runner::which_exists(name))
                .ok_or("An AUR helper is required to check AUR updates")?;
            let output = runner::capture(helper, &["-Qua".into()])?;
            mark_updates(
                &mut installed,
                ProviderId::Aur,
                parse_update_ids(ProviderId::Aur, &output),
            );
        }
    }

    if let Some(provider) = installed.iter().find_map(|package| match package.provider {
        ProviderId::Apt | ProviderId::Dnf | ProviderId::Zypper => Some(package.provider),
        _ => None,
    }) {
        let updates = match provider {
            ProviderId::Apt => runner::capture("apt", &["list".into(), "--upgradable".into()])?,
            ProviderId::Dnf => {
                let output = runner::capture_output("dnf", &["check-update".into()])?;
                if !output.status.success() && output.status.code() != Some(100) {
                    return Err(output_error("DNF update check", &output));
                }
                String::from_utf8_lossy(&output.stdout).into_owned()
            }
            ProviderId::Zypper => runner::capture(
                "zypper",
                &["list-updates".into(), "--type".into(), "package".into()],
            )?,
            _ => String::new(),
        };
        mark_updates(
            &mut installed,
            provider,
            parse_update_ids(provider, &updates),
        );
    }

    Ok(installed)
}

fn mark_updates(packages: &mut [InstalledPackage], provider: ProviderId, updates: HashSet<String>) {
    for package in packages
        .iter_mut()
        .filter(|package| package.provider == provider)
    {
        package.update_available = updates.contains(&package.package_id);
    }
}

fn parse_update_ids(provider: ProviderId, output: &str) -> HashSet<String> {
    output
        .lines()
        .filter_map(|line| {
            let name = match provider {
                ProviderId::Apt => {
                    let name = line.trim().split('/').next()?;
                    if name.starts_with("Listing") {
                        return None;
                    }
                    name
                }
                ProviderId::Zypper => *line.split('|').map(str::trim).collect::<Vec<_>>().get(2)?,
                ProviderId::Dnf => {
                    let name = line.split_whitespace().next()?;
                    name.strip_suffix(".x86_64")
                        .or_else(|| name.strip_suffix(".aarch64"))
                        .or_else(|| name.strip_suffix(".noarch"))
                        .or_else(|| name.strip_suffix(".i686"))
                        .unwrap_or(name)
                }
                _ => line
                    .split_whitespace()
                    .next()?
                    .rsplit('/')
                    .next()
                    .unwrap_or_default(),
            };
            valid_update_name(name).then(|| name.to_string())
        })
        .collect()
}

fn valid_update_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_alphanumeric())
}

fn output_error(context: &str, output: &std::process::Output) -> String {
    let message = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if message.is_empty() {
        format!("{context} failed with {}", output.status)
    } else {
        format!("{context} failed: {message}")
    }
}

#[tauri::command]
fn launch_app(pkg: InstalledPackage) -> Result<(), String> {
    if let Some(path) = pkg.appimage_path.as_deref() {
        if PathBuf::from(path)
            .extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| extension.eq_ignore_ascii_case("appimage"))
            != Some(true)
        {
            return Err("Refusing to launch a file that is not an AppImage".into());
        }
        return providers::appimage::launch(PathBuf::from(path).as_path());
    }
    let (binary, args) = match pkg.provider {
        ProviderId::Flatpak => ("flatpak", vec!["run".to_string(), pkg.package_id]),
        _ => ("gtk-launch", vec![pkg.package_id]),
    };
    if !runner::which_exists(binary) {
        return Err(format!("Cannot find `{binary}` to launch this application"));
    }
    Command::new(binary)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("Cannot launch application: {error}"))?;
    Ok(())
}

#[tauri::command]
fn update_package(
    pkg: InstalledPackage,
    job_id: String,
    app: tauri::AppHandle,
    data: State<'_, AppData>,
) -> Result<(), String> {
    provider_for(pkg.provider, data.store.snapshot()?.settings).update(&pkg, &app, &job_id)
}

#[tauri::command]
fn uninstall_package(
    pkg: InstalledPackage,
    job_id: String,
    app: tauri::AppHandle,
    data: State<'_, AppData>,
) -> Result<(), String> {
    if pkg.provider == ProviderId::AppImage {
        let state = data.store.snapshot()?;
        if !state.appimages.iter().any(|entry| {
            entry.id == pkg.package_id && Some(entry.path.as_str()) == pkg.appimage_path.as_deref()
        }) {
            return Err("Refusing to remove an AppImage that is not managed by QuellDock".into());
        }
        providers::appimage::AppImageProvider.uninstall(&pkg, &app, &job_id)?;
        data.store
            .update(|state| state.appimages.retain(|entry| entry.id != pkg.package_id))?;
        return Ok(());
    }
    provider_for(pkg.provider, data.store.snapshot()?.settings).uninstall(&pkg, &app, &job_id)?;
    data.store.update(|state| {
        state
            .tracked_packages
            .retain(|tracked| tracked.package_id != pkg.package_id)
    })?;
    Ok(())
}

#[tauri::command]
fn toggle_desktop_entry(id: String, enabled: bool, data: State<'_, AppData>) -> Result<(), String> {
    let state = data.store.snapshot()?;
    let entry = state
        .appimages
        .iter()
        .find(|entry| entry.id == id)
        .ok_or("AppImage entry was not found")?;
    if enabled {
        desktop_entry::create_desktop_entry(
            &entry.id,
            &entry.name,
            PathBuf::from(&entry.path).as_path(),
        )?;
    } else {
        desktop_entry::remove_desktop_entry(&entry.id)?;
    }
    data.store.update(|state| {
        if let Some(entry) = state.appimages.iter_mut().find(|entry| entry.id == id) {
            entry.desktop_entry = enabled;
        }
    })?;
    Ok(())
}

#[tauri::command]
fn cancel_job(job_id: String) -> Result<(), String> {
    runner::cancel_job(&job_id)
}

#[tauri::command]
fn write_job_input(job_id: String, input: String) -> Result<(), String> {
    runner::write_job_input(&job_id, &input)
}

#[tauri::command]
fn get_provider_status() -> Vec<ProviderStatus> {
    let native = NativeProvider::detect();
    let arch = ArchProvider::new(AppSettings::default());
    let provider_objects: Vec<&dyn PackageProvider> =
        vec![&AppImageProvider, &FlatpakProvider, &arch, &native];
    let mut statuses: Vec<ProviderStatus> = provider_objects
        .into_iter()
        .map(|provider| ProviderStatus {
            id: provider.id(),
            name: provider.display_name().into(),
            available: provider.is_available(),
            distro: (provider.id() == native.id()).then(|| format!("{:?}", native.family())),
            is_root: runner::is_root(),
        })
        .collect();
    for (binary, provider) in [
        ("yay", ProviderId::Aur),
        ("paru", ProviderId::Aur),
        ("pacman", ProviderId::Pacman),
        ("apt", ProviderId::Apt),
        ("apt-get", ProviderId::Apt),
        ("dnf", ProviderId::Dnf),
        ("zypper", ProviderId::Zypper),
    ] {
        statuses.push(ProviderStatus {
            id: provider,
            name: binary.into(),
            available: runner::which_exists(binary),
            distro: None,
            is_root: runner::is_root(),
        });
    }
    statuses
}

fn provider_for(provider: ProviderId, settings: AppSettings) -> Box<dyn PackageProvider> {
    match provider {
        ProviderId::Flatpak => Box::new(FlatpakProvider),
        ProviderId::Aur | ProviderId::Pacman => Box::new(ArchProvider::new(settings)),
        ProviderId::Apt | ProviderId::Dnf | ProviderId::Zypper => {
            Box::new(NativeProvider::detect())
        }
        ProviderId::AppImage => Box::new(AppImageProvider),
    }
}

fn upsert_appimage(entries: &mut Vec<AppImageEntry>, entry: AppImageEntry) {
    if let Some(existing) = entries
        .iter_mut()
        .find(|existing| existing.id == entry.id || existing.path == entry.path)
    {
        *existing = entry;
    } else {
        entries.push(entry);
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppData {
            store: StateStore::open(StateStore::default_path()),
        })
        .invoke_handler(tauri::generate_handler![
            pull_search,
            pull_install,
            download_appimage_url,
            import_appimage_file,
            scan_appimages,
            list_installed_all,
            check_for_updates,
            launch_app,
            update_package,
            uninstall_package,
            toggle_desktop_entry,
            cancel_job,
            write_job_input,
            get_settings,
            save_settings,
            reset_state,
            get_state_path,
            get_provider_status
        ])
        .run(tauri::generate_context!())
        .expect("error while running QuellDock");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appimage_state_entries_are_upserted() {
        let mut entries = Vec::new();
        let entry = AppImageEntry {
            id: "editor".into(),
            name: "Editor".into(),
            path: "/apps/editor.AppImage".into(),
            source: None,
            desktop_entry: false,
        };
        upsert_appimage(&mut entries, entry.clone());
        let mut changed = entry;
        changed.desktop_entry = true;
        upsert_appimage(&mut entries, changed);
        assert_eq!(entries.len(), 1);
        assert!(entries[0].desktop_entry);
    }

    #[test]
    fn update_check_parser_extracts_provider_package_names() {
        let apt = parse_update_ids(
            ProviderId::Apt,
            "Listing... Done\neditor/noble 1.2 amd64 [upgradable from: 1.1]\n",
        );
        assert!(apt.contains("editor"));
        let flatpak = parse_update_ids(ProviderId::Flatpak, "org.example.Editor\n");
        assert!(flatpak.contains("org.example.Editor"));
        let dnf = parse_update_ids(ProviderId::Dnf, "editor.x86_64 1.2 updates\n");
        assert!(dnf.contains("editor"));
        let zypper = parse_update_ids(
            ProviderId::Zypper,
            "| repo | editor | 1.1 | 1.2 | x86_64 |\n",
        );
        assert!(zypper.contains("editor"));
    }
}
