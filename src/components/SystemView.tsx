import { Cpu, ShieldCheck } from "lucide-react";
import InstalledView from "./InstalledView";
import type { InstalledPackage, ProviderStatus } from "../types";

export default function SystemView({ packages, providers, onLaunch, onUpdate, onUninstall, onDesktop, onCheckUpdates, checkingUpdates }: {
  packages: InstalledPackage[];
  providers: ProviderStatus[];
  onLaunch: (pkg: InstalledPackage) => void;
  onUpdate: (pkg: InstalledPackage) => void;
  onUninstall: (pkg: InstalledPackage) => void;
  onDesktop: (pkg: InstalledPackage) => void;
  onCheckUpdates: () => void;
  checkingUpdates: boolean;
}) {
  const distro = providers.find((provider) => provider.distro)?.distro ?? "Unknown";
  return <>
    <section className="page system-tools">
      <div className="page-heading"><div><span className="eyebrow">PACKAGE SOURCES</span><h1>System & AUR</h1><p>Detected distribution: {distro} Linux. System package actions use pkexec when needed.</p></div><div className="heading-icon"><Cpu size={21} /></div></div>
      <div className="system-tool-grid">{providers.filter((provider) => !["AppImage", "Flatpak", "AUR / Arch", "Native distribution"].includes(provider.name)).map((provider) =>
        <div className="system-tool" key={provider.name}><span className={`status-dot ${provider.available ? "ready" : ""}`} /><strong>{provider.name}</strong><span>{provider.available ? "Detected" : "Unavailable"}</span></div>)}</div>
      <div className="system-note"><ShieldCheck size={14} />Review each command before authorizing a privileged package operation.</div>
    </section>
    <InstalledView packages={packages} onLaunch={onLaunch} onUpdate={onUpdate} onUninstall={onUninstall} onDesktop={onDesktop} onCheckUpdates={onCheckUpdates} checkingUpdates={checkingUpdates} />
  </>;
}
