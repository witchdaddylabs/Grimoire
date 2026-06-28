# M2 Local Project + SQLite Plan

> _Historical milestone plan (completed). In this plan "Palace" is today's "Vault". The current roadmap is [get-it-working-plan](../../get-it-working-plan.md)._

Goal: create the first real desktop persistence foundation for Grimoire.

## Scope

- [x] Add Rust command types for project metadata.
- [x] Implement `project_create`.
- [x] Implement `project_open`.
- [x] Implement `project_get_metadata`.
- [x] Implement `db_init`.
- [x] Create `.grimoire` folder shape.
- [x] Write `metadata.json`.
- [x] Create `grimoire.sqlite`.
- [x] Add schema migration table.
- [x] Apply initial Palace schema.
- [x] Enable SQLite foreign keys.
- [x] Seed a tiny demo Palace dataset.
- [x] Surface local project state in the React shell.

## Deferred

- [ ] Native folder picker.
- [ ] Recent project tracking.
- [x] `db_get_palace_tree`.
- [ ] Reading the sidebar tree from SQLite instead of static demo data.
- [ ] Canvas save/autosave commands.

## Verification

- [ ] `npm run build`
- [ ] `./script/build_and_run.sh --verify`
