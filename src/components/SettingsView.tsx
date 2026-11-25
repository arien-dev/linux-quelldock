import { FolderOpen, Info, Save, ShieldCheck } from "lucide-react";
import { useEffect, useState } from "react";
import type { AppSettings, ProviderStatus } from "../types";

export default function SettingsView({ settings, statePath, providers, onSave, onReset }: {
  settings: AppSettings;
  statePath: string;
  providers: ProviderStatus[];
  onSave: (settings: AppSettings) => void;
  onReset: () => void;
}) {
  const [draft, setDraft] = useState(settings);
  useEffect(() => setDraft(settings), [settings]);
  const set = <K extends keyof AppSettings>(key: K, value: AppSettings[K]) => setDraft((current) => ({ ...current, [key]: value }));
  return (
    <section className="page">
      <div className="page-heading"><div><span className="eyebrow">PREFERENCES</span><h1>Settings</h1><p>Configure storage, discovery, and package helper behavior.</p></div><div className="heading-icon"><ShieldCheck size={21} /></div></div>
      <div className="settings-card">
        <div className="settings-heading"><div className="panel-icon mint"><FolderOpen size={18} /></div><div><h3>AppImage locations</h3><p>Where QuellDock keeps imported binaries and searches for existing files.</p></div></div>
        <label className="field-label">MANAGED APPIMAGE DIRECTORY</label><input value={draft.appimage_directory} onChange={(event) => set("appimage_directory", event.target.value)} />
        <label className="field-label">SCAN DIRECTORIES <small>one path per line</small></label><textarea rows={3} value={draft.scan_directories.join("\n")} onChange={(event) => set("scan_directories", event.target.value.split("\n").map((s) => s.trim()).filter(Boolean))} />
      </div>
      <div className="settings-card">
        <div className="settings-heading"><div className="panel-icon blue"><Info size={18} /></div><div><h3>Package manager behavior</h3><p>Set conservative defaults for system-level operations.</p></div></div>
        <div className="settings-info"><ShieldCheck size={15} /><span>System package managers use pkexec when administrator privileges are required. Authentication is handled by the system; QuellDock never stores credentials.</span></div>
        <label className="toggle-row"><span><strong>Pass --noconfirm to AUR helpers</strong><small>When disabled (recommended), yay and paru retain interactive prompts.</small></span><input type="checkbox" checked={draft.aur_no_confirm} onChange={(event) => set("aur_no_confirm", event.target.checked)} /></label>
        <label className="field-label">AUR HELPER ORDER</label><input value={draft.aur_helper_order.join(", ")} onChange={(event) => set("aur_helper_order", event.target.value.split(",").map((s) => s.trim()).filter(Boolean))} placeholder="yay, paru" />
      </div>
      <div className="settings-card">
        <div className="settings-heading"><div className="panel-icon mint"><Info size={18} /></div><div><h3>Detected providers</h3><p>QuellDock detects these local tools and your distribution at startup.</p></div></div>
        <div className="provider-grid">{providers.map((provider, index) => <div className="provider-row" key={`${provider.name}-${index}`}>
          <span className={`status-dot ${provider.available ? "ready" : ""}`} /><strong>{provider.name}</strong><span>{provider.available ? "Available" : "Not found"}</span>
          {provider.distro && <small>{provider.distro} Linux</small>}
        </div>)}</div>
      </div>
      <div className="state-path"><Info size={15} /><span>State file</span><code>{statePath || "~/.local/share/quelldock/state.json"}</code><button className="button quiet" onClick={onReset}>Reset local state</button></div>
      <div className="settings-save"><button className="button primary" onClick={() => onSave(draft)}><Save size={15} /> Save settings</button></div>
    </section>
  );
}
