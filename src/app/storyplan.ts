import { invoke } from "@tauri-apps/api/core";
import type { WardScanResponse, WardScanHit } from "./vault";
import type { AiProviderKind } from "./ai";

// ── Story Plan types (Schema v3) ──

export type StoryPlanStatus = "draft" | "outline" | "drafting" | "revision" | "done";

export type StoryBeatType =
  | "action"
  | "dialogue"
  | "revelation"
  | "conflict"
  | "transition"
  | "other";

export type StoryPlan = {
  id: string;
  projectName: string;
  logline: string | null;
  synopsis: string | null;
  status: StoryPlanStatus;
  createdAt: string;
  updatedAt: string;
};

export type StoryScene = {
  id: string;
  planId: string;
  title: string;
  setting: string | null;
  summary: string | null;
  /** Optional link to a Vault item (chapter/scene) holding the prose. */
  linkedItemId: string | null;
  sortOrder: number;
  createdAt: string;
  updatedAt: string;
};

export type StoryBeat = {
  id: string;
  sceneId: string;
  beatType: StoryBeatType;
  content: string;
  characters: string[] | null;
  /** Pinned by the writer — warded from regeneration. */
  locked: boolean;
  sortOrder: number;
  createdAt: string;
  updatedAt: string;
};

export type StorySceneWithBeats = StoryScene & {
  beats: StoryBeat[];
};

export type StoryPlanDetail = StoryPlan & {
  scenes: StorySceneWithBeats[];
};

export type StoryPlanListResponse = {
  plans: StoryPlan[];
};

export type CandidateTargetKind = "plan" | "scene" | "beat" | "script";

export type StoryCandidateStatus = "pending" | "accepted" | "rejected";

export type StoryCandidate = {
  id: string;
  targetKind: CandidateTargetKind;
  targetId: string;
  provider: string;
  model: string;
  promptSummary: string | null;
  candidateIndex: number;
  content: string;
  wardScan: WardScanHit[];
  status: StoryCandidateStatus;
  createdAt: string;
};

// ── Regeneration (Fabula-style convergent iteration) ──

export type BeatAnchor = {
  sceneTitle: string;
  beatType: StoryBeatType;
  content: string;
};

export type CharacterFact = {
  character: string;
  sourceTitle: string;
  sourcePath: string;
  snippet: string;
};

export type SceneOutlineEntry = {
  title: string;
  summary: string | null;
};

/** The six-point context the model saw, echoed back for transparency. */
export type RegenerationContext = {
  planName: string;
  logline: string | null;
  synopsis: string | null;
  scene: StoryScene | null;
  characterFacts: CharacterFact[];
  previousSceneAnchor: BeatAnchor | null;
  nextSceneAnchor: BeatAnchor | null;
  lockedBeats: BeatAnchor[];
  sceneOutline: SceneOutlineEntry[];
  instruction: string;
};

export type StoryRegenerateCandidate = StoryCandidate & {
  wardScan: WardScanResponse;
};

export type StoryRegenerateResponse = {
  provider: AiProviderKind;
  model: string;
  context: RegenerationContext;
  candidates: StoryRegenerateCandidate[];
};

// ── Tauri command wrappers ──

export function listStoryPlans(projectPath: string) {
  return invoke<StoryPlanListResponse>("storyplan_list", { projectPath });
}

export function createStoryPlan(projectPath: string, projectName: string, logline?: string, synopsis?: string) {
  return invoke<StoryPlanDetail>("storyplan_create", {
    request: { projectPath, projectName, logline, synopsis },
  });
}

export function getStoryPlan(projectPath: string, planId: string) {
  return invoke<StoryPlanDetail>("storyplan_get", { projectPath, planId });
}

export function updateStoryPlan(
  projectPath: string,
  planId: string,
  updates: {
    projectName?: string;
    logline?: string;
    synopsis?: string;
    status?: StoryPlanStatus;
  },
) {
  return invoke<StoryPlanDetail>("storyplan_update", {
    request: { projectPath, planId, ...updates },
  });
}

