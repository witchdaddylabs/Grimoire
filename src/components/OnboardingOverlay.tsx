// src/components/OnboardingOverlay.tsx
import { Check, Database, X, Clipboard, FileText, Loader2, BrainCircuit } from "lucide-react";
import type { AiProviderKind } from "../app/ai";
import { providerLabels } from "../app/ai";
import type { AsyncState, OnboardingStep } from "./types";

export type { AiProviderKind } from "../app/ai";
export type { AsyncState, OnboardingStep };

export interface OnboardingOverlayProps {
  step: OnboardingStep;
  projectReady: boolean;
  projectName: string;
  importTitle: string;
  importBody: string;
  importState: AsyncState;
  importStatus: string;
  activeProvider: AiProviderKind;
  engineStatus: string;
  onImportTitleChange: (v: string) => void;
  onImportBodyChange: (v: string) => void;
  onImportFiles: (files: FileList | null) => Promise<void>;
  onImportPaste: () => Promise<void>;
  onSelectProvider: (p: AiProviderKind) => void;
  onWardPresetSelect: (v: string) => void;
  onContinue: () => void;
  onSkip?: () => void;
  onDone: () => void;
}

const onboardingSteps: OnboardingStep[] = ["welcome", "vault", "feed", "engine", "wards", "canvas"];
const wardPresetOptions = ["very", "really", "suddenly", "somehow", "actually"];

const onboardingCopy: Record<OnboardingStep, { title: string; body: string }> = {
  welcome: { title: "Welcome to Grimoire", body: "A local-first writing desk for long work, canon memory, and grounded assistance." },
  vault: { title: "The Vault", body: "Your project is arranged as Wings, Halls, Rooms, Drawers, and editable writing items." },
  feed: { title: "Feed the Vault", body: "Paste text or import Markdown and plain text files when you are ready to stock local memory." },
  engine: { title: "Local Engine", body: "Ollama stays optional. Writing, search, import, wards, and export still work without it." },
  wards: { title: "Wards", body: "Wards are banned-word and banned-phrase checks for language you want Grimoire to warn about before AI text is inserted." },
  canvas: { title: "The Canvas", body: "Write in the calm graphite editor, switch to ivory manuscript mode when useful, and let autosave handle the rest." },
};

function OnboardingAction({
  step, activeProvider, engineStatus, importBody, importState, importStatus, importTitle, projectName, projectReady,
  onImportBodyChange, onImportFiles, onImportPaste, onImportTitleChange, onSelectProvider, onWardPresetSelect,
}: {
  activeProvider: AiProviderKind; engineStatus: string; importBody: string; importState: AsyncState;
  importStatus: string; importTitle: string; projectName: string; projectReady: boolean;
  step: OnboardingStep;
  onImportBodyChange: (v: string) => void; onImportFiles: (f: FileList | null) => Promise<void>;
  onImportPaste: () => Promise<void>; onImportTitleChange: (v: string) => void;
  onSelectProvider: (p: AiProviderKind) => void; onWardPresetSelect: (v: string) => void;
}) {
  if (step === "vault") {
    return (
      <div className="onboarding-action-card">
        <strong>Project: {projectName}</strong>
        <span>{projectReady ? "SQLite project ready for Wings, Halls, Rooms, Drawers, and Items." : "Preparing local SQLite project storage…"}</span>
      </div>
    );
  }

  if (step === "feed") {
    const IMPORT_WORD_LIMIT = 10_000;
    return (
      <div className="onboarding-action-card">
        <strong>Bring your existing writing into the Vault.</strong>
        <span>You can always do this later from the Co-Writer panel.</span>
        <form className="tool-form" onSubmit={(e) => { e.preventDefault(); onImportPaste(); }}>
          <input className="compact-input" value={importTitle} onChange={(e) => onImportTitleChange(e.target.value)} placeholder="Import title" />
          <textarea className="compact-textarea" value={importBody} onChange={(e) => onImportBodyChange(e.target.value)} placeholder={`Paste text or Markdown, up to ${IMPORT_WORD_LIMIT.toLocaleString()} words`} />
          <div className="inline-actions">
            <button className="button button-primary" type="button" onClick={onImportPaste} disabled={importState === "working"}>
              {importState === "working" ? <Loader2 size={16} /> : <Clipboard size={16} />}
              Import Paste
            </button>
            <label className="file-button">
              <FileText size={16} aria-hidden="true" />
              Files
              <input type="file" accept=".txt,.md,.markdown,text/plain,text/markdown" multiple onChange={(e) => onImportFiles(e.currentTarget.files)} />
            </label>
          </div>
        </form>
        <p className={`operation-status ${importState}`}>{importStatus}</p>
      </div>
    );
  }

  if (step === "engine") {
    return (
      <div className="onboarding-action-card">
        <strong>Ollama runs locally. Cloud providers are optional.</strong>
        <span>{engineStatus}</span>
        <div className="provider-grid" role="radiogroup" aria-label="AI provider">
          {(["ollama", "openAi", "openAiCompatible", "googleAiStudio"] as AiProviderKind[]).map((provider) => (
            <button key={provider} className={provider === activeProvider ? "provider-button active" : "provider-button"} type="button" role="radio"
              aria-checked={provider === activeProvider} onClick={() => onSelectProvider(provider)}>
              <span>{providerLabels[provider]}</span>
              <small>{provider === "ollama" ? "Local" : "BYOK cloud"}</small>
            </button>
          ))}
        </div>
      </div>
    );
  }

  if (step === "wards") {
    return (
      <div className="onboarding-action-card">
        <strong>Wards are banned words and banned phrases.</strong>
        <span>Grimoire scans Co-Writer output before insertion and warns when these words appear. Choose a starter below or add your own later.</span>
        <div className="ward-preset-grid" aria-label="Banned-word starter options">
          {wardPresetOptions.map((preset) => (
            <button className="provider-button" key={preset} type="button" onClick={() => onWardPresetSelect(preset)}>
              <span>{preset}</span>
              <small>Warn</small>
            </button>
          ))}
        </div>
      </div>
    );
  }

  return null;
}

export function OnboardingOverlay(props: OnboardingOverlayProps) {
  const { step, onContinue, onSkip, onDone } = props;
  const copy = onboardingCopy[step];
  const finalStep = step === "canvas";
  const optionalOnboardingSteps = new Set<OnboardingStep>(["feed", "engine", "wards"]);

  return (
    <div className="onboarding-backdrop" role="dialog" aria-modal="true" aria-labelledby="onboarding-title">
      <section className="onboarding-panel">
        <button className="icon-button dismiss" type="button" aria-label="Close onboarding" onClick={onDone}>
          <X size={16} />
        </button>
        <p className="eyebrow">First run</p>
        <h2 id="onboarding-title">{copy.title}</h2>
        <p>{copy.body}</p>
        <div className="onboarding-progress" aria-label="Onboarding progress">
          {onboardingSteps.map((candidate) => (
            <span key={candidate} className={candidate === step ? "active" : undefined} />
          ))}
        </div>
        <div className="onboarding-status">
          <Database size={15} aria-hidden="true" />
          {props.projectReady ? "Vault project ready" : "Preparing local Vault"}
        </div>
        <OnboardingAction {...props} />
        <div className="inline-actions">
          {onSkip ? <button className="button button-secondary" type="button" onClick={onSkip}>Skip</button> : null}
          <button className="button button-primary" type="button" onClick={finalStep ? onDone : onContinue}>
            {finalStep ? "Enter Grimoire" : "Continue"}
          </button>
        </div>
      </section>
    </div>
  );
}
