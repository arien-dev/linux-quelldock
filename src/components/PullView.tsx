import { ArrowDownToLine, ExternalLink, LoaderCircle, Search, ShieldCheck } from "lucide-react";
import { useState } from "react";
import type { PackageCandidate, ProviderId } from "../types";
import { providerLabel } from "../types";

export default function PullView({
  candidates, busy, onSearch, onInstall, onDownload, error,
}: {
  candidates: PackageCandidate[];
  busy: boolean;
  onSearch: (query: string) => void;
  onInstall: (candidate: PackageCandidate) => void;
  onDownload: (url: string) => void;
  error?: string;
}) {
  const [query, setQuery] = useState("");
  const [url, setUrl] = useState("");
  const submit = (event: React.FormEvent) => { event.preventDefault(); onSearch(query.trim()); };
  return (
    <section className="page">
      <div className="page-heading">
        <div><span className="eyebrow">DISCOVER</span><h1>Pull an app</h1><p>Search package sources side by side, then review the command before installing.</p></div>
        <div className="heading-icon"><ArrowDownToLine size={21} /></div>
      </div>
      <form className="search-box pull-search" onSubmit={submit}>
        <Search size={19} />
        <input autoFocus value={query} onChange={(event) => setQuery(event.target.value)} placeholder="Search neovim, obs-studio, ghostty…" aria-label="Search packages" />
        <button className="button primary" disabled={!query.trim() || busy}>{busy ? <LoaderCircle className="spin" size={16} /> : "Search"}</button>
      </form>
      <div className="source-chips"><span>SEARCHING</span>{(["app_image", "flatpak", "aur", "apt", "dnf", "zypper"] as ProviderId[]).map((provider) => <span className="chip" key={provider}>{providerLabel[provider]}</span>)}</div>
      <form className="direct-pull" onSubmit={(event) => { event.preventDefault(); if (url.trim()) onDownload(url.trim()); }}>
        <div><strong>Have an AppImage URL?</strong><span>Pull a direct release asset without leaving Discover.</span></div>
        <input aria-label="Direct AppImage URL" value={url} onChange={(event) => setUrl(event.target.value)} placeholder="https://releases.example/app.AppImage" />
        <button className="button secondary" disabled={!url.trim()}><ArrowDownToLine size={14} /> Pull AppImage</button>
      </form>
      {error && <div className="notice error">{error}</div>}
      {!candidates.length ? (
        <div className="empty-state"><div className="empty-icon"><Search size={23} /></div><h3>Find your next app</h3><p>Results from Flatpak, AUR/Arch, and your detected native package manager appear here.</p><div className="safe-caption"><ShieldCheck size={15} /> Nothing is installed until you confirm its command.</div></div>
      ) : (
        <div className="candidate-list">
          <div className="list-heading"><span>{candidates.length} results</span><span>Choose a source and inspect its install command</span></div>
          {candidates.map((candidate) => (
            <article className="candidate-card" key={`${candidate.provider}:${candidate.package_id}`}>
              <div className="app-avatar">{candidate.name.slice(0, 1).toUpperCase()}</div>
              <div className="candidate-info">
                <div className="candidate-title"><h3>{candidate.name}</h3><span className={`badge ${candidate.provider}`}>{providerLabel[candidate.provider]}</span></div>
                <div className="candidate-id">{candidate.package_id}{candidate.version ? ` · ${candidate.version}` : ""}</div>
                {candidate.description && <p>{candidate.description}</p>}
              </div>
              <button className="button secondary" onClick={() => onInstall(candidate)}><ArrowDownToLine size={15} /> Install</button>
            </article>
          ))}
          <p className="source-footnote"><ExternalLink size={13} /> Availability and package details are provided by the configured system package tools.</p>
        </div>
      )}
    </section>
  );
}
