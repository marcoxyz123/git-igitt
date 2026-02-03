# Draft: GitLab Pipeline Visualization for git-igitt

## Requirements (confirmed)
- Migrate from `tui` v0.19 to `ratatui` (maintained fork)
- Apply Nord theme colors across entire UI
- Add GitLab pipeline panel showing stages as columns
- Jobs within each stage with Nerdfont status icons
- Status icons: success (green ✓), failed (red ✗), running (spinner), pending (clock)
- Auto-scroll for large pipelines
- Layout: 50:50 vertical split with pipeline panel below existing UI
- Config: GitLab URL + PAT in `~/.config/git-igitt/gitlab.toml`
- Pipeline selection based on currently selected commit in graph view

## Technical Decisions (Confirmed)
- **API Strategy**: Async with tokio - Non-blocking, UI stays responsive during API calls
- **Empty Pipeline**: Toggleable panel (e.g., 'P' key) - Hidden by default if no pipeline
- **Panel Layout**: Dynamic height, auto-size based on content but max 2/3 of terminal to preserve git graph space
- **Error Handling**: Show error in pipeline panel with retry option ('r' key to refresh)
- **Caching**: In-memory cache by commit SHA, cleared on manual refresh

## Research Findings

### Current Codebase Structure
- Current deps: tui 0.19, crossterm 0.29, git2 0.15
- 18 source files across src/, src/widgets/, src/util/
- Uses StatefulWidget pattern for widgets
- Builder pattern for widget configuration

### Widget Architecture (from explore agent)
- **Two-struct pattern**: Widget struct (rendering config) + State struct (data)
- **StatefulWidget trait**: `type State = WidgetNameState; fn render(self, area, buf, state)`
- **Widget wrapper**: Implements Widget by delegating to StatefulWidget with default state
- **Builder pattern**: All methods take `mut self`, return `WidgetName<'a>`, enable chaining

### App State Management (from explore agent)
- **App struct** contains all view states: `graph_state`, `commit_state`, `diff_state`, etc.
- **ActiveView enum**: Branches, Graph, Commit, Files, Diff, Models, Search, Help
- **Navigation**: Left/Right arrows cycle through views
- **Data loading**: Lazy - commit details loaded on selection change
- **Refresh**: Manual ('r' key) or automatic change detection

### UI Layout System (from explore agent)
- **Hierarchical Layout**: Screen → top_chunks → chunks → right_chunks
- **Constraint types**: Length(n), Percentage(n), Min(n)
- **Current structure**:
  - Branches panel: 25 cols if visible
  - Graph panel: 50%
  - Right area: Commit (50%) + Files (50%)
- **To add pipeline**: Split vertically first, then apply existing horizontal split

### tui → ratatui Migration (from librarian)
- **Simple rename**: `tui::` → `ratatui::` for most imports
- **Breaking change**: `Spans` → `Line` (v0.24.0)
- **Drop-in option**: `tui = { package = "ratatui", ... }` requires no code changes
- **Current crossterm 0.29 is compatible** with ratatui 0.30

### GitLab API (from librarian)
- **Auth**: `PRIVATE-TOKEN` header with PAT
- **Pipelines endpoint**: `GET /projects/:id/pipelines?sha=<commit_sha>`
- **Jobs endpoint**: `GET /projects/:id/pipelines/:pipeline_id/jobs`
- **Response includes**: id, status, ref, sha, stages, jobs with status
- **Job statuses**: created, pending, running, success, failed, canceled, skipped, manual

## Open Questions (Resolved)
1. ✅ Async with tokio for responsive UI
2. ✅ Toggleable panel, hidden by default if no pipeline
3. ✅ Yes, toggleable with 'P' key
4. ✅ In-memory cache by commit SHA
5. ✅ Show error in panel, retry with 'r' key
6. ✅ GitLab-only for now (GitHub Actions out of scope)

## Test Strategy Decision
- **Infrastructure exists**: NO
- **User wants tests**: NO (manual verification only)
- **QA approach**: Manual verification via running the app

## Scope Boundaries
- INCLUDE: tui→ratatui migration, Nord theme, GitLab pipeline panel
- EXCLUDE: GitHub Actions, other CI systems (for now)

## Nord Color Palette Reference
```
nord0  #2E3440 (background)
nord1  #3B4252 (elevated surfaces)
nord2  #434C5E (subtle surfaces)
nord3  #4C566A (borders)
nord4  #D8DEE9 (text primary)
nord5  #E5E9F0 (text secondary)
nord6  #ECEFF4 (text bright)
nord7  #8FBCBB (cyan/frost)
nord8  #88C0D0 (bright cyan)
nord9  #81A1C1 (blue)
nord10 #5E81AC (dark blue)
nord11 #BF616A (red - error/failed)
nord12 #D08770 (orange - warning)
nord13 #EBCB8B (yellow - pending)
nord14 #A3BE8C (green - success)
nord15 #B48EAD (purple)
```

## Files to Modify
- Cargo.toml - update deps
- src/main.rs - imports
- src/ui.rs - layout, rendering
- src/app.rs - pipeline state
- src/lib.rs - exports
- src/widgets/*.rs - imports

## Files to Create
- src/theme.rs - Nord colors
- src/gitlab/mod.rs - API client
- src/gitlab/models.rs - API types  
- src/widgets/pipeline_view.rs - pipeline widget
- src/config.rs - GitLab credentials
