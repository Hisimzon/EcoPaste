# Local AI Working Constraints

> Scope: this document is a local overlay for the personal `my-next` workflow.
> It is intentionally separate from upstream-maintained files to reduce merge
> conflicts when syncing `upstream/master`.

## Priority

Follow instructions in this order:

1. The current user request in the active conversation.
2. Repository rules in `AGENTS.md`.
3. Trellis workflow and specs in `.trellis/workflow.md` and `.trellis/spec/**`.
4. Platform-specific project files under `.agents/`, `.codex/`, `.cursor/`,
   `.claude/`, `.gemini/`, `.opencode/`, `.kiro/`, and `.github/`.
5. This local overlay document.
6. External or global skills and notes.

If this file conflicts with `AGENTS.md` or `.trellis/**`, treat the repository
rules as the source of truth unless the user explicitly says otherwise.

## Skill Usage

- Do not use the global `eco-paste` skill as implementation rules for this
  Rust-first rewrite branch.
- The global `eco-paste` skill may be used only as business/background
  reference, and only after checking that it does not conflict with this
  repository's `AGENTS.md` and Trellis specs.
- For code changes, read the relevant repository-local rules first:
  `AGENTS.md`, `.trellis/workflow.md`, and the matching files under
  `.trellis/spec/**`.

## Upstream Sync Discipline

- Treat `upstream = https://github.com/EcoPasteHub/EcoPaste.git` as the official
  upstream source.
- Keep local `next` as a clean mirror of `upstream/master`.
- Do personal development on `my-next`, then merge or rebase from `next` after
  syncing upstream.
- Push personal work to `origin/my-next`.
- Avoid editing upstream-owned documentation files only to store personal AI
  preferences. Put those notes in this document or another clearly local file.

## Dirty Worktree Rules

- Do not revert or overwrite local changes that were not made in the current
  task.
- Before touching an already modified file, inspect the diff and preserve the
  existing intent.
- Keep documentation-only changes separate from source/config changes when
  possible so commits remain reviewable.

## Usage Note

This file is not automatically loaded by all AI tools. For future sessions,
reference it explicitly when needed, for example:

```text
请同时参考 docs/local-ai-working-constraints.md，但 AGENTS.md 和 .trellis/** 优先。
```
