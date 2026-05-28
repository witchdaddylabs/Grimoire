// src/components/types.ts
// Shared types used across feature modules

import type { ReactNode } from "react";

export type SaveState = "idle" | "editing" | "saving" | "saved" | "failed" | "preview";
export type AsyncState = "idle" | "working" | "success" | "failed";
export type OnboardingStep = "welcome" | "vault" | "feed" | "engine" | "wards" | "canvas";
export type ToolSectionId = "feed" | "retrieval" | "engine" | "cowriter" | "wards" | "about";

export interface OnboardingState {
  complete: boolean;
  step: OnboardingStep;
}

export type OnboardingStore = Record<string, OnboardingState>;

export interface WorkspacePrefs {
  leftCollapsed: boolean;
  rightCollapsed: boolean;
  openToolSections: ToolSectionId[];
  theme: "dark" | "ivory";
}
