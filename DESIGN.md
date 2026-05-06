---
name: Grimoire
description: Local-first writing studio with memory for fiction writers.
colors:
  charcoal-void: "#050607"
  charcoal: "#090b0d"
  graphite-panel: "#111417"
  graphite-panel-soft: "#171a1d"
  graphite-raised: "#1c2023"
  text-primary: "#e8e5dd"
  text-secondary: "#b8bbb4"
  text-muted: "#7f847d"
  text-faint: "#5f645e"
  bronze-accent: "#b88762"
  bronze-bright: "#d3aa7c"
  ivory-canvas: "#f1eadf"
  ivory-ink: "#242220"
  ward-emerald: "#10b981"
  danger: "#ef4444"
  warning: "#f59e0b"
typography:
  display:
    fontFamily: "Merriweather, Crimson Pro, Georgia, serif"
    fontSize: "48px"
    fontWeight: 500
    lineHeight: 1.1
    letterSpacing: "0"
  manuscript:
    fontFamily: "Merriweather, Crimson Text, Crimson Pro, Georgia, serif"
    fontSize: "20px"
    fontWeight: 400
    lineHeight: 1.85
    letterSpacing: "0"
  telemetry:
    fontFamily: "JetBrains Mono, IBM Plex Mono, ui-monospace, SFMono-Regular, Menlo, monospace"
    fontSize: "13px"
    fontWeight: 400
    lineHeight: 1.5
    letterSpacing: "0"
  label:
    fontFamily: "JetBrains Mono, IBM Plex Mono, ui-monospace, SFMono-Regular, Menlo, monospace"
    fontSize: "11px"
    fontWeight: 600
    lineHeight: 1.4
    letterSpacing: "0.12em"
rounded:
  sm: "6px"
  md: "8px"
  lg: "12px"
  panel: "16px"
spacing:
  xs: "4px"
  sm: "8px"
  md: "16px"
  lg: "24px"
  xl: "32px"
components:
  button-primary:
    backgroundColor: "{colors.graphite-raised}"
    textColor: "{colors.text-primary}"
    rounded: "{rounded.lg}"
    padding: "12px 20px"
  button-secondary:
    backgroundColor: "{colors.graphite-panel-soft}"
    textColor: "{colors.text-primary}"
    rounded: "{rounded.lg}"
    padding: "10px 16px"
  input:
    backgroundColor: "{colors.charcoal-void}"
    textColor: "{colors.text-primary}"
    rounded: "{rounded.lg}"
    padding: "10px 12px"
  status-chip:
    backgroundColor: "{colors.graphite-panel-soft}"
    textColor: "{colors.text-secondary}"
    rounded: "{rounded.panel}"
    padding: "6px 10px"
---

# Design System: Grimoire

## 1. Overview

**Creative North Star: "The Private Manuscript Chamber"**

Grimoire is digital dark academia shaped into a serious local writing instrument. It should feel like a private manuscript chamber, spatial archive, and local AI console for writers who care about canon, voice, and ownership. The atmosphere is dark, literary, precise, tactile, and quietly premium.

The design serves long writing sessions. It should be calm enough for prose, dense enough for memory and retrieval, and theatrical only where the product language benefits from it. The brand can use occult and archive motifs, but the interaction model stays practical.

It explicitly rejects generic SaaS dashboard aesthetics, bloated AI writing suites, startup hype, childish wizard roleplay copy, game UI excess, neon cyberpunk, pastel gradients, and standard admin panel layouts.

**Key Characteristics:**

- Local-first trust signals are visible without becoming marketing.
- The Canvas gets the strongest visual priority.
- The Palace feels spatial and archival, not like a plain file tree.
- The Co-Writer feels useful and cited, not magical in a hand-wavy way.
- Motion is restrained, with status changes and focus transitions only.

## 2. Colors

The palette is charcoal, graphite, near-black, manuscript text, tertiary bronze, and ward emerald. The structural system is dark neutral by default: app chrome, panels, sidebars, cards, controls, and the Canvas frame must be carried by charcoal and graphite tones.

