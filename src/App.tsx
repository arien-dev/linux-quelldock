import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { Activity, RefreshCw, Terminal } from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import Sidebar from "./components/Sidebar";
import PullView from "./components/PullView";
import InstalledView from "./components/InstalledView";
import AppImagesView from "./components/AppImagesView";
import ConsolePanel from "./components/ConsolePanel";
import CommandModal, { type PendingAction } from "./components/CommandModal";
import SettingsView from "./components/SettingsView";
import SystemView from "./components/SystemView";
import type { AppImageEntry, AppSettings, InstalledPackage, JobEvent, PackageCandidate, PageId, ProviderId, ProviderStatus } from "./types";

const defaultSettings: AppSettings = {
  appimage_directory: "~/Applications",
  scan_directories: ["~/Applications", "~/Downloads"],
  aur_no_confirm: false,
  aur_helper_order: ["yay", "paru"],
};

export default function App() {
  const [page, setPage] = useState<PageId>("pull");
  const [settings, setSettings] = useState(defaultSettings);
  const [packages, setPackages] = useState<InstalledPackage[]>([]);
  const [appImages, setAppImages] = useState<AppImageEntry[]>([]);
  const [candidates, setCandidates] = useState<PackageCandidate[]>([]);
  const [providers, setProviders] = useState<ProviderStatus[]>([]);
  const [events, setEvents] = useState<JobEvent[]>([]);
  const [activeJob, setActiveJob] = useState<string>();
  const [pending, setPending] = useState<PendingAction>();
  const [searchBusy, setSearchBusy] = useState(false);
  const [checkingUpdates, setCheckingUpdates] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [statePath, setStatePath] = useState("");

  const refresh = useCallback(async () => {
    try {
      const [installed, currentSettings, imageEntries, status] = await Promise.all([
        invoke<InstalledPackage[]>("list_installed_all"),
        invoke<AppSettings>("get_settings"),
        invoke<AppImageEntry[]>("scan_appimages"),
        invoke<ProviderStatus[]>("get_provider_status"),
      ]);
      setPackages(installed);
      setSettings(currentSettings);
      setAppImages(imageEntries);
      setProviders(status);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    }
  }, []);

  useEffect(() => {
    void refresh();
    void invoke<string>("get_state_path").then(setStatePath).catch(() => undefined);
    const unlisten = listen<JobEvent>("job-output", (event) => {
      const payload = event.payload;
      setEvents((current) => [...current.slice(-1499), payload]);
      if (payload.stream === "system" && /completed successfully|exited with/.test(payload.line)) {
        setActiveJob((current) => current === payload.job_id ? undefined : current);
      }
    });
    return () => { void unlisten.then((stop) => stop()); };
  }, [refresh]);

  const runSearch = async (query: string) => {
    if (!query) { setCandidates([]); return; }
    setError(""); setSearchBusy(true);
    try { setCandidates(await invoke<PackageCandidate[]>("pull_search", { query })); }
    catch (reason) { setError(String(reason)); }
    finally { setSearchBusy(false); }
  };

  const beginJob = async (action: PendingAction) => {
    const id = crypto.randomUUID();
    setBusy(true); setError(""); setActiveJob(id); setPending(undefined);
    try {
      if (action.kind === "install") {
        const candidate = action.candidate;
        await invoke("pull_install", { spec: { provider: candidate.provider, package_id: candidate.package_id, name: candidate.name }, jobId: id });
      } else if (action.kind === "update") {
        await invoke("update_package", { pkg: action.pkg, jobId: id });
      } else {
        await invoke("uninstall_package", { pkg: action.pkg, jobId: id });
      }
      await refresh();
    } catch (reason) {
      setError(String(reason));
      setEvents((current) => [...current, { job_id: id, timestamp_ms: Date.now(), stream: "system", line: String(reason) }]);
    } finally {
      setBusy(false); setActiveJob((current) => current === id ? undefined : current);
      await refresh();
    }
  };

  const isRoot = providers[0]?.is_root ?? false;
  const aurHelper = settings.aur_helper_order.find((name) => providers.some((provider) => provider.name === name && provider.available)) ?? "pacman";
  const preview = useMemo(() => pending ? commandPreview(pending, settings, isRoot, aurHelper) : [], [pending, settings, isRoot, aurHelper]);
  const launch = async (pkg: InstalledPackage) => {
    try { await invoke("launch_app", { pkg }); } catch (reason) { setError(String(reason)); }
  };
  const toggleDesktop = async (pkg: InstalledPackage) => {
    try {
      await invoke("toggle_desktop_entry", { id: pkg.package_id, enabled: !pkg.desktop_entry });
      await refresh();
    } catch (reason) { setError(String(reason)); }
  };
  const appImageLaunch = (entry: AppImageEntry) => launch({
    provider: "app_image", package_id: entry.id, name: entry.name, desktop_entry: entry.desktop_entry,
    appimage_path: entry.path, update_available: false,
  });
  const appImageToggle = (entry: AppImageEntry) => toggleDesktop({
    provider: "app_image", package_id: entry.id, name: entry.name, desktop_entry: entry.desktop_entry,
    appimage_path: entry.path, update_available: false,
  });
  const updateSettings = async (value: AppSettings) => {
    try { setSettings(await invoke<AppSettings>("save_settings", { settings: value })); setError(""); await refresh(); }
    catch (reason) { setError(String(reason)); }
  };
  const checkUpdates = async () => {
    setCheckingUpdates(true); setError("");
    try { setPackages(await invoke<InstalledPackage[]>("check_for_updates")); }
    catch (reason) { setError(String(reason)); }
    finally { setCheckingUpdates(false); }
  };
  const pageTitle = page === "console" ? "Console & Logs" : "QuellDock";

  return (
    <div className="app-shell">
      <Sidebar active={page} onNavigate={setPage} />
      <main className="main-area">
        <header className="topbar"><div className="breadcrumb"><span>QUELDOCK</span><b>/</b>{pageTitle}</div><div className="topbar-right">
          <div className="provider-status">{providers.filter((provider) => provider.available).length} providers ready</div>
          <button className="icon-button" title="Refresh library" onClick={() => void refresh()}><RefreshCw size={15} /></button>
        </div></header>
        <div className="content-scroll">
          {error && <div className="global-error">{error}<button onClick={() => setError("")}>Dismiss</button></div>}
          {page === "pull" && <PullView candidates={candidates} busy={searchBusy} onSearch={runSearch} onInstall={(candidate) => setPending({ kind: "install", candidate })} onDownload={(url) => {
            const id = crypto.randomUUID(); setActiveJob(id); setError("");
            void invoke("download_appimage_url", { url, jobId: id }).then(() => refresh()).catch((reason) => setError(String(reason))).finally(() => setActiveJob(undefined));
          }} error="" />}
          {page === "installed" && <InstalledView packages={packages} onLaunch={launch} onUpdate={(pkg) => setPending({ kind: "update", pkg })} onUninstall={(pkg) => setPending({ kind: "uninstall", pkg })} onDesktop={toggleDesktop} onCheckUpdates={checkUpdates} checkingUpdates={checkingUpdates} />}
          {page === "appimages" && <AppImagesView
            entries={appImages}
            onScan={() => void invoke<AppImageEntry[]>("scan_appimages").then(setAppImages).catch((reason) => setError(String(reason)))}
            onImport={(path) => void invoke("import_appimage_file", { path }).then(() => refresh()).catch((reason) => setError(String(reason)))}
            onDownload={(url) => {
              const id = crypto.randomUUID(); setActiveJob(id);
              void invoke("download_appimage_url", { url, jobId: id }).then(() => refresh()).catch((reason) => setError(String(reason))).finally(() => setActiveJob(undefined));
            }}
            onToggle={appImageToggle}
            onLaunch={appImageLaunch}
            onBrowse={() => void open({ multiple: false, filters: [{ name: "AppImage", extensions: ["AppImage", "appimage"] }] }).then((path) => {
              if (typeof path === "string") {
                void invoke("import_appimage_file", { path }).then(() => refresh()).catch((reason) => setError(String(reason)));
              }
            }).catch((reason) => setError(String(reason)))}
          />}
          {page === "flatpak" && <InstalledView packages={packages} scope="flatpak" onLaunch={launch} onUpdate={(pkg) => setPending({ kind: "update", pkg })} onUninstall={(pkg) => setPending({ kind: "uninstall", pkg })} onDesktop={toggleDesktop} onCheckUpdates={checkUpdates} checkingUpdates={checkingUpdates} />}
          {page === "system" && <SystemView packages={packages.filter((pkg) => ["aur", "pacman", "apt", "dnf", "zypper"].includes(pkg.provider))} providers={providers} onLaunch={launch} onUpdate={(pkg) => setPending({ kind: "update", pkg })} onUninstall={(pkg) => setPending({ kind: "uninstall", pkg })} onDesktop={toggleDesktop} onCheckUpdates={checkUpdates} checkingUpdates={checkingUpdates} />}
          {page === "console" && <div className="page console-page"><div className="page-heading"><div><span className="eyebrow">PROCESS OUTPUT</span><h1>Console & Logs</h1><p>Live output from package manager and AppImage jobs.</p></div><div className="heading-icon"><Terminal size={20} /></div></div><ConsolePanel events={events} activeJob={activeJob} onCancel={() => activeJob && void invoke("cancel_job", { jobId: activeJob }).catch((reason) => setError(String(reason)))} onSendInput={(input) => activeJob && void invoke("write_job_input", { jobId: activeJob, input }).catch((reason) => setError(String(reason)))} onClear={() => setEvents([])} expanded /></div>}
          {page === "settings" && <SettingsView settings={settings} statePath={statePath} providers={providers} onSave={updateSettings} onReset={() => {
            if (!window.confirm("Reset QuellDock settings and managed AppImage records? Installed packages and files will not be removed.")) return;
            void invoke<AppSettings>("reset_state").then(setSettings).then(() => refresh()).catch((reason) => setError(String(reason)));
          }} />}
        </div>
        {page !== "console" && <div className="bottom-console">
          <div className="bottom-console-label"><Activity size={14} /><span>LIVE CONSOLE</span><span className={`status-dot ${activeJob ? "running" : ""}`} />{activeJob && <code>{activeJob.slice(0, 8)}</code>}</div>
          <div className="bottom-console-output">{events.length ? events.slice(-2).map((event, index) => <span key={`${event.timestamp_ms}-${index}`}><b>{event.stream}</b> {event.line}</span>) : <span className="muted">Command output will appear here when a job runs.</span>}</div>
          {activeJob && <button className="bottom-stop" onClick={() => void invoke("cancel_job", { jobId: activeJob }).catch((reason) => setError(String(reason)))}>Stop</button>}
        </div>}
      </main>
      {pending && <CommandModal action={pending} preview={preview} busy={busy} onClose={() => setPending(undefined)} onConfirm={() => void beginJob(pending)} />}
    </div>
  );
}

