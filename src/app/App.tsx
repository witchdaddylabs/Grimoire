import { invoke } from "@tauri-apps/api/core";
import {
  AlertTriangle,
  Archive,
  BookOpenText,
  BrainCircuit,
  Check,
  ChevronDown,
  ChevronLeft,
  ChevronRight,
  Clipboard,
  Copy,
  Database,
  Download,
  Feather,
  FileText,
  Info,
  Loader2,
  Moon,
  Plus,
  Search,
  ShieldCheck,
  Sparkles,
  SunMedium,
  Trash2,
  Upload,
  WandSparkles,
  X,
} from "lucide-react";
import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type CSSProperties,
  type FormEvent,
  type ReactNode,
} from "react";
import {
  acceptCloudDisclosure,
  aiChat,
  cloudProvider,
  deleteProviderApiKey,
  getProviderSettings,
  listProviderModels,
  providerLabels,
  saveProviderSettings,
  selectProvider,
  setProviderApiKey,
  type AiProviderKind,
  type AiProviderModelsResponse,
  type AiProviderSettings,
  type AiProviderSettingsResponse,
} from "./ai";
import {
  addWard,
  archiveVaultItem,
  deleteVaultItem,
  exportItemMarkdown,
  exportProjectJson,
  fallbackVaultTree,
  flattenVaultItems,
  getVaultItem,
  importText,
  listWards,
  loadVaultTree,
  removeWard,
  scanWards,
  searchChunks,
  updateVaultItem,
  type BannedWord,
  type SearchChunkResult,
  type VaultDrawerNode,
  type VaultHallNode,
  type VaultItemDetail,
  type VaultItemNode,
  type VaultRoomNode,
  type VaultTreeResponse,
  type VaultWingNode,
  type WardScanHit,
  type WardSeverity,
} from "./vault";
import { compactPath, createProject, createDemoProject, openProject, recordRecentProject, removeRecentProject, getRecentProjects, describeError, type ProjectMetadata, type RecentProject } from "./project";
import { open } from "@tauri-apps/plugin-dialog";
import { VaultPanel } from "../features/vault/VaultPanel";
import { CanvasPanel } from "../features/canvas/CanvasPanel";
import { CoWriterPanel } from "../features/cowriter/CoWriterPanel";
import { OnboardingOverlay } from "../components/OnboardingOverlay";
import { CloudDisclosureDialog } from "../components/CloudDisclosureDialog";
import { StatusChip, saveStateTone, saveStateLabel } from "../components/ui/StatusChip";

type TauriState = "checking" | "awake" | "browser";
type SaveState = "idle" | "editing" | "saving" | "saved" | "failed" | "preview";
type AsyncState = "idle" | "working" | "success" | "failed";
type OnboardingStep = "welcome" | "vault" | "feed" | "engine" | "wards" | "canvas";
type ToolSectionId = "feed" | "retrieval" | "engine" | "cowriter" | "wards" | "about";
type OnboardingState = {
  complete: boolean;
  step: OnboardingStep;
};
type OnboardingStore = Record<string, OnboardingState>;
type WorkspacePrefs = {
  leftCollapsed: boolean;
  rightCollapsed: boolean;
  openToolSections: ToolSectionId[];
  theme: "dark" | "ivory";
};

const ONBOARDING_STORAGE_KEY = "grimoire.onboarding.v2";
const ONBOARDING_PREVIEW_SCOPE = "__preview__";
const WORKSPACE_PREFS_KEY = "grimoire.workspace.v1";
const EDITOR_AUTOSAVE_DELAY = 850;
const IMPORT_WORD_LIMIT = 10_000;
const COWRITER_RETRIEVAL_LIMIT = 5;
const AI_PROVIDERS: AiProviderKind[] = [
  "ollama",
  "openAi",
  "openAiCompatible",
  "googleAiStudio",
];
const FALLBACK_CLOUD_DISCLOSURE =
  "Cloud providers may receive your prompt, relevant Vault excerpts, and active Canvas context.";

const onboardingSteps: OnboardingStep[] = [
  "welcome",
  "vault",
  "feed",
  "engine",
  "wards",
  "canvas",
];

const optionalOnboardingSteps = new Set<OnboardingStep>(["feed", "engine", "wards"]);
const defaultOpenToolSections: ToolSectionId[] = ["engine", "cowriter"];
const wardPresetOptions = ["very", "really", "suddenly", "somehow", "actually"];

const providerModelPresets: Partial<Record<AiProviderKind, string[]>> = {
  openAi: ["gpt-5-mini", "gpt-5.2", "gpt-4.1-mini"],
  googleAiStudio: ["gemini-3-flash-preview", "gemini-3.1-pro-preview", "gemini-2.5-flash"],
};

const onboardingCopy: Record<OnboardingStep, { title: string; body: string }> = {
  welcome: {
    title: "Welcome to Grimoire",
    body: "A local-first writing desk for long work, canon memory, and grounded assistance.",
  },
  vault: {
    title: "The Vault",
    body: "Your project is arranged as Wings, Halls, Rooms, Drawers, and editable writing items.",
  },
  feed: {
    title: "Feed the Vault",
    body: "Paste text or import Markdown and plain text files when you are ready to stock local memory.",
  },
  engine: {
    title: "Local Engine",
    body: "Ollama stays optional. Writing, search, import, wards, and export still work without it.",
  },
  wards: {
    title: "Wards",
    body: "Wards are banned-word and banned-phrase checks for language you want Grimoire to warn about before AI text is inserted.",
  },
  canvas: {
    title: "The Canvas",
    body: "Write in the calm graphite editor, switch to ivory manuscript mode when useful, and let autosave handle the rest.",
  },
};

