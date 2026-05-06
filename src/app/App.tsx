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
  archivePalaceItem,
  deletePalaceItem,
  exportItemMarkdown,
  exportProjectJson,
  fallbackPalaceTree,
  flattenPalaceItems,
  getPalaceItem,
  importText,
  listWards,
  loadPalaceTree,
  removeWard,
  scanWards,
  searchChunks,
  updatePalaceItem,
  type BannedWord,
  type PalaceDrawerNode,
  type PalaceHallNode,
  type PalaceItemDetail,
  type PalaceItemNode,
  type PalaceRoomNode,
  type PalaceTreeResponse,
  type PalaceWingNode,
  type SearchChunkResult,
  type WardScanHit,
  type WardSeverity,
} from "./palace";
import { compactPath, createDemoProject, describeError, type ProjectMetadata } from "./project";

type TauriState = "checking" | "awake" | "browser";
type SaveState = "idle" | "editing" | "saving" | "saved" | "failed" | "preview";
type AsyncState = "idle" | "working" | "success" | "failed";
type OnboardingStep = "welcome" | "palace" | "feed" | "engine" | "wards" | "canvas";
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
  "Cloud providers may receive your prompt, relevant Palace excerpts, and active Canvas context.";

const onboardingSteps: OnboardingStep[] = [
  "welcome",
  "palace",
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
  palace: {
    title: "The Palace",
    body: "Your project is arranged as Wings, Halls, Rooms, Drawers, and editable writing items.",
  },
  feed: {
    title: "Feed the Palace",
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
  const [palaceTree, setPalaceTree] = useState<PalaceTreeResponse>(fallbackPalaceTree);
  const [activeItemId, setActiveItemId] = useState(
    flattenPalaceItems(fallbackPalaceTree)[0]?.id ?? "",
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
  const [cowriterStatus, setCowriterStatus] = useState("Ask for help grounded in local Palace search.");
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

  const palaceFlatItems = useMemo(() => flattenPalaceItems(palaceTree), [palaceTree]);
  const activeItem = useMemo<PalaceItemNode | null>(
    () => palaceFlatItems.find((item) => item.id === activeItemId) ?? palaceFlatItems[0] ?? null,
    [activeItemId, palaceFlatItems],
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
      const tree = await loadPalaceTree(project.projectPath);
      setPalaceTree(tree);
      if (selectItemId) {
        setActiveItemId(selectItemId);
      } else if (!flattenPalaceItems(tree).some((item) => item.id === activeItemId)) {
        const firstItem = flattenPalaceItems(tree)[0];
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

  function loadItemIntoEditor(item: PalaceItemDetail) {
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

        try {
          const metadata = await createDemoProject();
          if (cancelled) return;
          setProject(metadata);
          setProjectError(null);

          try {
            const tree = await loadPalaceTree(metadata.projectPath);
            if (cancelled) return;
            setPalaceTree(tree);
            setTreeError(null);
            const firstItem = flattenPalaceItems(tree)[0];
            if (firstItem) setActiveItemId(firstItem.id);
          } catch (error) {
            if (cancelled) return;
            setTreeError(describeError(error));
          }
        } catch (error) {
          if (cancelled) return;
          setProjectError(describeError(error));
        }
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

  useEffect(() => {
    if (!activeItem) return;
    const selectedItem = activeItem;
    const currentProject = project;
    let cancelled = false;

    async function loadItem() {
      try {
        const item =
          currentProject && tauriState === "awake"
            ? await getPalaceItem(currentProject.projectPath, selectedItem.id)
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
        const saved = await updatePalaceItem(
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
        setSearchResults(browserSearch(palaceFlatItems, query));
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
      let lastItem: PalaceItemDetail | null = null;
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
        "Provider connectivity test. Do not use Palace or Canvas content.",
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
      setCowriterError("Accept the cloud model disclosure before sending Palace context.");
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
    setCowriterStatus("Consulting the Palace");

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
      const detail = await getPalaceItem(project.projectPath, activeItem.id);
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

  function selectFirstItemFromTree(tree: PalaceTreeResponse) {
    const firstItem = flattenPalaceItems(tree)[0];
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
      setExportStatus("Choose a Palace item before using Safe Remove.");
      return;
    }
    const item = palaceFlatItems.find((candidate) => candidate.id === itemId);
    const title = item?.title ?? "this item";
    const confirmed = window.confirm(
      `Archive "${title}"?\n\nArchived items are hidden from the Palace tree and search, but kept in the local database for future restore tooling.`,
    );
    if (!confirmed) return;

    setExportState("working");
    try {
      const tree = await archivePalaceItem(project.projectPath, itemId);
      setPalaceTree(tree);
      selectFirstItemFromTree(tree);
      setExportState("success");
      setExportStatus(`Safely removed ${title}. It is hidden from Palace search/tree but retained in the local database.`);
    } catch (error) {
      setExportState("failed");
      setExportStatus(describeError(error));
    }
  }

  async function handleDeleteItem(itemId = activeItem?.id ?? "") {
    if (!project || tauriState !== "awake" || !itemId) {
      setExportState("failed");
      setExportStatus("Choose a Palace item before deleting.");
      return;
    }
    const item = palaceFlatItems.find((candidate) => candidate.id === itemId);
    const title = item?.title ?? "this item";
    const confirmed = window.confirm(
      `Permanently delete "${title}"?\n\nThis removes the item and its search chunks from the local project database. This cannot be undone.`,
    );
    if (!confirmed) return;

    setExportState("working");
    try {
      const tree = await deletePalaceItem(project.projectPath, itemId);
      setPalaceTree(tree);
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

      <section className="workspace" aria-label="Grimoire workspace">
        <aside className="panel palace-panel" aria-label="The Palace">
          {workspacePrefs.leftCollapsed ? (
            <CollapsedRail
              icon={<Database size={18} aria-hidden="true" />}
              label="Open Palace"
              onExpand={() => setSideCollapsed("left", false)}
              side="left"
            />
          ) : (
            <>
              <PanelHeader
                action={
                  <button
                    className="icon-button panel-collapse-button"
                    type="button"
                    aria-label="Collapse Palace"
                    onClick={() => setSideCollapsed("left", true)}
                    title="Collapse Palace"
                  >
                    <ChevronLeft size={16} aria-hidden="true" />
                  </button>
                }
                icon={<Database size={17} aria-hidden="true" />}
                title="The Palace"
                subtitle={project ? "SQLite project ready" : "Wings / Halls / Rooms / Drawers"}
              />

              <div className="panel-scroll palace-scroll">
                <div className={projectError ? "project-card warning" : "project-card"}>
                  <span>
                    {project
                      ? "Local project"
                      : projectLoading
                        ? "Project storage"
                        : tauriState === "browser"
                          ? "Browser preview"
                          : "Project storage"}
                  </span>
                  <strong>{project ? project.name : projectLoading ? "Preparing SQLite" : "Static demo"}</strong>
                  <small>
                    {project
                      ? `${compactPath(project.projectPath)} - ${palaceTree.itemCount} items`
                      : treeError ?? projectError ?? "The desktop shell will create a .grimoire folder here."}
                  </small>
                </div>

                <form className="palace-search" onSubmit={handleSearch}>
                  <Search size={15} aria-hidden="true" />
                  <input
                    type="search"
                    placeholder="Search Palace"
                    aria-label="Search Palace"
                    value={searchQuery}
                    onChange={(event) => setSearchQuery(event.target.value)}
                  />
                </form>

                {searchQuery && searchState === "success" ? (
                  <div className="search-summary">
                    <strong>{searchResults.length ? `${searchResults.length} local matches` : "No local matches"}</strong>
                    <button className="text-button" type="button" onClick={() => setSearchResults([])}>
                      Clear
                    </button>
                  </div>
                ) : null}
                {searchError ? <p className="inline-error">{searchError}</p> : null}

                <PalaceTree
                  tree={palaceTree}
                  activeItemId={activeItem?.id ?? ""}
                  expandedNodeIds={expandedNodeIds}
                  onArchiveItem={handleArchiveItem}
                  onToggle={toggleNode}
                  onSelectItem={setActiveItemId}
                />
              </div>
            </>
          )}
        </aside>

        <article className="canvas-panel" aria-label="Canvas">
          <div className="canvas-toolbar">
            <div>
              <p className="eyebrow">The Canvas</p>
              <p className="path-label">{activeItem?.path ?? "The Palace"}</p>
            </div>
            <div className="canvas-stats" aria-live="polite">
              <span>{editorWordCount} words</span>
              <span className={`save-pill ${saveState}`}>{saveStateLabel(saveState)}</span>
            </div>
          </div>

          <label className="sr-only" htmlFor="canvas-title">
            Canvas title
          </label>
          <input
            id="canvas-title"
            className="title-input"
            value={editorTitle}
            onChange={(event) => {
              setEditorTitle(event.target.value);
              markEditorChanged();
            }}
            spellCheck
          />

          <label className="sr-only" htmlFor="canvas-editor">
            Canvas editor
          </label>
          <textarea
            id="canvas-editor"
            className="editor-surface editor-textarea"
            value={editorContent}
            onChange={(event) => {
              setEditorContent(event.target.value);
              markEditorChanged();
            }}
            placeholder="Create your first Wing or import writing to begin."
            spellCheck
          />

          {saveError ? <p className="inline-error">{saveError}</p> : null}

          <div className="canvas-actions">
            <button className="button button-secondary" type="button" onClick={handleExportItem}>
              <FileText size={16} aria-hidden="true" />
              Export Markdown
            </button>
            <button className="button button-secondary" type="button" onClick={handleExportProject}>
              <Download size={16} aria-hidden="true" />
              Export Project
            </button>
            <button className="button button-secondary" type="button" onClick={() => handleArchiveItem()}>
              <Archive size={16} aria-hidden="true" />
              Safe Remove
            </button>
            <button className="button button-danger" type="button" onClick={() => handleDeleteItem()}>
              <Trash2 size={16} aria-hidden="true" />
              Delete Item
            </button>
            <span className={`operation-status ${exportState}`}>{exportStatus}</span>
          </div>
        </article>

        <aside className="panel cowriter-panel" aria-label="The Co-Writer">
          {workspacePrefs.rightCollapsed ? (
            <CollapsedRail
              icon={<BrainCircuit size={18} aria-hidden="true" />}
              label="Open Co-Writer"
              onExpand={() => setSideCollapsed("right", false)}
              side="right"
            />
          ) : (
            <>
              <PanelHeader
                action={
                  <button
                    className="icon-button panel-collapse-button"
                    type="button"
                    aria-label="Collapse Co-Writer"
                    onClick={() => setSideCollapsed("right", true)}
                    title="Collapse Co-Writer"
                  >
                    <ChevronRight size={16} aria-hidden="true" />
                  </button>
                }
                icon={<BrainCircuit size={17} aria-hidden="true" />}
                title="The Co-Writer"
                subtitle={`${providerLabels[activeProvider]}${selectedModel ? ` / ${selectedModel}` : " / choose model"}`}
              />

              <div className="panel-scroll tools-scroll">

          <ToolAccordion
            id="feed"
            icon={<Upload size={15} />}
            open={openToolSectionSet.has("feed")}
            title="Feed"
            onToggle={toggleToolSection}
          >
            <form className="tool-form" onSubmit={handlePasteImport}>
              <input
                className="compact-input"
                value={importTitle}
                onChange={(event) => setImportTitle(event.target.value)}
                placeholder="Import title"
              />
              <textarea
                className="compact-textarea"
                value={importBody}
                onChange={(event) => setImportBody(event.target.value)}
                placeholder={`Paste text or Markdown, up to ${IMPORT_WORD_LIMIT.toLocaleString()} words`}
              />
              <p className="tool-hint">
                Import multiple `.md`, `.markdown`, or `.txt` files. Each file is capped at{" "}
                {IMPORT_WORD_LIMIT.toLocaleString()} words; add more chunks later if needed.
              </p>
              <div className="inline-actions">
                <button className="button button-primary" type="submit" disabled={importState === "working"}>
                  {importState === "working" ? <Loader2 size={16} /> : <Clipboard size={16} />}
                  Import Paste
                </button>
                <label className="file-button">
                  <FileText size={16} aria-hidden="true" />
                  Files
                  <input
                    type="file"
                    accept=".txt,.md,.markdown,text/plain,text/markdown"
                    multiple
                    onChange={(event) => handleFileImport(event.currentTarget.files)}
                  />
                </label>
              </div>
            </form>
            <ProgressList labels={importProgress} />
            <p className={`operation-status ${importState}`}>{importStatus}</p>
          </ToolAccordion>

          <ToolAccordion
            id="retrieval"
            icon={<Search size={15} />}
            open={openToolSectionSet.has("retrieval")}
            title="Retrieval"
            onToggle={toggleToolSection}
          >
            <div className="retrieval-card" role="status" aria-live="polite">
              {retrievalLabels(cowriterState, cowriterStatus).map((step, index) => (
                <p key={`${step}-${index}`} className={index === 0 ? "active-step" : undefined}>
                  <Sparkles size={14} aria-hidden="true" />
                  {step}
                </p>
              ))}
            </div>
            <ResultList results={searchResults.length ? searchResults : retrievalResults} onSelect={setActiveItemId} />
          </ToolAccordion>

          <ToolAccordion
            id="engine"
            icon={<WandSparkles size={15} />}
            open={openToolSectionSet.has("engine")}
            title="Engine"
            onToggle={toggleToolSection}
          >
            <div className="engine-row">
              <button className="button button-secondary" type="button" onClick={refreshEngine}>
                {engineState === "working" ? <Loader2 size={16} /> : <BrainCircuit size={16} />}
                Refresh Models
              </button>
              <button className="button button-secondary" type="button" onClick={handleProviderTest}>
                <Sparkles size={16} />
                Test Provider
              </button>
              <span className={providerReady(activeProvider, activeProviderSettings, providerModels) ? "engine-dot online" : "engine-dot"} />
            </div>
            <div className="provider-grid" role="radiogroup" aria-label="AI provider">
              {AI_PROVIDERS.map((provider) => (
                <button
                  key={provider}
                  className={provider === activeProvider ? "provider-button active" : "provider-button"}
                  type="button"
                  role="radio"
                  aria-checked={provider === activeProvider}
                  onClick={() => handleProviderSelection(provider)}
                >
                  <span>{providerLabels[provider]}</span>
                  <small>{cloudProvider(provider) ? "BYOK cloud" : "Local"}</small>
                </button>
              ))}
            </div>
            <p className={`operation-status ${engineState}`}>{engineStatus}</p>
            {engineError ? <p className="inline-error compact-error">{engineError}</p> : null}
            {modelOptions.length ? (
              <select
                className="compact-input"
                value={modelDraft}
                onChange={(event) => setModelDraft(event.target.value)}
              >
                <option value="" disabled>
                  Choose model
                </option>
                {modelOptions.map((model) => (
                  <option key={model} value={model}>
                    {model}
                  </option>
                ))}
              </select>
            ) : null}
            {activeProviderIsCloud ? (
              <div className="cloud-settings">
                <div className="key-status">
                  <span className={activeProviderSettings?.apiKeyPresent ? "engine-dot online" : "engine-dot"} />
                  {activeProviderSettings?.apiKeyPresent ? "API key saved" : "No API key saved"}
                </div>
                <p className="tool-hint">
                  macOS may ask for Keychain permission because Grimoire stores API keys there instead of inside your
                  project files. You should only see this when saving, deleting, or using a saved key.
                </p>
                <form className="tool-form" onSubmit={handleApiKeySave}>
                  <input
                    className="compact-input"
                    type="password"
                    autoComplete="off"
                    value={apiKeyDraft}
                    onChange={(event) => setApiKeyDraft(event.target.value)}
                    placeholder={`Paste ${providerLabels[activeProvider]} API key`}
                  />
                  <div className="inline-actions">
                    <button className="button button-primary" type="submit" disabled={!apiKeyDraft.trim()}>
                      <ShieldCheck size={16} />
                      Save Key
                    </button>
                    <button
                      className="button button-secondary"
                      type="button"
                      disabled={!activeProviderSettings?.apiKeyPresent}
                      onClick={handleApiKeyDelete}
                    >
                      <Trash2 size={16} />
                      Delete Key
                    </button>
                  </div>
                </form>
                {activeProvider === "openAiCompatible" ? (
                  <input
                    className="compact-input"
                    value={baseUrlDraft}
                    onChange={(event) => setBaseUrlDraft(event.target.value)}
                    placeholder="Base URL, e.g. https://api.example.com"
                  />
                ) : null}
              </div>
            ) : null}
            <form className="tool-form" onSubmit={handleEngineSettingsSave}>
              <input
                className="compact-input"
                value={modelDraft}
                onChange={(event) => setModelDraft(event.target.value)}
                placeholder={activeProvider === "ollama" ? "Choose a detected local model" : "Model ID"}
              />
              <button className="button button-secondary full-width" type="submit">
                <Check size={16} />
                Save Engine Settings
              </button>
            </form>
          </ToolAccordion>

          <ToolAccordion
            id="cowriter"
            icon={<BrainCircuit size={15} />}
            open={openToolSectionSet.has("cowriter")}
            title="Co-Writer"
            onToggle={toggleToolSection}
          >
            <textarea
              className="compact-textarea"
              value={cowriterPrompt}
              onChange={(event) => setCowriterPrompt(event.target.value)}
              placeholder="Ask a grounded question across the Palace"
            />
            <p className="tool-hint">
              Searches the whole Palace first, then uses the active Canvas as extra context when it helps.
            </p>
            <button className="button button-primary full-width" type="button" onClick={() => runCowriter()}>
              {cowriterState === "working" ? <Loader2 size={16} /> : <Sparkles size={16} />}
              Ask Co-Writer
            </button>
            {cowriterError ? <p className="inline-error">{cowriterError}</p> : null}
            {cowriterAnswer ? (
              <div className="answer-card">
                <p>{cowriterAnswer}</p>
                <CitationList results={retrievalResults} />
                <WardWarnings hits={answerWardHits} />
                <div className="inline-actions">
                  <button className="button button-secondary" type="button" onClick={insertCowriterAnswer}>
                    <Check size={16} />
                    {answerWardHits.length ? "Insert Anyway" : "Insert"}
                  </button>
                  <button className="icon-button" type="button" aria-label="Copy answer" onClick={copyCowriterAnswer}>
                    <Copy size={16} />
                  </button>
                  <button
                    className="icon-button"
                    type="button"
                    aria-label="Rewrite clean"
                    onClick={() => runCowriter("Rewrite cleanly and avoid every warded phrase.")}
                  >
                    <WandSparkles size={16} />
                  </button>
                  <button
                    className="icon-button"
                    type="button"
                    aria-label="Discard answer"
                    onClick={() => {
                      setCowriterAnswer("");
                      setAnswerWardHits([]);
                    }}
                  >
                    <X size={16} />
                  </button>
                </div>
              </div>
            ) : null}
          </ToolAccordion>

          <ToolAccordion
            id="wards"
            icon={<ShieldCheck size={15} />}
            open={openToolSectionSet.has("wards")}
            title="Wards"
            onToggle={toggleToolSection}
          >
            <form className="ward-form" onSubmit={handleWardAdd}>
              <input
                className="compact-input"
                value={wardInput}
                onChange={(event) => setWardInput(event.target.value)}
                placeholder="Phrase to warn on"
              />
              <select
                className="compact-input severity-select"
                value={wardSeverity}
                onChange={(event) => setWardSeverity(event.target.value as WardSeverity)}
              >
                <option value="warn">Warn</option>
                <option value="block">Block</option>
              </select>
              <button className="icon-button" type="submit" aria-label="Add ward phrase">
                <Plus size={16} />
              </button>
            </form>
            <div className="ward-list">
              {wards.slice(0, 10).map((ward) => (
                <span key={ward.id} className="ward-token">
                  {ward.value}
                  <small>{ward.severity}</small>
                  {!ward.isDefault ? (
                    <button type="button" aria-label={`Remove ${ward.value}`} onClick={() => handleWardRemove(ward.id)}>
                      <Trash2 size={12} />
                    </button>
                  ) : null}
                </span>
              ))}
            </div>
            <p className={`operation-status ${wardState}`}>{wardStatus}</p>
          </ToolAccordion>

          <ToolAccordion
            id="about"
            icon={<Info size={15} />}
            open={openToolSectionSet.has("about")}
            title="About"
            onToggle={toggleToolSection}
          >
            <p>
              Grimoire is an independent Witch Daddy Labs project. The Palace memory
              model is inspired by the MIT-licensed MemPalace project; Grimoire is not
              affiliated with MemPalace.
            </p>
          </ToolAccordion>
              </div>
            </>
          )}
        </aside>
      </section>

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

function PalaceTree({
  tree,
  activeItemId,
  expandedNodeIds,
  onArchiveItem,
  onToggle,
  onSelectItem,
}: {
  tree: PalaceTreeResponse;
  activeItemId: string;
  expandedNodeIds: Set<string>;
  onArchiveItem: (itemId: string) => void;
  onToggle: (nodeId: string) => void;
  onSelectItem: (itemId: string) => void;
}) {
  if (tree.itemCount === 0) {
    return (
      <div className="palace-empty">
        <strong>The Palace is quiet</strong>
        <span>Create your first Wing or import writing to begin.</span>
      </div>
    );
  }

  return (
    <nav className="palace-tree" aria-label="Palace memory">
      {tree.wings.map((wing) => (
        <WingBranch
          key={wing.id}
          wing={wing}
          activeItemId={activeItemId}
          expandedNodeIds={expandedNodeIds}
          onArchiveItem={onArchiveItem}
          onToggle={onToggle}
          onSelectItem={onSelectItem}
        />
      ))}
    </nav>
  );
}

function CollapsedRail({
  icon,
  label,
  onExpand,
  side,
}: {
  icon: ReactNode;
  label: string;
  onExpand: () => void;
  side: "left" | "right";
}) {
  return (
    <button className={`collapsed-rail ${side}`} type="button" onClick={onExpand} aria-label={label} title={label}>
      {icon}
      {side === "left" ? <ChevronRight size={16} aria-hidden="true" /> : <ChevronLeft size={16} aria-hidden="true" />}
    </button>
  );
}

function ToolAccordion({
  id,
  icon,
  open,
  title,
  onToggle,
  children,
}: {
  id: ToolSectionId;
  icon: ReactNode;
  open: boolean;
  title: string;
  onToggle: (id: ToolSectionId) => void;
  children: ReactNode;
}) {
  const titleId = `${id}-title`;
  return (
    <section className={open ? "tool-section open" : "tool-section"} aria-labelledby={titleId}>
      <button
        className="tool-section-toggle"
        type="button"
        aria-expanded={open}
        aria-controls={`${id}-body`}
        onClick={() => onToggle(id)}
      >
        <SectionTitle icon={icon} id={titleId} title={title} />
        {open ? <ChevronDown size={15} aria-hidden="true" /> : <ChevronRight size={15} aria-hidden="true" />}
      </button>
      {open ? (
        <div className="tool-section-body" id={`${id}-body`}>
          {children}
        </div>
      ) : null}
    </section>
  );
}

function WingBranch({
  wing,
  activeItemId,
  expandedNodeIds,
  onArchiveItem,
  onToggle,
  onSelectItem,
}: {
  wing: PalaceWingNode;
  activeItemId: string;
  expandedNodeIds: Set<string>;
  onArchiveItem: (itemId: string) => void;
  onToggle: (nodeId: string) => void;
  onSelectItem: (itemId: string) => void;
}) {
  const expanded = expandedNodeIds.has(wing.id);
  return (
    <TreeBranch
      id={wing.id}
      label={wing.name}
      meta={`Wing / ${countWingItems(wing)} items`}
      level={0}
      expanded={expanded}
      onToggle={onToggle}
    >
      {wing.halls.map((hall) => (
        <HallBranch
          key={hall.id}
          hall={hall}
          activeItemId={activeItemId}
          expandedNodeIds={expandedNodeIds}
          onArchiveItem={onArchiveItem}
          onToggle={onToggle}
          onSelectItem={onSelectItem}
        />
      ))}
    </TreeBranch>
  );
}

function HallBranch({
  hall,
  activeItemId,
  expandedNodeIds,
  onArchiveItem,
  onToggle,
  onSelectItem,
}: {
  hall: PalaceHallNode;
  activeItemId: string;
  expandedNodeIds: Set<string>;
  onArchiveItem: (itemId: string) => void;
  onToggle: (nodeId: string) => void;
  onSelectItem: (itemId: string) => void;
}) {
  const expanded = expandedNodeIds.has(hall.id);
  return (
    <TreeBranch
      id={hall.id}
      label={hall.name}
      meta={`Hall / ${countHallItems(hall)} items`}
      level={1}
      expanded={expanded}
      onToggle={onToggle}
    >
      {hall.rooms.map((room) => (
        <RoomBranch
          key={room.id}
          room={room}
          activeItemId={activeItemId}
          expandedNodeIds={expandedNodeIds}
          onArchiveItem={onArchiveItem}
          onToggle={onToggle}
          onSelectItem={onSelectItem}
        />
      ))}
    </TreeBranch>
  );
}

function RoomBranch({
  room,
  activeItemId,
  expandedNodeIds,
  onArchiveItem,
  onToggle,
  onSelectItem,
}: {
  room: PalaceRoomNode;
  activeItemId: string;
  expandedNodeIds: Set<string>;
  onArchiveItem: (itemId: string) => void;
  onToggle: (nodeId: string) => void;
  onSelectItem: (itemId: string) => void;
}) {
  const expanded = expandedNodeIds.has(room.id);
  return (
    <TreeBranch
      id={room.id}
      label={room.name}
      meta={`Room / ${countRoomItems(room)} items`}
      level={2}
      expanded={expanded}
      onToggle={onToggle}
    >
      {room.drawers.map((drawer) => (
        <DrawerBranch
          key={drawer.id}
          drawer={drawer}
          activeItemId={activeItemId}
          expandedNodeIds={expandedNodeIds}
          onArchiveItem={onArchiveItem}
          onToggle={onToggle}
          onSelectItem={onSelectItem}
        />
      ))}
    </TreeBranch>
  );
}

function DrawerBranch({
  drawer,
  activeItemId,
  expandedNodeIds,
  onArchiveItem,
  onToggle,
  onSelectItem,
}: {
  drawer: PalaceDrawerNode;
  activeItemId: string;
  expandedNodeIds: Set<string>;
  onArchiveItem: (itemId: string) => void;
  onToggle: (nodeId: string) => void;
  onSelectItem: (itemId: string) => void;
}) {
  const expanded = expandedNodeIds.has(drawer.id);
  return (
    <TreeBranch
      id={drawer.id}
      label={drawer.name}
      meta={`Drawer / ${drawer.items.length} items`}
      level={3}
      expanded={expanded}
      onToggle={onToggle}
    >
      {drawer.items.map((item) => (
        <button
          key={item.id}
          className={item.id === activeItemId ? "tree-item active" : "tree-item"}
          type="button"
          aria-current={item.id === activeItemId ? "page" : undefined}
          style={treeDepthStyle(4)}
          onClick={() => onSelectItem(item.id)}
          onContextMenu={(event) => {
            event.preventDefault();
            onArchiveItem(item.id);
          }}
          title="Right-click to archive"
        >
          <span>{item.title}</span>
          <small>{item.itemType}</small>
        </button>
      ))}
    </TreeBranch>
  );
}

function TreeBranch({
  id,
  label,
  meta,
  level,
  expanded,
  onToggle,
  children,
}: {
  id: string;
  label: string;
  meta: string;
  level: number;
  expanded: boolean;
  onToggle: (nodeId: string) => void;
  children: ReactNode;
}) {
  return (
    <div className="tree-branch">
      <button
        className="tree-branch-button"
        type="button"
        aria-expanded={expanded}
        style={treeDepthStyle(level)}
        onClick={() => onToggle(id)}
      >
        {expanded ? <ChevronDown size={15} aria-hidden="true" /> : <ChevronRight size={15} aria-hidden="true" />}
        <span>{label}</span>
        <small>{meta}</small>
      </button>
      {expanded ? <div className="tree-branch-children">{children}</div> : null}
    </div>
  );
}

function ResultList({
  results,
  onSelect,
}: {
  results: SearchChunkResult[];
  onSelect: (itemId: string) => void;
}) {
  if (results.length === 0) {
    return <p className="operation-status">No retrieval results yet.</p>;
  }

  return (
    <div className="result-list">
      {results.slice(0, 5).map((result) => (
        <button key={result.chunkId} className="result-item" type="button" onClick={() => onSelect(result.itemId)}>
          <strong>{result.title}</strong>
          <span>{result.palacePath}</span>
          <small>{result.confidence} confidence</small>
        </button>
      ))}
    </div>
  );
}

function CitationList({ results }: { results: SearchChunkResult[] }) {
  if (results.length === 0) return null;

  return (
    <div className="citation-list">
      <strong>Citations</strong>
      {results.slice(0, 3).map((result, index) => (
        <span key={result.chunkId}>
          [{index + 1}] {result.palacePath}
        </span>
      ))}
    </div>
  );
}

function WardWarnings({ hits }: { hits: WardScanHit[] }) {
  if (hits.length === 0) return null;

  return (
    <div className="ward-warning">
      <AlertTriangle size={15} aria-hidden="true" />
      <span>
        Wards found {hits.map((hit) => `${hit.value} (${hit.count})`).join(", ")}. This is a warning, not a perfect style check.
      </span>
    </div>
  );
}

function ProgressList({ labels }: { labels: string[] }) {
  if (labels.length === 0) return null;
  return (
    <div className="progress-list">
      {labels.map((label) => (
        <span key={label}>
          <Check size={12} />
          {label}
        </span>
      ))}
    </div>
  );
}

function OnboardingOverlay({
  step,
  projectReady,
  projectName,
  importTitle,
  importBody,
  importState,
  importStatus,
  activeProvider,
  engineStatus,
  onImportTitleChange,
  onImportBodyChange,
  onImportFiles,
  onImportPaste,
  onSelectProvider,
  onWardPresetSelect,
  onContinue,
  onSkip,
  onDone,
}: {
  step: OnboardingStep;
  projectReady: boolean;
  projectName: string;
  importTitle: string;
  importBody: string;
  importState: AsyncState;
  importStatus: string;
  activeProvider: AiProviderKind;
  engineStatus: string;
  onImportTitleChange: (value: string) => void;
  onImportBodyChange: (value: string) => void;
  onImportFiles: (files: FileList | null) => Promise<void>;
  onImportPaste: () => Promise<void>;
  onSelectProvider: (provider: AiProviderKind) => void;
  onWardPresetSelect: (value: string) => void;
  onContinue: () => void;
  onSkip?: () => void;
  onDone: () => void;
}) {
  const copy = onboardingCopy[step];
  const finalStep = step === "canvas";

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
          {projectReady ? "Palace project ready" : "Preparing local Palace"}
        </div>
        <OnboardingAction
          activeProvider={activeProvider}
          engineStatus={engineStatus}
          importBody={importBody}
          importState={importState}
          importStatus={importStatus}
          importTitle={importTitle}
          projectName={projectName}
          projectReady={projectReady}
          step={step}
          onImportBodyChange={onImportBodyChange}
          onImportFiles={onImportFiles}
          onImportPaste={onImportPaste}
          onImportTitleChange={onImportTitleChange}
          onSelectProvider={onSelectProvider}
          onWardPresetSelect={onWardPresetSelect}
        />
        <div className="inline-actions">
          {onSkip ? (
            <button className="button button-secondary" type="button" onClick={onSkip}>
              Skip
            </button>
          ) : null}
          <button className="button button-primary" type="button" onClick={finalStep ? onDone : onContinue}>
            {finalStep ? "Enter Grimoire" : "Continue"}
          </button>
        </div>
      </section>
    </div>
  );
}

function OnboardingAction({
  activeProvider,
  engineStatus,
  importBody,
  importState,
  importStatus,
  importTitle,
  projectName,
  projectReady,
  step,
  onImportBodyChange,
  onImportFiles,
  onImportPaste,
  onImportTitleChange,
  onSelectProvider,
  onWardPresetSelect,
}: {
  activeProvider: AiProviderKind;
  engineStatus: string;
  importBody: string;
  importState: AsyncState;
  importStatus: string;
  importTitle: string;
  projectName: string;
  projectReady: boolean;
  step: OnboardingStep;
  onImportBodyChange: (value: string) => void;
  onImportFiles: (files: FileList | null) => Promise<void>;
  onImportPaste: () => Promise<void>;
  onImportTitleChange: (value: string) => void;
  onSelectProvider: (provider: AiProviderKind) => void;
  onWardPresetSelect: (value: string) => void;
}) {
  if (step === "palace") {
    return (
      <div className="onboarding-action-card">
        <strong>{projectReady ? projectName : "Preparing project"}</strong>
        <span>{projectReady ? "Local SQLite storage is ready." : "Grimoire is creating local project storage."}</span>
      </div>
    );
  }

  if (step === "feed") {
    return (
      <form
        className="onboarding-action-card"
        onSubmit={async (event) => {
          event.preventDefault();
          await onImportPaste();
        }}
      >
        <input
          className="compact-input"
          value={importTitle}
          onChange={(event) => onImportTitleChange(event.target.value)}
          placeholder="Starter import title"
        />
        <textarea
          className="compact-textarea"
          value={importBody}
          onChange={(event) => onImportBodyChange(event.target.value)}
          placeholder={`Paste writing, or choose files below. ${IMPORT_WORD_LIMIT.toLocaleString()} words per import.`}
        />
        <div className="inline-actions">
          <button className="button button-primary" type="submit" disabled={importState === "working" || !importBody.trim()}>
            {importState === "working" ? <Loader2 size={16} /> : <Clipboard size={16} />}
            Import Paste
          </button>
          <label className="file-button">
            <FileText size={16} aria-hidden="true" />
            Choose Files
            <input
              type="file"
              accept=".txt,.md,.markdown,text/plain,text/markdown"
              multiple
              onChange={(event) => onImportFiles(event.currentTarget.files)}
            />
          </label>
        </div>
        <span className="tool-hint">Markdown and text files are imported into the Palace Feed.</span>
        <span className={`operation-status ${importState}`}>{importStatus}</span>
      </form>
    );
  }

  if (step === "engine") {
    return (
      <div className="onboarding-action-card">
        <div className="provider-grid onboarding-provider-grid" role="radiogroup" aria-label="Choose AI provider">
          {AI_PROVIDERS.map((provider) => (
            <button
              key={provider}
              className={provider === activeProvider ? "provider-button active" : "provider-button"}
              type="button"
              role="radio"
              aria-checked={provider === activeProvider}
              onClick={() => onSelectProvider(provider)}
            >
              <span>{providerLabels[provider]}</span>
              <small>{cloudProvider(provider) ? "BYOK cloud" : "Local"}</small>
            </button>
          ))}
        </div>
        <span className="operation-status">{engineStatus}</span>
      </div>
    );
  }

  if (step === "wards") {
    return (
      <div className="onboarding-action-card">
        <strong>Wards are banned words and banned phrases.</strong>
        <span>
          Grimoire scans Co-Writer output before insertion and warns when these words appear. Choose a starter below or add your own later.
        </span>
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

function CloudDisclosureDialog({
  provider,
  copy,
  onAccept,
  onCancel,
}: {
  provider: AiProviderKind;
  copy: string;
  onAccept: () => void;
  onCancel: () => void;
}) {
  return (
    <div className="onboarding-backdrop" role="dialog" aria-modal="true" aria-labelledby="cloud-disclosure-title">
      <section className="onboarding-panel cloud-disclosure-panel">
        <button className="icon-button dismiss" type="button" aria-label="Cancel cloud provider" onClick={onCancel}>
          <X size={16} />
        </button>
        <p className="eyebrow">Cloud model disclosure</p>
        <h2 id="cloud-disclosure-title">{providerLabels[provider]}</h2>
        <p className="cloud-disclosure-copy">{copy}</p>
        <div className="inline-actions">
          <button className="button button-secondary" type="button" onClick={onCancel}>
            Keep Local
          </button>
          <button className="button button-primary" type="button" onClick={onAccept}>
            <ShieldCheck size={16} aria-hidden="true" />
            Accept
          </button>
        </div>
      </section>
    </div>
  );
}

function SectionTitle({ icon, id, title }: { icon: ReactNode; id: string; title: string }) {
  return (
    <div className="section-title">
      {icon}
      <h4 id={id}>{title}</h4>
    </div>
  );
}

function treeDepthStyle(level: number) {
  return { "--tree-indent": `${level * 15}px` } as CSSProperties;
}

function countWingItems(wing: PalaceWingNode) {
  return wing.halls.reduce((count, hall) => count + countHallItems(hall), 0);
}

function countHallItems(hall: PalaceHallNode) {
  return hall.rooms.reduce((count, room) => count + countRoomItems(room), 0);
}

function countRoomItems(room: PalaceRoomNode) {
  return room.drawers.reduce((count, drawer) => count + drawer.items.length, 0);
}

function PanelHeader({
  icon,
  title,
  subtitle,
  action,
}: {
  icon: ReactNode;
  title: string;
  subtitle: string;
  action?: ReactNode;
}) {
  return (
    <div className="panel-header">
      <div className="panel-heading">
        <div className="panel-icon">{icon}</div>
        <div>
          <h3>{title}</h3>
          <p>{subtitle}</p>
        </div>
      </div>
      {action}
    </div>
  );
}

function StatusChip({
  tone,
  label,
}: {
  tone: "success" | "neutral" | "warning";
  label: string;
}) {
  return <span className={`status-chip ${tone}`}>{label}</span>;
}

function fallbackDetail(item: PalaceItemNode): PalaceItemDetail {
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

function browserSearch(items: PalaceItemNode[], query: string): SearchChunkResult[] {
  const lowerQuery = query.toLowerCase();
  return items
    .filter((item) => `${item.title} ${item.content ?? ""} ${item.path}`.toLowerCase().includes(lowerQuery))
    .slice(0, 8)
    .map((item, index) => ({
      chunkId: `${item.id}-browser-${index}`,
      itemId: item.id,
      title: item.title,
      itemType: item.itemType,
      palacePath: item.path,
      snippet: item.content?.slice(0, 180) ?? "",
      score: 1,
      confidence: "low",
    }));
}

function buildGroundedContext(
  results: SearchChunkResult[],
  activeItem: PalaceItemNode | null,
  editorContent: string,
) {
  const sources = results
    .slice(0, 5)
    .map((result, index) => `[${index + 1}] ${result.palacePath}\n${result.snippet}`)
    .join("\n\n");
  const active = activeItem
    ? `Active Canvas: ${activeItem.path}\n${editorContent.slice(0, 1600)}`
    : "No active Canvas item.";
  return [
    "Use only the local context below. If context is thin, say so plainly.",
    active,
    sources ? `Retrieved Palace sources:\n${sources}` : "Retrieved Palace sources: none.",
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
  return ["Consulting the Palace", "Reading canon traces", "Checking slop wards", "Composing grounded answer"];
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

function saveStateTone(saveState: SaveState): "success" | "neutral" | "warning" {
  if (saveState === "saved") return "success";
  if (saveState === "failed") return "warning";
  return "neutral";
}

function saveStateLabel(saveState: SaveState) {
  switch (saveState) {
    case "editing":
      return "Unsaved edits";
    case "saving":
      return "Saving";
    case "saved":
      return "Saved locally";
    case "failed":
      return "Save failed";
    case "preview":
      return "Preview only";
    default:
      return "Local-first";
  }
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