function commandPreview(action: PendingAction, settings: AppSettings, isRoot: boolean, aurHelper: string): string[] {
  if (action.kind === "install") return action.candidate.command_preview;
  const { pkg } = action;
  if (pkg.provider === "flatpak") return ["flatpak", action.kind === "update" ? "update" : "uninstall", "-y", pkg.package_id];
  if (pkg.provider === "app_image") return action.kind === "uninstall"
    ? ["Remove AppImage file", pkg.appimage_path ?? pkg.package_id]
    : ["curl", "--location", "--fail", "--show-error", "--output", `${pkg.appimage_path ?? pkg.package_id}.part`, "--", pkg.description ?? ""];
  const isArch = pkg.provider === "aur" || pkg.provider === "pacman";
  let binary = isArch ? (pkg.provider === "aur" ? aurHelper : "pacman") : pkg.provider === "apt" ? "apt-get" : pkg.provider;
  let args: string[];
  if (isArch) args = [action.kind === "uninstall" ? "-Rns" : "-S", pkg.package_id, ...(binary !== "pacman" && settings.aur_no_confirm ? ["--noconfirm"] : [])];
  else if (pkg.provider === "apt") args = action.kind === "update" ? ["install", "--only-upgrade", "-y", pkg.package_id] : [action.kind === "uninstall" ? "remove" : "install", "-y", pkg.package_id];
  else if (pkg.provider === "zypper") args = action.kind === "update" ? ["update", "-y", pkg.package_id] : [action.kind === "uninstall" ? "remove" : "install", "-y", pkg.package_id];
  else args = action.kind === "update" ? ["upgrade", "-y", pkg.package_id] : [action.kind === "uninstall" ? "remove" : "install", "-y", pkg.package_id];
  const command = !isRoot && (isArch ? binary === "pacman" : true) ? ["pkexec", binary] : [binary];
  return [...command, ...args];
}
