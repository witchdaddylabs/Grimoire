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

export async function createDemoProject() {
  return invoke<ProjectMetadata>("project_create", {
    request: {
      name: "Grimoire Demo",
      seedDemoData: true,
    },
  });
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