Bronze, sepia, and amber are decorative flourish colours only. Use them for fine dividers, subtle borders, small brand accents, selected-state details, and rare ornamental glyphs. They must not become the dominant panel, button, or text system.

Emerald remains reserved for local model status, retrieval activity, successful save state, memory confidence, and AI process indicators.

### Primary

- **Charcoal Void**: Deep application background, outer shell, and quiet negative space.
- **Graphite Panel**: Sidebars, Co-Writer messages, settings surfaces, cards, and compact controls.
- **Manuscript Text**: High-contrast prose, active titles, and user-authored writing.

### Secondary

- **Bronze Accent**: Tertiary brand detail, selected-state edge, fine rules, and focus accents. Never use it as the dominant UI fill.
- **Ward Emerald**: Local connection status, retrieval progress, successful save status, memory confidence, and guardrail pass states. It is not a general accent.
- **Danger Red**: Banned phrase warnings, failed saves, destructive actions, and provider errors.

### Neutral

- **Charcoal**: Deep application background and insets.
- **Graphite Panel**: Sidebars, Co-Writer messages, settings surfaces, and compact panels.
- **Graphite Raised**: Hovered or active surfaces that need tactile lift.
- **Text Primary**: Canvas prose, active titles, and high-priority text.
- **Text Secondary**: Metadata, secondary labels, and non-primary controls.
- **Text Muted**: Disabled copy, secondary hints, and low-priority status text.

### Named Rules

**The Emerald Rarity Rule.** Emerald only means local, connected, saved, retrieved, or passed. If it is decorative, it is wrong.

**The Bronze Rarity Rule.** Bronze only means brand detail, selected state, fine divider, or focused control. If it carries whole panels or body text, it is wrong.

**The Calm Graphite Rule.** Use graphite and charcoal as the main surface language. Avoid brown or bronze dominance, and avoid generic Tailwind slate drift.

## 3. Typography

**Canvas Default Font:** Merriweather with Georgia fallback.
**Canvas Alternate Font:** Crimson Text or Crimson Pro with Georgia fallback.
**Label/Mono Font:** JetBrains Mono or IBM Plex Mono with system monospace fallback.

**Character:** The pairing separates manuscript from telemetry. Merriweather carries long-form prose by default. Crimson Text or Crimson Pro can be offered as an alternate literary manuscript mode. Highly decorative display serifs are not body fonts; reserve them for splash screens or rare major headings only.

### Hierarchy

- **Display** (500, 40px to 48px, 1.1): Canvas titles, onboarding headings, and major literary moments.
- **Headline** (500, 28px to 40px, 1.15): Panel-level headings and first-run screens.
- **Title** (600, 18px to 22px, 1.3): Active item names, source cards, settings sections, and component titles.
- **Body** (400, 16px to 20px, 1.65 to 1.85): Prose, onboarding body, and editor content. Canvas lines should stay comfortable and never exceed 65 to 75 characters.
- **Label** (600, 11px to 13px, 0.12em uppercase where needed): Navigation labels, metadata, retrieval statuses, paths, and settings labels.

### Named Rules

**The Manuscript First Rule.** Serif prose must be more comfortable than every surrounding control. If the UI chrome competes with the writing, reduce the chrome.

**The Uppercase Sparingly Rule.** Uppercase telemetry labels are allowed, but never as paragraph text or long instructions.

## 4. Elevation

Grimoire uses tonal layering, graphite borders, and restrained status glows more than conventional shadows. Surfaces should feel inset into a dark writing chamber. Shadows are reserved for major shells, hover lift, and emerald process emphasis.

### Shadow Vocabulary

- **Deep Shell** (`0 28px 80px rgba(0, 0, 0, 0.45)`): Main app shell and prototype-level framed surfaces.
- **Bronze Detail** (`inset 0 0 0 1px rgba(184, 135, 98, 0.08)`): Selected rows, fine dividers, and tiny brand accents.
- **Emerald Pulse** (`0 0 22px rgba(16, 185, 129, 0.32)`): Connection and retrieval indicators only.

### Named Rules

**The Inset Chamber Rule.** Panels feel built into the surface. Avoid bright floating cards and generic heavy drop shadows.

