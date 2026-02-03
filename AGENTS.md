# AGENTS.md - git-igitt

Interactive Git terminal application for browsing and visualizing Git history graphs.

## Project Overview

- **Language**: Rust (Edition 2021)
- **MSRV**: 1.83.0
- **Type**: Terminal UI (TUI) application
- **Key Dependencies**: `git2`, `tui`, `crossterm`, `git-graph`, `syntect`

## Build/Test/Lint Commands

```bash
# Build
cargo build                    # Debug build
cargo build --release          # Release build (optimized with LTO)

# Test
cargo test --all               # Run all tests
cargo test <test_name>         # Run single test by name
cargo test --lib               # Run library tests only
cargo test -- --nocapture      # Show test output

# Lint (must pass in CI)
cargo fmt --all -- --check     # Check formatting
cargo clippy --all --all-targets -- --deny warnings  # Lint with strict warnings

# Format
cargo fmt --all                # Auto-format all code

# Run
cargo run                      # Run the application
cargo run -- --help            # Show CLI help
cargo run -- --path /path/to/repo  # Open specific repo
```

## Project Structure

```
src/
  main.rs          # Entry point, CLI parsing, main event loop
  lib.rs           # Library root - exports public modules
  app.rs           # Application state and logic
  settings.rs      # Configuration types
  dialogs.rs       # File dialogs
  ui.rs            # UI rendering
  widgets/         # TUI widget components
    mod.rs         # Widget module exports
    graph_view.rs  # Git graph visualization
    commit_view.rs # Commit details view
    diff_view.rs   # Diff display
    files_view.rs  # File list
    branches_view.rs # Branch list
    list.rs        # Reusable list widget
    models_view.rs # Branching model selection
  util/            # Utility modules
    mod.rs
    format.rs      # Commit formatting
    syntax_highlight.rs
    ctrl_chars.rs  # Terminal control character handling
```

## Code Style Guidelines

### Formatting
- Use `cargo fmt` defaults (no rustfmt.toml override)
- 4-space indentation
- Max line width: 100 characters (Rust default)

### Naming Conventions
- Types/Traits: `CamelCase` (e.g., `ActiveView`, `DiffType`)
- Functions/Methods: `snake_case` (e.g., `on_enter`, `reload_diff_files`)
- Constants: `SCREAMING_SNAKE_CASE` (e.g., `REPO_CONFIG_FILE`, `CHECK_CHANGE_RATE`)
- Modules: `snake_case` (e.g., `syntax_highlight`, `ctrl_chars`)

### Imports
- Group imports: std -> external crates -> internal modules
- Use nested imports for same crate (e.g., `use tui::{backend::CrosstermBackend, Terminal}`)
- Prefer explicit imports over glob imports
- Example:
```rust
use std::path::PathBuf;
use std::time::Duration;

use git2::Repository;
use tui::style::Style;

use crate::app::App;
use crate::widgets::list::StatefulList;
```

### Type Annotations
- Use type aliases for complex types: `pub type CurrentBranches = Vec<(Option<String>, Option<Oid>)>`
- Derive common traits: `#[derive(Default, Clone, PartialEq, Eq)]`
- Use `#[derive(Default)]` for structs with sensible defaults

### Error Handling
- Functions return `Result<T, String>` - errors mapped to strings
- Use `.map_err(|err| err.message().to_string())?` for git2 errors
- Use `.map_err(|err| err.to_string())?` for other error types
- Avoid `unwrap()` in library code; prefer `?` operator

### Visibility
- Use `pub(crate)` for internal-only public items
- Keep module contents private by default
- Export via `pub mod` in parent mod.rs

### Builder Pattern
Widgets use builder pattern with method chaining:
```rust
impl<'a> CommitView<'a> {
    pub fn block(mut self, block: Block<'a>) -> CommitView<'a> {
        self.block = Some(block);
        self
    }
    pub fn style(mut self, style: Style) -> CommitView<'a> {
        self.style = style;
        self
    }
}
```

### Pattern Matching
- Use exhaustive matching; avoid catch-all `_` when possible
- Prefer `if let` for single-variant matches
- Use `match` for multi-variant handling

### Comments & Documentation
- Use `//` for inline comments
- Minimal doc comments; focus on non-obvious behavior
- TODO comments for planned features: `// TODO: implement search in diff panel`

## Clippy Rules

Clippy runs with `--deny warnings`. Key rules enforced:
- No unused imports, variables, or code
- No unnecessary clones or borrows
- Use `#[allow(clippy::rule)]` sparingly and only with justification:
```rust
#[allow(clippy::needless_borrow)]
textwrap::fill(text_line, &wrapping)
```

## Testing

- Tests in same file with `#[cfg(test)]` module
- Integration tests in `tests/` directory
- Use `#[test]` attribute for test functions
- Prefer `assert_eq!` with descriptive messages

## Git & CI

- CI runs on all branches (push) and PRs to master
- All tests must pass: `cargo test --all`
- Format check must pass: `cargo fmt --all -- --check`
- Clippy must pass with no warnings: `cargo clippy --all --all-targets -- --deny warnings`

## Key Patterns

### State Management
- Application state in `App` struct
- View states (e.g., `GraphViewState`, `CommitViewState`) manage UI state
- Use `Option<T>` for nullable/optional state

### Event Handling
- Keyboard events processed in main event loop
- Methods like `on_up()`, `on_down()`, `on_enter()` return `Result<bool, String>`
- Boolean return indicates whether UI reload is needed

### TUI Widgets
- Implement `StatefulWidget` for stateful components
- Implement `Widget` as wrapper around `StatefulWidget::render`
- Use `Block` for borders and titles

## Common Gotchas

1. **libgit2 limitations**: No shallow clone support
2. **Terminal state**: Always restore terminal on panic (see `chain_panic_hook()`)
3. **Syntax highlighting**: Can be slow for large files
4. **Git2 errors**: Always map to String for consistency

## Dependencies Worth Knowing

- `git2`: Rust bindings for libgit2
- `tui`: Terminal UI framework
- `crossterm`: Cross-platform terminal manipulation
- `git-graph`: Git graph visualization library (sister project)
- `syntect`: Syntax highlighting
- `clap`: CLI argument parsing
