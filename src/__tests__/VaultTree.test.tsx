import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { VaultTree } from "../features/vault/VaultTree";
import type { VaultTreeResponse } from "../app/vault";

// Mock lucide-react to avoid SVG rendering issues in jsdom
vi.mock("lucide-react", () => ({
  ChevronDown: (props: any) => <span data-testid="chevron-down" {...props} />,
  ChevronRight: (props: any) => <span data-testid="chevron-right" {...props} />,
  Archive: (props: any) => <span data-testid="archive" {...props} />,
}));

const emptyTree: VaultTreeResponse = { wings: [], itemCount: 0 };
const emptyWithWings: VaultTreeResponse = {
  itemCount: 0,
  wings: [
    {
      id: "w1", name: "Act One", description: null,
      halls: [{
        id: "h1", name: "Characters", description: null,
        rooms: [{
          id: "r1", name: "Protagonists", description: null,
          drawers: [{
            id: "d1", name: "Main Cast", description: null,
            items: [],
          }],
        }],
      }],
    },
  ],
};

const treeWithItems: VaultTreeResponse = {
  itemCount: 2,
  wings: [
    {
      id: "w1", name: "Act One", description: null,
      halls: [{
        id: "h1", name: "Characters", description: null,
        rooms: [{
          id: "r1", name: "Protagonists", description: null,
          drawers: [{
            id: "d1", name: "Main Cast", description: null,
            items: [
              { id: "i1", title: "Alice", itemType: "character", content: null, wordCount: 0, path: "Alice" },
              { id: "i2", title: "Bob", itemType: "character", content: null, wordCount: 0, path: "Bob" },
            ],
          }],
        }],
      }],
    },
  ],
};

const noop = () => {};

describe("VaultTree", () => {
  it("shows empty state when no wings and no items", () => {
    render(<VaultTree tree={emptyTree} activeItemId="" expandedNodeIds={new Set()} onArchiveItem={noop} onCreateNode={noop} onToggle={noop} onSelectItem={noop} />);
    expect(screen.getByText("The Vault is quiet")).toBeInTheDocument();
  });

  it("shows tree when wings exist but itemCount is zero", () => {
    render(<VaultTree tree={emptyWithWings} activeItemId="" expandedNodeIds={new Set()} onArchiveItem={noop} onCreateNode={noop} onToggle={noop} onSelectItem={noop} />);
    expect(screen.queryByText("The Vault is quiet")).not.toBeInTheDocument();
    expect(screen.getByText("Act One")).toBeInTheDocument();
  });

  it("renders wing names", () => {
    render(<VaultTree tree={treeWithItems} activeItemId="" expandedNodeIds={new Set(["w1", "h1", "r1", "d1"])} onArchiveItem={noop} onCreateNode={noop} onToggle={noop} onSelectItem={noop} />);
    expect(screen.getByText("Act One")).toBeInTheDocument();
    expect(screen.getByText("Characters")).toBeInTheDocument();
    expect(screen.getByText("Protagonists")).toBeInTheDocument();
    expect(screen.getByText("Main Cast")).toBeInTheDocument();
  });

  it("renders item titles", () => {
    render(<VaultTree tree={treeWithItems} activeItemId="" expandedNodeIds={new Set(["w1", "h1", "r1", "d1"])} onArchiveItem={noop} onCreateNode={noop} onToggle={noop} onSelectItem={noop} />);
    expect(screen.getByText("Alice")).toBeInTheDocument();
    expect(screen.getByText("Bob")).toBeInTheDocument();
  });

  it("calls onSelectItem when item is clicked", async () => {
    const user = userEvent.setup();
    const onSelectItem = vi.fn();
    render(<VaultTree tree={treeWithItems} activeItemId="" expandedNodeIds={new Set(["w1", "h1", "r1", "d1"])} onArchiveItem={noop} onCreateNode={noop} onToggle={noop} onSelectItem={onSelectItem} />);
    await user.click(screen.getByText("Alice"));
    expect(onSelectItem).toHaveBeenCalledWith("i1");
  });

  it("calls onToggle when branch is clicked", async () => {
    const user = userEvent.setup();
    const onToggle = vi.fn();
    render(<VaultTree tree={treeWithItems} activeItemId="" expandedNodeIds={new Set()} onArchiveItem={noop} onCreateNode={noop} onToggle={onToggle} onSelectItem={noop} />);
    await user.click(screen.getByText("Act One"));
    expect(onToggle).toHaveBeenCalledWith("w1");
  });

  it("shows archive button for items", () => {
    render(<VaultTree tree={treeWithItems} activeItemId="" expandedNodeIds={new Set(["w1", "h1", "r1", "d1"])} onArchiveItem={noop} onCreateNode={noop} onToggle={noop} onSelectItem={noop} />);
    const archiveButtons = screen.getAllByLabelText("Archive Alice");
    expect(archiveButtons.length).toBeGreaterThan(0);
  });

  it("shows create buttons for branches", () => {
    render(<VaultTree tree={treeWithItems} activeItemId="" expandedNodeIds={new Set(["w1"])} onArchiveItem={noop} onCreateNode={noop} onToggle={noop} onSelectItem={noop} />);
    expect(screen.getByText("New Hall")).toBeInTheDocument();
  });

  it("highlights active item", () => {
    render(<VaultTree tree={treeWithItems} activeItemId="i1" expandedNodeIds={new Set(["w1", "h1", "r1", "d1"])} onArchiveItem={noop} onCreateNode={noop} onToggle={noop} onSelectItem={noop} />);
    const alice = screen.getByText("Alice").closest("[class*='tree-item']");
    expect(alice).toHaveClass("active");
  });
});
