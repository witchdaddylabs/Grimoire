// src/features/settings/SettingsPanel.tsx
import { useState } from "react";
import { Settings, X, Moon, SunMedium, Globe, Key, Trash2 } from "lucide-react";
import type { AiProviderKind } from "../../app/ai";
import { providerLabels } from "../../app/ai";

interface SettingsPanelProps {
  isOpen: boolean;
  onClose: () => void;
  theme: "dark" | "ivory";
  onThemeChange: (theme: "dark" | "ivory") => void;
  projectName: string;
  projectPath: string;
  onProjectNameChange: (name: string) => void;
  ollamaUrl: string;
  onOllamaUrlChange: (url: string) => void;
  activeProvider: AiProviderKind;
  onProviderChange: (provider: AiProviderKind) => void;
  apiKey: string;
  onApiKeyChange: (key: string) => void;
  onApiKeySave: () => void;
  onApiKeyDelete: () => void;
  hasApiKey: boolean;
}

export function SettingsPanel({
  isOpen,
  onClose,
  theme,
  onThemeChange,
  projectName,
  projectPath,
  onProjectNameChange,
  ollamaUrl,
  onOllamaUrlChange,
  activeProvider,
  onProviderChange,
  apiKey,
  onApiKeyChange,
  onApiKeySave,
  onApiKeyDelete,
  hasApiKey,
}: SettingsPanelProps) {
  if (!isOpen) return null;

  return (
    <div className="onboarding-backdrop" role="dialog" aria-modal="true" aria-labelledby="settings-title">
      <section className="onboarding-panel settings-panel">
        <button className="icon-button dismiss" type="button" aria-label="Close settings" onClick={onClose}>
          <X size={16} />
        </button>
        <p className="eyebrow">Settings</p>
        <h2 id="settings-title">Project Settings</h2>

        <div className="settings-section">
          <h3><Settings size={14} /> Project</h3>
          <label className="settings-label">Project Name</label>
          <input
            className="compact-input"
            value={projectName}
            onChange={(e) => onProjectNameChange(e.target.value)}
          />
          <p className="tool-hint">{projectPath}</p>
        </div>

        <div className="settings-section">
          <h3><Moon size={14} /> Appearance</h3>
          <div className="provider-grid">
            <button
              className={theme === "dark" ? "provider-button active" : "provider-button"}
              type="button"
              onClick={() => onThemeChange("dark")}
            >
              <span><Moon size={14} /> Dark</span>
            </button>
            <button
              className={theme === "ivory" ? "provider-button active" : "provider-button"}
              type="button"
              onClick={() => onThemeChange("ivory")}
            >
              <span><SunMedium size={14} /> Ivory</span>
            </button>
          </div>
        </div>

        <div className="settings-section">
          <h3><Globe size={14} /> AI Provider</h3>
          <div className="provider-grid">
            {(["ollama", "openAi", "openAiCompatible", "googleAiStudio"] as AiProviderKind[]).map((p) => (
              <button
                key={p}
                className={activeProvider === p ? "provider-button active" : "provider-button"}
                type="button"
                onClick={() => onProviderChange(p)}
              >
                <span>{providerLabels[p]}</span>
                <small>{p === "ollama" ? "Local" : "Cloud"}</small>
              </button>
            ))}
          </div>
        </div>

        {activeProvider === "ollama" && (
          <div className="settings-section">
            <h3><Globe size={14} /> Ollama</h3>
            <label className="settings-label">Ollama URL</label>
            <input
              className="compact-input"
              value={ollamaUrl}
              onChange={(e) => onOllamaUrlChange(e.target.value)}
              placeholder="http://127.0.0.1:11434"
            />
          </div>
        )}

        {activeProvider !== "ollama" && (
          <div className="settings-section">
            <h3><Key size={14} /> API Key</h3>
            <input
              className="compact-input"
              type="password"
              value={apiKey}
              onChange={(e) => onApiKeyChange(e.target.value)}
              placeholder={`Paste ${providerLabels[activeProvider]} API key`}
            />
            <div className="inline-actions">
              <button className="button button-primary" type="button" onClick={onApiKeySave} disabled={!apiKey.trim()}>
                <Key size={16} /> Save Key
              </button>
              <button className="button button-secondary" type="button" onClick={onApiKeyDelete} disabled={!hasApiKey}>
                <Trash2 size={16} /> Delete Key
              </button>
            </div>
          </div>
        )}
      </section>
    </div>
  );
}
