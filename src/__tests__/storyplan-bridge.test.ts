import { describe, it, expect, vi, beforeEach } from "vitest";

// Mock @tauri-apps/api/core before importing storyplan
const mockInvoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...args: any[]) => mockInvoke(...args) }));

import {
  listStoryPlans, createStoryPlan, getStoryPlan, updateStoryPlan, deleteStoryPlan,
  createStoryScene, updateStoryScene, deleteStoryScene,
  createStoryBeat, updateStoryBeat, deleteStoryBeat, lockStoryBeat,
  reorderStoryNode, storeStoryCandidate, listStoryCandidates, resolveStoryCandidate,
} from "../app/storyplan";

beforeEach(() => {
  mockInvoke.mockReset();
});

describe("storyplan bridge functions", () => {
  it("listStoryPlans calls storyplan_list", async () => {
    const fake = { plans: [] };
    mockInvoke.mockResolvedValue(fake);
    const result = await listStoryPlans("/test/project.grimoire");
    expect(mockInvoke).toHaveBeenCalledWith("storyplan_list", { projectPath: "/test/project.grimoire" });
    expect(result).toEqual(fake);
  });

  it("createStoryPlan wraps args in request", async () => {
    const fakeDetail = { id: "plan_1", scenes: [] };
    mockInvoke.mockResolvedValue(fakeDetail);
    await createStoryPlan("/test/project.grimoire", "11 Grey Street", "A prequel.", "Synopsis.");
    expect(mockInvoke).toHaveBeenCalledWith("storyplan_create", {
      request: {
        projectPath: "/test/project.grimoire",
        projectName: "11 Grey Street",
        logline: "A prequel.",
        synopsis: "Synopsis.",
      },
    });
  });

  it("getStoryPlan passes planId", async () => {
    mockInvoke.mockResolvedValue({ id: "plan_1" });
    await getStoryPlan("/test/project.grimoire", "plan_1");
    expect(mockInvoke).toHaveBeenCalledWith("storyplan_get", {
      projectPath: "/test/project.grimoire",
      planId: "plan_1",
    });
  });

  it("updateStoryPlan spreads updates into request", async () => {
    mockInvoke.mockResolvedValue({ id: "plan_1" });
    await updateStoryPlan("/test/project.grimoire", "plan_1", { status: "drafting", logline: "New line" });
    expect(mockInvoke).toHaveBeenCalledWith("storyplan_update", {
      request: {
        projectPath: "/test/project.grimoire",
        planId: "plan_1",
        status: "drafting",
        logline: "New line",
      },
    });
  });

  it("deleteStoryPlan sends request object", async () => {
    mockInvoke.mockResolvedValue({ plans: [] });
    await deleteStoryPlan("/test/project.grimoire", "plan_1");
    expect(mockInvoke).toHaveBeenCalledWith("storyplan_delete", {
      request: { projectPath: "/test/project.grimoire", planId: "plan_1" },
    });
  });

  it("createStoryScene spreads options into request", async () => {
    mockInvoke.mockResolvedValue({ id: "plan_1", scenes: [] });
    await createStoryScene("/test/project.grimoire", "plan_1", "Opening", { setting: "St Kilda, 1994" });
    expect(mockInvoke).toHaveBeenCalledWith("storyplan_scene_create", {
      request: {
        projectPath: "/test/project.grimoire",
        planId: "plan_1",
        title: "Opening",
        setting: "St Kilda, 1994",
      },
    });
  });

  it("updateStoryScene passes updates", async () => {
    mockInvoke.mockResolvedValue({});
    await updateStoryScene("/test/project.grimoire", "scene_1", { title: "Renamed", linkedItemId: "item_9" });
    expect(mockInvoke).toHaveBeenCalledWith("storyplan_scene_update", {
      request: {
        projectPath: "/test/project.grimoire",
        sceneId: "scene_1",
        title: "Renamed",
        linkedItemId: "item_9",
      },
    });
  });

  it("deleteStoryScene sends request object", async () => {
    mockInvoke.mockResolvedValue({});
    await deleteStoryScene("/test/project.grimoire", "scene_1");
    expect(mockInvoke).toHaveBeenCalledWith("storyplan_scene_delete", {
      request: { projectPath: "/test/project.grimoire", sceneId: "scene_1" },
    });
  });

  it("createStoryBeat spreads beat options", async () => {
    mockInvoke.mockResolvedValue({});
    await createStoryBeat("/test/project.grimoire", "scene_1", "Mara enters.", { beatType: "action", characters: ["Mara"] });
    expect(mockInvoke).toHaveBeenCalledWith("storyplan_beat_create", {
      request: {
        projectPath: "/test/project.grimoire",
        sceneId: "scene_1",
        content: "Mara enters.",
        beatType: "action",
        characters: ["Mara"],
      },
    });
  });

  it("updateStoryBeat passes updates", async () => {
    mockInvoke.mockResolvedValue({});
    await updateStoryBeat("/test/project.grimoire", "beat_1", { content: "Rewritten beat." });
    expect(mockInvoke).toHaveBeenCalledWith("storyplan_beat_update", {
      request: { projectPath: "/test/project.grimoire", beatId: "beat_1", content: "Rewritten beat." },
    });
  });

  it("lockStoryBeat sends locked flag", async () => {
    mockInvoke.mockResolvedValue({});
    await lockStoryBeat("/test/project.grimoire", "beat_1", true);
    expect(mockInvoke).toHaveBeenCalledWith("storyplan_beat_lock", {
      request: { projectPath: "/test/project.grimoire", beatId: "beat_1", locked: true },
    });
  });

  it("reorderStoryNode sends kind, id and direction", async () => {
    mockInvoke.mockResolvedValue({});
    await reorderStoryNode("/test/project.grimoire", "beat", "beat_1", "up");
    expect(mockInvoke).toHaveBeenCalledWith("storyplan_reorder", {
      request: { projectPath: "/test/project.grimoire", kind: "beat", id: "beat_1", direction: "up" },
    });
  });

  it("storeStoryCandidate spreads candidate fields", async () => {
    mockInvoke.mockResolvedValue({ id: "candidate_1" });
    await storeStoryCandidate("/test/project.grimoire", {
      targetKind: "scene",
      targetId: "scene_1",
      provider: "anthropic",
      model: "claude-sonnet-4-5",
      candidateIndex: 0,
      content: "variant",
    });
    expect(mockInvoke).toHaveBeenCalledWith("storyplan_candidate_store", {
      request: {
        projectPath: "/test/project.grimoire",
        targetKind: "scene",
        targetId: "scene_1",
        provider: "anthropic",
        model: "claude-sonnet-4-5",
        candidateIndex: 0,
        content: "variant",
      },
    });
  });

  it("listStoryCandidates uses positional args", async () => {
    mockInvoke.mockResolvedValue([]);
    await listStoryCandidates("/test/project.grimoire", "scene", "scene_1");
    expect(mockInvoke).toHaveBeenCalledWith("storyplan_candidate_list", {
      projectPath: "/test/project.grimoire",
      targetKind: "scene",
      targetId: "scene_1",
    });
  });

  it("resolveStoryCandidate sends resolution", async () => {
    mockInvoke.mockResolvedValue({});
    await resolveStoryCandidate("/test/project.grimoire", "candidate_1", "accepted");
    expect(mockInvoke).toHaveBeenCalledWith("storyplan_candidate_resolve", {
      request: { projectPath: "/test/project.grimoire", candidateId: "candidate_1", resolution: "accepted" },
    });
  });
});
