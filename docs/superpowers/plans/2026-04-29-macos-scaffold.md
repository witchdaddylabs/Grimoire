# macOS Scaffold Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the smallest macOS-first Grimoire shell that proves Tauri, React, Tailwind, and the Rust command boundary.

**Architecture:** The first scaffold is a Tauri v2 desktop shell with a Vite/React frontend and one Rust command, `app_ping`, used to confirm the frontend is running inside Tauri. The shell renders static demo data from the existing project docs and intentionally defers SQLite, import, retrieval, Ollama, and exports to later milestones.

**Tech Stack:** Tauri v2, Vite, React, TypeScript, Tailwind CSS v4, Rust, macOS.

---

### Task 1: Bootstrap Runnable Shell

**Files:**
- Create: `package.json`
- Create: `index.html`
- Create: `vite.config.ts`
- Create: `tsconfig.json`
- Create: `tsconfig.node.json`
- Create: `src/main.tsx`
- Create: `src/app/App.tsx`
- Create: `src/app/demoData.ts`
- Create: `src/styles/global.css`
- Create: `src/vite-env.d.ts`
- Create: `src-tauri/Cargo.toml`
- Create: `src-tauri/build.rs`
- Create: `src-tauri/src/main.rs`
- Create: `src-tauri/tauri.conf.json`
- Create: `src-tauri/capabilities/default.json`

- [x] **Step 1: Create Vite/React package scripts**

Add `dev`, `build`, `preview`, and `tauri` scripts to `package.json`.

- [x] **Step 2: Create the React entrypoint**

Render `src/app/App.tsx` from `src/main.tsx`.

- [x] **Step 3: Create a static Grimoire shell**

Build a three-panel desktop surface: Palace, Canvas, and Co-Writer.

- [x] **Step 4: Create the Tauri Rust command boundary**

Add an `app_ping` command in `src-tauri/src/main.rs`.

### Task 2: macOS Run Path

**Files:**
- Create: `script/build_and_run.sh`
- Create: `.codex/environments/environment.toml`
- Modify: `README.md`

- [x] **Step 1: Add project-local run script**

`script/build_and_run.sh` kills stale Grimoire processes, checks for Rust/npm, installs Node dependencies if missing, and runs `npm run tauri dev`.

- [x] **Step 2: Wire the Codex Run action**

`.codex/environments/environment.toml` points the Run action at `./script/build_and_run.sh`.

- [x] **Step 3: Document Rust as a build prerequisite**

`README.md` calls out that Tauri cannot compile until `cargo` and `rustc` are on PATH.

### Task 3: Verification

**Files:**
- Generated: `package-lock.json`

- [x] **Step 1: Install Node dependencies**

Run:

```bash
npm install
```

Expected: `node_modules/` and `package-lock.json` are created.

- [x] **Step 2: Verify frontend build**

Run:

```bash
npm run build
```

Expected: TypeScript passes and Vite writes `dist/`.

- [x] **Step 3: Verify Tauri blocker**

Run:

```bash
./script/build_and_run.sh --verify
```

Expected until Rust is installed: fails with `cargo was not found on PATH`.

### Next Milestone

After Rust is installed, run `./script/build_and_run.sh --verify`.

M1 visual shell and the first M2 local project/SQLite command slice are now underway in the scaffold. The Rust command boundary includes `.grimoire` project creation, `metadata.json`, `grimoire.sqlite`, schema migration `1`, and demo Palace seed data.
