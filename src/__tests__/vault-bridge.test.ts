import { describe, it, expect, vi, beforeEach } from "vitest";

// Mock @tauri-apps/api/core before importing vault
const mockInvoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...args: any[]) => mockInvoke(...args) }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));

import {
  loadVaultTree,
  getVaultItem,
  updateVaultItem,
  createVaultNode,
  archiveVaultItem,
  deleteVaultItem,
  exportItemMarkdown,
  exportProjectJson,
  searchChunks,
} from "../app/vault";

beforeEach(() => {
  mockInvoke.mockReset();
});

describe("vault bridge functions", () => {
  it("loadVaultTree calls db_get_vault_tree", async () => {
    const fakeTree = { wings: [], itemCount: 0 };
    mockInvoke.mockResolvedValue(fakeTree);
    const result = await loadVaultTree("/test/project.grimoire");
    expect(mockInvoke).toHaveBeenCalledWith("db_get_vault_tree", { projectPath: "/test/project.grimoire" });
    expect(result).toEqual(fakeTree);
  });

  it("getVaultItem calls db_get_item", async () => {
    const fakeItem = { id: "i1", title: "Test", content: "body" };
    mockInvoke.mockResolvedValue(fakeItem);
    const result = await getVaultItem("/test/project.grimoire", "i1");
    expect(mockInvoke).toHaveBeenCalledWith("db_get_item", { projectPath: "/test/project.grimoire", itemId: "i1" });
    expect(result).toEqual(fakeItem);
  });

  it("updateVaultItem calls db_update_item with all fields", async () => {
    const fakeTree = { wings: [], itemCount: 0 };
    mockInvoke.mockResolvedValue({ tree: fakeTree });
    await updateVaultItem("/test/project.grimoire", "i1", "New Title", "New Content");
    expect(mockInvoke).toHaveBeenCalledWith("db_update_item", {
      request: {
        projectPath: "/test/project.grimoire",
        itemId: "i1",
        title: "New Title",
        content: "New Content",
      },
    });
  });

  it("createVaultNode calls db_create_vault_node", async () => {
    const fakeResponse = { id: "new-1", tree: { wings: [], itemCount: 0 } };
    mockInvoke.mockResolvedValue(fakeResponse);
    const result = await createVaultNode("/test/project.grimoire", "wing", "My Wing", "parent-id", "A description", "note");
    expect(mockInvoke).toHaveBeenCalledWith("db_create_vault_node", {
      request: {
        projectPath: "/test/project.grimoire",
        nodeType: "wing",
        parentId: "parent-id",
        name: "My Wing",
        description: "A description",
        itemType: "note",
      },
    });
    expect(result).toEqual(fakeResponse);
  });

  it("archiveVaultItem calls db_archive_item", async () => {
    const fakeTree = { wings: [], itemCount: 0 };
    mockInvoke.mockResolvedValue(fakeTree);
    const result = await archiveVaultItem("/test/project.grimoire", "i1");
    expect(mockInvoke).toHaveBeenCalledWith("db_archive_item", {
      request: { projectPath: "/test/project.grimoire", itemId: "i1" },
    });
    expect(result).toEqual(fakeTree);
  });

  it("deleteVaultItem calls db_delete_item", async () => {
    const fakeTree = { wings: [], itemCount: 0 };
    mockInvoke.mockResolvedValue(fakeTree);
    const result = await deleteVaultItem("/test/project.grimoire", "i1");
    expect(mockInvoke).toHaveBeenCalledWith("db_delete_item", {
      request: { projectPath: "/test/project.grimoire", itemId: "i1" },
    });
    expect(result).toEqual(fakeTree);
  });

  it("exportItemMarkdown calls export_item_markdown", async () => {
    mockInvoke.mockResolvedValue({ path: "/exports/test.md", message: "Done" });
    const result = await exportItemMarkdown("/test/project.grimoire", "i1");
    expect(mockInvoke).toHaveBeenCalledWith("export_item_markdown", {
      request: { projectPath: "/test/project.grimoire", itemId: "i1" },
    });
    expect(result.path).toBe("/exports/test.md");
  });

  it("searchChunks calls db_search_chunks", async () => {
    mockInvoke.mockResolvedValue({ results: [], query: "test" });
    const result = await searchChunks("/test/project.grimoire", "test query");
    expect(mockInvoke).toHaveBeenCalledWith("db_search_chunks", {
      request: { projectPath: "/test/project.grimoire", query: "test query", limit: 8, mode: undefined },
    });
  });

  it("exportProjectJson calls export_project_json", async () => {
    mockInvoke.mockResolvedValue({ path: "/exports/project.json", message: "Done" });
    await exportProjectJson("/test/project.grimoire");
    expect(mockInvoke).toHaveBeenCalledWith("export_project_json", {
      projectPath: "/test/project.grimoire",
    });
  });
});
