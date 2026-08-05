// src/features/storyplan/StoryPlanPanel.tsx
// Story Plan editor — Fabula-style structure layer (Sprint 2 / PR #23).
// Tree: Plan → Scenes → Beats, inline editing, beat pinning (locked beats
// are warded from regeneration), scene ↔ Vault item links, reorder.
import {
  ArrowDown, ArrowUp, ChevronDown, ChevronRight, Film, Link2, Plus,
  ScrollText, Shield, ShieldCheck, Trash2, X,
} from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { describeError } from "../../app/project";
import type { VaultItemNode } from "../../app/vault";
import {
  createStoryBeat, createStoryPlan, createStoryScene, deleteStoryBeat,
  deleteStoryPlan, deleteStoryScene, getStoryPlan, listStoryPlans, lockStoryBeat,
  reorderStoryNode, updateStoryBeat, updateStoryPlan, updateStoryScene,
  type StoryBeat, type StoryBeatType, type StoryPlan, type StoryPlanDetail,
  type StoryPlanStatus, type StorySceneWithBeats,
} from "../../app/storyplan";

const BEAT_TYPES: StoryBeatType[] = ["action", "dialogue", "revelation", "conflict", "transition", "other"];
const PLAN_STATUSES: StoryPlanStatus[] = ["draft", "outline", "drafting", "revision", "done"];

interface BeatDraft {
  content: string;
  beatType: StoryBeatType;
  charactersText: string;
}

interface SceneDraft {
  title: string;
  setting: string;
  summary: string;
  linkedItemId: string;
}

interface StoryPlanPanelProps {
  projectPath: string;
  vaultItems: VaultItemNode[];
  showToast: (msg: string) => void;
  onOpenLinkedItem: (itemId: string) => void;
}

function charactersTextOf(beat: StoryBeat): string {
  return beat.characters?.join(", ") ?? "";
}

function parseCharactersText(text: string): string[] {
  return text.split(",").map((name) => name.trim()).filter(Boolean);
}

function beatDraftOf(beat: StoryBeat): BeatDraft {
  return { content: beat.content, beatType: beat.beatType, charactersText: charactersTextOf(beat) };
}

function sceneDraftOf(scene: StorySceneWithBeats): SceneDraft {
  return {
    title: scene.title,
    setting: scene.setting ?? "",
    summary: scene.summary ?? "",
    linkedItemId: scene.linkedItemId ?? "",
  };
}