export function App() {
  const [vaultTree, setVaultTree] = useState<VaultTreeResponse>(fallbackVaultTree);
  const [activeItemId, setActiveItemId] = useState(
    flattenVaultItems(fallbackVaultTree)[0]?.id ?? "",
  );
  const [expandedNodeIds, setExpandedNodeIds] = useState(
    () =>
      new Set([
        "wing_novel",
        "wing_imports",
        "hall_characters",
        "hall_world",
        "hall_drafts",
        "hall_feed",
        "room_protagonists",
        "room_cities",
        "room_act_one",
        "room_imports",
        "drawer_main_cast",
        "drawer_northern_cities",
        "drawer_opening_sequence",
        "drawer_imported_text",
      ]),
  );
  const [tauriState, setTauriState] = useState<TauriState>("checking");
  const [project, setProject] = useState<ProjectMetadata | null>(null);
  const [projectError, setProjectError] = useState<string | null>(null);
  const [treeError, setTreeError] = useState<string | null>(null);
  const [projectLoading, setProjectLoading] = useState(true);
  const [focusMode, setFocusMode] = useState(false);
  const [showProjectPicker, setShowProjectPicker] = useState(false);
  const [recentProjects, setRecentProjects] = useState<RecentProject[]>(() => getRecentProjects());
  const [projectCreateName, setProjectCreateName] = useState("");
  const [projectCreateError, setProjectCreateError] = useState<string | null>(null);

  const [editorTitle, setEditorTitle] = useState("");
  const [editorContent, setEditorContent] = useState("");
  const [loadedItemId, setLoadedItemId] = useState("");
  const [saveState, setSaveState] = useState<SaveState>("idle");
  const [saveError, setSaveError] = useState<string | null>(null);
  const lastSavedRef = useRef({ itemId: "", title: "", content: "" });

  const [searchQuery, setSearchQuery] = useState("");
  const [searchResults, setSearchResults] = useState<SearchChunkResult[]>([]);
  const [searchState, setSearchState] = useState<AsyncState>("idle");
  const [searchError, setSearchError] = useState<string | null>(null);

  const [importTitle, setImportTitle] = useState("");
  const [importBody, setImportBody] = useState("");
  const [importState, setImportState] = useState<AsyncState>("idle");
  const [importStatus, setImportStatus] = useState("Paste text or choose .txt / .md files.");
  const [importProgress, setImportProgress] = useState<string[]>([]);

  const [providerSettings, setProviderSettings] = useState<AiProviderSettingsResponse | null>(null);
  const [providerModels, setProviderModels] = useState<AiProviderModelsResponse | null>(null);
  const [engineState, setEngineState] = useState<AsyncState>("idle");
  const [engineStatus, setEngineStatus] = useState("Ollama has not been checked yet.");
  const [engineError, setEngineError] = useState<string | null>(null);
  const [apiKeyDraft, setApiKeyDraft] = useState("");
  const [baseUrlDraft, setBaseUrlDraft] = useState("");
  const [modelDraft, setModelDraft] = useState("");
  const [disclosureProvider, setDisclosureProvider] = useState<AiProviderKind | null>(null);
  const [pendingProvider, setPendingProvider] = useState<AiProviderKind | null>(null);
  const [cowriterPrompt, setCowriterPrompt] = useState("");
  const [cowriterState, setCowriterState] = useState<AsyncState>("idle");
  const [cowriterStatus, setCowriterStatus] = useState("Ask for help grounded in local Vault search.");
  const [cowriterAnswer, setCowriterAnswer] = useState("");
  const [cowriterError, setCowriterError] = useState<string | null>(null);
  const [retrievalResults, setRetrievalResults] = useState<SearchChunkResult[]>([]);
  const [answerWardHits, setAnswerWardHits] = useState<WardScanHit[]>([]);

  const [wards, setWards] = useState<BannedWord[]>([]);
  const [wardInput, setWardInput] = useState("");
  const [wardSeverity, setWardSeverity] = useState<WardSeverity>("warn");
  const [wardState, setWardState] = useState<AsyncState>("idle");
  const [wardStatus, setWardStatus] = useState("Default phrase wards are active when project storage is available.");

  const [exportState, setExportState] = useState<AsyncState>("idle");
  const [exportStatus, setExportStatus] = useState("Exports write into the project’s local exports folder.");

  const [showOnboarding, setShowOnboarding] = useState(false);
  const [onboardingStep, setOnboardingStep] = useState<OnboardingStep>("welcome");
  const onboardingScopeRef = useRef<string>(ONBOARDING_PREVIEW_SCOPE);
  const [workspacePrefs, setWorkspacePrefs] = useState<WorkspacePrefs>(() => readWorkspacePrefs());

  const vaultFlatItems = useMemo(() => flattenVaultItems(vaultTree), [vaultTree]);
  const activeItem = useMemo<VaultItemNode | null>(
    () => vaultFlatItems.find((item) => item.id === activeItemId) ?? vaultFlatItems[0] ?? null,
    [activeItemId, vaultFlatItems],
  );
  const activeProvider = providerSettings?.activeProvider ?? "ollama";
  const activeProviderSettings = useMemo(
    () => providerSettings?.providers.find((provider) => provider.provider === activeProvider) ?? null,
    [activeProvider, providerSettings],
  );
  const activeProviderIsCloud = cloudProvider(activeProvider);
  const selectedModel = providerModels?.selectedModel ?? activeProviderSettings?.selectedModel ?? "";
  const modelOptions = useMemo(() => {
    const detected = providerModels?.models.map((model) => model.name) ?? [];
    const presets = providerModelPresets[activeProvider] ?? [];
    return Array.from(new Set([...detected, ...presets, selectedModel, modelDraft].filter(Boolean)));
  }, [activeProvider, modelDraft, providerModels, selectedModel]);
  const editorWordCount = useMemo(() => countWords(editorContent), [editorContent]);
  const canUseNative = tauriState === "awake" && project !== null;
  const shellClassName = [
    "app-shell",
    focusMode ? "focus-mode" : "",
    workspacePrefs.theme === "ivory" ? "theme-ivory" : "theme-dark",
    workspacePrefs.leftCollapsed ? "left-collapsed" : "",
    workspacePrefs.rightCollapsed ? "right-collapsed" : "",
  ]
    .filter(Boolean)
    .join(" ");
  const openToolSectionSet = useMemo(
    () => new Set(workspacePrefs.openToolSections),
    [workspacePrefs.openToolSections],
  );

  const refreshTree = useCallback(
    async (selectItemId?: string) => {
      if (!project) return;
      const tree = await loadVaultTree(project.projectPath);
      setVaultTree(tree);
      if (selectItemId) {
        setActiveItemId(selectItemId);
      } else if (!flattenVaultItems(tree).some((item) => item.id === activeItemId)) {
        const firstItem = flattenVaultItems(tree)[0];
        if (firstItem) setActiveItemId(firstItem.id);
      }
    },
    [activeItemId, project],
  );

  function toggleNode(nodeId: string) {
    setExpandedNodeIds((current) => {
      const next = new Set(current);
      if (next.has(nodeId)) {
        next.delete(nodeId);
      } else {
        next.add(nodeId);
      }
      return next;
    });
  }

  function markEditorChanged() {
    if (!canUseNative) {
      setSaveState("preview");
      return;
    }
    setSaveState("editing");
    setSaveError(null);
  }

  function syncProviderDrafts(settings: AiProviderSettingsResponse, models?: AiProviderModelsResponse | null) {
    const active = settings.providers.find((provider) => provider.provider === settings.activeProvider);
    setBaseUrlDraft(active?.baseUrl ?? "");
    setModelDraft(models?.selectedModel ?? active?.selectedModel ?? "");
    setApiKeyDraft("");
  }

  function updateWorkspacePrefs(update: (current: WorkspacePrefs) => WorkspacePrefs) {
    setWorkspacePrefs((current) => {
      const next = update(current);
      writeWorkspacePrefs(next);
      return next;
    });
  }

  function toggleToolSection(section: ToolSectionId) {
    updateWorkspacePrefs((current) => {
      const openSections = new Set(current.openToolSections);
      if (openSections.has(section)) {
        openSections.delete(section);
      } else {
        openSections.add(section);
      }
      return { ...current, openToolSections: Array.from(openSections) };
    });
  }

  function setSideCollapsed(side: "left" | "right", collapsed: boolean) {
    updateWorkspacePrefs((current) => ({
      ...current,
      leftCollapsed: side === "left" ? collapsed : current.leftCollapsed,
      rightCollapsed: side === "right" ? collapsed : current.rightCollapsed,
    }));
  }

  function toggleTheme() {
    updateWorkspacePrefs((current) => ({
      ...current,
      theme: current.theme === "ivory" ? "dark" : "ivory",
    }));
  }

  function loadItemIntoEditor(item: VaultItemDetail) {
    setActiveItemId(item.id);
    setEditorTitle(item.title);
    setEditorContent(item.content);
    setLoadedItemId(item.id);
    setSaveState("saved");
    setSaveError(null);
    lastSavedRef.current = { itemId: item.id, title: item.title, content: item.content };
  }

  async function loadProviderSystems(projectPath: string, provider?: AiProviderKind) {
    setEngineState("working");
    setEngineError(null);
    const settings = await getProviderSettings(projectPath);
    const providerToLoad = provider ?? settings.activeProvider;
    const models = await listProviderModels(projectPath, providerToLoad);
    setProviderSettings(settings);
    setProviderModels(models);
    syncProviderDrafts(settings, models);
    setEngineStatus(models.message);
    setEngineState(models.reachable || models.models.length > 0 ? "success" : "failed");
    return { settings, models };
  }

  useEffect(() => {
    let cancelled = false;

    async function bootDesktopProject() {
      try {
        await invoke<string>("app_ping");
        if (cancelled) return;

        setTauriState("awake");

        // Check if there are recent projects; if so, show the picker.
        // The user can then choose to open an existing project, create a new one,
        // or load the demo.
        setShowProjectPicker(true);
      } catch {
        if (cancelled) return;
        setTauriState("browser");
        setProjectError("Project storage is available in the macOS desktop shell.");
      } finally {
        if (!cancelled) setProjectLoading(false);
      }
    }

    bootDesktopProject();

    return () => {
      cancelled = true;
    };
  }, []);

  // ── Project loading helper ──

  const loadProjectIntoWorkspace = useCallback(
    async (metadata: ProjectMetadata) => {
      setProject(metadata);
      setProjectError(null);
      setShowProjectPicker(false);
      recordRecentProject(metadata);
      setRecentProjects(getRecentProjects());

      try {
        const tree = await loadVaultTree(metadata.projectPath);
        setVaultTree(tree);
        setTreeError(null);
        const firstItem = flattenVaultItems(tree)[0];
        if (firstItem) setActiveItemId(firstItem.id);
      } catch (error) {
        setTreeError(describeError(error));
      }
    },
    [],
  );

  // ── Project open/create handlers ──

  const handleOpenExistingProject = useCallback(async () => {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: "Open Grimoire Project",
      });
      if (selected && typeof selected === "string") {
        const metadata = await openProject(selected);
        await loadProjectIntoWorkspace(metadata);
      }
    } catch (error) {
      setProjectCreateError(describeError(error));
    }
  }, [loadProjectIntoWorkspace]);

  const handleCreateProject = useCallback(async () => {
    const name = projectCreateName.trim();
    if (!name) {
      setProjectCreateError("Enter a project name.");
      return;
    }
    setProjectCreateError(null);
    try {
      const metadata = await createProject(name);
      await loadProjectIntoWorkspace(metadata);
      setProjectCreateName("");
    } catch (error) {
      setProjectCreateError(describeError(error));
    }
  }, [projectCreateName, loadProjectIntoWorkspace]);

  const handleLoadDemo = useCallback(async () => {
    setProjectCreateError(null);
    try {
      const metadata = await createDemoProject();
      await loadProjectIntoWorkspace(metadata);
    } catch (error) {
      setProjectCreateError(describeError(error));
    }
  }, [loadProjectIntoWorkspace]);

  const handleOpenRecentProject = useCallback(
    async (path: string) => {
      try {
        const metadata = await openProject(path);
        await loadProjectIntoWorkspace(metadata);
      } catch (error) {
        setProjectCreateError(describeError(error));
        // Remove from recent list if it no longer exists
        removeRecentProject(path);
        setRecentProjects(getRecentProjects());
      }
    },
    [loadProjectIntoWorkspace],
  );

  // ... rest of effects ...

  useEffect(() => {
    if (!activeItem) return;
    const selectedItem = activeItem;
    const currentProject = project;
    let cancelled = false;

    async function loadItem() {
      try {
        const item =
          currentProject && tauriState === "awake"
            ? await getVaultItem(currentProject.projectPath, selectedItem.id)
            : fallbackDetail(selectedItem);
        if (cancelled) return;
        setLoadedItemId(item.id);
        setEditorTitle(item.title);
        setEditorContent(item.content);
        setSaveState(currentProject && tauriState === "awake" ? "saved" : "preview");
        setSaveError(null);
        lastSavedRef.current = { itemId: item.id, title: item.title, content: item.content };
      } catch (error) {
        if (cancelled) return;
        const fallback = fallbackDetail(selectedItem);
        setLoadedItemId(fallback.id);
        setEditorTitle(fallback.title);
        setEditorContent(fallback.content);
        setSaveState("failed");
        setSaveError(describeError(error));
      }
    }

    loadItem();

    return () => {
      cancelled = true;
    };
  }, [activeItem, project, tauriState]);

  useEffect(() => {
    if (!project || !activeItem || loadedItemId !== activeItem.id || saveState !== "editing") {
      return;
    }

    const lastSaved = lastSavedRef.current;
    if (
      lastSaved.itemId === activeItem.id &&
      lastSaved.title === editorTitle &&
      lastSaved.content === editorContent
    ) {
      setSaveState("saved");
      return;
    }

    let cancelled = false;
    const timer = window.setTimeout(async () => {
      try {
        setSaveState("saving");
        const saved = await updateVaultItem(
          project.projectPath,
          activeItem.id,
          editorTitle,
          editorContent,
        );
        if (cancelled) return;
        lastSavedRef.current = {
          itemId: saved.id,
          title: saved.title,
          content: saved.content,
        };
        setSaveError(null);
        setSaveState("saved");
        await refreshTree(saved.id);
      } catch (error) {
        if (cancelled) return;
        setSaveError(saveFailureCopy(error));
        setSaveState("failed");
      }
    }, EDITOR_AUTOSAVE_DELAY);

    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
  }, [
    activeItem,
    editorContent,
    editorTitle,
    loadedItemId,
    project,
    refreshTree,
  ]);

  useEffect(() => {
    if (!project || tauriState !== "awake") return;
    const currentProject = project;
    let cancelled = false;

    async function loadLocalSystems() {
      try {
        setEngineState("working");
        const [wardList, settings] = await Promise.all([
          listWards(currentProject.projectPath),
          getProviderSettings(currentProject.projectPath),
        ]);
        const models = await listProviderModels(currentProject.projectPath, settings.activeProvider);
        if (cancelled) return;
        setWards(wardList);
        setProviderSettings(settings);
        setProviderModels(models);
        syncProviderDrafts(settings, models);
        setWardStatus(`${wardList.length} ward phrases ready.`);
        setEngineStatus(models.message);
        setEngineError(null);
        setEngineState(models.reachable || models.models.length > 0 ? "success" : "failed");
      } catch (error) {
        if (cancelled) return;
        setWardStatus(describeError(error));
        setEngineError(describeError(error));
        setEngineStatus("AI provider check failed.");
        setEngineState("failed");
      }
    }

    loadLocalSystems();

    return () => {
      cancelled = true;
    };
  }, [project, tauriState]);

  useEffect(() => {
    if (projectLoading || tauriState === "checking") return;
    const scope =
      project && tauriState === "awake"
        ? onboardingScopeForProject(project.projectPath)
        : ONBOARDING_PREVIEW_SCOPE;
    onboardingScopeRef.current = scope;
    const state = readOnboardingState(scope);
    setOnboardingStep(state.step);
    setShowOnboarding(!state.complete);
  }, [project, projectLoading, tauriState]);

  async function handleSearch(event?: FormEvent) {
    event?.preventDefault();
    const query = searchQuery.trim();
    if (!query) {
      setSearchResults([]);
      setSearchState("idle");
      setSearchError(null);
      return;
    }

    setSearchState("working");
    setSearchError(null);
    try {
      if (project && tauriState === "awake") {
        const response = await searchChunks(project.projectPath, query, 8);
        setSearchResults(response.results);
      } else {
        setSearchResults(browserSearch(vaultFlatItems, query));
      }
      setSearchState("success");
    } catch (error) {
      setSearchError(describeError(error));
      setSearchState("failed");
    }
  }

  async function performPasteImport() {
    if (!project || tauriState !== "awake") {
      setImportState("failed");
      setImportStatus("Import needs the desktop shell and local project storage.");
      return;
    }

    setImportState("working");
    setImportProgress(["Reading the bones"]);
    setImportStatus("Importing pasted text...");
    try {
      const prepared = prepareImportContent(importBody);
      const response = await importText(
        project.projectPath,
        importTitle || "Imported writing",
        prepared.content,
        "Pasted text",
      );
      loadItemIntoEditor(response.item);
      setImportProgress(response.progressLabels);
      setImportStatus(
        prepared.truncated
          ? `Imported the first ${IMPORT_WORD_LIMIT.toLocaleString()} words of ${response.item.title}; add the next section when ready.`
          : `Imported ${response.item.title} with ${response.createdChunks} searchable chunks.`,
      );
      setImportBody("");
      setImportTitle("");
      setImportState("success");
      await refreshTree(response.item.id);
    } catch (error) {
      setImportState("failed");
      setImportStatus(describeError(error));
    }
  }

  async function handlePasteImport(event: FormEvent) {
    event.preventDefault();
    await performPasteImport();
  }

  async function handleFileImport(files: FileList | null) {
    if (!files || files.length === 0) return;
    if (!project || tauriState !== "awake") {
      setImportState("failed");
      setImportStatus("File import needs the desktop shell and local project storage.");
      return;
    }

    setImportState("working");
    setImportProgress(["Reading the bones"]);
    try {
      let lastItemId = "";
      let lastItem: VaultItemDetail | null = null;
      let importedCount = 0;
      let truncatedCount = 0;
      let skippedCount = 0;
      for (const file of Array.from(files)) {
        const isSupported = /\.(md|txt|markdown)$/i.test(file.name);
        if (!isSupported) {
          skippedCount += 1;
          continue;
        }
        const content = await file.text();
        const prepared = prepareImportContent(content);
        if (prepared.truncated) truncatedCount += 1;
        const title = file.name.replace(/\.(md|txt|markdown)$/i, "");
        const response = await importText(project.projectPath, title, prepared.content, file.name);
        lastItemId = response.item.id;
        lastItem = response.item;
        importedCount += 1;
        setImportProgress(response.progressLabels);
      }
      if (importedCount === 0) {
        setImportState("failed");
        setImportStatus("No .txt or .md files were selected.");
        return;
      }
      if (lastItem) loadItemIntoEditor(lastItem);
      setImportState("success");
      const notes = [
        `Imported ${importedCount} file${importedCount === 1 ? "" : "s"}.`,
        truncatedCount
          ? `${truncatedCount} file${truncatedCount === 1 ? " was" : "s were"} capped at ${IMPORT_WORD_LIMIT.toLocaleString()} words.`
          : "",
        skippedCount ? `${skippedCount} unsupported file${skippedCount === 1 ? "" : "s"} skipped.` : "",
      ].filter(Boolean);
      setImportStatus(notes.join(" "));
      await refreshTree(lastItemId);
    } catch (error) {
      setImportState("failed");
      setImportStatus(describeError(error));
    }
  }

  async function refreshEngine() {
    if (!project || tauriState !== "awake") return;
    try {
      await loadProviderSystems(project.projectPath);
    } catch (error) {
      setEngineState("failed");
      setEngineError(describeError(error));
      setEngineStatus("AI provider check failed.");
    }
  }

  async function handleProviderSelection(provider: AiProviderKind) {
    if (!project) return;
    const providerSetting = providerSettings?.providers.find((candidate) => candidate.provider === provider);
    if (cloudProvider(provider) && !providerSetting?.disclosureAcceptedAt) {
      setPendingProvider(provider);
      setDisclosureProvider(provider);
      return;
    }
    setEngineState("working");
    try {
      const settings = await selectProvider(project.projectPath, provider);
      const models = await listProviderModels(project.projectPath, provider);
      setProviderSettings(settings);
      setProviderModels(models);
      syncProviderDrafts(settings, models);
      setEngineStatus(models.message);
      setEngineError(null);
      setEngineState(models.reachable || models.models.length > 0 ? "success" : "failed");
    } catch (error) {
      setEngineState("failed");
      setEngineError(describeError(error));
      setEngineStatus("Provider selection failed.");
    }
  }

  async function acceptProviderDisclosure(provider: AiProviderKind) {
    if (!project) return;
    setEngineState("working");
    try {
      const settings = await acceptCloudDisclosure(project.projectPath, provider);
      setProviderSettings(settings);
      setDisclosureProvider(null);
      const providerToSelect = pendingProvider ?? provider;
      setPendingProvider(null);
      const selectedSettings = await selectProvider(project.projectPath, providerToSelect);
      const models = await listProviderModels(project.projectPath, providerToSelect);
      setProviderSettings(selectedSettings);
      setProviderModels(models);
      syncProviderDrafts(selectedSettings, models);
      setEngineStatus(models.message);
      setEngineError(null);
      setEngineState(models.reachable || models.models.length > 0 ? "success" : "failed");
    } catch (error) {
      setEngineState("failed");
      setEngineError(describeError(error));
    }
  }

  async function handleEngineSettingsSave(event?: FormEvent) {
    event?.preventDefault();
    if (!project) return;
    setEngineState("working");
    try {
      const settings = await saveProviderSettings(
        project.projectPath,
        activeProvider,
        activeProvider === "openAiCompatible" ? baseUrlDraft : undefined,
        modelDraft,
      );
      const models = await listProviderModels(project.projectPath, activeProvider);
      setProviderSettings(settings);
      setProviderModels(models);
      syncProviderDrafts(settings, models);
      setEngineStatus(models.message);
      setEngineError(null);
      setEngineState(models.reachable || models.models.length > 0 ? "success" : "failed");
    } catch (error) {
      setEngineState("failed");
      setEngineError(describeError(error));
      setEngineStatus("Could not save AI settings.");
    }
  }

  async function handleProviderTest() {
    if (!project || tauriState !== "awake") {
      setEngineState("failed");
      setEngineError("Provider testing needs the desktop shell and local project storage.");
      return;
    }
    const model = (modelDraft || selectedModel).trim();
    if (!model) {
      setEngineState("failed");
      setEngineError("Choose or enter a model before testing this provider.");
      return;
    }
    if (activeProviderIsCloud && !activeProviderSettings?.disclosureAcceptedAt) {
      setDisclosureProvider(activeProvider);
      setEngineState("failed");
      setEngineError("Accept the cloud model disclosure before testing this provider.");
      return;
    }
    if (activeProviderIsCloud && !activeProviderSettings?.apiKeyPresent) {
      setEngineState("failed");
      setEngineError(`Save a ${providerLabels[activeProvider]} API key before testing.`);
      return;
    }

    setEngineState("working");
    setEngineError(null);
    setEngineStatus(`Testing ${providerLabels[activeProvider]}...`);
    try {
      const settings = await saveProviderSettings(
        project.projectPath,
        activeProvider,
        activeProvider === "openAiCompatible" ? baseUrlDraft : undefined,
        model,
      );
      setProviderSettings(settings);
      const response = await aiChat(
        project.projectPath,
        activeProvider,
        model,
        "Reply with exactly: OK",
        "Provider connectivity test. Do not use Vault or Canvas content.",
      );
      setEngineState("success");
      setEngineStatus(
        `${providerLabels[response.provider]} responded using ${response.model}.`,
      );
    } catch (error) {
      setEngineState("failed");
      setEngineError(safeProviderError(describeError(error)));
      setEngineStatus("Provider test failed.");
    }
  }

  async function handleApiKeySave(event: FormEvent) {
    event.preventDefault();
    if (!project || !activeProviderIsCloud) return;
    setEngineState("working");
    try {
      const settings = await setProviderApiKey(project.projectPath, activeProvider, apiKeyDraft);
      const models = await listProviderModels(project.projectPath, activeProvider);
      setProviderSettings(settings);
      setProviderModels(models);
      syncProviderDrafts(settings, models);
      setEngineStatus("API key saved in the system keychain.");
      setEngineError(null);
      setEngineState("success");
    } catch (error) {
      setEngineState("failed");
      setEngineError(describeError(error));
      setEngineStatus("Could not save API key.");
    }
  }

  async function handleApiKeyDelete() {
    if (!project || !activeProviderIsCloud) return;
    setEngineState("working");
    try {
      const settings = await deleteProviderApiKey(project.projectPath, activeProvider);
      const models = await listProviderModels(project.projectPath, activeProvider);
      setProviderSettings(settings);
      setProviderModels(models);
      syncProviderDrafts(settings, models);
      setEngineStatus("API key deleted.");
      setEngineError(null);
      setEngineState("failed");
    } catch (error) {
      setEngineState("failed");
      setEngineError(describeError(error));
      setEngineStatus("Could not delete API key.");
    }
  }

  async function runCowriter(extraInstruction?: string) {
    if (!project || tauriState !== "awake") {
      setCowriterState("failed");
      setCowriterError("Co-Writer needs the desktop shell for local retrieval and AI providers.");
      return;
    }
    const model = (modelDraft || selectedModel).trim();
    if (!model) {
      setCowriterState("failed");
      setCowriterError("Select or enter a model before asking the Co-Writer.");
      return;
    }
    if (activeProviderIsCloud && !activeProviderSettings?.disclosureAcceptedAt) {
      setDisclosureProvider(activeProvider);
      setCowriterState("failed");
      setCowriterError("Accept the cloud model disclosure before sending Vault context.");
      return;
    }
    if (activeProviderIsCloud && !activeProviderSettings?.apiKeyPresent) {
      setCowriterState("failed");
      setCowriterError(`Add an API key for ${providerLabels[activeProvider]} before asking the Co-Writer.`);
      return;
    }
    const prompt = [cowriterPrompt.trim(), extraInstruction].filter(Boolean).join("\n\n");
    if (!prompt) return;

    setCowriterState("working");
    setCowriterError(null);
    setCowriterAnswer("");
    setAnswerWardHits([]);
    setCowriterStatus("Consulting the Vault");

    try {
      const contextEditorContent = await ensureActiveEditorContext();
      const retrievalResults = await retrieveCowriterResults(project.projectPath, prompt);
      setRetrievalResults(retrievalResults);
      setCowriterStatus("Reading canon traces");
      const context = buildGroundedContext(retrievalResults, activeItem, contextEditorContent);
      setCowriterStatus("Composing grounded answer");
      const response = await aiChat(project.projectPath, activeProvider, model, prompt, context);
      setCowriterStatus("Checking slop wards");
      const wardScan = await scanWards(project.projectPath, response.text);
      setAnswerWardHits(wardScan.hits);
      setCowriterAnswer(response.text);
      setCowriterState("success");
      setCowriterStatus(
        wardScan.hits.length > 0
          ? `${wardScan.hits.length} ward warning${wardScan.hits.length === 1 ? "" : "s"} found.`
          : "Grounded answer ready.",
      );
    } catch (error) {
      setCowriterState("failed");
      setCowriterError(describeError(error));
      setCowriterStatus("Co-Writer request failed.");
    }
  }

  async function ensureActiveEditorContext() {
    if (!project || !activeItem || tauriState !== "awake" || loadedItemId === activeItem.id) {
      return editorContent;
    }

    try {
      const detail = await getVaultItem(project.projectPath, activeItem.id);
      loadItemIntoEditor(detail);
      return detail.content;
    } catch {
      return editorContent;
    }
  }

  async function retrieveCowriterResults(projectPath: string, prompt: string) {
    const seen = new Set<string>();
    const results: SearchChunkResult[] = [];

    for (const query of cowriterSearchQueries(prompt)) {
      try {
        const response = await searchChunks(projectPath, query, COWRITER_RETRIEVAL_LIMIT, "broad");
        for (const result of response.results) {
          if (seen.has(result.chunkId)) continue;
          seen.add(result.chunkId);
          results.push(result);
          if (results.length >= COWRITER_RETRIEVAL_LIMIT) return results;
        }
      } catch {
        // Some natural-language prompts do not produce usable FTS terms; keyword fallbacks below still can.
      }
    }

    return results;
  }

  function insertCowriterAnswer() {
    if (!cowriterAnswer.trim()) return;
    setEditorContent((current) =>
      current.trim()
        ? `${current.trimEnd()}\n\n${cowriterAnswer.trim()}`
        : cowriterAnswer.trim(),
    );
    markEditorChanged();
  }

  async function copyCowriterAnswer() {
    if (!cowriterAnswer.trim()) return;
    await navigator.clipboard.writeText(cowriterAnswer);
    setCowriterStatus("Answer copied.");
  }

  async function handleWardAdd(event: FormEvent) {
    event.preventDefault();
    if (!project || tauriState !== "awake") {
      setWardState("failed");
      setWardStatus("Wards need local project storage.");
      return;
    }
    setWardState("working");
    try {
      const nextWards = await addWard(project.projectPath, wardInput, wardSeverity);
      setWards(nextWards);
      setWardInput("");
      setWardState("success");
      setWardStatus(`${nextWards.length} ward phrases ready.`);
    } catch (error) {
      setWardState("failed");
      setWardStatus(describeError(error));
    }
  }

  async function handleWardRemove(id: string) {
    if (!project || tauriState !== "awake") return;
    setWardState("working");
    try {
      const nextWards = await removeWard(project.projectPath, id);
      setWards(nextWards);
      setWardState("success");
      setWardStatus(`${nextWards.length} ward phrases ready.`);
    } catch (error) {
      setWardState("failed");
      setWardStatus(describeError(error));
    }
  }

  function handleWardPresetSelect(value: string) {
    setWardInput(value);
    setWardSeverity("warn");
    updateWorkspacePrefs((current) => ({
      ...current,
      openToolSections: Array.from(new Set([...current.openToolSections, "wards"])),
    }));
    setWardStatus(`Selected "${value}" as a banned-word example. Add it in Wards after onboarding if you want it custom.`);
  }

  async function handleExportItem() {
    if (!project || !activeItem || tauriState !== "awake") {
      setExportState("failed");
      setExportStatus("Markdown export needs an active desktop project item.");
      return;
    }
    setExportState("working");
    try {
      const response = await exportItemMarkdown(project.projectPath, activeItem.id);
      setExportState("success");
      setExportStatus(`${response.message} ${compactPath(response.path)}`);
    } catch (error) {
      setExportState("failed");
      setExportStatus(describeError(error));
    }
  }

  async function handleExportProject() {
    if (!project || tauriState !== "awake") {
      setExportState("failed");
      setExportStatus("Project export needs the desktop shell.");
      return;
    }
    setExportState("working");
    try {
      const response = await exportProjectJson(project.projectPath);
      setExportState("success");
      setExportStatus(`${response.message} ${compactPath(response.path)}`);
    } catch (error) {
      setExportState("failed");
      setExportStatus(describeError(error));
    }
  }

  function selectFirstItemFromTree(tree: VaultTreeResponse) {
    const firstItem = flattenVaultItems(tree)[0];
    setActiveItemId(firstItem?.id ?? "");
    if (!firstItem) {
      setEditorTitle("");
      setEditorContent("");
      setLoadedItemId("");
      setSaveState("idle");
    }
  }

  async function handleArchiveItem(itemId = activeItem?.id ?? "") {
    if (!project || tauriState !== "awake" || !itemId) {
      setExportState("failed");
      setExportStatus("Choose a Vault item before using Safe Remove.");
      return;
    }
    const item = vaultFlatItems.find((candidate) => candidate.id === itemId);
    const title = item?.title ?? "this item";
    const confirmed = window.confirm(
      `Archive "${title}"?\n\nArchived items are hidden from the Vault tree and search, but kept in the local database for future restore tooling.`,
    );
    if (!confirmed) return;

    setExportState("working");
    try {
      const tree = await archiveVaultItem(project.projectPath, itemId);
      setVaultTree(tree);
      selectFirstItemFromTree(tree);
      setExportState("success");
      setExportStatus(`Safely removed ${title}. It is hidden from Vault search/tree but retained in the local database.`);
    } catch (error) {
      setExportState("failed");
      setExportStatus(describeError(error));
    }
  }

  async function handleDeleteItem(itemId = activeItem?.id ?? "") {
    if (!project || tauriState !== "awake" || !itemId) {
      setExportState("failed");
      setExportStatus("Choose a Vault item before deleting.");
      return;
    }
    const item = vaultFlatItems.find((candidate) => candidate.id === itemId);
    const title = item?.title ?? "this item";
    const confirmed = window.confirm(
      `Permanently delete "${title}"?\n\nThis removes the item and its search chunks from the local project database. This cannot be undone.`,
    );
    if (!confirmed) return;

    setExportState("working");
    try {
      const tree = await deleteVaultItem(project.projectPath, itemId);
      setVaultTree(tree);
      selectFirstItemFromTree(tree);
      setExportState("success");
      setExportStatus(`Deleted ${title}.`);
    } catch (error) {
      setExportState("failed");
      setExportStatus(describeError(error));
    }
  }

  function advanceOnboarding() {
    const currentIndex = onboardingSteps.indexOf(onboardingStep);
    const nextStep = onboardingSteps[currentIndex + 1];
    if (!nextStep) {
      completeOnboarding();
      return;
    }
    setOnboardingStep(nextStep);
    writeOnboardingState(onboardingScopeRef.current, false, nextStep);
  }

  function completeOnboarding() {
    writeOnboardingState(onboardingScopeRef.current, true, "canvas");
    setShowOnboarding(false);
  }

  function restartOnboarding() {
    setOnboardingStep("welcome");
    setShowOnboarding(true);
    writeOnboardingState(onboardingScopeRef.current, false, "welcome");
  }

  return (
    <main className={shellClassName}>
      <header className="top-bar">
        <div className="brand-lockup" aria-label="Grimoire">
          <BookOpenText size={22} aria-hidden="true" />
          <div>
            <p className="eyebrow">Witch Daddy Labs</p>
            <h1>Grimoire</h1>
          </div>
        </div>

        <div className="status-strip" aria-live="polite">
          <StatusChip
            tone={tauriState === "awake" ? "success" : "neutral"}
            label={
              tauriState === "checking"
                ? "Checking shell"
                : tauriState === "awake"
                  ? "Tauri shell awake"
                  : "Browser preview"
            }
          />
          <StatusChip
            tone={projectError ? "warning" : project ? "success" : "neutral"}
            label={
              project
                ? `${project.name} ready`
                : projectLoading
                  ? "Preparing project"
                  : "Demo data only"
            }
          />
          <StatusChip tone={saveStateTone(saveState)} label={saveStateLabel(saveState)} />
        </div>

        <div className="top-actions">
          <button
            className="icon-button"
            type="button"
            aria-label="Replay onboarding"
            onClick={restartOnboarding}
            title="Replay onboarding"
          >
            <Info size={17} aria-hidden="true" />
          </button>

          <button
            className="icon-button"
            type="button"
            aria-label={workspacePrefs.theme === "ivory" ? "Use dark desk" : "Use ivory desk"}
            aria-pressed={workspacePrefs.theme === "ivory"}
            onClick={toggleTheme}
            title={workspacePrefs.theme === "ivory" ? "Dark Desk" : "Ivory Desk"}
          >
            {workspacePrefs.theme === "ivory" ? (
              <Moon size={17} aria-hidden="true" />
            ) : (
              <SunMedium size={17} aria-hidden="true" />
            )}
          </button>

          <button
            className="button button-primary"
            type="button"
            aria-pressed={focusMode}
            onClick={() => setFocusMode((value) => !value)}
          >
            <Feather size={16} aria-hidden="true" />
            {focusMode ? "Exit Focus" : "Focus Mode"}
          </button>
        </div>
      </header>

      {showProjectPicker ? (
        <section className="project-picker" aria-label="Project picker">
          <div className="project-picker-inner">
            <div className="project-picker-header">
              <BookOpenText size={32} aria-hidden="true" />
              <h2>Welcome to Grimoire</h2>
              <p className="project-picker-subtitle">
                A local-first writing studio with memory for fiction writers.
              </p>
            </div>

            <div className="project-picker-actions">
              <button
                className="button button-primary"
                type="button"
                onClick={handleCreateProject}
              >
                <Plus size={16} aria-hidden="true" />
                Create New Project
              </button>
              <button
                className="button button-secondary"
                type="button"
                onClick={handleOpenExistingProject}
              >
                <Archive size={16} aria-hidden="true" />
                Open Existing Project
              </button>
              <button
                className="button button-secondary"
                type="button"
                onClick={handleLoadDemo}
              >
                <WandSparkles size={16} aria-hidden="true" />
                Load Demo Project
              </button>
            </div>

            {recentProjects.length > 0 && (
              <div className="project-picker-recent">
                <h3>Recent Projects</h3>
                <ul className="recent-list">
                  {recentProjects.map((rp) => (
                    <li key={rp.path}>
                      <button
                        className="recent-project-button"
                        type="button"
                        onClick={() => handleOpenRecentProject(rp.path)}
                      >
                        <FileText size={14} aria-hidden="true" />
                        <div>
                          <span className="recent-name">{rp.name}</span>
                          <span className="recent-path">{compactPath(rp.path)}</span>
                        </div>
                      </button>
                    </li>
                  ))}
                </ul>
              </div>
            )}

            <div className="project-picker-create">
              <form
                onSubmit={(e) => {
                  e.preventDefault();
                  handleCreateProject();
                }}
              >
                <input
                  className="input"
                  type="text"
                  placeholder="Project name"
                  value={projectCreateName}
                  onChange={(e) => setProjectCreateName(e.target.value)}
                />
                <button className="button button-primary" type="submit">
                  Create
                </button>
              </form>
            </div>

            {projectCreateError && (
              <p className="project-picker-error" role="alert">
                <AlertTriangle size={14} aria-hidden="true" />
                {projectCreateError}
              </p>
            )}
          </div>
        </section>
      ) : (

      <section className="workspace" aria-label="Grimoire workspace">
        <VaultPanel
          tree={vaultTree}
          project={project}
          projectError={projectError}
          projectLoading={projectLoading}
          tauriState={tauriState}
          searchQuery={searchQuery}
          searchResults={searchResults}
          searchError={searchError}
          activeItem={activeItem}
          expandedNodeIds={expandedNodeIds}
          leftCollapsed={workspacePrefs.leftCollapsed}
          onSearchChange={setSearchQuery}
          onSearch={handleSearch}
          onSearchClear={() => { setSearchResults([]); setSearchQuery(""); }}
          onToggle={toggleNode}
          onSelectItem={setActiveItemId}
          onArchiveItem={handleArchiveItem}
          onDeleteItem={() => handleDeleteItem()}
          onExpandLeft={() => setSideCollapsed("left", false)}
          onCollapseLeft={() => setSideCollapsed("left", true)}
          compactPath={compactPath}
        />

        <CanvasPanel
          activeItem={activeItem}
          editorTitle={editorTitle}
          editorContent={editorContent}
          editorWordCount={editorWordCount}
          saveState={saveState}
          saveError={saveError}
          exportState={exportState}
          exportStatus={exportStatus}
          onTitleChange={setEditorTitle}
          onContentChange={setEditorContent}
          onExportItem={handleExportItem}
          onExportProject={handleExportProject}
          onArchiveItem={() => handleArchiveItem(activeItemId)}
          onDeleteItem={() => handleDeleteItem()}
        />

        <CoWriterPanel
          rightCollapsed={workspacePrefs.rightCollapsed}
          activeProvider={activeProvider}
          selectedModel={selectedModel}
          providerLabels={providerLabels}
          providerModels={providerModels}
          activeProviderSettings={activeProviderSettings}
          activeProviderIsCloud={activeProviderIsCloud}
          openToolSectionSet={openToolSectionSet}
          importTitle={importTitle}
          importBody={importBody}
          importState={importState}
          importStatus={importStatus}
          importProgress={importProgress}
          engineState={engineState}
          engineStatus={engineStatus}
          engineError={engineError}
          modelDraft={modelDraft}
          modelOptions={modelOptions}
          apiKeyDraft={apiKeyDraft}
          baseUrlDraft={baseUrlDraft}
          cowriterPrompt={cowriterPrompt}
          cowriterState={cowriterState}
          cowriterStatus={cowriterStatus}
          cowriterAnswer={cowriterAnswer}
          cowriterError={cowriterError}
          retrievalResults={retrievalResults}
          answerWardHits={answerWardHits}
          wards={wards}
          wardInput={wardInput}
          wardSeverity={wardSeverity}
          wardState={wardState}
          wardStatus={wardStatus}
          searchResults={searchResults}
          onToggleToolSection={toggleToolSection}
          onImportTitleChange={setImportTitle}
          onImportBodyChange={setImportBody}
          onPasteImport={handlePasteImport}
          onFileImport={handleFileImport}
          onRefreshEngine={refreshEngine}
          onProviderTest={handleProviderTest}
          onProviderSelection={handleProviderSelection}
          onModelDraftChange={setModelDraft}
          onApiKeyDraftChange={setApiKeyDraft}
          onBaseUrlDraftChange={setBaseUrlDraft}
          onApiKeySave={handleApiKeySave}
          onApiKeyDelete={handleApiKeyDelete}
          onEngineSettingsSave={handleEngineSettingsSave}
          onCowriterPromptChange={setCowriterPrompt}
          onRunCowriter={() => runCowriter()}
          onInsertAnswer={insertCowriterAnswer}
          onCopyAnswer={copyCowriterAnswer}
          onDiscardAnswer={() => { setCowriterAnswer(""); setAnswerWardHits([]); }}
          onRewriteClean={() => runCowriter("Rewrite cleanly and avoid every warded phrase.")}
          onWardInputChange={setWardInput}
          onWardSeverityChange={setWardSeverity}
          onWardAdd={handleWardAdd}
          onWardRemove={handleWardRemove}
          onSelectItem={setActiveItemId}
          onExpandRight={() => setSideCollapsed("right", false)}
          onCollapseRight={() => setSideCollapsed("right", true)}
        />
      </section>
      )}

      {showOnboarding ? (
        <OnboardingOverlay
          step={onboardingStep}
          projectReady={Boolean(project)}
          projectName={project?.name ?? "Demo project"}
          importTitle={importTitle}
          importBody={importBody}
          importState={importState}
          importStatus={importStatus}
          activeProvider={activeProvider}
          engineStatus={engineStatus}
          onImportTitleChange={setImportTitle}
          onImportBodyChange={setImportBody}
          onImportFiles={handleFileImport}
          onImportPaste={performPasteImport}
          onSelectProvider={handleProviderSelection}
          onWardPresetSelect={handleWardPresetSelect}
          onContinue={() => advanceOnboarding()}
          onSkip={
            optionalOnboardingSteps.has(onboardingStep)
              ? () => advanceOnboarding()
              : undefined
          }
          onDone={completeOnboarding}
        />
      ) : null}

      {disclosureProvider ? (
        <CloudDisclosureDialog
          provider={disclosureProvider}
          copy={providerSettings?.cloudDisclosureCopy ?? FALLBACK_CLOUD_DISCLOSURE}
          onAccept={() => acceptProviderDisclosure(disclosureProvider)}
          onCancel={() => {
            setDisclosureProvider(null);
            setPendingProvider(null);
          }}
        />
      ) : null}
    </main>
  );
}

