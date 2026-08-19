// Guards the workspace grid's structural contract.
//
// WHY THIS EXISTS
// The responsive breakpoints in global.css position the workspace's children as
// grid items — .vault-panel, .canvas-panel and .cowriter-panel. That only works
// if each of those is exactly ONE element.
//
// CoWriterPanel returns a FRAGMENT (PanelHeader + .panel-scroll). When it was
// mounted directly as a grid child those two became SEPARATE grid items: at
// <=1080px the header landed under the Vault and the tool content under the
// Canvas, and no rule could height-bound them together. The `.cowriter-panel`
// selector matched nothing in the DOM, so every responsive rule targeting it
// was silently dead. (Codex P1 on PR #28.)
//
// These assert the invariant, not the pixels: every panel the CSS positions
// must render as a single element.
//
// NOTE: global.css is deliberately NOT imported here. The Tailwind Vite plugin
// intercepts `?raw` on CSS and returns an empty string, so any assertion
// against it silently passes on nothing. CSS-value regressions belong in the
// resize check documented in references/tauri-responsive-layout.md.

import { describe, expect, it } from "vitest";

// Loaded as raw source via Vite so this test needs no Node type definitions.
import appSource from "../app/App.tsx?raw";
import coWriterSource from "../features/cowriter/CoWriterPanel.tsx?raw";

describe("workspace grid structure", () => {
  it("has real source to inspect", () => {
    // Guard against the empty-string trap that made earlier versions of these
    // tests pass vacuously.
    expect(appSource.length).toBeGreaterThan(1000);
    expect(coWriterSource.length).toBeGreaterThan(1000);
  });

  it("wraps CoWriterPanel so it is a single grid item", () => {
    // CoWriterPanel returning a fragment is fine — but it means the call site
    // MUST supply the wrapper element the grid and breakpoints target.
    expect(
      /return \(\s*<>/.test(coWriterSource),
      "CoWriterPanel no longer returns a fragment; re-check whether the " +
        "wrapper in App.tsx is still required",
    ).toBe(true);

    expect(
      /className=\{`cowriter-panel panel/.test(appSource),
      "CoWriterPanel must be wrapped in a .cowriter-panel element, or its " +
        "fragment children become separate workspace grid items",
    ).toBe(true);
  });

  it("keeps a real element for every panel class the responsive CSS targets", () => {
    // Each of these is positioned or height-bounded by a breakpoint rule.
    for (const panel of ["vault-panel", "canvas-panel", "cowriter-panel"]) {
      expect(
        appSource.includes(panel),
        `.${panel} is targeted by responsive CSS but no element renders it`,
      ).toBe(true);
    }
  });

  it("renders the collapsed Co-Writer inside the same wrapper", () => {
    // The collapsed rail is also a child of the wrapper, so it stays one grid
    // item in both states; the wrapper drops its own chrome via .is-collapsed.
    expect(
      appSource.includes("is-collapsed"),
      "the wrapper needs a collapsed variant so the rail isn't double-framed",
    ).toBe(true);
  });
});
