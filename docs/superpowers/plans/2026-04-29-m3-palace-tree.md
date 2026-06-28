# M3 Palace Tree Plan

> _Historical milestone plan (completed). In this plan "Palace" is today's "Vault". The current roadmap is [get-it-working-plan](../../get-it-working-plan.md)._

Goal: render The Palace hierarchy from SQLite instead of a flat static list.

## Scope

- [x] Add `db_get_palace_tree` Tauri command.
- [x] Return nested Wings, Halls, Rooms, Drawers, and Items from Rust.
- [x] Derive Palace item paths from hierarchy names.
- [x] Keep browser fallback demo data.
- [x] Add Palace tree TypeScript types.
- [x] Render a collapsible sidebar tree.
- [x] Support active item selection from tree leaves.
- [x] Add item type badges.
- [x] Add search input placeholder.
- [x] Add empty state copy.

## Deferred

- [ ] Native project open/create UI.
- [ ] Search filtering inside the tree.
- [ ] Node creation flows.
- [ ] Persist expanded/collapsed state.
- [ ] Canvas save/autosave commands.

## Verification

- [x] `npm run build`
- [ ] `./script/build_and_run.sh --verify` once Rust is installed.
