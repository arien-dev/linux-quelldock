export type ProviderId = "app_image" | "flatpak" | "aur" | "pacman" | "apt" | "dnf" | "zypper";
export type PageId = "pull" | "installed" | "appimages" | "flatpak" | "system" | "console" | "settings";

export interface PackageCandidate {
  provider: ProviderId;
  package_id: string;
  name: string;
  version?: string | null;
  description?: string | null;
  available: boolean;
  command_preview: string[];
}

export interface InstalledPackage {
  provider: ProviderId;
  package_id: string;
  name: string;
  version?: string | null;
  description?: string | null;
  desktop_entry: boolean;
  appimage_path?: string | null;
  update_available: boolean;
}

export interface InstallSpec {
  provider: ProviderId;
  package_id: string;
  name: string;
  remote?: string | null;
}

export interface AppImageEntry {
  id: string;
  name: string;
  path: string;
  source?: string | null;
  desktop_entry: boolean;
}

export interface AppSettings {
  appimage_directory: string;
  scan_directories: string[];
  aur_no_confirm: boolean;
  aur_helper_order: string[];
}

export interface JobEvent {
  job_id: string;
  timestamp_ms: number;
  stream: "stdout" | "stderr" | "system";
  line: string;
}

export interface ProviderStatus {
  id: ProviderId;
  name: string;
  available: boolean;
  distro?: string | null;
  is_root: boolean;
}

export const providerLabel: Record<ProviderId, string> = {
  app_image: "AppImage",
  flatpak: "Flatpak",
  aur: "AUR",
  pacman: "Arch",
  apt: "APT",
  dnf: "DNF",
  zypper: "Zypper",
};
