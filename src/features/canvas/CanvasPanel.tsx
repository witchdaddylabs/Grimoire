// src/features/canvas/CanvasPanel.tsx
import { FileText, Download, Archive, Trash2 } from "lucide-react";
import type { VaultItemNode } from "../../app/vault";
import type { SaveState, AsyncState } from "../../components/types";

interface CanvasPanelProps {
  activeItem: VaultItemNode | null;
  editorTitle: string;
  editorContent: string;
  editorWordCount: number;
  saveState: SaveState;
  saveError: string | null;
  exportState: AsyncState;
  exportStatus: string;
  onTitleChange: (v: string) => void;
  onContentChange: (v: string) => void;
  onExportItem: () => void;
  onExportProject: () => void;
  onExportVaultItems: () => void;
  onArchiveItem: () => void;
  onDeleteItem: () => void;
}

function saveStateLabel(saveState: SaveState): string {
  switch (saveState) {
    case "editing": return "Editing…";
    case "saving": return "Saving…";
    case "saved": return "Saved";
    case "failed": return "Failed to save";
    case "preview": return "Preview";
    default: return "";
  }
}

export function CanvasPanel({
  activeItem, editorTitle, editorContent, editorWordCount,
  saveState, saveError, exportState, exportStatus,
  onTitleChange, onContentChange, onExportItem, onExportProject, onExportVaultItems,
  onArchiveItem, onDeleteItem,
}: CanvasPanelProps) {
  return (
    <article className="canvas-panel" aria-label="Canvas">
      <div className="canvas-toolbar">
        <div>
          <p className="eyebrow">The Canvas</p>
          <p className="path-label">{activeItem?.path ?? "The Vault"}</p>
        </div>
        <div className="canvas-stats" aria-live="polite">
          <span>{editorWordCount} words</span>
          <span className={`save-pill ${saveState}`}>{saveStateLabel(saveState)}</span>
        </div>
      </div>

      <label className="sr-only" htmlFor="canvas-title">Canvas title</label>
      <input
        id="canvas-title"
        className="title-input"
        value={editorTitle}
        onChange={(event) => onTitleChange(event.target.value)}
        spellCheck
      />

      <label className="sr-only" htmlFor="canvas-editor">Canvas editor</label>
      <textarea
        id="canvas-editor"
        className="editor-surface editor-textarea"
        value={editorContent}
        onChange={(event) => onContentChange(event.target.value)}
        placeholder="Create your first Wing or import writing to begin."
        spellCheck
      />

      {saveError ? <p className="inline-error">{saveError}</p> : null}

      <div className="canvas-actions">
        <button className="button button-secondary" type="button" onClick={onExportItem}>
          <FileText size={16} aria-hidden="true" />
          Export Markdown
        </button>
        <button className="button button-secondary" type="button" onClick={onExportProject}>
          <Download size={16} aria-hidden="true" />
          Export Project
        </button>
        <button className="button button-secondary" type="button" onClick={onExportVaultItems}>
          <Download size={16} aria-hidden="true" />
          Export Vault JSON
        </button>
        <button className="button button-secondary" type="button" onClick={onArchiveItem}>
          <Archive size={16} aria-hidden="true" />
          Safe Remove
        </button>
        <button className="button button-danger" type="button" onClick={onDeleteItem}>
          <Trash2 size={16} aria-hidden="true" />
          Delete Item
        </button>
        <span className={`operation-status ${exportState}`}>{exportStatus}</span>
      </div>
    </article>
  );
}