## 5. Components

### Buttons

Buttons are tactile and compact. Primary buttons use graphite fill, manuscript text, and a quiet bronze edge only where hierarchy needs it. Secondary buttons use graphite surfaces and subtle neutral borders. Ghost buttons are quiet and should only appear when the surrounding layout already makes the action obvious.

- **Shape:** Gently curved corners (12px radius).
- **Primary:** Graphite fill, high-contrast text, bold mono label, 12px vertical padding, 20px horizontal padding.
- **Hover / Focus:** Slight brightness lift, bronze focus ring, no bounce, no layout shift.
- **Secondary / Ghost:** Graphite fill or transparent background, manuscript text, thin neutral border.

### Chips

Chips are status-bearing, not decorative. Emerald chips mean local, retrieval, memory confidence, or successful state. Bronze chips mean selected detail only. Danger chips mean flagged terms or blocked actions.

- **Style:** Full border, subtle tinted background, mono label.
- **State:** Selected chips should change both border strength and background tint, not color alone.

### Cards / Containers

Cards are used for repeated items, source citations, Co-Writer messages, and onboarding choices. Page sections and major panels should not be styled as floating cards.

- **Corner Style:** Compact rounded surfaces (8px to 16px radius).
- **Background:** Graphite panel or panel-soft, never pure black and never bright white.
- **Shadow Strategy:** Mostly tonal layering and borders; use glows only for meaningful active states.
- **Border:** Thin neutral border by default. Bronze only for selected or focused details.
- **Internal Padding:** 12px to 24px depending on density.

### Inputs / Fields

Inputs should feel like dark inset controls inside the app shell. Labels must be visible and must not rely on placeholders.

- **Style:** Charcoal background, manuscript text, neutral border, 12px radius.
- **Focus:** Bronze focus outline with 2px offset.
- **Error / Disabled:** Danger or muted parchment paired with explicit text, never color alone.

### Navigation

Navigation is archival and spatial. The Palace tree should show hierarchy clearly through indentation, disclosure state, item type, and active selection. It must not look like a generic file explorer.

- **Style:** Mono metadata, compact row height, warm hover tint, visible active item.
- **Mobile Treatment:** Collapse side panels before compressing the Canvas. The writing surface stays readable first.

### Source Citation Cards

Citation cards prove that the Co-Writer retrieved real Palace memory. Each card should show source title, item type, Palace path, and a short excerpt. Confidence belongs near the answer, not hidden in settings.

## 6. Do's and Don'ts

### Do:

- **Do** keep the Canvas visually dominant over Palace and Co-Writer panels.
- **Do** use charcoal and graphite as the structural UI system.
- **Do** use bronze only for fine borders, selected-state details, small brand accents, and focus indicators.
- **Do** reserve ward emerald for local status, retrieval progress, save success, memory confidence, and guardrail pass states.
- **Do** offer an optional ivory manuscript mode for users who prefer a lighter long-form writing surface.
- **Do** show source paths and citations whenever the Co-Writer uses Palace memory.
- **Do** use visible labels for inputs and accessible names for icon-only buttons.
- **Do** make project ownership visible through local save state, export actions, and privacy copy.
- **Do** support keyboard focus with a visible brass outline.

### Don't:

- **Don't** use a generic SaaS dashboard aesthetic.
- **Don't** build a bloated AI writing suite.
- **Don't** use startup hype or generic AI productivity copy.
- **Don't** write childish wizard roleplay copy or long fantasy paragraphs.
- **Don't** use fake urgency or SaaS upgrade language.
- **Don't** make the app feel like a developer tool.
- **Don't** use a standard admin panel, game UI, bright cyberpunk overload, pastel gradient marketing, or cartoon fantasy styling.
- **Don't** use blue as the primary action color.
- **Don't** make the Co-Writer dominate the Canvas.
- **Don't** use brown, bronze, sepia, or amber as the dominant panel or text system.
- **Don't** use decorative display serifs for Canvas body writing.
- **Don't** overuse emoji inside the app UI.
- **Don't** introduce unnecessary component libraries or heavy design system packages.
