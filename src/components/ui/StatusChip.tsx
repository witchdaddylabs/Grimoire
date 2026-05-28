// src/components/ui/StatusChip.tsx
import { Check, AlertTriangle } from "lucide-react";
import type { SaveState } from "../types";

interface StatusChipProps {
  // New API: pass saveState directly
  saveState?: SaveState;
  // Legacy API: pass tone + label
  tone?: "success" | "neutral" | "warning";
  label?: string;
}

export function saveStateTone(saveState: SaveState): "success" | "neutral" | "warning" {
  if (saveState === "saved" || saveState === "editing") return "success";
  if (saveState === "failed") return "warning";
  return "neutral";
}

export function saveStateLabel(saveState: SaveState): string {
  switch (saveState) {
    case "editing": return "Editing…";
    case "saving": return "Saving…";
    case "saved": return "Saved";
    case "failed": return "Failed to save";
    case "preview": return "Preview";
    default: return "";
  }
}

export function StatusChip({ saveState, tone, label }: StatusChipProps) {
  const resolvedTone = tone ?? (saveState ? saveStateTone(saveState) : "neutral");
  const resolvedLabel = label ?? (saveState ? saveStateLabel(saveState) : "");

  return (
    <span className={`status-chip ${resolvedTone}`}>
      {resolvedLabel}
    </span>
  );
}