function fallbackDetail(item: VaultItemNode): VaultItemDetail {
  return {
    id: item.id,
    title: item.title,
    itemType: item.itemType,
    content: item.content ?? "",
    plainText: item.content ?? "",
    wordCount: item.wordCount,
    path: item.path,
    updatedAt: "",
  };
}

function browserSearch(items: VaultItemNode[], query: string): SearchChunkResult[] {
  const lowerQuery = query.toLowerCase();
  return items
    .filter((item) => `${item.title} ${item.content ?? ""} ${item.path}`.toLowerCase().includes(lowerQuery))
    .slice(0, 8)
    .map((item, index) => ({
      chunkId: `${item.id}-browser-${index}`,
      itemId: item.id,
      title: item.title,
      itemType: item.itemType,
      vaultPath: item.path,
      snippet: item.content?.slice(0, 180) ?? "",
      score: 1,
      confidence: "low",
    }));
}

function buildGroundedContext(
  results: SearchChunkResult[],
  activeItem: VaultItemNode | null,
  editorContent: string,
) {
  const sources = results
    .slice(0, 5)
    .map((result, index) => `[${index + 1}] ${result.vaultPath}\n${result.snippet}`)
    .join("\n\n");
  const active = activeItem
    ? `Active Canvas: ${activeItem.path}\n${editorContent.slice(0, 1600)}`
    : "No active Canvas item.";
  return [
    "Use only the local context below. If context is thin, say so plainly.",
    active,
    sources ? `Retrieved Vault sources:\n${sources}` : "Retrieved Vault sources: none.",
    "Answer with citations like [1] when using retrieved sources. Do not insert text into the Canvas.",
  ].join("\n\n");
}

