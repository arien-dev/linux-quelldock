import {
  AppWindow, Boxes, CircleHelp, Command, Download, LayoutGrid, PackageSearch, Settings2,
} from "lucide-react";
import type { PageId } from "../types";

const links: { id: PageId; label: string; icon: typeof PackageSearch }[] = [
  { id: "pull", label: "Pull & Discover", icon: PackageSearch },
  { id: "installed", label: "Installed Apps", icon: LayoutGrid },
  { id: "appimages", label: "AppImages", icon: AppWindow },
  { id: "flatpak", label: "Flatpak", icon: Boxes },
  { id: "system", label: "System & AUR", icon: Download },
  { id: "console", label: "Console & Logs", icon: Command },
  { id: "settings", label: "Settings", icon: Settings2 },
];

export default function Sidebar({ active, onNavigate }: { active: PageId; onNavigate: (id: PageId) => void }) {
  return (
    <aside className="sidebar">
      <div className="brand">
        <div className="brand-mark">Q</div>
        <div><strong>QuellDock</strong><small>Linux app manager</small></div>
      </div>
      <nav aria-label="Main navigation">
        <span className="nav-caption">WORKSPACE</span>
        {links.map(({ id, label, icon: Icon }) => (
          <button key={id} className={`nav-item ${active === id ? "active" : ""}`} onClick={() => onNavigate(id)}>
            <Icon size={17} strokeWidth={1.8} /><span>{label}</span>
          </button>
        ))}
      </nav>
      <div className="sidebar-bottom">
        <div className="trust-note"><CircleHelp size={16} /><span>Review commands before running them.</span></div>
        <span className="version">QUELDOCK <b>0.1.0</b></span>
      </div>
    </aside>
  );
}
