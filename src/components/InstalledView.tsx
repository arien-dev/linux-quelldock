import { AppWindow, ArrowDown, ArrowUpCircle, Play, Search, Trash2 } from "lucide-react";
import { useMemo, useState } from "react";
import type { InstalledPackage, ProviderId } from "../types";
import { providerLabel } from "../types";

export default function InstalledView({
  packages, scope, onLaunch, onUpdate, onUninstall, onDesktop,
  onCheckUpdates, checkingUpdates = false,
}: {
  packages: InstalledPackage[];
  scope?: ProviderId | "all";
  onLaunch: (pkg: InstalledPackage) => void;
  onUpdate: (pkg: InstalledPackage) => void;
  onUninstall: (pkg: InstalledPackage) => void;
  onDesktop: (pkg: InstalledPackage) => void;
  onCheckUpdates?: () => void;
  checkingUpdates?: boolean;
}) {
  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState<ProviderId | "all">(scope ?? "all");
  const [updates, setUpdates] = useState<"all" | "available" | "current">("all");
  const visible = useMemo(() => packages.filter((pkg) =>
    (filter === "all" || pkg.provider === filter)
    && (updates === "all" || (updates === "available" ? pkg.update_available : !pkg.update_available))
    && `${pkg.name} ${pkg.package_id}`.toLowerCase().includes(query.toLowerCase())), [packages, filter, updates, query]);
  return (
    <section className="page">
      <div className="page-heading">
        <div><span className="eyebrow">YOUR LIBRARY</span><h1>{scope === "app_image" ? "AppImages" : scope === "flatpak" ? "Flatpak" : scope === "aur" || scope === "pacman" ? "System & AUR" : "Installed apps"}</h1><p>Applications and packages discovered from your configured providers.</p></div>
        <div className="heading-actions">{onCheckUpdates && <button className="button secondary" onClick={onCheckUpdates} disabled={checkingUpdates}><ArrowUpCircle size={14} />{checkingUpdates ? "Checking…" : "Check updates"}</button>}<div className="count-card"><strong>{visible.length}</strong><span>installed</span></div></div>
      </div>
      <div className="library-toolbar">
        <label className="search-box"><Search size={17} /><input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="Filter your library…" /></label>
        {!scope && <select value={filter} onChange={(event) => setFilter(event.target.value as ProviderId | "all")} aria-label="Filter by source">
          <option value="all">All sources</option>{Object.entries(providerLabel).map(([id, label]) => <option value={id} key={id}>{label}</option>)}
        </select>}
        <select value={updates} onChange={(event) => setUpdates(event.target.value as "all" | "available" | "current")} aria-label="Filter by update status">
          <option value="all">Any update status</option><option value="available">Updates available</option><option value="current">Up to date</option>
        </select>
      </div>
      {visible.length === 0 ? <div className="empty-state compact"><div className="empty-icon"><AppWindow size={22} /></div><h3>No matching apps</h3><p>Try a different filter, or scan your AppImage folders.</p></div> : (
        <div className="installed-list">
          {visible.map((pkg) => <article className="installed-row" key={`${pkg.provider}:${pkg.package_id}`}>
            <div className={`app-avatar small ${pkg.provider}`}>{pkg.name.slice(0, 1).toUpperCase()}</div>
            <div className="installed-info"><strong>{pkg.name}</strong><span>{pkg.package_id}{pkg.version ? ` · ${pkg.version}` : ""}</span></div>
            <span className={`badge ${pkg.provider}`}>{providerLabel[pkg.provider]}</span>
            {pkg.update_available && <span className="badge update-badge">Update available</span>}
            <div className="row-actions">
              <button title="Launch" onClick={() => onLaunch(pkg)}><Play size={15} /></button>
              {pkg.provider === "app_image" && <button title={pkg.desktop_entry ? "Remove desktop entry" : "Create desktop entry"} onClick={() => onDesktop(pkg)}><AppWindow size={15} /></button>}
              {(pkg.provider !== "app_image" || pkg.description?.startsWith("http")) && <button title="Update" onClick={() => onUpdate(pkg)}><ArrowUpCircle size={16} /></button>}
              <button title="Uninstall" className="danger-action" onClick={() => onUninstall(pkg)}><Trash2 size={15} /></button>
            </div>
          </article>)}
        </div>
      )}
      <div className="library-footer"><ArrowDown size={14} /> Installed package versions are reported by local provider tools.</div>
    </section>
  );
}
