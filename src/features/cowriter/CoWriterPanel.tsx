// src/features/cowriter/CoWriterPanel.tsx
import {
  BrainCircuit, ChevronRight, Upload, Loader2, Clipboard,
  FileText, Search, Sparkles, WandSparkles, Copy, Check, X, ShieldCheck,
  Trash2, Info, Plus,
} from "lucide-react";
import type {
  SearchChunkResult, BannedWord, WardScanHit, WardSeverity,
} from "../../app/vault";
import type { AiProviderKind, AiProviderSettings, AiProviderModelsResponse } from "../../app/ai";
import { providerLabels, cloudProvider } from "../../app/ai";
import type { AsyncState, ToolSectionId } from "../../components/types";
import { PanelHeader } from "../../components/PanelHeader";
import { CollapsedRail } from "../../components/CollapsedRail";
import { ToolAccordion } from "../../components/ToolAccordion";

const AI_PROVIDERS: AiProviderKind[] = ["ollama", "openAi", "openAiCompatible", "googleAiStudio"];

interface CoWriterPanelProps {
  rightCollapsed: boolean;
  activeProvider: AiProviderKind;
  selectedModel: string;
  providerLabels: Record<AiProviderKind, string>;
  providerModels: AiProviderModelsResponse | null;
  activeProviderSettings: AiProviderSettings | null;
  activeProviderIsCloud: boolean;
  openToolSectionSet: Set<ToolSectionId>;
  // Feed state
  importTitle: string; importBody: string; importState: AsyncState; importStatus: string; importProgress: string[];
  // Engine state
  engineState: AsyncState; engineStatus: string; engineError: string | null;
  modelDraft: string; modelOptions: string[];
  apiKeyDraft: string; baseUrlDraft: string;
  // Cowriter state
  cowriterPrompt: string; cowriterState: AsyncState; cowriterStatus: string;
  cowriterAnswer: string; cowriterError: string | null;
  retrievalResults: SearchChunkResult[]; answerWardHits: WardScanHit[];
  // Wards state
  wards: BannedWord[]; wardInput: string; wardSeverity: WardSeverity; wardState: AsyncState; wardStatus: string;
  // Search pass-through
  searchResults: SearchChunkResult[];
  // Handlers
  onToggleToolSection: (id: ToolSectionId) => void;
  onImportTitleChange: (v: string) => void; onImportBodyChange: (v: string) => void;
  onPasteImport: (e: React.FormEvent) => void; onFileImport: (files: FileList | null) => void;
  onRefreshEngine: () => void; onProviderTest: () => void;
  onProviderSelection: (p: AiProviderKind) => void;
  onModelDraftChange: (v: string) => void;
  onApiKeyDraftChange: (v: string) => void; onBaseUrlDraftChange: (v: string) => void;
  onApiKeySave: (e: React.FormEvent) => void; onApiKeyDelete: () => void;
  onEngineSettingsSave: (e: React.FormEvent) => void;
  onCowriterPromptChange: (v: string) => void; onRunCowriter: () => void;
  onInsertAnswer: () => void; onCopyAnswer: () => void;
  onDiscardAnswer: () => void; onRewriteClean: () => void;
  onWardInputChange: (v: string) => void; onWardSeverityChange: (v: WardSeverity) => void;
  onWardAdd: (e: React.FormEvent) => void; onWardRemove: (id: string) => void;
  onSelectItem: (id: string) => void;
  onExpandRight: () => void; onCollapseRight: () => void;
}

function providerReady(
  provider: AiProviderKind, settings: AiProviderSettings | null, models: AiProviderModelsResponse | null,
): boolean {
  if (provider === "ollama") return Boolean(models?.reachable && (models.selectedModel || models.models.length > 0));
  return Boolean(settings?.apiKeyPresent);
}