function retrievalLabels(state: AsyncState, status: string) {
  if (state === "working") {
    return [status, "Reading canon traces", "Checking slop wards", "Composing grounded answer"];
  }
  if (state === "success") {
    return [status, "Citations ready", "Insertion requires user action"];
  }
  return ["Consulting the Vault", "Reading canon traces", "Checking slop wards", "Composing grounded answer"];
}

function providerReady(
  provider: AiProviderKind,
  settings: AiProviderSettings | null,
  models: AiProviderModelsResponse | null,
) {
  if (provider === "ollama") {
    return Boolean(models?.reachable && (models.selectedModel || models.models.length > 0));
  }
  return Boolean(settings?.apiKeyPresent && settings.disclosureAcceptedAt);
}

function countWords(text: string) {
  return text.trim().split(/\s+/).filter(Boolean).length;
}

function prepareImportContent(content: string) {
  const wordCount = countWords(content);
  if (wordCount <= IMPORT_WORD_LIMIT) {
    return { content, wordCount, originalWordCount: wordCount, truncated: false };
  }

  let seenWords = 0;
  let endIndex = content.length;
  for (const match of content.matchAll(/\S+/g)) {
    seenWords += 1;
    if (seenWords === IMPORT_WORD_LIMIT) {
      endIndex = (match.index ?? 0) + match[0].length;
      break;
    }
  }

  return {
    content: content.slice(0, endIndex),
    wordCount: IMPORT_WORD_LIMIT,
    originalWordCount: wordCount,
    truncated: true,
  };
}

