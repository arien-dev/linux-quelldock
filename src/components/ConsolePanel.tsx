import { Check, Copy, Square, Trash2 } from "lucide-react";
import { useMemo, useState } from "react";
import type { JobEvent } from "../types";

export default function ConsolePanel({
  events, activeJob, onCancel, onClear, onSendInput, expanded = false,
}: {
  events: JobEvent[];
  activeJob?: string;
  onCancel: () => void;
  onClear: () => void;
  onSendInput: (input: string) => void;
  expanded?: boolean;
}) {
  const [filter, setFilter] = useState("");
  const [copied, setCopied] = useState(false);
  const [input, setInput] = useState("");
  const visible = useMemo(() => events.filter((event) => !filter || event.job_id.includes(filter)), [events, filter]);
  const copy = async () => {
    await navigator.clipboard.writeText(visible.map((event) => `[${event.job_id}] ${event.stream}: ${event.line}`).join("\n"));
    setCopied(true); window.setTimeout(() => setCopied(false), 1200);
  };
  return (
    <section className={`console-panel ${expanded ? "expanded" : ""}`}>
      <header className="console-header">
        <div className="console-title"><span className={`status-dot ${activeJob ? "running" : ""}`} /><strong>{activeJob ? "Job running" : "Console"}</strong>{activeJob && <code>{activeJob.slice(0, 8)}</code>}</div>
        <div className="console-actions">
          {expanded && <input className="job-filter" placeholder="Filter by job ID" value={filter} onChange={(event) => setFilter(event.target.value)} />}
          {activeJob && <button title="Cancel job" className="stop-button" onClick={onCancel}><Square size={13} fill="currentColor" /> Stop</button>}
          <button title="Copy log" onClick={() => void copy()}>{copied ? <Check size={14} /> : <Copy size={14} />}</button>
          <button title="Clear log" onClick={onClear}><Trash2 size={14} /></button>
        </div>
      </header>
      <div className="console-output" role="log" aria-live="polite">
        {visible.length ? visible.map((event, index) => <div className={`log-line ${event.stream}`} key={`${event.job_id}-${event.timestamp_ms}-${index}`}>
          <time>{new Date(event.timestamp_ms).toLocaleTimeString()}</time><span className="log-stream">{event.stream}</span><code>{event.line}</code>
        </div>) : <div className="console-placeholder">Command output will appear here in real time.</div>}
      </div>
      {activeJob && <form className="console-input" onSubmit={(event) => { event.preventDefault(); if (!input.trim()) return; onSendInput(input); setInput(""); }}>
        <input aria-label="Send interactive command input" value={input} onChange={(event) => setInput(event.target.value)} placeholder="Respond to an interactive package prompt…" />
        <button className="button secondary" disabled={!input.trim()}>Send input</button>
      </form>}
    </section>
  );
}