function retrievalLabels(state: AsyncState, status: string): string[] {
  if (state === "working") return [status, "Reading canon traces", "Checking slop wards", "Composing grounded answer"];
  if (state === "success") return [status, "Citations ready", "Insertion requires user action"];
  return ["Consulting the Vault", "Reading canon traces", "Checking slop wards", "Composing grounded answer"];
}

function CitationList({ results }: { results: SearchChunkResult[] }) {
  if (results.length === 0) return null;
  return (
    <div className="citation-list">
      <strong>Citations</strong>
      {results.slice(0, 3).map((result, i) => (
        <span key={result.chunkId}>[{i + 1}] {result.vaultPath}</span>
      ))}
    </div>
  );
}

function WardWarnings({ hits }: { hits: WardScanHit[] }) {
  if (hits.length === 0) return null;
  return (
    <div className="ward-warning">
      <Sparkles size={15} aria-hidden="true" />
      <span>Wards found {hits.map(h => `${h.value} (${h.count})`).join(", ")}. This is a warning, not a perfect style check.</span>
    </div>
  );
}

function ProgressList({ labels }: { labels: string[] }) {
  if (labels.length === 0) return null;
  return (
    <div className="progress-list">
      {labels.map((label) => (
        <span key={label}><Check size={12} />{label}</span>
      ))}
    </div>
  );
}

function ResultList({ results, onSelect }: { results: SearchChunkResult[]; onSelect: (id: string) => void }) {
  if (results.length === 0) return null;
  return (
    <div className="result-list">
      {results.slice(0, 8).map((r) => (
        <button key={r.chunkId} className="result-item" type="button" onClick={() => onSelect(r.itemId)}>
          <strong>{r.title}</strong>
          <span className="result-snippet">{r.snippet?.slice(0, 120)}</span>
        </button>
      ))}
    </div>
  );
}

