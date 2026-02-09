---
globs: []
alwaysApply: true
description: "Compaction-resilient reinforcement: --no-ff, branch naming, Serena-first, branch cleanup"
---

<MANDATORY_RULE severity="CRITICAL" priority="HIGHEST">

# WASPWAY — Always-Enforce (Belt-and-Suspenders)

> These reinforce global AGENTS.md rules after context compaction.
> Only rules NOT already enforced by the Sisyphus agent prompt.

1. **Merge with `--no-ff` ALWAYS.** `git merge --no-ff branch-name`. NEVER fast-forward. Repo config enforces via `git config merge.ff false` — but always be explicit.
2. **Delete branch after merge.** `git branch -d branch-name && git push origin --delete branch-name`
3. **Branch naming.** `type/short-description` — prefixes: `feature/`, `fix/`, `perf/`, `hotfix/`, `refactor/`, `docs/`, `chore/`, `ci/`
4. **Conventional commits.** `type(scope): subject` — imperative, lowercase, no period, max 72 chars.
5. **Serena-first for code exploration.** Use `find_symbol`, `get_symbols_overview`, `find_referencing_symbols`, `search_for_pattern` directly. For non-code files, use built-in Grep/Read tools. Serena also supports markdown, yaml, toml, and bash LSPs when configured in `project.yml`.
6. **NEVER use `explore` agents.** The `explore` subagent is redundant — Serena and built-in Grep/Read cover all codebase search needs better. `task(subagent_type="explore")` is a banned pattern.
7. **ALWAYS delegate external library/API lookups to `librarian`.** When you need docs, examples, or API details for ANY dependency (npm, cargo, pip, etc.) — NEVER grep local caches (`~/.cargo/registry`, `node_modules`, `.venv`). Instead: `task(subagent_type="librarian", run_in_background=true, prompt="Find [library] docs for [specific API]")`. Librarian uses Context7, grep.app, and web search — always higher quality than grepping local files.
8. **Serena find → Serena replace (MANDATORY for code).** If you used `find_symbol` or `get_symbols_overview` to read code, you MUST use `replace_symbol_body` to edit it. NEVER fall back to Read → Edit after a Serena find — that wastes a round-trip and risks string-matching failures. The only exception is non-code files (markdown, yaml, config) where Serena has no symbols — use Read → Edit for those.
9. **No secrets in code.** Check `git diff` before committing. Env vars or vault only.
10. **Memory checkpoints.** After completing a major milestone or every ~30 minutes of active work, save progress to Serena memory: `mcp_serena_edit_memory(memory_file_name="active-tasks", ...)` with current todo state. This ensures session crashes don't lose task context.
11. **Serena memory naming convention.** Use these standardized names: `project-overview`, `session-handoff`, `active-tasks`, `learned-patterns`, `learned-mistakes`. Domain memories use kebab-case: `api-patterns`, `deployment-workflow`, `domain-glossary`.

</MANDATORY_RULE>
