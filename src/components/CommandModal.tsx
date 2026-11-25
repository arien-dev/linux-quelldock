import { AlertTriangle, ArrowRight, KeyRound, X } from "lucide-react";
import type { InstalledPackage, PackageCandidate } from "../types";
import { providerLabel } from "../types";

export type PendingAction =
  | { kind: "install"; candidate: PackageCandidate }
  | { kind: "update" | "uninstall"; pkg: InstalledPackage };

export default function CommandModal({
  action, preview, onConfirm, onClose, busy,
}: {
  action: PendingAction;
  preview: string[];
  onConfirm: () => void;
  onClose: () => void;
  busy: boolean;
}) {
  const pkg = action.kind === "install" ? action.candidate : action.pkg;
  const title = action.kind === "install" ? "Install application" : action.kind === "update" ? "Update application" : "Uninstall application";
  return (
    <div className="modal-backdrop" role="presentation" onMouseDown={(event) => { if (event.target === event.currentTarget) onClose(); }}>
      <section className="command-modal" role="dialog" aria-modal="true" aria-labelledby="modal-title">
        <button className="modal-close" onClick={onClose} aria-label="Close"><X size={18} /></button>
        <div className="modal-icon"><AlertTriangle size={20} /></div>
        <span className="eyebrow">COMMAND REVIEW</span><h2 id="modal-title">{title}</h2>
        <p className="modal-description"><strong>{pkg.name}</strong> from <span className={`badge ${pkg.provider}`}>{providerLabel[pkg.provider]}</span></p>
        <label className="field-label">{pkg.provider === "app_image" && action.kind === "uninstall" ? "FILE ACTION" : "EXACT ARGUMENTS"}</label>
        <div className="command-preview">{preview.map((arg, index) => <code key={`${index}-${arg}`}>{arg}</code>)}</div>
        {(pkg.provider === "apt" || pkg.provider === "dnf" || pkg.provider === "zypper" || pkg.provider === "pacman") &&
          <div className="warning-note"><KeyRound size={15} /><span>Administrator authentication may be requested through pkexec. Check the operation in the system prompt.</span></div>}
        {pkg.provider === "aur" && <div className="warning-note"><AlertTriangle size={15} /><span>AUR build scripts are community supplied. Review them; interactive prompts may appear.</span></div>}
        <div className="modal-footer"><button className="button quiet" onClick={onClose}>Cancel</button><button className="button primary" onClick={onConfirm} disabled={busy}>{busy ? "Starting…" : <>{title}<ArrowRight size={15} /></>}</button></div>
      </section>
    </div>
  );
}
