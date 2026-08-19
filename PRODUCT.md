# Product

## Register

product

## Users

Grimoire is for fiction writers, storytellers, and world-builders who want a private place to write long-form prose, manage lore and canon, and use AI without losing ownership or voice.

The primary user may not be technical. They want the benefits of local AI and structured memory, but the app must not feel like a developer tool, SaaS dashboard, or model-management console. Their context is quiet, focused creative work, often with unpublished material they need to keep local and under their control.

## Product Purpose

Grimoire is a local-first writing studio with memory. It combines a long-form Canvas, a structured spatial archive called the Grimoire Vault, a Story Plan layer that keeps structure and prose aligned, a local AI Co-Writer that retrieves canon before answering, and wards that flag banned words, repeated phrases, cliché phrasing, and voice drift.

MVP success means proving one complete vertical slice:

Open the app, create a new project, feed the Vault writing, store the work locally, write in the Canvas, connect Ollama, ask the Co-Writer, retrieve Vault memory, show citations, scan banned words, insert or reject output, and export Markdown.

The Canvas remains the source of truth. AI is optional, cited, interruptible, and never allowed to dominate the writing workflow.

## Story Plan

The Story Plan is the structural layer: Plan → Scenes → Beats. It exists because the failure mode of AI-assisted revision is **drift** — you regenerate a scene and it quietly contradicts the chapter before it, or discards the one line you actually wanted to keep.

Three commitments define it:

1. **The writer pins what matters.** Any beat can be locked. Locked beats are hard constraints in the prompt and cannot be regenerated directly. Structure is the writer's to protect, not the model's to renegotiate.

2. **Revision is convergent, not stochastic.** Every regeneration carries an edit instruction plus the context appropriate to the layer being revised. Scene and beat targets get six points — logline and synopsis, Vault character facts, the adjacent scene anchors on both sides, locked beats, the current material, and the instruction. Plan targets get the full scene-by-scene outline plus every locked beat in the plan, scoped to its owning scene; adjacent anchors and per-character facts don't apply when the outline itself is what's under revision. Either way the model revises what exists; it does not start over.

3. **Nothing lands without a decision.** Variants are generated, ward-scanned, and stored as candidates. The writer compares and accepts. A candidate carrying a blocking ward cannot be accepted. Scanning can be turned off per run, in which case the candidate is labelled unscanned rather than clean — the UI never reports a protection that did not run. Rejected candidates are retained as history.

Story Plan is positioned as a **structural editor, not a content generator**. It does not write the book. It refuses to let the scaffolding rot while the writer does.

## Brand Personality

Grimoire is made by Witch Daddy Labs. The personality is intelligent, cheeky, writerly, practical, lightly magical, and edged with Australian irreverence.

The central tone rule is: magic in the labels, clarity in the instructions. The UI can wink, but it should not giggle. It should feel useful, trustworthy, serious enough for real writers, and never fake-spiritual.

Good product language includes "Feed the Vault", "Set the wards", "Word essence", "Consulting the Vault", "Reading canon traces", "Slop wards active", "Banish word", "The Vault is quiet", "The Co-Writer is asleep", and "Your words stay local".

## Anti-references

Do not make Grimoire look or feel like:

- a generic SaaS dashboard aesthetic
- a bloated AI writing suite
- startup hype
- generic AI productivity copy
- childish wizard roleplay copy
- long fantasy paragraphs
- fake urgency
- SaaS upgrade language
- a developer tool
- a standard admin panel
- a game UI
- bright cyberpunk overload
- pastel gradient marketing
- cartoon fantasy styling
- emoji-heavy UI
- cloud-first manuscript tracking
- publishing marketplace software
- enterprise collaboration software

## Design Principles

1. Local ownership is visible. The interface should repeatedly reassure the writer that files, memory, exports, and keys are under their control.

2. Writing comes first. The Canvas, active document, save state, and prose comfort outrank AI panels, citations, and metadata.

3. Memory must be inspectable. Retrieval should show source paths, confidence, and citations so the Co-Writer feels grounded rather than mystical.

4. Magic labels need plain instructions. Product terms can be theatrical, but actions, errors, privacy copy, and setup guidance must stay practical.

5. Build the smallest honest loop. Do not overbuild cloud sync, accounts, marketplace features, bundled models, heavy vector stacks, or complex provider systems before the MVP proves the core workflow.

## Accessibility & Inclusion

Target practical WCAG AA behavior for the MVP. The app must support keyboard navigation, visible focus states, readable contrast in dark mode, scalable text, semantic buttons and inputs, clear labels, no placeholder-only instructions, and reduced-motion consideration where practical.

Do not rely on color alone for status. Icon-only controls need accessible names. Status changes, especially save state, retrieval state, and provider failures, should be announced where practical.
