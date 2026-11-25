import { FileDown, FolderSearch, Link2, Plus, ScanLine } from "lucide-react";
import { useState } from "react";
import type { AppImageEntry } from "../types";

export default function AppImagesView({
  entries, onScan, onImport, onDownload, onToggle, onLaunch,
  onBrowse,
}: {
  entries: AppImageEntry[];
  onScan: () => void;
  onImport: (path: string) => void;
  onDownload: (url: string) => void;
  onToggle: (entry: AppImageEntry) => void;
  onLaunch: (entry: AppImageEntry) => void;
  onBrowse: () => void;
}) {
  const [path, setPath] = useState("");
  const [url, setUrl] = useState("");
  return (
    <section className="page">
      <div className="page-heading">
        <div><span className="eyebrow">PORTABLE APPS</span><h1>AppImages</h1><p>Import binaries, scan folders, and create desktop launchers.</p></div>
        <button className="button secondary" onClick={onScan}><ScanLine size={16} /> Scan folders</button>
      </div>
      <div className="action-panels">
        <form className="action-panel" onSubmit={(event) => { event.preventDefault(); if (path.trim()) onImport(path.trim()); }}>
          <div className="panel-icon mint"><Plus size={19} /></div><h3>Import a local file</h3><p>Copy a trusted `.AppImage` into your managed applications folder.</p>
          <label className="field-label">FILE PATH</label><div className="path-input"><input value={path} onChange={(event) => setPath(event.target.value)} placeholder="/home/you/Downloads/editor.AppImage" /><button className="button quiet" type="button" onClick={onBrowse}>Browse</button></div>
          <button className="button secondary" disabled={!path.trim()}><FileDown size={15} /> Import AppImage</button>
        </form>
        <form className="action-panel" onSubmit={(event) => { event.preventDefault(); if (url.trim()) onDownload(url.trim()); }}>
          <div className="panel-icon blue"><Link2 size={19} /></div><h3>Pull from URL</h3><p>Download a direct AppImage asset URL from a trusted source.</p>
          <label className="field-label">DIRECT URL</label><input value={url} onChange={(event) => setUrl(event.target.value)} placeholder="https://example.org/app.AppImage" />
          <button className="button secondary" disabled={!url.trim()}><FileDown size={15} /> Download</button>
        </form>
      </div>
      <div className="section-title"><div><h2>Managed & scanned</h2><p>{entries.length} AppImage{entries.length === 1 ? "" : "s"} found</p></div><FolderSearch size={18} /></div>
      <div className="installed-list">
        {entries.map((entry) => <article className="installed-row" key={entry.path}>
          <div className="app-avatar small app_image">{entry.name.slice(0, 1).toUpperCase()}</div>
          <div className="installed-info"><strong>{entry.name}</strong><span className="path-text">{entry.path}</span></div>
          <div className="row-actions">
            <button title="Launch" onClick={() => onLaunch(entry)}><span className="play-glyph">▶</span></button>
            <button title={entry.desktop_entry ? "Remove desktop entry" : "Create desktop entry"} onClick={() => onToggle(entry)}><FolderSearch size={15} /></button>
          </div>
        </article>)}
        {!entries.length && <div className="empty-inline">No AppImages discovered yet. Scan your configured folders or import one above.</div>}
      </div>
    </section>
  );
}
