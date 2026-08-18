// src/features/storyplan/CandidateReview.tsx
// Candidate review UI — Fabula-style convergent iteration (Sprint 4).
import { useCallback, useEffect, useState } from "react";
import {
  Check, Copy, History, ShieldAlert, ShieldCheck, X,
} from "lucide-react";
import { describeError } from "../../app/project";
import {
  listStoryCandidates, resolveStoryCandidate,
  type StoryCandidate,
} from "../../app/storyplan";
import type { WardScanHit } from "../../app/vault";

interface CandidateReviewProps {
  projectPath: string;
  targetKind: "plan" | "scene" | "beat" | "script";
  targetId: string;
  linkedItemId?: string | null;
  showToast: (msg: string) => void;
  onSelectItem: (itemId: string) => void;
  refreshKey: number;
}

type WardTone = "clean" | "warn" | "block";

function wardLabel(hits: WardScanHit[]): { tone: WardTone; label: string } {
  if (hits.length === 0) return { tone: "clean", label: "No slop detected" };
  const blocking = hits.filter((h) => h.severity === "block");
  if (blocking.length > 0) {
    return { tone: "block", label: `${blocking.length} blocking` };
  }
  return { tone: "warn", label: `${hits.length} warning${hits.length === 1 ? "" : "s"}` };
}

function wardClassName(tone: WardTone): string {
  if (tone === "block") return "ward-block";
  if (tone === "warn") return "ward-warn";
  return "ward-clean";
}

function wardIcon(tone: WardTone) {
  if (tone === "clean") return <ShieldCheck size={11} aria-hidden="true" />;
  return <ShieldAlert size={11} aria-hidden="true" />;
}

function severityClass(severity: string): string {
  return severity === "block" ? "ward-block" : "ward-warn";
}

export function CandidateReview({
  projectPath,
  targetKind,
  targetId,
  linkedItemId,
  showToast,
  onSelectItem,
  refreshKey,
}: CandidateReviewProps) {
  const [candidates, setCandidates] = useState<StoryCandidate[]>([]);
  const [open, setOpen] = useState(false);
  const [loading, setLoading] = useState(false);
  const [busyId, setBusyId] = useState("");

  const reload = useCallback(async () => {
    if (!targetId) return;
    setLoading(true);
    try {
      const list = await listStoryCandidates(projectPath, targetKind, targetId);
      setCandidates(list);
    } catch (err) {
      showToast(describeError(err));
    } finally {
      setLoading(false);
    }
  }, [projectPath, targetKind, targetId, showToast]);

  useEffect(() => {
    if (open) void reload();
  }, [open, refreshKey, reload]);

  const handleAccept = useCallback(async (candidate: StoryCandidate) => {
    setBusyId(candidate.id);
    try {
      await resolveStoryCandidate(projectPath, candidate.id, "accepted");
      showToast("Candidate accepted — plan updated.");
      if (linkedItemId) {
        const send = window.confirm(
          "Send the accepted text to the Canvas (the linked Vault item)?",
        );
        if (send) onSelectItem(linkedItemId);
      }
      await reload();
    } catch (err) {
      showToast(describeError(err));
    } finally {
      setBusyId("");
    }
  }, [projectPath, linkedItemId, onSelectItem, reload, showToast]);

  const handleReject = useCallback(async (candidate: StoryCandidate) => {
    setBusyId(candidate.id);
    try {
      await resolveStoryCandidate(projectPath, candidate.id, "rejected");
      showToast("Candidate rejected.");
      await reload();
    } catch (err) {
      showToast(describeError(err));
    } finally {
      setBusyId("");
    }
  }, [projectPath, reload, showToast]);

  const handleCopy = useCallback(async (text: string) => {
    try {
      await navigator.clipboard.writeText(text);
      showToast("Copied to clipboard.");
    } catch {
      showToast("Could not copy.");
    }
  }, [showToast]);

  const pending = candidates.filter((c) => c.status === "pending");
  const resolved = candidates.filter((c) => c.status !== "pending");

  return (
    <div className="sp-candidate-review">
      <button
        className="sp-section-toggle"
        type="button"
        aria-expanded={open}
        onClick={() => setOpen((v) => !v)}
      >
        {open ? <ShieldCheck size={14} aria-hidden="true" /> : <ShieldAlert size={14} aria-hidden="true" />}
        <span className="sp-section-label">
          <History size={13} aria-hidden="true" />
          Review Candidates
        </span>
        <small>{candidates.length} stored</small>
      </button>

      {open && (
        <div className="sp-candidate-body">
          {loading ? (
            <p className="sp-candidate-empty">Loading…</p>
          ) : candidates.length === 0 ? (
            <div className="sp-candidate-empty">
              <ShieldCheck size={18} aria-hidden="true" />
              <span>No candidates yet. Regenerate this layer to compare variants.</span>
            </div>
          ) : (
            <>
              {pending.length > 0 && (
                <div className="sp-candidate-group">
                  <strong className="sp-candidate-group-title">Pending</strong>
                  {pending.map((candidate) => {
                    const wardHits = candidate.wardScan ?? [];
                    const ward = wardLabel(wardHits);
                    const acceptLabel = linkedItemId ? "Accept & Send to Canvas" : "Accept";
                    const acceptDisabled = busyId === candidate.id || ward.tone === "block";
                    return (
                      <div key={candidate.id} className="sp-candidate-card">
                        <div className="sp-candidate-head">
                          <span className="sp-badge">#{candidate.candidateIndex + 1}</span>
                          <span className={`sp-ward-pill ${wardClassName(ward.tone)}`}>
                            {wardIcon(ward.tone)}
                            {ward.label}
                          </span>
                          <div className="sp-row-actions">
                            <button type="button" title="Copy text" onClick={() => void handleCopy(candidate.content)}>
                              <Copy size={12} aria-hidden="true" />
                            </button>
                          </div>
                        </div>
                        <p className="sp-candidate-content">{candidate.content}</p>
                        {wardHits.length > 0 && (
                          <ul className="sp-ward-hits">
                            {wardHits.map((hit, i) => (
                              <li key={i} className={severityClass(hit.severity)}>
                                <strong>{hit.value}</strong>
                                <span className="sp-ward-count">×{hit.count}</span>
                              </li>
                            ))}
                          </ul>
                        )}
                        <div className="sp-candidate-actions">
                          <button
                            className="button button-primary"
                            type="button"
                            disabled={acceptDisabled}
                            onClick={() => void handleAccept(candidate)}
                          >
                            <Check size={14} aria-hidden="true" />
                            {acceptLabel}
                          </button>
                          <button
                            className="button button-secondary"
                            type="button"
                            disabled={busyId === candidate.id}
                            onClick={() => void handleReject(candidate)}
                          >
                            <X size={14} aria-hidden="true" /> Reject
                          </button>
                        </div>
                      </div>
                    );
                  })}
                </div>
              )}

              {resolved.length > 0 && (
                <details className="sp-candidate-group">
                  <summary className="sp-candidate-group-title">
                    History ({resolved.length})
                  </summary>
                  {resolved.map((candidate) => (
                    <div key={candidate.id} className="sp-candidate-card historical">
                      <div className="sp-candidate-head">
                        <span className="sp-badge">#{candidate.candidateIndex + 1}</span>
                        <span className={`sp-status-pill ${candidate.status}`}>{candidate.status}</span>
                      </div>
                      <p className="sp-candidate-content">{candidate.content}</p>
                    </div>
                  ))}
                </details>
              )}
            </>
          )}
        </div>
      )}
    </div>
  );
}