export function StoryPlanPanel({ projectPath, vaultItems, showToast, onOpenLinkedItem }: StoryPlanPanelProps) {
  const [plans, setPlans] = useState<StoryPlan[]>([]);
  const [selectedPlanId, setSelectedPlanId] = useState("");
  const [detail, setDetail] = useState<StoryPlanDetail | null>(null);
  const [loading, setLoading] = useState(true);
  const [expandedSceneIds, setExpandedSceneIds] = useState<Set<string>>(new Set());
  const [planSectionOpen, setPlanSectionOpen] = useState(false);

  // Plan-level edit draft
  const [planDraft, setPlanDraft] = useState({ name: "", logline: "", synopsis: "", status: "draft" as StoryPlanStatus });

  // Per-row edit state (one editor open at a time keeps the narrow panel sane)
  const [editingSceneId, setEditingSceneId] = useState("");
  const [sceneDraft, setSceneDraft] = useState<SceneDraft>({ title: "", setting: "", summary: "", linkedItemId: "" });
  const [editingBeatId, setEditingBeatId] = useState("");
  const [beatDraft, setBeatDraft] = useState<BeatDraft>({ content: "", beatType: "action", charactersText: "" });
  const [busy, setBusy] = useState(false);

  // Linkable Vault items: chapters and scenes only (the two-layer bridge)
  const linkableItems = vaultItems.filter((item) => item.itemType === "chapter" || item.itemType === "scene");

  const refreshPlans = useCallback(async () => {
    const response = await listStoryPlans(projectPath);
    setPlans(response.plans);
    return response.plans;
  }, [projectPath]);

  // Initial load
  useEffect(() => {
    let cancelled = false;
    async function load() {
      try {
        const loaded = await refreshPlans();
        if (cancelled) return;
        if (loaded.length > 0) setSelectedPlanId((current) => current || loaded[0].id);
      } catch (err) {
        if (!cancelled) showToast(describeError(err));
      } finally {
        if (!cancelled) setLoading(false);
      }
    }
    load();
    return () => { cancelled = true; };
  }, [refreshPlans, showToast]);

  // Load detail when selection changes
  useEffect(() => {
    if (!selectedPlanId) { setDetail(null); return; }
    let cancelled = false;
    async function load() {
      try {
        const loaded = await getStoryPlan(projectPath, selectedPlanId);
        if (!cancelled) setDetail(loaded);
      } catch (err) {
        if (!cancelled) showToast(describeError(err));
      }
    }
    load();
    return () => { cancelled = true; };
  }, [selectedPlanId, projectPath, showToast]);

  // Sync plan draft when detail arrives
  useEffect(() => {
    if (!detail) return;
    setPlanDraft({
      name: detail.projectName,
      logline: detail.logline ?? "",
      synopsis: detail.synopsis ?? "",
      status: detail.status,
    });
  }, [detail]);

  // ── Mutations (every call returns the updated detail tree) ──

  const run = useCallback(async (action: () => Promise<unknown>, successMsg?: string) => {
    if (busy) return;
    setBusy(true);
    try {
      const result = await action();
      if (result && typeof result === "object" && "plan" in (result as Record<string, unknown>)) {
        setDetail(result as StoryPlanDetail);
      }
      if (successMsg) showToast(successMsg);
    } catch (err) {
      showToast(describeError(err));
    } finally {
      setBusy(false);
    }
  }, [busy, showToast]);

  const handleCreatePlan = useCallback(async () => {
    const name = window.prompt("Name this story plan:");
    if (!name?.trim()) return;
    setBusy(true);
    try {
      const created = await createStoryPlan(projectPath, name.trim());
      setDetail(created);
      setSelectedPlanId(created.id);
      await refreshPlans();
      showToast("Story plan created.");
    } catch (err) {
      showToast(describeError(err));
    } finally {
      setBusy(false);
    }
  }, [projectPath, refreshPlans, showToast]);

  const handleDeletePlan = useCallback(async () => {
    if (!detail) return;
    if (!window.confirm(`Delete the story plan "${detail.projectName}"? Its scenes and beats go with it.`)) return;
    setBusy(true);
    try {
      const response = await deleteStoryPlan(projectPath, detail.id);
      setPlans(response.plans);
      setSelectedPlanId(response.plans[0]?.id ?? "");
      if (!response.plans[0]) setDetail(null);
      showToast("Story plan deleted.");
    } catch (err) {
      showToast(describeError(err));
    } finally {
      setBusy(false);
    }
  }, [detail, projectPath, showToast]);

  const handleSavePlan = useCallback(() => {
    if (!detail || !planDraft.name.trim()) { showToast("The story plan needs a name."); return; }
    void run(() => updateStoryPlan(projectPath, detail.id, {
      projectName: planDraft.name.trim(),
      logline: planDraft.logline,
      synopsis: planDraft.synopsis,
      status: planDraft.status,
    }), "The plan remembers.");
  }, [detail, planDraft, projectPath, run, showToast]);

  const handleCreateScene = useCallback(async () => {
    if (!detail) return;
    const title = window.prompt("Title for the new scene:");
    if (!title?.trim()) return;
    setBusy(true);
    try {
      const updated = await createStoryScene(projectPath, detail.id, title.trim());
      setDetail(updated);
      // Expand the freshly created scene — read it from the RETURNED tree,
      // not the stale `detail` closure (self-review catch, post-merge).
      const last = updated.scenes[updated.scenes.length - 1];
      if (last) setExpandedSceneIds((prev) => new Set(prev).add(last.id));
      showToast("Scene added.");
    } catch (err) {
      showToast(describeError(err));
    } finally {
      setBusy(false);
    }
  }, [detail, projectPath, showToast]);

  const handleDeleteScene = useCallback((sceneId: string, title: string) => {
    if (!window.confirm(`Delete scene "${title}" and its beats?`)) return;
    if (editingSceneId === sceneId) setEditingSceneId("");
    void run(() => deleteStoryScene(projectPath, sceneId), "Scene deleted.");
  }, [editingSceneId, projectPath, run]);

  const handleSaveScene = useCallback((sceneId: string) => {
    if (!sceneDraft.title.trim()) { showToast("A scene needs a title."); return; }
    void run(() => updateStoryScene(projectPath, sceneId, {
      title: sceneDraft.title.trim(),
      setting: sceneDraft.setting,
      summary: sceneDraft.summary,
      linkedItemId: sceneDraft.linkedItemId,
    }), "Scene updated.");
    setEditingSceneId("");
  }, [sceneDraft, projectPath, run, showToast]);

  const handleCreateBeat = useCallback(async (sceneId: string) => {
    const content = window.prompt("What happens in this beat?");
    if (!content?.trim()) return;
    await run(() => createStoryBeat(projectPath, sceneId, content.trim()), "Beat added.");
    setExpandedSceneIds((prev) => new Set(prev).add(sceneId));
  }, [projectPath, run]);

  const handleDeleteBeat = useCallback((beatId: string) => {
    if (!window.confirm("Delete this beat?")) return;
    if (editingBeatId === beatId) setEditingBeatId("");
    void run(() => deleteStoryBeat(projectPath, beatId), "Beat deleted.");
  }, [editingBeatId, projectPath, run]);

  const handleSaveBeat = useCallback((beatId: string) => {
    if (!beatDraft.content.trim()) { showToast("A beat cannot be empty."); return; }
    void run(() => updateStoryBeat(projectPath, beatId, {
      content: beatDraft.content.trim(),
      beatType: beatDraft.beatType,
      characters: parseCharactersText(beatDraft.charactersText),
    }), "Beat updated.");
    setEditingBeatId("");
  }, [beatDraft, projectPath, run, showToast]);

  const handleToggleLock = useCallback((beat: StoryBeat) => {
    void run(
      () => lockStoryBeat(projectPath, beat.id, !beat.locked),
      beat.locked ? "Beat unpinned." : "Beat pinned — it will not drift.",
    );
  }, [projectPath, run]);

  const handleReorder = useCallback((kind: "scene" | "beat", id: string, direction: "up" | "down") => {
    void run(() => reorderStoryNode(projectPath, kind, id, direction));
  }, [projectPath, run]);

  // Drag-to-reorder beats within a scene: walks the up/down primitive
  // one step at a time until the beat reaches the drop target.
  const handleBeatDrop = useCallback(async (scene: StorySceneWithBeats, draggedBeatId: string, targetBeatId: string) => {
    if (draggedBeatId === targetBeatId) return;
    const order = scene.beats.map((beat) => beat.id);
    const from = order.indexOf(draggedBeatId);
    const to = order.indexOf(targetBeatId);
    if (from === -1 || to === -1) return;
    const direction: "up" | "down" = from > to ? "up" : "down";
    const steps = Math.abs(from - to);
    setBusy(true);
    try {
      for (let step = 0; step < steps; step += 1) {
        const result = await reorderStoryNode(projectPath, "beat", draggedBeatId, direction);
        setDetail(result);
      }
    } catch (err) {
      showToast(describeError(err));
    } finally {
      setBusy(false);
    }
  }, [projectPath, showToast]);

  const toggleScene = useCallback((sceneId: string) => {
    setExpandedSceneIds((prev) => {
      const next = new Set(prev);
      if (next.has(sceneId)) next.delete(sceneId); else next.add(sceneId);
      return next;
    });
  }, []);

  const openSceneEditor = useCallback((scene: StorySceneWithBeats) => {
    setEditingSceneId(scene.id);
    setSceneDraft(sceneDraftOf(scene));
    setEditingBeatId("");
  }, []);

  const openBeatEditor = useCallback((beat: StoryBeat) => {
    setEditingBeatId(beat.id);
    setBeatDraft(beatDraftOf(beat));
    setEditingSceneId("");
  }, []);

  // ── Render ──

  if (loading) {
    return <div className="storyplan-empty"><span>Consulting the plans…</span></div>;
  }

  return (
    <div className="storyplan-panel">
      {/* Toolbar: plan selector + actions */}
      <div className="storyplan-toolbar">
        <select
          className="compact-input storyplan-select"
          value={selectedPlanId}
          onChange={(e) => setSelectedPlanId(e.target.value)}
          aria-label="Select story plan"
        >
          {plans.length === 0 && <option value="">No plans yet</option>}
          {plans.map((plan) => (
            <option key={plan.id} value={plan.id}>{plan.projectName}</option>
          ))}
        </select>
        <button className="icon-button" type="button" title="New story plan" onClick={() => void handleCreatePlan()}>
          <Plus size={16} />
        </button>
        {detail && (
          <button className="icon-button" type="button" title="Delete story plan" onClick={() => void handleDeletePlan()}>
            <Trash2 size={15} />
          </button>
        )}
      </div>

      {plans.length === 0 ? (
        <div className="storyplan-empty">
          <ScrollText size={22} aria-hidden="true" />
          <strong>No story plans yet</strong>
          <span>Every aligned story starts with a plan. Create one and pin what matters.</span>
        </div>
      ) : detail ? (
        <div className="panel-scroll storyplan-scroll">
          {/* Plan details (collapsible) */}
          <div className="sp-section">
            <button className="sp-section-toggle" type="button" aria-expanded={planSectionOpen} onClick={() => setPlanSectionOpen((v) => !v)}>
              {planSectionOpen ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
              <ScrollText size={14} aria-hidden="true" />
              <span className="sp-section-label">{detail.projectName}</span>
              <small>{detail.status}</small>
            </button>
            {planSectionOpen && (
              <div className="sp-plan-form">
                <label className="sp-field-label" htmlFor="sp-plan-name">Name</label>
                <input id="sp-plan-name" className="compact-input" value={planDraft.name}
                  onChange={(e) => setPlanDraft((d) => ({ ...d, name: e.target.value }))} />
                <label className="sp-field-label" htmlFor="sp-plan-status">Status</label>
                <select id="sp-plan-status" className="compact-input" value={planDraft.status}
                  onChange={(e) => setPlanDraft((d) => ({ ...d, status: e.target.value as StoryPlanStatus }))}>
                  {PLAN_STATUSES.map((status) => <option key={status} value={status}>{status}</option>)}
                </select>
                <label className="sp-field-label" htmlFor="sp-plan-logline">Logline</label>
                <textarea id="sp-plan-logline" className="compact-input sp-textarea" rows={2} value={planDraft.logline}
                  placeholder="One sentence that holds the story."
                  onChange={(e) => setPlanDraft((d) => ({ ...d, logline: e.target.value }))} />
                <label className="sp-field-label" htmlFor="sp-plan-synopsis">Synopsis</label>
                <textarea id="sp-plan-synopsis" className="compact-input sp-textarea" rows={4} value={planDraft.synopsis}
                  placeholder="The shape of the tale."
                  onChange={(e) => setPlanDraft((d) => ({ ...d, synopsis: e.target.value }))} />
                <div className="sp-form-actions">
                  <button className="button button-primary" type="button" disabled={busy} onClick={handleSavePlan}>Save Plan</button>
                </div>
              </div>
            )}
          </div>

          {/* Scenes */}
          <div className="sp-scenes">
            {detail.scenes.map((scene, sceneIndex) => {
              const expanded = expandedSceneIds.has(scene.id);
              const isEditingScene = editingSceneId === scene.id;
              return (
                <div key={scene.id} className="sp-scene">
                  <div className="sp-scene-row">
                    <button className="sp-scene-toggle" type="button" aria-expanded={expanded} onClick={() => toggleScene(scene.id)}>
                      {expanded ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
                      <Film size={14} aria-hidden="true" />
                      <span className="sp-scene-title">{scene.title}</span>
                      <small>{scene.beats.length} beats</small>
                    </button>
                    <div className="sp-row-actions">
                      {scene.linkedItemId && (
                        <button type="button" title="Open linked Vault item"
                          onClick={() => onOpenLinkedItem(scene.linkedItemId as string)}>
                          <Link2 size={12} />
                        </button>
                      )}
                      <button type="button" title="Move scene up" disabled={sceneIndex === 0}
                        onClick={() => handleReorder("scene", scene.id, "up")}>
                        <ArrowUp size={12} />
                      </button>
                      <button type="button" title="Move scene down" disabled={sceneIndex === detail.scenes.length - 1}
                        onClick={() => handleReorder("scene", scene.id, "down")}>
                        <ArrowDown size={12} />
                      </button>
                      <button type="button" title="Edit scene" onClick={() => openSceneEditor(scene)}>
                        ✎
                      </button>
                      <button type="button" title="Delete scene" onClick={() => handleDeleteScene(scene.id, scene.title)}>
                        <X size={12} />
                      </button>
                    </div>
                  </div>

                  {isEditingScene && (
                    <div className="sp-scene-form">
                      <label className="sp-field-label" htmlFor={`scene-title-${scene.id}`}>Title</label>
                      <input id={`scene-title-${scene.id}`} className="compact-input" value={sceneDraft.title}
                        onChange={(e) => setSceneDraft((d) => ({ ...d, title: e.target.value }))} />
                      <label className="sp-field-label" htmlFor={`scene-setting-${scene.id}`}>Setting</label>
                      <input id={`scene-setting-${scene.id}`} className="compact-input" value={sceneDraft.setting}
                        placeholder="Where and when."
                        onChange={(e) => setSceneDraft((d) => ({ ...d, setting: e.target.value }))} />
                      <label className="sp-field-label" htmlFor={`scene-summary-${scene.id}`}>Summary</label>
                      <textarea id={`scene-summary-${scene.id}`} className="compact-input sp-textarea" rows={2} value={sceneDraft.summary}
                        onChange={(e) => setSceneDraft((d) => ({ ...d, summary: e.target.value }))} />
                      <label className="sp-field-label" htmlFor={`scene-link-${scene.id}`}>Linked Vault item</label>
                      <select id={`scene-link-${scene.id}`} className="compact-input" value={sceneDraft.linkedItemId}
                        onChange={(e) => setSceneDraft((d) => ({ ...d, linkedItemId: e.target.value }))}>
                        <option value="">None</option>
                        {linkableItems.map((item) => (
                          <option key={item.id} value={item.id}>{item.title} ({item.itemType})</option>
                        ))}
                      </select>
                      <div className="sp-form-actions">
                        <button className="button button-primary" type="button" disabled={busy} onClick={() => handleSaveScene(scene.id)}>Save Scene</button>
                        <button className="button button-secondary" type="button" onClick={() => setEditingSceneId("")}>Cancel</button>
                      </div>
                    </div>
                  )}

                  {expanded && (
                    <div className="sp-beats">
                      {scene.beats.map((beat, beatIndex) => (
                        <div
                          key={beat.id}
                          className={`sp-beat ${beat.locked ? "locked" : ""}`}
                          draggable={editingBeatId !== beat.id}
                          onDragStart={(e) => { e.dataTransfer.setData("text/sp-beat", beat.id); e.dataTransfer.effectAllowed = "move"; }}
                          onDragOver={(e) => e.preventDefault()}
                          onDrop={(e) => {
                            e.preventDefault();
                            const draggedId = e.dataTransfer.getData("text/sp-beat");
                            if (draggedId) void handleBeatDrop(scene, draggedId, beat.id);
                          }}
                        >
                          {editingBeatId === beat.id ? (
                            <div className="sp-beat-form">
                              <label className="sp-field-label" htmlFor={`beat-content-${beat.id}`}>Beat</label>
                              <textarea id={`beat-content-${beat.id}`} className="compact-input sp-textarea" rows={3} value={beatDraft.content}
                                onChange={(e) => setBeatDraft((d) => ({ ...d, content: e.target.value }))} />
                              <div className="sp-beat-form-row">
                                <select className="compact-input" value={beatDraft.beatType} aria-label="Beat type"
                                  onChange={(e) => setBeatDraft((d) => ({ ...d, beatType: e.target.value as StoryBeatType }))}>
                                  {BEAT_TYPES.map((type) => <option key={type} value={type}>{type}</option>)}
                                </select>
                                <input className="compact-input" value={beatDraft.charactersText} placeholder="Characters, comma-separated"
                                  aria-label="Characters"
                                  onChange={(e) => setBeatDraft((d) => ({ ...d, charactersText: e.target.value }))} />
                              </div>
                              <div className="sp-form-actions">
                                <button className="button button-primary" type="button" disabled={busy} onClick={() => handleSaveBeat(beat.id)}>Save Beat</button>
                                <button className="button button-secondary" type="button" onClick={() => setEditingBeatId("")}>Cancel</button>
                              </div>
                            </div>
                          ) : (
                            <>
                              <div className="sp-beat-head">
                                <span className="sp-badge">{beat.beatType}</span>
                                {beat.characters && beat.characters.length > 0 && (
                                  <span className="sp-beat-characters">{beat.characters.join(", ")}</span>
                                )}
                                <div className="sp-row-actions">
                                  <button type="button"
                                    title={beat.locked ? "Unpin beat" : "Pin the beat — pinned beats will not drift"}
                                    onClick={() => handleToggleLock(beat)}>
                                    {beat.locked ? <ShieldCheck size={13} /> : <Shield size={13} />}
                                  </button>
                                  <button type="button" title="Move beat up" disabled={beatIndex === 0}
                                    onClick={() => handleReorder("beat", beat.id, "up")}>
                                    <ArrowUp size={12} />
                                  </button>
                                  <button type="button" title="Move beat down" disabled={beatIndex === scene.beats.length - 1}
                                    onClick={() => handleReorder("beat", beat.id, "down")}>
                                    <ArrowDown size={12} />
                                  </button>
                                  <button type="button" title="Edit beat" onClick={() => openBeatEditor(beat)}>✎</button>
                                  <button type="button" title="Delete beat" onClick={() => handleDeleteBeat(beat.id)}>
                                    <X size={12} />
                                  </button>
                                </div>
                              </div>
                              <p className="sp-beat-content">{beat.content}</p>
                            </>
                          )}
                        </div>
                      ))}
                      <button className="text-button sp-add-beat" type="button" onClick={() => void handleCreateBeat(scene.id)}>
                        <Plus size={12} /> New Beat
                      </button>
                    </div>
                  )}
                </div>
              );
            })}
          </div>

          <div className="sp-footer">
            <button className="text-button" type="button" onClick={() => void handleCreateScene()}>
              <Plus size={12} /> New Scene
            </button>
          </div>
        </div>
      ) : null}
    </div>
  );
}