export function CoWriterPanel(props: CoWriterPanelProps) {
  if (props.rightCollapsed) {
    return (
      <CollapsedRail icon={<BrainCircuit size={18} aria-hidden="true" />} label="Open Co-Writer" onExpand={props.onExpandRight} side="right" />
    );
  }

  return (
    <>
      <PanelHeader
        action={
          <button className="icon-button panel-collapse-button" type="button" aria-label="Collapse Co-Writer" onClick={props.onCollapseRight} title="Collapse Co-Writer">
            <ChevronRight size={16} aria-hidden="true" />
          </button>
        }
        icon={<BrainCircuit size={17} aria-hidden="true" />}
        title="The Co-Writer"
        subtitle={`${providerLabels[props.activeProvider]}${props.selectedModel ? ` / ${props.selectedModel}` : " / choose model"}`}
      />

      <div className="panel-scroll tools-scroll">

        {/* Feed */}
        <ToolAccordion id="feed" icon={<Upload size={15} />} open={props.openToolSectionSet.has("feed")} title="Feed" onToggle={props.onToggleToolSection}>
          <form className="tool-form" onSubmit={props.onPasteImport}>
            <input className="compact-input" value={props.importTitle} onChange={(e) => props.onImportTitleChange(e.target.value)} placeholder="Import title" />
            <textarea className="compact-textarea" value={props.importBody} onChange={(e) => props.onImportBodyChange(e.target.value)} placeholder="Paste text or Markdown, up to 10,000 words" />
            <p className="tool-hint">Import multiple .md, .markdown, or .txt files. Each file is capped at 10,000 words; add more chunks later if needed.</p>
            <div className="inline-actions">
              <button className="button button-primary" type="submit" disabled={props.importState === "working"}>
                {props.importState === "working" ? <Loader2 size={16} /> : <Clipboard size={16} />}Import Paste
              </button>
              <label className="file-button"><FileText size={16} aria-hidden="true" />Files<input type="file" accept=".txt,.md,.markdown,text/plain,text/markdown" multiple onChange={(e) => props.onFileImport(e.currentTarget.files)} /></label>
            </div>
          </form>
          <ProgressList labels={props.importProgress} />
          <p className={`operation-status ${props.importState}`}>{props.importStatus}</p>
        </ToolAccordion>

        {/* Retrieval */}
        <ToolAccordion id="retrieval" icon={<Search size={15} />} open={props.openToolSectionSet.has("retrieval")} title="Retrieval" onToggle={props.onToggleToolSection}>
          <div className="retrieval-card" role="status" aria-live="polite">
            {retrievalLabels(props.cowriterState, props.cowriterStatus).map((step, i) => (
              <p key={`${step}-${i}`} className={i === 0 ? "active-step" : undefined}><Sparkles size={14} aria-hidden="true" />{step}</p>
            ))}
          </div>
          <ResultList results={props.searchResults.length ? props.searchResults : props.retrievalResults} onSelect={props.onSelectItem} />
        </ToolAccordion>

        {/* Engine */}
        <ToolAccordion id="engine" icon={<WandSparkles size={15} />} open={props.openToolSectionSet.has("engine")} title="Engine" onToggle={props.onToggleToolSection}>
          <div className="engine-row">
            <button className="button button-secondary" type="button" onClick={props.onRefreshEngine}>
              {props.engineState === "working" ? <Loader2 size={16} /> : <BrainCircuit size={16} />}Refresh Models
            </button>
            <button className="button button-secondary" type="button" onClick={props.onProviderTest}><Sparkles size={16} />Test Provider</button>
            <span className={providerReady(props.activeProvider, props.activeProviderSettings, props.providerModels) ? "engine-dot online" : "engine-dot"} />
          </div>
          <div className="provider-grid" role="radiogroup" aria-label="AI provider">
            {AI_PROVIDERS.map((provider) => (
              <button key={provider} className={provider === props.activeProvider ? "provider-button active" : "provider-button"} type="button" role="radio" aria-checked={provider === props.activeProvider} onClick={() => props.onProviderSelection(provider)}>
                <span>{providerLabels[provider]}</span><small>{cloudProvider(provider) ? "BYOK cloud" : "Local"}</small>
              </button>
            ))}
          </div>
          <p className={`operation-status ${props.engineState}`}>{props.engineStatus}</p>
          {props.engineError ? <p className="inline-error compact-error">{props.engineError}</p> : null}
          {props.modelOptions.length ? (
            <select className="compact-input" value={props.modelDraft} onChange={(e) => props.onModelDraftChange(e.target.value)}>
              <option value="" disabled>Choose model</option>
              {props.modelOptions.map((m) => <option key={m} value={m}>{m}</option>)}
            </select>
          ) : null}
          {props.activeProviderIsCloud ? (
            <div className="cloud-settings">
              <div className="key-status">
                <span className={props.activeProviderSettings ? "engine-dot online" : "engine-dot"} />
                {props.activeProviderSettings?.apiKeyPresent ? "API key saved" : "No API key saved"}
              </div>
              <p className="tool-hint">macOS may ask for Keychain permission because Grimoire stores API keys there instead of inside your project files.</p>
              <form className="tool-form" onSubmit={props.onApiKeySave}>
                <input className="compact-input" type="password" autoComplete="off" value={props.apiKeyDraft} onChange={(e) => props.onApiKeyDraftChange(e.target.value)} placeholder={`Paste ${providerLabels[props.activeProvider]} API key`} />
                <div className="inline-actions">
                  <button className="button button-primary" type="submit" disabled={!props.apiKeyDraft.trim()}><ShieldCheck size={16} />Save Key</button>
                  <button className="button button-secondary" type="button" disabled={!props.activeProviderSettings?.apiKeyPresent} onClick={props.onApiKeyDelete}><Trash2 size={16} />Delete Key</button>
                </div>
              </form>
              {props.activeProvider === "openAiCompatible" ? <input className="compact-input" value={props.baseUrlDraft} onChange={(e) => props.onBaseUrlDraftChange(e.target.value)} placeholder="Base URL, e.g. https://api.example.com" /> : null}
            </div>
          ) : null}
          <form className="tool-form" onSubmit={props.onEngineSettingsSave}>
            <input className="compact-input" value={props.modelDraft} onChange={(e) => props.onModelDraftChange(e.target.value)} placeholder={props.activeProvider === "ollama" ? "Choose a detected local model" : "Model ID"} />
            <button className="button button-secondary full-width" type="submit"><Check size={16} />Save Engine Settings</button>
          </form>
        </ToolAccordion>

        {/* Co-Writer */}
        <ToolAccordion id="cowriter" icon={<BrainCircuit size={15} />} open={props.openToolSectionSet.has("cowriter")} title="Co-Writer" onToggle={props.onToggleToolSection}>
          <textarea className="compact-textarea" value={props.cowriterPrompt} onChange={(e) => props.onCowriterPromptChange(e.target.value)} placeholder="Ask a grounded question across the Vault" />
          <p className="tool-hint">Searches the whole Vault first, then uses the active Canvas as extra context when it helps.</p>
          <button className="button button-primary full-width" type="button" onClick={props.onRunCowriter}>
            {props.cowriterState === "working" ? <Loader2 size={16} /> : <Sparkles size={16} />}Ask Co-Writer
          </button>
          {props.cowriterError ? <p className="inline-error">{props.cowriterError}</p> : null}
          {props.cowriterAnswer ? (
            <div className="answer-card">
              <p>{props.cowriterAnswer}</p>
              <CitationList results={props.retrievalResults} />
              <WardWarnings hits={props.answerWardHits} />
              <div className="inline-actions">
                <button className="button button-secondary" type="button" onClick={props.onInsertAnswer}><Check size={16} />{props.answerWardHits.length ? "Insert Anyway" : "Insert"}</button>
                <button className="icon-button" type="button" aria-label="Copy answer" onClick={props.onCopyAnswer}><Copy size={16} /></button>
                <button className="icon-button" type="button" aria-label="Rewrite clean" onClick={props.onRewriteClean}><WandSparkles size={16} /></button>
                <button className="icon-button" type="button" aria-label="Discard answer" onClick={props.onDiscardAnswer}><X size={16} /></button>
              </div>
            </div>
          ) : null}
        </ToolAccordion>

        {/* Wards */}
        <ToolAccordion id="wards" icon={<ShieldCheck size={15} />} open={props.openToolSectionSet.has("wards")} title="Wards" onToggle={props.onToggleToolSection}>
          <form className="ward-form" onSubmit={props.onWardAdd}>
            <input className="compact-input" value={props.wardInput} onChange={(e) => props.onWardInputChange(e.target.value)} placeholder="Phrase to warn on" />
            <select className="compact-input severity-select" value={props.wardSeverity} onChange={(e) => props.onWardSeverityChange(e.target.value as WardSeverity)}>
              <option value="warn">Warn</option><option value="block">Block</option>
            </select>
            <button className="icon-button" type="submit" aria-label="Add ward phrase"><Plus size={16} /></button>
          </form>
          <div className="ward-list">
            {props.wards.slice(0, 10).map((ward) => (
              <span key={ward.id} className="ward-token">
                {ward.value}<small>{ward.severity}</small>
                {!ward.isDefault ? <button type="button" aria-label={`Remove ${ward.value}`} onClick={() => props.onWardRemove(ward.id)}><Trash2 size={12} /></button> : null}
              </span>
            ))}
          </div>
          <p className={`operation-status ${props.wardState}`}>{props.wardStatus}</p>
        </ToolAccordion>

        {/* About */}
        <ToolAccordion id="about" icon={<Info size={15} />} open={props.openToolSectionSet.has("about")} title="About" onToggle={props.onToggleToolSection}>
          <p>Grimoire is an independent Witch Daddy Labs project. The Vault memory model is inspired by the MIT-licensed MemPalace project; Grimoire is not affiliated with MemPalace.</p>
        </ToolAccordion>
      </div>
    </>
  );
}
