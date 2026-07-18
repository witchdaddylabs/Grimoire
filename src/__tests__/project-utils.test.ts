import { describe, it, expect } from "vitest";
import { describeError, compactPath } from "../app/project";
import { cloudProvider, providerLabels } from "../app/ai";

describe("describeError", () => {
  it("returns string errors as-is", () => {
    expect(describeError("something broke")).toBe("something broke");
  });

  it("extracts message from Error objects", () => {
    expect(describeError(new Error("bad input"))).toBe("bad input");
  });

  it("returns fallback for unknown types", () => {
    expect(describeError(42)).toBe("Unknown project storage error");
    expect(describeError(null)).toBe("Unknown project storage error");
    expect(describeError(undefined)).toBe("Unknown project storage error");
  });
});

describe("compactPath", () => {
  it("replaces /Users/<name>/ with ~/ ", () => {
    expect(compactPath("/Users/billy/Documents/project")).toBe("~/Documents/project");
  });

  it("returns non-/Users/ paths unchanged", () => {
    expect(compactPath("/tmp/project")).toBe("/tmp/project");
  });

  it("handles root /Users/ edge case", () => {
    expect(compactPath("/Users/")).toBe("/Users/");
  });
});

describe("cloudProvider", () => {
  it("returns false for ollama", () => {
    expect(cloudProvider("ollama")).toBe(false);
  });

  it("returns true for cloud providers", () => {
    expect(cloudProvider("openAi")).toBe(true);
    expect(cloudProvider("anthropic")).toBe(true);
    expect(cloudProvider("googleAiStudio")).toBe(true);
    expect(cloudProvider("openAiCompatible")).toBe(true);
  });
});

describe("providerLabels", () => {
  it("has labels for all providers", () => {
    expect(providerLabels.ollama).toBeDefined();
    expect(providerLabels.openAi).toBeDefined();
    expect(providerLabels.anthropic).toBeDefined();
    expect(providerLabels.googleAiStudio).toBeDefined();
  });
});
