// src/features/vault/VaultPanel.tsx
import { Database, Search, ChevronLeft, Trash2, Archive } from "lucide-react";
import type { ReactNode } from "react";
import type {
  VaultDrawerNode,
  VaultHallNode,
  VaultItemNode,
  VaultRoomNode,
  VaultTreeResponse,
  VaultWingNode,
  SearchChunkResult,
  ExternalVaultStructure,
} from "../../app/vault";
import { flattenVaultItems } from "../../app/vault";
import { PanelHeader } from "../../components/PanelHeader";
import { CollapsedRail } from "../../components/CollapsedRail";
import type { AsyncState } from "../../components/types";
import { VaultTree } from "./VaultTree";

interface VaultPanelProps {
  tree: VaultTreeResponse;
  project: { projectPath: string; name: string } | null;
  projectError: string | null;
  projectLoading: boolean;
  tauriState: "checking" | "awake" | "browser";
  searchQuery: string;
  searchResults: SearchChunkResult[];
  searchError: string | null;
  activeItem: VaultItemNode | null;
  expandedNodeIds: Set<string>;
  leftCollapsed: boolean;
  onSearchChange: (q: string) => void;
  onSearch: (e: React.FormEvent) => void;
  onSearchClear: () => void;
  onToggle: (nodeId: string) => void;
  onSelectItem: (itemId: string) => void;
  onArchiveItem: (itemId: string) => void;
  onDeleteItem: (itemId: string) => void;
  onCreateNode: (nodeType: "wing" | "hall" | "room" | "drawer" | "item", parentId?: string) => void;
  externalVault: ExternalVaultStructure | null;
  externalVaultState: AsyncState;
  externalVaultStatus: string;
  onOpenExternalVault: () => void;
  onClearExternalVault: () => void;
  onExpandLeft: () => void;
  onCollapseLeft: () => void;
  compactPath: (p: string) => string;
}

export function VaultPanel({
  tree,
  project,
  projectError,
  projectLoading,
  tauriState,
  searchQuery,
  searchResults,
  searchError,
  activeItem,
  expandedNodeIds,
  leftCollapsed,
  onSearchChange,
  onSearch,
  onSearchClear,
  onToggle,
  onSelectItem,
  onArchiveItem,
  onDeleteItem,
  onCreateNode,
  externalVault,
  externalVaultState,
  externalVaultStatus,
  onOpenExternalVault,
  onClearExternalVault,
  onExpandLeft,
  onCollapseLeft,
  compactPath,
}: VaultPanelProps) {
  if (leftCollapsed) {
    return (
      <CollapsedRail
        icon={<Database size={18} aria-hidden="true" />}
        label="Open Vault"
        onExpand={onExpandLeft}
        side="left"
      />
    );
  }

  return (
    <>
      <PanelHeader
        action={
          <button
            className="icon-button panel-collapse-button"
            type="button"
            aria-label="Collapse Vault"
            onClick={onCollapseLeft}
            title="Collapse Vault"
          >
            <ChevronLeft size={16} aria-hidden="true" />
          </button>
        }
        icon={<Database size={17} aria-hidden="true" />}
        title="The Vault"
        subtitle={project ? "SQLite project ready" : "Wings / Halls / Rooms / Drawers"}
      />

      <div className="panel-scroll vault-scroll">
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
              ? `${compactPath(project.projectPath)} - ${tree.itemCount} items`
              : projectError ?? "The desktop shell will create a .grimoire folder here."}
          </small>
        </div>

        <div className="search-summary">
          <strong>Build your Vault hierarchy</strong>
          <button className="text-button" type="button" onClick={() => onCreateNode("wing")}>
            New Wing
          </button>
        </div>

        <form className="vault-search" onSubmit={onSearch}>
          <Search size={15} aria-hidden="true" />
          <input
            type="search"
            placeholder="Search Vault"
            aria-label="Search Vault"
            value={searchQuery}
            onChange={(event) => onSearchChange(event.target.value)}
          />
        </form>

        {searchQuery && searchResults.length > 0 ? (
          <div className="search-summary">
            <strong>{searchResults.length ? `${searchResults.length} local matches` : "No local matches"}</strong>
            <button className="text-button" type="button" onClick={onSearchClear}>
              Clear
            </button>
          </div>
        ) : null}
        {searchError ? <p className="inline-error">{searchError}</p> : null}


        <div className="external-vault-card">
          <div className="search-summary external-vault-header">
            <strong>External Vault YAML</strong>
            <span className={`operation-status ${externalVaultState}`}>{externalVaultStatus}</span>
          </div>
          <div className="external-vault-actions">
            <button className="text-button" type="button" onClick={onOpenExternalVault}>
              Open YAML
            </button>
            {externalVault ? (
              <button className="text-button" type="button" onClick={onClearExternalVault}>
                Clear
              </button>
            ) : null}
          </div>
          {externalVault ? (
            <div className="external-vault-tree">
              <small>{compactPath(externalVault.sourceFile)}</small>
              {externalVault.wings.map((wing) => (
                <details key={`${externalVault.sourceFile}:${wing.name}`} open>
                  <summary>{wing.name} <span>{wing.rooms.length} rooms</span></summary>
                  {wing.rooms.map((room) => (
                    <details key={`${wing.name}:${room.name}`}>
                      <summary>{room.name} <span>{room.drawers.length} drawers</span></summary>
                      <ul>
                        {room.drawers.map((drawer) => (
                          <li key={`${wing.name}:${room.name}:${drawer.name}`}>{drawer.name}</li>
                        ))}
                      </ul>
                    </details>
                  ))}
                </details>
              ))}
            </div>
          ) : null}
        </div>
        <VaultTree
          tree={tree}
          activeItemId={activeItem?.id ?? ""}
          expandedNodeIds={expandedNodeIds}
          onArchiveItem={onArchiveItem}
          onCreateNode={onCreateNode}
          onToggle={onToggle}
          onSelectItem={onSelectItem}
        />
      </div>
    </>
  );
}