const cowriterStopWords = new Set([
  "about",
  "after",
  "again",
  "also",
  "because",
  "before",
  "could",
  "does",
  "from",
  "have",
  "into",
  "know",
  "like",
  "please",
  "should",
  "tell",
  "that",
  "their",
  "there",
  "this",
  "what",
  "when",
  "where",
  "which",
  "with",
  "would",
]);

function cowriterSearchQueries(prompt: string) {
  const keywords = Array.from(
    new Set(
      (prompt.toLowerCase().match(/[a-z0-9][a-z0-9'-]{2,}/g) ?? []).filter(
        (word) => !cowriterStopWords.has(word),
      ),
    ),
  );
  const focusedQuery = keywords.slice(0, 4).join(" ");
  return Array.from(
    new Set([prompt.trim(), focusedQuery, ...keywords.slice(0, 6)].filter(Boolean)),
  );
}


function safeProviderError(message: string) {
  const lowerMessage = message.toLowerCase();
  if (lowerMessage.includes("401") || lowerMessage.includes("unauthorized")) {
    return "Provider rejected the API key. Check that the key is valid and active.";
  }
  if (lowerMessage.includes("403") || lowerMessage.includes("permission")) {
    return "Provider rejected the request permissions. Check account access, billing, or model availability.";
  }
  if (lowerMessage.includes("404") || lowerMessage.includes("not found")) {
    return "Provider could not find that model or endpoint. Try another model ID.";
  }
  if (lowerMessage.includes("429") || lowerMessage.includes("quota") || lowerMessage.includes("rate")) {
    return "Provider rate limit or quota was reached. Check billing/quota or try again later.";
  }
  return message;
}

function saveFailureCopy(error: unknown) {
  return `Autosave could not write this change. Your text is still visible here, but it has not been safely saved yet. ${describeError(error)}`;
}

function readWorkspacePrefs(): WorkspacePrefs {
  try {
    const raw = window.localStorage.getItem(WORKSPACE_PREFS_KEY);
    if (!raw) {
      return defaultWorkspacePrefs();
    }
    const parsed = JSON.parse(raw) as Partial<WorkspacePrefs>;
    return {
      leftCollapsed: Boolean(parsed.leftCollapsed),
      rightCollapsed: Boolean(parsed.rightCollapsed),
      openToolSections: sanitizeToolSections(parsed.openToolSections),
      theme: parsed.theme === "ivory" ? "ivory" : "dark",
    };
  } catch {
    return defaultWorkspacePrefs();
  }
}

function writeWorkspacePrefs(prefs: WorkspacePrefs) {
  try {
    window.localStorage.setItem(WORKSPACE_PREFS_KEY, JSON.stringify(prefs));
  } catch {
    // Local storage can be unavailable in hardened browser contexts.
  }
}

function defaultWorkspacePrefs(): WorkspacePrefs {
  return {
    leftCollapsed: false,
    rightCollapsed: false,
    openToolSections: defaultOpenToolSections,
    theme: "dark",
  };
}

function sanitizeToolSections(value: unknown): ToolSectionId[] {
  if (!Array.isArray(value)) {
    return defaultOpenToolSections;
  }
  const validSections: ToolSectionId[] = ["feed", "retrieval", "engine", "cowriter", "wards", "about"];
  const sections = value.filter((section): section is ToolSectionId =>
    validSections.includes(section as ToolSectionId),
  );
  return sections.length ? sections : defaultOpenToolSections;
}

function onboardingScopeForProject(projectPath: string) {
  return `project:${projectPath.trim().toLowerCase()}`;
}

function readOnboardingStore(): OnboardingStore {
  try {
    const raw = window.localStorage.getItem(ONBOARDING_STORAGE_KEY);
    if (!raw) return {};
    const parsed = JSON.parse(raw) as unknown;
    if (!parsed || typeof parsed !== "object") return {};
    return parsed as OnboardingStore;
  } catch {
    return {};
  }
}

function readOnboardingState(scope: string): OnboardingState {
  const store = readOnboardingStore();
  const value = store[scope];
  if (!value) {
    return { complete: false, step: "welcome" };
  }
  const safeStep = onboardingSteps.includes(value.step) ? value.step : "welcome";
  return {
    complete: Boolean(value.complete),
    step: safeStep,
  };
}

function writeOnboardingState(scope: string, complete: boolean, step: OnboardingStep) {
  try {
    const store = readOnboardingStore();
    store[scope] = { complete, step };
    window.localStorage.setItem(ONBOARDING_STORAGE_KEY, JSON.stringify(store));
  } catch {
    // Local storage can be unavailable in hardened browser contexts.
  }
}
