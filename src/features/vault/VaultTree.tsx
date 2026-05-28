// src/features/vault/VaultTree.tsx
import { ChevronDown, ChevronRight, Archive } from "lucide-react";
import type {
  VaultDrawerNode,
  VaultHallNode,
  VaultRoomNode,
  VaultTreeResponse,
  VaultWingNode,
} from "../../app/vault";

interface VaultTreeProps {
  tree: VaultTreeResponse;
  activeItemId: string;
  expandedNodeIds: Set<string>;
  onArchiveItem: (itemId: string) => void;
  onCreateNode: (nodeType: "wing" | "hall" | "room" | "drawer" | "item", parentId?: string) => void;
  onToggle: (nodeId: string) => void;
  onSelectItem: (itemId: string) => void;
}

function countWingItems(wing: VaultWingNode): number {
  return wing.halls.reduce((sum, hall) => sum + countHallItems(hall), 0);
}

function countHallItems(hall: VaultHallNode): number {
  return hall.rooms.reduce((sum, room) => sum + countRoomItems(room), 0);
}

function countRoomItems(room: VaultRoomNode): number {
  return room.drawers.reduce((sum, drawer) => sum + drawer.items.length, 0);
}

function treeDepthStyle(level: number): React.CSSProperties {
  return { "--depth": level } as React.CSSProperties;
}

function TreeBranch({
  id, label, meta, level, expanded, onToggle, children,
}: {
  id: string; label: string; meta: string; level: number;
  expanded: boolean; onToggle: (id: string) => void; children: React.ReactNode;
}) {
  return (
    <div className="tree-branch" style={treeDepthStyle(level)}>
      <button className="tree-branch-toggle" type="button" aria-expanded={expanded} onClick={() => onToggle(id)}>
        {expanded ? <ChevronDown size={14} aria-hidden="true" /> : <ChevronRight size={14} aria-hidden="true" />}
        <span className="tree-branch-label">{label}</span>
        <small>{meta}</small>
      </button>
      {expanded ? <div className="tree-branch-children">{children}</div> : null}
    </div>
  );
}

function DrawerBranch({ drawer, activeItemId, expandedNodeIds, onArchiveItem, onCreateNode, onToggle, onSelectItem }: {
  drawer: VaultDrawerNode; activeItemId: string; expandedNodeIds: Set<string>;
  onArchiveItem: (id: string) => void; onCreateNode: (nodeType: "item", parentId: string) => void; onToggle: (id: string) => void; onSelectItem: (id: string) => void;
}) {
  const expanded = expandedNodeIds.has(drawer.id);
  return (
    <TreeBranch id={drawer.id} label={drawer.name} meta={`Drawer / ${drawer.items.length} items`} level={3} expanded={expanded} onToggle={onToggle}>
      <div className="tree-actions">
        <button className="text-button" type="button" onClick={() => onCreateNode("item", drawer.id)}>New Item</button>
      </div>
      {drawer.items.map((item) => (
        <div key={item.id} className={`tree-item ${item.id === activeItemId ? "active" : ""}`}
          onClick={() => onSelectItem(item.id)} role="button" tabIndex={0}
          onKeyDown={(e) => { if (e.key === "Enter") onSelectItem(item.id); }}>
          <span>{item.title}</span>
          <div className="tree-item-actions">
            <button type="button" aria-label={`Archive ${item.title}`} onClick={(e) => { e.stopPropagation(); onArchiveItem(item.id); }}>
              <Archive size={12} />
            </button>
          </div>
        </div>
      ))}
    </TreeBranch>
  );
}

