// src/features/storyplan/RegeneratePanel.tsx
// Trigger panel for Fabula-style convergent iteration (Sprint 4).
// Writer picks a target, enters an edit instruction, and fires regeneration.
import { useCallback, useState } from "react";
import {
  Loader2, RefreshCw, Send,
} from "lucide-react";
import type {
  AiProviderKind, AiProviderSettings, AiProviderModelsResponse,
} from "../../app/ai";
import { cloudProvider } from "../../app/ai";
import type { VaultItemNode } from "../../app/vault";
import {
  regenerateStoryLayer, type CandidateTargetKind,
} from "../../app/storyplan";

interface RegeneratePanelProps {
  projectPath: string;
  targetKind: CandidateTargetKind;
  targetId: string;
  linkedItemId?: string | null;
  providers: AiProviderKind[];
  activeProvider: AiProviderKind;
  providerSettings: AiProviderSettings | null;
  providerModels: AiProviderModelsResponse | null;
  onProviderChange: (provider: AiProviderKind) => void;
  onRefreshModels: () => void;
  showToast: (msg: string) => void;
  /** Bump to clear the instruction after a successful run. */
  onGenerationDone: () => void;
}

function providerReady(
  provider: AiProviderKind,
  settings: AiProviderSettings | null,
  models: AiProviderModelsResponse | null,
): boolean {
  if (provider === "ollama") return Boolean(models?.reachable && (models.selectedModel || models.models.length > 0));
  return Boolean(settings?.apiKeyPresent);
}

function selectedModel(
  provider: AiProviderKind,
  settings: AiProviderSettings | null,
  models: AiProviderModelsResponse | null,
): string {
  if (provider === "ollama") return models?.selectedModel ?? models?.models[0]?.name ?? "";
  return settings?.selectedModel ?? "";
}

export function RegeneratePanel({
  projectPath,
  targetKind,
  targetId,
  providers,
  activeProvider,
  providerSettings,
  providerModels,
  onProviderChange,
  onRefreshModels,
  showToast,
  onGenerationDone,
}: RegeneratePanelProps) {
  const [instruction, setInstruction] = useState("");
  const [candidateCount, setCandidateCount] = useState(3);
  const [scanWards, setScanWards] = useState(true);
  const [busy, setBusy] = useState(false);

  const ready = providerReady(activeProvider, providerSettings, providerModels);
  const model = selectedModel(activeProvider, providerSettings, providerModels);
  const isCloud = cloudProvider(activeProvider);

  const handleRegenerate = useCallback(async () => {
    if (!instruction.trim()) {
      showToast("Give the regeneration an edit instruction first.");
      return;
    }
    if (!ready) {
      showToast("Add an API key or start a local model before regenerating.");
      return;
    }
    setBusy(true);
    try {
      await regenerateStoryLayer(projectPath, {
        targetKind,
        targetId,
        instruction: instruction.trim(),
        provider: activeProvider,
        model,
        candidateCount,
        scanWards,
      });
      showToast(`Regenerating ${targetKind}…`);
      setInstruction("");
      onGenerationDone();
    } catch (err) {
      showToast(`${err ?? "Regeneration failed."}`);
    } finally {
      setBusy(false);
    }
  }, [instruction, ready, projectPath, targetKind, targetId, activeProvider, model, candidateCount, scanWards, showToast, onGenerationDone]);

  return (
    <div className="sp-regenerate">
      <div className="sp-regenerate-head">
        <RefreshCw size={13} aria-hidden="true" />
        <strong>Regenerate this {targetKind}</strong>
      </div>

      <label className="sp-field-label" htmlFor={`regen-provider-${targetKind}-${targetId}`}>Provider</label>
      <div className="sp-provider-row">
        <select
          id={`regen-provider-${targetKind}-${targetId}`}
          className="compact-input"
          value={activeProvider}
          onChange={(e) => onProviderChange(e.target.value as AiProviderKind)}
        >
          {providers.map((p) => (
            <option key={p} value={p}>{p}</option>
          ))}
        </select>
        <button
          className="button button-secondary"
          type="button"
          onClick={() => onRefreshModels()}
          title="Refresh available models"
        >
          ↻
        </button>
      </div>

      <label className="sp-field-label" htmlFor={`regen-instruction-${targetKind}-${targetId}`}>Edit instruction</label>
      <textarea
        id={`regen-instruction-${targetKind}-${targetId}`}
        className="compact-input sp-textarea"
        rows={3}
        placeholder="e.g. tighten the dialogue, raise the tension, cut the fat"
        value={instruction}
        onChange={(e) => setInstruction(e.target.value)}
      />

      <div className="sp-regenerate-row">
        <label className="sp-field-label" htmlFor={`regen-count-${targetKind}-${targetId}`}>Variants</label>
        <input
          id={`regen-count-${targetKind}-${targetId}`}
          className="compact-input sp-count-input"
          type="number"
          min={1}
          max={5}
          value={candidateCount}
          onChange={(e) => setCandidateCount(Math.max(1, Math.min(5, Number(e.target.value) || 1)))}
        />
        <label className="sp-checkbox-label">
          <input
            type="checkbox"
            checked={scanWards}
            onChange={(e) => setScanWards(e.target.checked)}
          />
          Scan wards
        </label>
      </div>

      <div className="sp-regenerate-actions">
        <button
          className="button button-primary"
          type="button"
          disabled={busy || !ready}
          onClick={() => void handleRegenerate()}
        >
          {busy ? <Loader2 size={14} className="animate-spin" aria-hidden="true" /> : <Send size={14} aria-hidden="true" />}
          {busy ? "Generating…" : "Generate variants"}
        </button>
        {!ready && (
          <small className="sp-regenerate-hint">
            {isCloud ? "Add an API key in Co-Writer settings." : "Start Ollama to regenerate locally."}
          </small>
        )}
      </div>
    </div>
  );
}