export function deleteStoryPlan(projectPath: string, planId: string) {
  return invoke<StoryPlanListResponse>("storyplan_delete", {
    request: { projectPath, planId },
  });
}

export function createStoryScene(
  projectPath: string,
  planId: string,
  title: string,
  options?: { setting?: string; summary?: string; linkedItemId?: string },
) {
  return invoke<StoryPlanDetail>("storyplan_scene_create", {
    request: { projectPath, planId, title, ...options },
  });
}

export function updateStoryScene(
  projectPath: string,
  sceneId: string,
  updates: { title?: string; setting?: string; summary?: string; linkedItemId?: string },
) {
  return invoke<StoryPlanDetail>("storyplan_scene_update", {
    request: { projectPath, sceneId, ...updates },
  });
}

export function deleteStoryScene(projectPath: string, sceneId: string) {
  return invoke<StoryPlanDetail>("storyplan_scene_delete", {
    request: { projectPath, sceneId },
  });
}

export function createStoryBeat(
  projectPath: string,
  sceneId: string,
  content: string,
  options?: { beatType?: StoryBeatType; characters?: string[] },
) {
  return invoke<StoryPlanDetail>("storyplan_beat_create", {
    request: { projectPath, sceneId, content, ...options },
  });
}

export function updateStoryBeat(
  projectPath: string,
  beatId: string,
  updates: { beatType?: StoryBeatType; content?: string; characters?: string[] },
) {
  return invoke<StoryPlanDetail>("storyplan_beat_update", {
    request: { projectPath, beatId, ...updates },
  });
}

export function deleteStoryBeat(projectPath: string, beatId: string) {
  return invoke<StoryPlanDetail>("storyplan_beat_delete", {
    request: { projectPath, beatId },
  });
}

export function lockStoryBeat(projectPath: string, beatId: string, locked: boolean) {
  return invoke<StoryPlanDetail>("storyplan_beat_lock", {
    request: { projectPath, beatId, locked },
  });
}

export function reorderStoryNode(projectPath: string, kind: "scene" | "beat", id: string, direction: "up" | "down") {
  return invoke<StoryPlanDetail>("storyplan_reorder", {
    request: { projectPath, kind, id, direction },
  });
}

export function storeStoryCandidate(
  projectPath: string,
  candidate: {
    targetKind: CandidateTargetKind;
    targetId: string;
    provider: string;
    model: string;
    promptSummary?: string;
    candidateIndex: number;
    content: string;
  },
) {
  return invoke<StoryCandidate>("storyplan_candidate_store", {
    request: { projectPath, ...candidate },
  });
}

export function listStoryCandidates(projectPath: string, targetKind: CandidateTargetKind, targetId: string) {
  return invoke<StoryCandidate[]>("storyplan_candidate_list", {
    projectPath,
    targetKind,
    targetId,
  });
}

export function resolveStoryCandidate(projectPath: string, candidateId: string, resolution: "accepted" | "rejected") {
  return invoke<StoryCandidate>("storyplan_candidate_resolve", {
    request: { projectPath, candidateId, resolution },
  });
}

/**
 * Regenerate a story-plan layer (plan / scene / beat / script) and store the
 * candidate variants. The backend assembles the six-point convergence context
 * (logline+synopsis, character facts, adjacent-scene anchors, locked beats,
 * edit instruction), generates `candidateCount` variants with a temperature
 * spread, stores each in story_candidates, and scans them against the wards.
 */
export function regenerateStoryLayer(
  projectPath: string,
  target: {
    targetKind: CandidateTargetKind;
    targetId: string;
    instruction: string;
    provider: AiProviderKind;
    model: string;
    candidateCount?: number;
    scanWards?: boolean;
  },
) {
  return invoke<StoryRegenerateResponse>("storyplan_regenerate", {
    request: { projectPath, ...target },
  });
}
