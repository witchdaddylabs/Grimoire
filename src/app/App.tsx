import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import {
  BookOpenText, ChevronLeft, ChevronRight, Feather,
  Loader2, Moon, SunMedium, Plus, Archive, FileText,
  AlertTriangle, X, Search,
} from "lucide-react";
import {
  createProject, createDemoProject, openProject,
  recordRecentProject, getRecentProjects, compactPath,
  describeError, type ProjectMetadata, type RecentProject,
} from "./project";
import {
  loadVaultTree, getVaultItem, updateVaultItem, importText,
  createVaultNode, archiveVaultItem, deleteVaultItem,
  searchChunks, exportItemMarkdown, exportProjectJson,
  exportVaultItemsJson, fallbackVaultTree, flattenVaultItems,
  type VaultItemNode, type VaultTreeResponse, type VaultItemDetail,
} from "./vault";

// ── Types ──
type TauriState = "checking" | "awake" | "browser";
type SaveState = "idle" | "editing" | "saving" | "saved" | "failed" | "preview";
type AppView = "picker" | "workspace";

// ── Helpers ──
function countWords(text: string) {
  return text.trim().split(/\s+/).filter(Boolean).length;
}

function saveStateLabel(s: SaveState): string {
  switch (s) {
    case "editing": return "Editing…";
    case "saving": return "Saving…";
    case "saved": return "Saved";
    case "failed": return "Failed to save";
    case "preview": return "Preview";
    default: return "";
  }
}

function saveStateTone(s: SaveState): "success" | "neutral" | "warning" {
  if (s === "saved" || s === "editing") return "success";
  if (s === "failed") return "warning";
  return "neutral";
}

function fallbackDetail(item: VaultItemNode): VaultItemDetail {
  return {
    id: item.id, title: item.title, itemType: item.itemType,
    content: item.content ?? "", plainText: item.content ?? "",
    wordCount: item.wordCount, path: item.path, updatedAt: "",
  };
}

const IMPORT_WORD_LIMIT = 10_000;

function prepareImportContent(content: string) {
  const wordCount = countWords(content);
  if (wordCount <= IMPORT_WORD_LIMIT) return { content, wordCount, originalWordCount: wordCount, truncated: false };
  let seenWords = 0, endIndex = content.length;
  for (const match of content.matchAll(/\S+/g)) {
    seenWords += 1;
    if (seenWords === IMPORT_WORD_LIMIT) { endIndex = (match.index ?? 0) + match[0].length; break; }
  }
  return { content: content.slice(0, endIndex), wordCount: IMPORT_WORD_LIMIT, originalWordCount: wordCount, truncated: true };
}

