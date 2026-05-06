import { invoke } from "@tauri-apps/api/core";
import { palaceItems } from "./demoData";

export type PalaceItemType =
  | "chapter"
  | "scene"
  | "character"
  | "location"
  | "lore"
  | "timeline"
  | "faction"
  | "research"
  | "note";

export type PalaceItemNode = {
  id: string;
  title: string;
  itemType: PalaceItemType;
  content: string | null;
  wordCount: number;
  path: string;
};

export type PalaceDrawerNode = {
  id: string;
  name: string;
  description: string | null;
  items: PalaceItemNode[];
};

export type PalaceRoomNode = {
  id: string;
  name: string;
  description: string | null;
  drawers: PalaceDrawerNode[];
};

export type PalaceHallNode = {
  id: string;
  name: string;
  description: string | null;
  rooms: PalaceRoomNode[];
};

export type PalaceWingNode = {
  id: string;
  name: string;
  description: string | null;
  halls: PalaceHallNode[];
};

export type PalaceTreeResponse = {
  wings: PalaceWingNode[];
  itemCount: number;
};

export type PalaceItemDetail = {
  id: string;
  title: string;
  itemType: PalaceItemType;
  content: string;
  plainText: string;
  wordCount: number;
  path: string;
  updatedAt: string;
};

export type ImportTextResponse = {
  item: PalaceItemDetail;
  progressLabels: string[];
  createdChunks: number;
};

export type SearchChunkResult = {
  chunkId: string;
  itemId: string;
  title: string;
  itemType: PalaceItemType;
  palacePath: string;
  snippet: string;
  score: number;
  confidence: "high" | "medium" | "low" | "none";
};

export type SearchChunksResponse = {
  query: string;
  results: SearchChunkResult[];
  confidence: "high" | "medium" | "low" | "none";
};

export type WardSeverity = "warn" | "block";

export type BannedWord = {
  id: string;
  value: string;
  severity: WardSeverity;
  isDefault: boolean;
};

export type WardScanHit = {
  id: string;
  value: string;
  severity: WardSeverity;
  count: number;
};

export type WardScanResponse = {
  hits: WardScanHit[];
  hasBlockingHits: boolean;
};

export type OllamaModel = {
  name: string;
  modifiedAt: string | null;
  size: number | null;
};

export type OllamaStatus = {
  baseUrl: string;
  reachable: boolean;
  models: OllamaModel[];
  selectedModel: string | null;
  message: string;
};

export type OllamaChatResponse = {
  model: string;
  text: string;
};

export type ExportResponse = {
  path: string;
  message: string;
};

export function loadPalaceTree(projectPath: string) {
  return invoke<PalaceTreeResponse>("db_get_palace_tree", { projectPath });
}

export function getPalaceItem(projectPath: string, itemId: string) {
  return invoke<PalaceItemDetail>("db_get_item", { projectPath, itemId });
}

export function updatePalaceItem(
  projectPath: string,
  itemId: string,
  title: string,
  content: string,
) {
  return invoke<PalaceItemDetail>("db_update_item", {
    request: { projectPath, itemId, title, content },
  });
}

export function importText(projectPath: string, title: string, content: string, sourceName?: string) {
  return invoke<ImportTextResponse>("db_import_text", {
    request: { projectPath, title, content, sourceName },
  });
}

export function archivePalaceItem(projectPath: string, itemId: string) {
  return invoke<PalaceTreeResponse>("db_archive_item", {
    request: { projectPath, itemId },
  });
}

export function deletePalaceItem(projectPath: string, itemId: string) {
  return invoke<PalaceTreeResponse>("db_delete_item", {
    request: { projectPath, itemId },
  });
}

export function searchChunks(projectPath: string, query: string, limit = 8, mode?: "default" | "broad") {
  return invoke<SearchChunksResponse>("db_search_chunks", {
    request: { projectPath, query, limit, mode },
  });
}

export function listWards(projectPath: string) {
  return invoke<BannedWord[]>("wards_list", { projectPath });
}

export function addWard(projectPath: string, value: string, severity: WardSeverity) {
  return invoke<BannedWord[]>("wards_add", {
    request: { projectPath, value, severity },
  });
}

export function removeWard(projectPath: string, id: string) {
  return invoke<BannedWord[]>("wards_remove", { projectPath, id });
}

export function scanWards(projectPath: string, text: string) {
  return invoke<WardScanResponse>("wards_scan", {
    request: { projectPath, text },
  });
}

export function getOllamaStatus(projectPath: string) {
  return invoke<OllamaStatus>("ollama_get_status", { projectPath });
}

export function selectOllamaModel(projectPath: string, model: string) {
  return invoke<OllamaStatus>("ollama_select_model", {
    request: { projectPath, model },
  });
}

export function ollamaChat(
  projectPath: string,
  model: string,
  prompt: string,
  context?: string,
) {
  return invoke<OllamaChatResponse>("ollama_chat", {
    request: { projectPath, model, prompt, context },
  });
}

export function exportItemMarkdown(projectPath: string, itemId: string) {
  return invoke<ExportResponse>("export_item_markdown", {
    request: { projectPath, itemId },
  });
}

export function exportProjectJson(projectPath: string) {
  return invoke<ExportResponse>("export_project_json", { projectPath });
}

export const fallbackPalaceTree: PalaceTreeResponse = {
  itemCount: palaceItems.length,
  wings: [
    {
      id: "wing_novel",
      name: "The Novel",
      description: "Browser preview demo project",
      halls: [
        {
          id: "hall_characters",
          name: "Characters",
          description: null,
          rooms: [
            {
              id: "room_protagonists",
              name: "Protagonists",
              description: null,
              drawers: [
                {
                  id: "drawer_main_cast",
                  name: "Main Cast",
                  description: null,
                  items: [fallbackItem("mara", "character")],
                },
              ],
            },
          ],
        },
        {
          id: "hall_world",
          name: "World",
          description: null,
          rooms: [
            {
              id: "room_cities",
              name: "Cities",
              description: null,
              drawers: [
                {
                  id: "drawer_northern_cities",
                  name: "Northern Cities",
                  description: null,
                  items: [fallbackItem("vel-ashen", "location")],
                },
              ],
            },
          ],
        },
        {
          id: "hall_drafts",
          name: "Drafts",
          description: null,
          rooms: [
            {
              id: "room_act_one",
              name: "Act One",
              description: null,
              drawers: [
                {
                  id: "drawer_opening_sequence",
                  name: "Opening Sequence",
                  description: null,
                  items: [fallbackItem("chapter-01", "chapter")],
                },
              ],
            },
          ],
        },
      ],
    },
  ],
};

export function flattenPalaceItems(tree: PalaceTreeResponse) {
  return tree.wings.flatMap((wing) =>
    wing.halls.flatMap((hall) =>
      hall.rooms.flatMap((room) => room.drawers.flatMap((drawer) => drawer.items)),
    ),
  );
}

function fallbackItem(id: string, itemType: PalaceItemType): PalaceItemNode {
  const item = palaceItems.find((candidate) => candidate.id === id);

  if (!item) {
    return {
      id,
      title: "Untitled",
      itemType,
      content: null,
      wordCount: 0,
      path: "The Novel",
    };
  }

  return {
    id: item.id,
    title: item.title,
    itemType,
    content: item.body,
    wordCount: item.body.split(/\s+/).filter(Boolean).length,
    path: `${item.path} / ${item.title}`,
  };
}
