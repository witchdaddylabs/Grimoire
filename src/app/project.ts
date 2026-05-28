import { invoke } from "@tauri-apps/api/core";

export type ProjectMetadata = {
  name: string;
  appVersion: string;
  schemaVersion: number;
  projectPath: string;
  databasePath: string;
  createdAt: string;
  updatedAt: string;
};

export type RecentProject = {
  name: string;
  path: string;
  lastOpenedAt: string;
};

const RECENT_PROJECTS_KEY = "grimoire.recent.projects";
const MAX_RECENT_PROJECTS = 10;

/**
 * Create a new project with the given name in the specified directory.
 */
export async function createProject(name: string, parentDir?: string): Promise<ProjectMetadata> {
  return invoke<ProjectMetadata>("project_create", {
    request: {
      name,
      parentDir: parentDir ?? null,
      seedDemoData: false,
    },
  });
}

/**
 * Create a new project pre-loaded with demo data for exploration.
 */
export async function createDemoProject(): Promise<ProjectMetadata> {
  return invoke<ProjectMetadata>("project_create", {
    request: {
      name: "Grimoire Demo",
      parentDir: null,
      seedDemoData: true,
    },
  });
}

/**
 * Open an existing .grimoire project by its folder path.
 */
export async function openProject(projectPath: string): Promise<ProjectMetadata> {
  return invoke<ProjectMetadata>("project_open", { projectPath });
}

/**
 * Record a project as recently opened and return the updated list.
 */
export function recordRecentProject(metadata: ProjectMetadata): RecentProject[] {
  const recent = getRecentProjects();
  const filtered = recent.filter((entry) => entry.path !== metadata.projectPath);
  const entry: RecentProject = {
    name: metadata.name,
    path: metadata.projectPath,
    lastOpenedAt: new Date().toISOString(),
  };
  const updated = [entry, ...filtered].slice(0, MAX_RECENT_PROJECTS);
  localStorage.setItem(RECENT_PROJECTS_KEY, JSON.stringify(updated));
  return updated;
}

/**
 * Get the list of recently opened projects.
 */
export function getRecentProjects(): RecentProject[] {
  try {
    const raw = localStorage.getItem(RECENT_PROJECTS_KEY);
    if (!raw) return [];
    return JSON.parse(raw) as RecentProject[];
  } catch {
    return [];
  }
}

/**
 * Remove a project from the recent list.
 */
export function removeRecentProject(path: string): void {
  const recent = getRecentProjects().filter((entry) => entry.path !== path);
  localStorage.setItem(RECENT_PROJECTS_KEY, JSON.stringify(recent));
}

export function describeError(error: unknown) {
  if (typeof error === "string") return error;
  if (error instanceof Error) return error.message;
  return "Unknown project storage error";
}

export function compactPath(path: string) {
  const home = "/Users/";
  if (!path.startsWith(home)) return path;

  const [, user, ...rest] = path.slice(1).split("/");
  if (!user || rest.length === 0) return path;

  return `~/${rest.join("/")}`;
}
