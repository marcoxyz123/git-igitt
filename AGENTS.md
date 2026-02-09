# git-igitt+ — Interactive Git History TUI with CI/CD Pipeline Integration

> Global standards from `~/.config/opencode/AGENTS.md` apply automatically.

## Project Context

- **Stack**: Rust 2021, MSRV 1.83.0, TUI (ratatui/crossterm), git2, syntect, reqwest
- **Type**: Desktop CLI/TUI — no web server, no REST API
- **Remotes**: `origin` (github/git-bahn), `gitlab` (gitlab.berger.sx/mac — primary CI), `fork` (github/marcoxyz123)

## Build Commands

| Task | Command |
|------|---------|
| Build | `cargo build --release` |
| Lint | `cargo clippy --all --all-targets -- --deny warnings` |
| Test | `cargo test --all` |
| Format | `cargo fmt --all -- --check` |

## Commit Scopes

`app`, `ui`, `widgets`, `gitlab`, `util`, `theme`, `settings`, `ci`

## Architecture

- Entry: `main.rs` (CLI + event loop) → `app.rs` (state/logic) → `ui.rs` (rendering)
- Widgets: `widgets/` (graph, commit, diff, files, branches, pipeline, list, models)
- GitLab: `gitlab/` (API client + models), `gitlab_config.rs` (config dialog)
- Error pattern: `Result<T, String>`, `.map_err(|e| e.to_string())?`
- Builder pattern for widgets, `StatefulWidget` trait, Nord color theme
- Imports: std → external → `crate::` (explicit, no globs)

## Serena Activation (MANDATORY)

```
mcp_serena_activate_project(project="git-igitt")
mcp_serena_check_onboarding_performed()
mcp_serena_list_memories()
mcp_serena_read_memory("project-overview")
```

<!-- WASPWAY-STANDARDS-START -->
## WASPWAY Standards (auto-managed by /ww-init)

### Security
- No secrets in code (GitLab tokens via `.git-igitt.toml` gitignored or env), validate all input
- Dependency audits: `cargo audit`, no `unsafe` without justification

### Testing
- Bugfix = regression test, never delete/skip failing tests, new features need tests
- 70/20/10 ratio (unit/integration/e2e), mock external only (GitLab API)

### Error Handling
- Retry with backoff for GitLab API calls, timeout all HTTP requests
- Graceful degradation: pipeline panel works offline, git ops never crash terminal

### CI/CD
- Pipeline: lint → test → build → package (`.gitlab-ci.yml` + `.github/workflows/`)
- No manual skip of quality gates, `cargo audit` in CI, CHANGELOG.md with releases

### Documentation
- All docs in `docs/` (Diataxis), README = hub (max 150 lines), Mermaid for diagrams
- ADRs for architecture decisions, CHANGELOG in same PR as code

Full standards: https://gitlab.berger.sx/ai/OPENCODE
<!-- WASPWAY-STANDARDS-END -->
