import { describe, it, expect } from "vitest";
import { flattenVaultItems, type VaultTreeResponse } from "../app/vault";

function makeItem(id: string, title: string, itemType: "note" | "chapter" | "scene" = "note") {
  return { id, title, itemType, content: null, wordCount: 0, path: title };
}

describe("flattenVaultItems", () => {
  it("returns empty array for empty tree", () => {
    const tree: VaultTreeResponse = { wings: [], itemCount: 0 };
    expect(flattenVaultItems(tree)).toEqual([]);
  });

  it("flattens all items across wings/halls/rooms/drawers", () => {
    const tree: VaultTreeResponse = {
      itemCount: 3,
      wings: [
        {
          id: "w1", name: "Wing 1", description: null,
          halls: [{
            id: "h1", name: "Hall 1", description: null,
            rooms: [{
              id: "r1", name: "Room 1", description: null,
              drawers: [{
                id: "d1", name: "Drawer 1", description: null,
                items: [makeItem("i1", "Item 1"), makeItem("i2", "Item 2", "chapter")],
              }],
            }],
          }],
        },
        {
          id: "w2", name: "Wing 2", description: null,
          halls: [{
            id: "h2", name: "Hall 2", description: null,
            rooms: [{
              id: "r2", name: "Room 2", description: null,
              drawers: [{
                id: "d2", name: "Drawer 2", description: null,
                items: [makeItem("i3", "Item 3", "scene")],
              }],
            }],
          }],
        },
      ],
    };

    const items = flattenVaultItems(tree);
    expect(items).toHaveLength(3);
    expect(items.map((i) => i.id)).toEqual(["i1", "i2", "i3"]);
  });

  it("skips empty drawers", () => {
    const tree: VaultTreeResponse = {
      itemCount: 0,
      wings: [{
        id: "w1", name: "Wing", description: null,
        halls: [{
          id: "h1", name: "Hall", description: null,
          rooms: [{
            id: "r1", name: "Room", description: null,
            drawers: [
              { id: "d1", name: "Empty Drawer", description: null, items: [] },
              { id: "d2", name: "Full Drawer", description: null, items: [makeItem("i1", "Only Item")] },
            ],
          }],
        }],
      }],
    };

    const items = flattenVaultItems(tree);
    expect(items).toHaveLength(1);
    expect(items[0].id).toBe("i1");
  });
});