// ── Main App ──
export function App() {
  // Core state
  const [tauriState, setTauriState] = useState<TauriState>("checking");
  const [view, setView] = useState<AppView>("picker");
  const [project, setProject] = useState<ProjectMetadata | null>(null);
  const [projectLoading, setProjectLoading] = useState(true);
  const [recentProjects, setRecentProjects] = useState<RecentProject[]>(getRecentProjects);

  // Vault + editor
  const [vaultTree, setVaultTree] = useState<VaultTreeResponse>(fallbackVaultTree);
  const [activeItemId, setActiveItemId] = useState("");
  const [expandedNodeIds, setExpandedNodeIds] = useState<Set<string>>(new Set());
  const [editorTitle, setEditorTitle] = useState("");
  const [editorContent, setEditorContent] = useState("");
  const [loadedItemId, setLoadedItemId] = useState("");
  const [saveState, setSaveState] = useState<SaveState>("idle");
  const [saveError, setSaveError] = useState<string | null>(null);
  const lastSavedRef = useRef({ itemId: "", title: "", content: "" });

  // Import
  const [importTitle, setImportTitle] = useState("");
  const [importBody, setImportBody] = useState("");
  const [importStatus, setImportStatus] = useState("Paste text or choose .txt / .md files.");
  const [importRunning, setImportRunning] = useState(false);

  // Search
  const [searchQuery, setSearchQuery] = useState("");
  const [searchResults, setSearchResults] = useState<{ id: string; title: string; snippet: string }[]>([]);

  // Project picker state
  const [pickerName, setPickerName] = useState("");

  // UI prefs
  const [leftOpen, setLeftOpen] = useState(true);
  const [focusMode, setFocusMode] = useState(false);
  const [theme, setTheme] = useState<"dark" | "ivory">("dark");
  const [toast, setToast] = useState<string | null>(null);

  const vaultFlatItems = useMemo(() => flattenVaultItems(vaultTree), [vaultTree]);
  const activeItem = useMemo(
    () => vaultFlatItems.find((i) => i.id === activeItemId) ?? vaultFlatItems[0] ?? null,
    [activeItemId, vaultFlatItems],
  );
  const canUseNative = tauriState === "awake" && project !== null;
  const editorWordCount = useMemo(() => countWords(editorContent), [editorContent]);

  // Toast helper
  const showToast = useCallback((msg: string) => {
    setToast(msg);
    setTimeout(() => setToast(null), 3000);
  }, []);

  // Refresh tree
  const refreshTree = useCallback(async (selectItemId?: string) => {
    if (!project) return;
    const tree = await loadVaultTree(project.projectPath);
    setVaultTree(tree);
    if (selectItemId) {
      setActiveItemId(selectItemId);
    } else if (!flattenVaultItems(tree).some((i) => i.id === activeItemId)) {
      const first = flattenVaultItems(tree)[0];
      if (first) setActiveItemId(first.id);
    }
  }, [project, activeItemId]);

  // Load project into workspace
  const loadProjectIntoWorkspace = useCallback(async (metadata: ProjectMetadata) => {
    setProject(metadata);
    setView("workspace");
    recordRecentProject(metadata);
    setRecentProjects(getRecentProjects());
    try {
      const tree = await loadVaultTree(metadata.projectPath);
      setVaultTree(tree);
      const first = flattenVaultItems(tree)[0];
      if (first) setActiveItemId(first.id);
    } catch { /* empty project */ }
  }, []);

  // Boot
  useEffect(() => {
    let cancelled = false;
    async function boot() {
      try {
        await invoke<string>("app_ping");
        if (!cancelled) setTauriState("awake");
      } catch {
        if (!cancelled) setTauriState("browser");
      } finally {
        if (!cancelled) setProjectLoading(false);
      }
    }
    boot();
    return () => { cancelled = true; };
  }, []);

  // Load active item
  useEffect(() => {
    if (!activeItem) return;
    const currentProject = project;
    let cancelled = false;
    async function load() {
      try {
        const item = currentProject && tauriState === "awake"
          ? await getVaultItem(currentProject.projectPath, activeItem.id)
          : fallbackDetail(activeItem);
        if (cancelled) return;
        setLoadedItemId(item.id);
        setEditorTitle(item.title);
        setEditorContent(item.content);
        setSaveState(currentProject && tauriState === "awake" ? "saved" : "preview");
        setSaveError(null);
        lastSavedRef.current = { itemId: item.id, title: item.title, content: item.content };
      } catch {
        if (cancelled) return;
        const fb = fallbackDetail(activeItem);
        setLoadedItemId(fb.id);
        setEditorTitle(fb.title);
        setEditorContent(fb.content);
        setSaveState("failed");
        setSaveError("Could not load item.");
      }
    }
    load();
    return () => { cancelled = true; };
  }, [activeItem, project, tauriState]);

  // Autosave
  useEffect(() => {
    if (!project || !activeItem || loadedItemId !== activeItem.id || saveState !== "editing") return;
    const lastSaved = lastSavedRef.current;
    if (lastSaved.itemId === activeItem.id && lastSaved.title === editorTitle && lastSaved.content === editorContent) {
      setSaveState("saved");
      return;
    }
    let cancelled = false;
    const timer = window.setTimeout(async () => {
      try {
        setSaveState("saving");
        const saved = await updateVaultItem(project.projectPath, activeItem.id, editorTitle, editorContent);
        if (cancelled) return;
        lastSavedRef.current = { itemId: saved.id, title: saved.title, content: saved.content };
        setSaveError(null);
        setSaveState("saved");
        await refreshTree(saved.id);
      } catch (err) {
        if (cancelled) return;
        setSaveError(describeError(err));
        setSaveState("failed");
      }
    }, 850);
    return () => { cancelled = true; window.clearTimeout(timer); };
  }, [activeItem, editorContent, editorTitle, loadedItemId, project, refreshTree, saveState]);

  // Project handlers
  const handleCreateProject = useCallback(async (name: string) => {
    if (!name.trim()) return;
    const metadata = await createProject(name.trim());
    await loadProjectIntoWorkspace(metadata);
  }, [loadProjectIntoWorkspace]);

  const handleOpenProject = useCallback(async () => {
    const selected = await open({ directory: true, multiple: false, title: "Open Grimoire Project" });
    if (selected && typeof selected === "string") {
      const metadata = await openProject(selected);
      await loadProjectIntoWorkspace(metadata);
    }
  }, [loadProjectIntoWorkspace]);

  const handleLoadDemo = useCallback(async () => {
    const metadata = await createDemoProject();
    await loadProjectIntoWorkspace(metadata);
  }, [loadProjectIntoWorkspace]);

  // Import
  const handleImport = useCallback(async () => {
    if (!project || tauriState !== "awake") { showToast("Import needs a local project."); return; }
    if (!importBody.trim()) return;
    setImportRunning(true);
    try {
      const prepared = prepareImportContent(importBody);
      const response = await importText(project.projectPath, importTitle || "Imported writing", prepared.content, "Pasted text");
      setImportBody("");
      setImportTitle("");
      setImportStatus(prepared.truncated
        ? `Imported first ${IMPORT_WORD_LIMIT.toLocaleString()} words.`
        : `Imported ${response.item.title}.`);
      await refreshTree(response.item.id);
      showToast("Import complete.");
    } catch (err) {
      setImportStatus(describeError(err));
    } finally {
      setImportRunning(false);
    }
  }, [project, tauriState, importBody, importTitle, refreshTree, showToast]);

  const handleFileImport = useCallback(async (files: FileList | null) => {
    if (!files || !project || tauriState !== "awake") return;
    setImportRunning(true);
    let lastItemId = "";
    let importedCount = 0;
    for (const file of Array.from(files)) {
      if (!/\.(md|txt|markdown)$/i.test(file.name)) continue;
      const content = prepareImportContent(await file.text());
      const title = file.name.replace(/\.(md|txt|markdown)$/i, "");
      const response = await importText(project.projectPath, title, content.content, file.name);
      lastItemId = response.item.id;
      importedCount++;
    }
    if (importedCount > 0) {
      setImportStatus(`Imported ${importedCount} file${importedCount > 1 ? "s" : ""}.`);
      await refreshTree(lastItemId);
      showToast(`${importedCount} file${importedCount > 1 ? "s" : ""} imported.`);
    }
    setImportRunning(false);
  }, [project, tauriState, refreshTree, showToast]);

  // Search
  const handleSearch = useCallback(async () => {
    const query = searchQuery.trim();
    if (!query) { setSearchResults([]); return; }
    if (!project || tauriState !== "awake") {
      setSearchResults(vaultFlatItems
        .filter(i => `${i.title} ${i.content ?? ""}`.toLowerCase().includes(query.toLowerCase()))
        .slice(0, 8)
        .map(i => ({ id: i.id, title: i.title, snippet: (i.content ?? "").slice(0, 120) })));
      return;
    }
    try {
      const response = await searchChunks(project.projectPath, query, 8);
      setSearchResults(response.results.map(r => ({ id: r.itemId, title: r.title, snippet: r.snippet?.slice(0, 120) })));
    } catch { setSearchResults([]); }
  }, [searchQuery, project, tauriState, vaultFlatItems]);

  // Create vault node
  const handleCreateNode = useCallback(async (nodeType: "wing" | "hall" | "room" | "drawer" | "item") => {
    if (!project || tauriState !== "awake") { showToast("Open a project first."); return; }
    const name = window.prompt(`Name this ${nodeType}:`);
    if (!name?.trim()) return;
    const itemType = nodeType === "item"
      ? (window.prompt("Type (chapter, scene, character, location, lore, note):", "note") ?? "note")
      : undefined;
    try {
      const response = await createVaultNode(project.projectPath, nodeType, name.trim(), undefined, undefined, itemType?.trim() || "note");
      setVaultTree(response.tree);
      if (nodeType !== "item") setExpandedNodeIds(prev => new Set([...prev, response.id]));
      if (nodeType === "item") setActiveItemId(response.id);
      showToast(`Created ${nodeType}: ${name.trim()}`);
    } catch (err) { showToast(describeError(err)); }
  }, [project, tauriState, showToast]);

  // Export item
  const handleExportItem = useCallback(async () => {
    if (!project || !activeItem || tauriState !== "awake") { showToast("Open a project with an active item first."); return; }
    try {
      const r = await exportItemMarkdown(project.projectPath, activeItem.id);
      showToast(`Exported to exports folder.`);
    } catch (err) { showToast(describeError(err)); }
  }, [project, activeItem, tauriState, showToast]);

  // Title/content changes
  const handleTitleChange = useCallback((v: string) => {
    setEditorTitle(v);
    if (canUseNative) { setSaveState("editing"); setSaveError(null); }
    else setSaveState("preview");
  }, [canUseNative]);

  const handleContentChange = useCallback((v: string) => {
    setEditorContent(v);
    if (canUseNative) { setSaveState("editing"); setSaveError(null); }
    else setSaveState("preview");
  }, [canUseNative]);

  // Toggle node
  const toggleNode = useCallback((nodeId: string) => {
    setExpandedNodeIds(prev => {
      const next = new Set(prev);
      if (next.has(nodeId)) next.delete(nodeId); else next.add(nodeId);
      return next;
    });
  }, []);

  // ── Shell class ──
  const shellClassName = [
    "app-shell",
    focusMode ? "focus-mode" : "",
    theme === "ivory" ? "theme-ivory" : "theme-dark",
    leftOpen ? "" : "left-collapsed",
  ].filter(Boolean).join(" ");

  // ── Render ──
  return (
    <main className={shellClassName}>
      {/* Top bar */}
      <header className="top-bar">
        <div className="brand-lockup">
          <BookOpenText size={22} aria-hidden="true" />
          <div>
            <p className="eyebrow">Witch Daddy Labs</p>
            <h1>Grimoire</h1>
          </div>
        </div>

        <div className="status-strip">
          <span className={`status-chip ${saveStateTone(saveState)}`} style={{ display: "inline-flex", alignItems: "center", gap: 4 }}>
            {saveState === "saving" ? <Loader2 size={12} className="animate-spin" /> : null}
            {saveStateLabel(saveState)}
          </span>
          {project && (
            <span className="status-chip success">{project.name}</span>
          )}
        </div>

        <div className="top-actions">
          <button className="icon-button" type="button" aria-label="Toggle theme" onClick={() => setTheme(t => t === "ivory" ? "dark" : "ivory")}>
            {theme === "ivory" ? <Moon size={17} /> : <SunMedium size={17} />}
          </button>
          <button
            className="button button-primary" type="button"
            aria-pressed={focusMode}
            onClick={() => setFocusMode(v => !v)}
          >
            <Feather size={16} />
            {focusMode ? "Exit Focus" : "Focus"}
          </button>
          {canUseNative && (
            <button className="icon-button" type="button" title="New Project" onClick={() => setView("picker")}>
              <Plus size={17} />
            </button>
          )}
        </div>
      </header>

      {/* Project picker */}
      {view === "picker" && (
        <section className="project-picker" role="dialog" aria-label="Open or create project">
          <div className="project-picker-inner">
            <BookOpenText size={32} style={{ color: "var(--accent-bronze-bright)" }} />
            <h2>Welcome to Grimoire</h2>
            <p>Local-first writing studio with Vault memory.</p>

            <div className="project-picker-actions">
              <form onSubmit={async (e) => { e.preventDefault(); if (pickerName.trim()) { await handleCreateProject(pickerName.trim()); setPickerName(""); } }}>
                <input
                  className="input" type="text" placeholder="Project name"
                  value={pickerName} onChange={e => setPickerName(e.target.value)}
                />
                <button className="button button-primary" type="submit">
                  <Plus size={16} /> Create New Project
                </button>
              </form>
                  <button className="button button-secondary" type="button" onClick={handleOpenProject}>
                    <Archive size={16} /> Open Existing Project
                  </button>
                  <button className="button button-secondary" type="button" onClick={handleLoadDemo}>
                    <FileText size={16} /> Load Demo
                  </button>
                </div>

            {recentProjects.length > 0 && (
              <div className="project-picker-recent">
                <h3>Recent Projects</h3>
                <ul className="recent-list">
                  {recentProjects.map(rp => (
                    <li key={rp.path}>
                      <button className="recent-project-button" type="button" onClick={async () => {
                        try { await loadProjectIntoWorkspace(await openProject(rp.path)); }
                        catch { showToast("Could not open project."); }
                      }}>
                        <FileText size={14} />
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
          </div>
        </section>
      )}

      {/* Workspace */}
      {view === "workspace" && (
        <section className="workspace">
          {/* Left: Vault panel (collapsible) */}
          {leftOpen ? (
            <div className="vault-panel panel">
              <div className="panel-header">
                <div className="panel-heading">
                  <div className="panel-icon"><BookOpenText size={17} /></div>
                  <div style={{ minWidth: 0 }}>
                    <h3 style={{ margin: 0, fontSize: 14 }}>The Vault</h3>
                    <p style={{ margin: 0, fontSize: 11, color: "var(--parchment-muted)" }}>
                      {project ? `${vaultFlatItems.length} items` : "Demo"}
                    </p>
                  </div>
                </div>
                <button className="icon-button panel-collapse-button" type="button" aria-label="Collapse Vault" onClick={() => setLeftOpen(false)}>
                  <ChevronLeft size={16} />
                </button>
              </div>
              <div className="panel-scroll" style={{ padding: "8px 12px" }}>
                {/* Create buttons */}
                <div style={{ display: "flex", flexWrap: "wrap", gap: 4, marginBottom: 8 }}>
                  {(["wing", "hall", "room", "drawer", "item"] as const).map(t => (
                    <button key={t} className="text-button" type="button" onClick={() => handleCreateNode(t)}>
                      + {t.charAt(0).toUpperCase() + t.slice(1)}
                    </button>
                  ))}
                </div>

                {/* Vault tree */}
                <VaultTreeRead
                  tree={vaultTree}
                  activeItemId={activeItem?.id ?? ""}
                  expandedNodeIds={expandedNodeIds}
                  onToggle={toggleNode}
                  onSelectItem={setActiveItemId}
                />
              </div>
            </div>
          ) : (
            <button className="collapsed-rail left" type="button" onClick={() => setLeftOpen(true)} title="Open Vault">
              <BookOpenText size={18} />
              <ChevronRight size={14} />
            </button>
          )}

          {/* Center: Canvas */}
          <div className="canvas-panel">
            <div className="canvas-toolbar">
              <div style={{ minWidth: 0 }}>
                <p className="eyebrow">The Canvas</p>
                <p className="path-label" style={{ fontSize: 11, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                  {activeItem?.path ?? project?.name ?? "Grimoire"}
                </p>
              </div>
              <div style={{ display: "flex", alignItems: "center", gap: 8, flexShrink: 0 }}>
                <span style={{ fontSize: 12, color: "var(--parchment-muted)" }}>{editorWordCount} words</span>
                <span className={`status-chip ${saveStateTone(saveState)}`} style={{ fontSize: 11 }}>
                  {saveState === "saving" ? <Loader2 size={10} style={{ display: "inline", animation: "spin 1s linear infinite" }} /> : null}
                  {saveStateLabel(saveState)}
                </span>
              </div>
            </div>

            <label className="sr-only" htmlFor="canvas-title">Title</label>
            <input
              id="canvas-title" className="title-input" value={editorTitle}
              onChange={e => handleTitleChange(e.target.value)}
              placeholder="Untitled"
            />

            <label className="sr-only" htmlFor="canvas-editor">Editor</label>
            <textarea
              id="canvas-editor" className="editor-surface editor-textarea"
              value={editorContent}
              onChange={e => handleContentChange(e.target.value)}
              placeholder={canUseNative ? "Start writing your story…" : "Create or open a project to start writing."}
              style={{ flex: 1, minHeight: 300 }}
            />

            {saveError && (
              <p className="inline-error" style={{ marginTop: 8 }}>
                <AlertTriangle size={14} /> {saveError}
              </p>
            )}

            <div className="canvas-actions" style={{ marginTop: 12, paddingTop: 12, borderTop: "1px solid var(--border-subtle)", display: "flex", flexWrap: "wrap", gap: 8 }}>
              <button className="button button-secondary" type="button" onClick={handleExportItem} disabled={!canUseNative}>
                <FileText size={14} /> Export Markdown
              </button>
              <button className="button button-secondary" type="button" onClick={async () => {
                if (!project || tauriState !== "awake") return;
                try { await exportProjectJson(project.projectPath); showToast("Project exported."); }
                catch (err) { showToast(describeError(err)); }
              }}>
                <Archive size={14} /> Export Project
              </button>
            </div>
          </div>

          {/* Right: Tools panel (inside workspace grid, below top bar) — hidden in focus mode */}
          {!focusMode && (
            <div className="cowriter-panel panel">
              <div className="panel-header">
                <div className="panel-heading">
                  <div className="panel-icon"><Search size={17} /></div>
                  <div>
                    <h3 style={{ margin: 0, fontSize: 14 }}>Tools</h3>
                    <p style={{ margin: 0, fontSize: 11, color: "var(--parchment-muted)" }}>Import & search</p>
                  </div>
                </div>
              </div>
              <div className="panel-scroll" style={{ padding: 14 }}>
                {/* Search */}
                <form onSubmit={e => { e.preventDefault(); handleSearch(); }} style={{ display: "flex", gap: 6, marginBottom: 12 }}>
                  <input
                    className="compact-input" type="search" placeholder="Search Vault"
                    value={searchQuery} onChange={e => setSearchQuery(e.target.value)}
                    style={{ flex: 1 }}
                  />
                  <button className="button button-secondary" type="submit" style={{ padding: "6px 10px" }}>
                    <Search size={14} />
                  </button>
                </form>
                {searchResults.length > 0 && (
                  <div style={{ marginBottom: 12 }}>
                    {searchResults.map(r => (
                      <button key={r.id} className="result-item" type="button" onClick={() => setActiveItemId(r.id)} style={{ display: "block", width: "100%", textAlign: "left", marginBottom: 4 }}>
                        <strong style={{ fontSize: 12 }}>{r.title}</strong>
                        {r.snippet && <span style={{ fontSize: 11, color: "var(--parchment-muted)", display: "block", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>{r.snippet}</span>}
                      </button>
                    ))}
                    <button className="text-button" type="button" onClick={() => { setSearchResults([]); setSearchQuery(""); }} style={{ marginTop: 4 }}>Clear</button>
                  </div>
                )}

                {/* Import */}
                <div style={{ borderTop: "1px solid var(--border-subtle)", paddingTop: 12 }}>
                  <p style={{ fontSize: 12, color: "var(--parchment-secondary)", fontWeight: 600, margin: "0 0 8px" }}>Feed the Vault</p>
                  <form onSubmit={e => { e.preventDefault(); handleImport(); }}>
                    <input className="compact-input" value={importTitle} onChange={e => setImportTitle(e.target.value)} placeholder="Title (optional)" style={{ marginBottom: 6 }} />
                    <textarea className="compact-textarea" value={importBody} onChange={e => setImportBody(e.target.value)} placeholder="Paste text or Markdown (max 10,000 words)" style={{ minHeight: 80 }} />
                    <div style={{ display: "flex", flexWrap: "wrap", gap: 6, marginTop: 6 }}>
                      <button className="button button-primary" type="submit" disabled={importRunning}>
                        {importRunning ? <Loader2 size={14} /> : <FileText size={14} />} Import
                      </button>
                      <label className="file-button" style={{ display: "inline-flex", alignItems: "center", gap: 4, cursor: "pointer", fontSize: 12, padding: "5px 10px", border: "1px solid var(--border-subtle)", borderRadius: 6, background: "var(--obsidian-panel)", color: "var(--parchment-secondary)" }}>
                        <FileText size={14} /> Files
                        <input type="file" accept=".txt,.md,.markdown,text/plain,text/markdown" multiple onChange={e => handleFileImport(e.currentTarget.files)} style={{ display: "none" }} />
                      </label>
                    </div>
                  </form>
                  {importStatus && <p style={{ fontSize: 11, color: "var(--parchment-muted)", marginTop: 6 }}>{importStatus}</p>}
                </div>
              </div>
            </div>
          )}
        </section>
      )}

      {/* Toast */}
      {toast && (
        <div className="toast" role="status" aria-live="polite">
          {toast}
          <button className="icon-button" type="button" onClick={() => setToast(null)} style={{ marginLeft: 8 }}>
            <X size={14} />
          </button>
        </div>
      )}
    </main>
  );
}

// ── Simple Vault Tree (inline to avoid import complexity) ──
function VaultTreeRead({ tree, activeItemId, expandedNodeIds, onToggle, onSelectItem }: {
  tree: VaultTreeResponse;
  activeItemId: string;
  expandedNodeIds: Set<string>;
  onToggle: (id: string) => void;
  onSelectItem: (id: string) => void;
}) {
  if (!tree.wings.length) {
    return <p style={{ fontSize: 12, color: "var(--parchment-muted)", padding: "8px 0" }}>No Vault items yet. Create a Wing above to start building your story's memory.</p>;
  }

  return (
    <div className="vault-tree">
      {tree.wings.map(wing => (
        <details key={wing.id} open={expandedNodeIds.has(wing.id)}>
          <summary onClick={e => { e.preventDefault(); onToggle(wing.id); }} className="vault-tree-node">
            📁 {wing.name} <span style={{ color: "var(--parchment-muted)", fontSize: 11 }}>({tree.itemCount} items total)</span>
          </summary>
          <div style={{ paddingLeft: 14 }}>
            {wing.halls.map((hall: { id: string; name: string; rooms: { id: string; name: string; drawers: { id: string; name: string; items: { id: string; title: string }[] }[] }[] }) => (
              <details key={hall.id} open={expandedNodeIds.has(hall.id)}>
                <summary onClick={e => { e.preventDefault(); onToggle(hall.id); }} className="vault-tree-node">
                  📂 {hall.name}
                </summary>
                <div style={{ paddingLeft: 14 }}>
                  {hall.rooms.map((room: { id: string; name: string; drawers: { id: string; name: string; items: { id: string; title: string }[] }[] }) => (
                    <details key={room.id} open={expandedNodeIds.has(room.id)}>
                      <summary onClick={e => { e.preventDefault(); onToggle(room.id); }} className="vault-tree-node">
                        🗂️ {room.name}
                      </summary>
                      <div style={{ paddingLeft: 14 }}>
                        {room.drawers.map((drawer: { id: string; name: string; items: { id: string; title: string }[] }) => (
                          <details key={drawer.id} open={expandedNodeIds.has(drawer.id)}>
                            <summary onClick={e => { e.preventDefault(); onToggle(drawer.id); }} className="vault-tree-node">
                              📄 {drawer.name}
                            </summary>
                            <div style={{ paddingLeft: 14 }}>
                              {drawer.items.map((item: { id: string; title: string }) => (
                                <button
                                  key={item.id}
                                  type="button"
                                  className="vault-tree-item"
                                  onClick={() => onSelectItem(item.id)}
                                  style={{
                                    background: item.id === activeItemId ? "var(--accent-bronze-soft)" : "transparent",
                                    fontWeight: item.id === activeItemId ? 600 : 400,
                                  }}
                                >
                                  {item.title}
                                </button>
                              ))}
                            </div>
                          </details>
                        ))}
                        {room.drawers.length === 0 && (
                          <p style={{ fontSize: 11, color: "var(--parchment-muted)", padding: "4px 0" }}>Empty room</p>
                        )}
                      </div>
                    </details>
                  ))}
                </div>
              </details>
            ))}
          </div>
        </details>
      ))}
    </div>
  );
}
