// src/features/cowriter/useCoWriter.ts
import { useCallback, useEffect, useMemo, useState } from "react";
import type { SearchChunkResult } from "../../app/vault";
import type { AiProviderKind, AiProviderModelsResponse, AiProviderSettings } from "../../app/ai";
import { providerLabels } from "../../app/ai";
import type { AsyncState, ToolSectionId } from "../../components/types";
import type { BannedWord, WardScanHit, WardSeverity } from "../../app/vault";
import {
  getProviderSettings, saveProviderSettings, setProviderApiKey, deleteProviderApiKey,
  acceptCloudDisclosure, selectProvider, listProviderModels, chatWithVault,
} from "../../app/ai";
import { listWards, addWard, removeWard } from "../../app/vault";

const AI_PROVIDERS: AiProviderKind[] = ["ollama", "openAi", "openAiCompatible", "anthropic", "googleAiStudio"];

export function useCoWriter(
  projectPath: string | null,
  tauriState: "checking" | "awake" | "browser",
  activeItemContent: string,
  searchResults: SearchChunkResult[],
  onSelectItem: (id: string) => void,
  onInsertText: (text: string) => void,
  showToast: (msg: string) => void,
) {
  const canUseNative = tauriState === "awake" && projectPath !== null;

  // Provider / engine state
  const [activeProvider, setActiveProvider] = useState<AiProviderKind>("ollama");
  const [providerModels, setProviderModels] = useState<AiProviderModelsResponse | null>(null);
  const [activeProviderSettings, setActiveProviderSettings] = useState<AiProviderSettings | null>(null);
  const [engineState, setEngineState] = useState<AsyncState>("idle");
  const [engineStatus, setEngineStatus] = useState("Choose a provider to begin.");
  const [engineError, setEngineError] = useState<string | null>(null);
  const [modelDraft, setModelDraft] = useState("");
  const [modelOptions, setModelOptions] = useState<string[]>([]);
  const [apiKeyDraft, setApiKeyDraft] = useState("");
  const [baseUrlDraft, setBaseUrlDraft] = useState("");

  // Cowriter state
  const [cowriterPrompt, setCowriterPrompt] = useState("");
  const [cowriterState, setCowriterState] = useState<AsyncState>("idle");
  const [cowriterStatus, setCowriterStatus] = useState("Ask a grounded question across the Vault.");
  const [cowriterAnswer, setCowriterAnswer] = useState("");
  const [cowriterError, setCowriterError] = useState<string | null>(null);
  const [retrievalResults, setRetrievalResults] = useState<SearchChunkResult[]>([]);
  const [answerWardHits, setAnswerWardHits] = useState<WardScanHit[]>([]);

  // Wards state
  const [wards, setWards] = useState<BannedWord[]>([]);
  const [wardInput, setWardInput] = useState("");
  const [wardSeverity, setWardSeverity] = useState<WardSeverity>("warn");
  const [wardState, setWardState] = useState<AsyncState>("idle");
  const [wardStatus, setWardStatus] = useState("");

  // Feed state
  const [importTitle, setImportTitle] = useState("");
  const [importBody, setImportBody] = useState("");
  const [importState, setImportState] = useState<AsyncState>("idle");
  const [importStatus, setImportStatus] = useState("Paste text or choose .txt / .md files.");
  const [importProgress, setImportProgress] = useState<string[]>([]);

  // UI state
  const [openToolSectionSet, setOpenToolSectionSet] = useState<Set<ToolSectionId>>(new Set(["cowriter"]));
  const [rightCollapsed, setRightCollapsed] = useState(false);

  const activeProviderIsCloud = activeProvider !== "ollama";
  const selectedModel = activeProviderSettings?.selectedModel ?? "";

  // Load provider settings when project changes
  const refreshProviderSettings = useCallback(async () => {
    if (!canUseNative || !projectPath) return;
    try {
      const response = await getProviderSettings(projectPath);
      setActiveProvider(response.activeProvider);
      const active = response.providers.find(p => p.provider === response.activeProvider);
      setActiveProviderSettings(active ?? null);
      setEngineStatus(`Active provider: ${providerLabels[response.activeProvider]}`);
    } catch {
      setEngineStatus("Could not load provider settings.");
    }
  }, [canUseNative, projectPath]);

  useEffect(() => {
    refreshProviderSettings();
  }, [refreshProviderSettings]);

  // Load wards
  const refreshWards = useCallback(async () => {
    if (!canUseNative || !projectPath) return;
    try {
      const response = await listWards(projectPath);
      setWards(response);
    } catch { /* empty */ }
  }, [canUseNative, projectPath]);

  useEffect(() => {
    refreshWards();
  }, [refreshWards]);

  // Handlers
  const handleToggleToolSection = useCallback((id: ToolSectionId) => {
    setOpenToolSectionSet(prev => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id); else next.add(id);
      return next;
    });
  }, []);

  const handleRefreshEngine = useCallback(async () => {
    if (!canUseNative || !projectPath) return;
    setEngineState("working");
    setEngineStatus("Refreshing models…");
    setEngineError(null);
    try {
      const response = await listProviderModels(projectPath, activeProvider);
      setProviderModels(response);
      const names = response.models.map(m => m.name);
      setModelOptions(names);
      setEngineState("success");
      setEngineStatus(response.message);
      if (response.selectedModel) setModelDraft(response.selectedModel);
      else if (names.length > 0) setModelDraft(names[0]);
    } catch (err) {
      setEngineState("failed");
      setEngineStatus("Could not refresh models.");
      setEngineError(String(err));
    }
  }, [canUseNative, projectPath, activeProvider]);

  const handleProviderTest = useCallback(async () => {
    if (!canUseNative || !projectPath) return;
    setEngineState("working");
    setEngineStatus("Testing provider…");
    setEngineError(null);
    try {
      const response = await listProviderModels(projectPath, activeProvider);
      setEngineState("success");
      setEngineStatus(response.reachable ? "Provider reachable." : "Provider not reachable.");
    } catch (err) {
      setEngineState("failed");
      setEngineStatus("Provider test failed.");
      setEngineError(String(err));
    }
  }, [canUseNative, projectPath, activeProvider]);

  const handleProviderSelection = useCallback(async (provider: AiProviderKind) => {
    if (!canUseNative || !projectPath) return;
    setActiveProvider(provider);
    try {
      const response = await selectProvider(projectPath, provider);
      const active = response.providers.find(p => p.provider === provider);
      setActiveProviderSettings(active ?? null);
      setEngineStatus(`Selected: ${providerLabels[provider]}`);
      if (provider !== "ollama") {
        // Accept disclosure automatically for cloud providers
        await acceptCloudDisclosure(projectPath, provider);
      }
    } catch {
      setEngineStatus("Could not select provider.");
    }
  }, [canUseNative, projectPath]);

  const handleApiKeySave = useCallback(async (e: React.FormEvent) => {
    e.preventDefault();
    if (!canUseNative || !projectPath || !apiKeyDraft.trim()) return;
    try {
      await setProviderApiKey(projectPath, activeProvider, apiKeyDraft.trim());
      setApiKeyDraft("");
      await refreshProviderSettings();
      showToast("API key saved.");
    } catch (err) {
      showToast(`Could not save key: ${err}`);
    }
  }, [canUseNative, projectPath, activeProvider, apiKeyDraft, refreshProviderSettings, showToast]);

  const handleApiKeyDelete = useCallback(async () => {
    if (!canUseNative || !projectPath) return;
    try {
      await deleteProviderApiKey(projectPath, activeProvider);
      await refreshProviderSettings();
      showToast("API key deleted.");
    } catch (err) {
      showToast(`Could not delete key: ${err}`);
    }
  }, [canUseNative, projectPath, activeProvider, refreshProviderSettings, showToast]);

  const handleEngineSettingsSave = useCallback(async (e: React.FormEvent) => {
    e.preventDefault();
    if (!canUseNative || !projectPath) return;
    try {
      await saveProviderSettings(projectPath, activeProvider, baseUrlDraft || null, modelDraft || null);
      await refreshProviderSettings();
      showToast("Engine settings saved.");
    } catch (err) {
      showToast(`Could not save settings: ${err}`);
    }
  }, [canUseNative, projectPath, activeProvider, baseUrlDraft, modelDraft, refreshProviderSettings, showToast]);

  const handleRunCowriter = useCallback(async () => {
    if (!canUseNative || !projectPath) return;
    if (!cowriterPrompt.trim()) {
      setCowriterStatus("Enter a question first.");
      return;
    }
    if (!selectedModel) {
      setCowriterStatus("Select a model first.");
      return;
    }
    setCowriterState("working");
    setCowriterStatus("Searching Vault…");
    setCowriterError(null);
    setCowriterAnswer("");
    setRetrievalResults([]);
    setAnswerWardHits([]);
    try {
      const canvasContext = activeItemContent.slice(0, 2000);
      const response = await chatWithVault(
        projectPath,
        activeProvider,
        selectedModel,
        cowriterPrompt.trim(),
        undefined,
        canvasContext,
        5,
      );
      setCowriterState("success");
      setCowriterStatus("Answer ready. Insert or discard.");
      setCowriterAnswer(response.text);
      setRetrievalResults(response.citations.map(c => ({
        chunkId: c.itemId,
        itemId: c.itemId,
        title: c.title,
        itemType: "note" as const,
        vaultPath: c.vaultPath,
        snippet: c.snippet,
        score: 0,
        confidence: "high" as const,
      })));
      setAnswerWardHits(response.wardHits);
    } catch (err) {
      setCowriterState("failed");
      setCowriterStatus("Co-Writer request failed.");
      setCowriterError(String(err));
    }
  }, [canUseNative, projectPath, cowriterPrompt, selectedModel, activeProvider, activeItemContent]);

  const handleInsertAnswer = useCallback(() => {
    if (!cowriterAnswer) return;
    onInsertText(cowriterAnswer);
    setCowriterAnswer("");
    setCowriterStatus("Answer inserted into Canvas.");
  }, [cowriterAnswer, onInsertText]);

  const handleCopyAnswer = useCallback(() => {
    if (!cowriterAnswer) return;
    navigator.clipboard.writeText(cowriterAnswer).then(() => {
      showToast("Answer copied.");
    }).catch(() => {
      showToast("Could not copy.");
    });
  }, [cowriterAnswer, showToast]);

  const handleDiscardAnswer = useCallback(() => {
    setCowriterAnswer("");
    setCowriterStatus("Answer discarded.");
    setRetrievalResults([]);
    setAnswerWardHits([]);
  }, []);

  const handleRewriteClean = useCallback(async () => {
    if (!canUseNative || !projectPath || !cowriterAnswer) return;
    setCowriterState("working");
    setCowriterStatus("Rewriting clean…");
    try {
      const response = await chatWithVault(
        projectPath,
        activeProvider,
        selectedModel,
        `Rewrite this answer cleanly, removing any AI slop and banned phrases:\n\n${cowriterAnswer}`,
        undefined,
        undefined,
        0,
      );
      setCowriterAnswer(response.text);
      setCowriterState("success");
      setCowriterStatus("Rewritten clean.");
    } catch (err) {
      setCowriterState("failed");
      setCowriterStatus("Rewrite failed.");
      setCowriterError(String(err));
    }
  }, [canUseNative, projectPath, cowriterAnswer, activeProvider, selectedModel]);

  const handleWardAdd = useCallback(async (e: React.FormEvent) => {
    e.preventDefault();
    if (!canUseNative || !projectPath || !wardInput.trim()) return;
    setWardState("working");
    setWardStatus("Adding ward…");
    try {
      const response = await addWard(projectPath, wardInput.trim(), wardSeverity);
      setWards(response);
      setWardInput("");
      setWardState("success");
      setWardStatus("Ward added.");
    } catch (err) {
      setWardState("failed");
      setWardStatus(String(err));
    }
  }, [canUseNative, projectPath, wardInput, wardSeverity]);

  const handleWardRemove = useCallback(async (id: string) => {
    if (!canUseNative || !projectPath) return;
    try {
      const response = await removeWard(projectPath, id);
      setWards(response);
    } catch { /* empty */ }
  }, [canUseNative, projectPath]);

  return {
    // Provider / engine
    providers: AI_PROVIDERS,
    activeProvider, selectedModel, providerModels, activeProviderSettings, activeProviderIsCloud,
    engineState, engineStatus, engineError, modelDraft, modelOptions, apiKeyDraft, baseUrlDraft,
    // Cowriter
    cowriterPrompt, cowriterState, cowriterStatus, cowriterAnswer, cowriterError,
    retrievalResults, answerWardHits,
    // Wards
    wards, wardInput, wardSeverity, wardState, wardStatus,
    // Feed
    importTitle, importBody, importState, importStatus, importProgress,
    // UI
    openToolSectionSet, rightCollapsed, searchResults,
    // Setters
    setModelDraft, setApiKeyDraft, setBaseUrlDraft, setCowriterPrompt,
    setWardInput, setWardSeverity, setImportTitle, setImportBody,
    // Handlers
    onToggleToolSection: handleToggleToolSection,
    onRefreshEngine: handleRefreshEngine,
    onProviderTest: handleProviderTest,
    onProviderSelection: handleProviderSelection,
    onApiKeySave: handleApiKeySave,
    onApiKeyDelete: handleApiKeyDelete,
    onEngineSettingsSave: handleEngineSettingsSave,
    onRunCowriter: handleRunCowriter,
    onInsertAnswer: handleInsertAnswer,
    onCopyAnswer: handleCopyAnswer,
    onDiscardAnswer: handleDiscardAnswer,
    onRewriteClean: handleRewriteClean,
    onWardAdd: handleWardAdd,
    onWardRemove: handleWardRemove,
    onSelectItem,
    onExpandRight: () => setRightCollapsed(false),
    onCollapseRight: () => setRightCollapsed(true),
  };
}