function RoomBranch({ room, activeItemId, expandedNodeIds, onArchiveItem, onCreateNode, onToggle, onSelectItem }: {
  room: VaultRoomNode; activeItemId: string; expandedNodeIds: Set<string>;
  onArchiveItem: (id: string) => void; onCreateNode: (nodeType: "drawer" | "item", parentId: string) => void; onToggle: (id: string) => void; onSelectItem: (id: string) => void;
}) {
  const expanded = expandedNodeIds.has(room.id);
  return (
    <TreeBranch id={room.id} label={room.name} meta={`Room / ${countRoomItems(room)} items`} level={2} expanded={expanded} onToggle={onToggle}>
      <div className="tree-actions">
        <button className="text-button" type="button" onClick={() => onCreateNode("drawer", room.id)}>New Drawer</button>
      </div>
      {room.drawers.map((drawer) => (
        <DrawerBranch key={drawer.id} drawer={drawer} activeItemId={activeItemId} expandedNodeIds={expandedNodeIds} onArchiveItem={onArchiveItem} onCreateNode={onCreateNode} onToggle={onToggle} onSelectItem={onSelectItem} />
      ))}
    </TreeBranch>
  );
}

function HallBranch({ hall, activeItemId, expandedNodeIds, onArchiveItem, onCreateNode, onToggle, onSelectItem }: {
  hall: VaultHallNode; activeItemId: string; expandedNodeIds: Set<string>;
  onArchiveItem: (id: string) => void; onCreateNode: (nodeType: "room" | "drawer" | "item", parentId: string) => void; onToggle: (id: string) => void; onSelectItem: (id: string) => void;
}) {
  const expanded = expandedNodeIds.has(hall.id);
  return (
    <TreeBranch id={hall.id} label={hall.name} meta={`Hall / ${countHallItems(hall)} items`} level={1} expanded={expanded} onToggle={onToggle}>
      <div className="tree-actions">
        <button className="text-button" type="button" onClick={() => onCreateNode("room", hall.id)}>New Room</button>
      </div>
      {hall.rooms.map((room) => (
        <RoomBranch key={room.id} room={room} activeItemId={activeItemId} expandedNodeIds={expandedNodeIds} onArchiveItem={onArchiveItem} onCreateNode={onCreateNode} onToggle={onToggle} onSelectItem={onSelectItem} />
      ))}
    </TreeBranch>
  );
}

function WingBranch({ wing, activeItemId, expandedNodeIds, onArchiveItem, onCreateNode, onToggle, onSelectItem }: {
  wing: VaultWingNode; activeItemId: string; expandedNodeIds: Set<string>;
  onArchiveItem: (id: string) => void; onCreateNode: (nodeType: "hall" | "room" | "drawer" | "item", parentId: string) => void; onToggle: (id: string) => void; onSelectItem: (id: string) => void;
}) {
  const expanded = expandedNodeIds.has(wing.id);
  return (
    <TreeBranch id={wing.id} label={wing.name} meta={`Wing / ${countWingItems(wing)} items`} level={0} expanded={expanded} onToggle={onToggle}>
      <div className="tree-actions">
        <button className="text-button" type="button" onClick={() => onCreateNode("hall", wing.id)}>New Hall</button>
      </div>
      {wing.halls.map((hall) => (
        <HallBranch key={hall.id} hall={hall} activeItemId={activeItemId} expandedNodeIds={expandedNodeIds} onArchiveItem={onArchiveItem} onCreateNode={onCreateNode} onToggle={onToggle} onSelectItem={onSelectItem} />
      ))}
    </TreeBranch>
  );
}

export function VaultTree({ tree, activeItemId, expandedNodeIds, onArchiveItem, onCreateNode, onToggle, onSelectItem }: VaultTreeProps) {
  if (tree.itemCount === 0) {
    return (
      <div className="vault-empty">
        <strong>The Vault is quiet</strong>
        <span>Create your first Wing or import writing to begin.</span>
      </div>
    );
  }

  return (
    <nav className="vault-tree" aria-label="Vault memory">
      {tree.wings.map((wing) => (
        <WingBranch key={wing.id} wing={wing} activeItemId={activeItemId} expandedNodeIds={expandedNodeIds} onArchiveItem={onArchiveItem} onCreateNode={onCreateNode} onToggle={onToggle} onSelectItem={onSelectItem} />
      ))}
    </nav>
  );
}
