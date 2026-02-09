---
description: Enforce documentation standards when architecture, API, or config files change
globs:
  - "src/**/*.rs"
  - "Cargo.toml"
  - ".gitlab-ci.yml"
  - ".github/workflows/**"
  - "docs/**/*.md"
  - "docs/**/*.mmd"
  - "README.md"
  - "CHANGELOG.md"
---

# Documentation Enforcement

> **ADAPT GLOBS**: The glob patterns above (`src/core/**`, `src/api/**`, `lib/**`) are examples.
> When `/ww-init` installs this file, adapt the globs to match your project's actual directory structure
> (e.g., `app/**`, `packages/**`, `crates/**`). Keep the `docs/**`, `README.md`, and `CHANGELOG.md` globs as-is.

When you touch architecture, API, or config files, documentation MUST be updated.

## Required Actions

### On Architecture/Core Changes
- Update `docs/architecture/` if component structure changed
- Update Mermaid diagrams if relationships changed
- Create ADR in `docs/adr/` if significant decision was made
- Update CHANGELOG.md under [Unreleased]

### On API Changes
- Update `docs/reference/api.md`
- Update sequence diagrams if flow changed
- Update CHANGELOG.md under [Unreleased]

### On Config Changes
- Update `docs/reference/configuration.md`

### On Any Change
- CHANGELOG.md entry required for features, fixes, breaking changes

## Documentation Rules

- All docs MUST live in `docs/` folder (Diataxis structure). Exception: standards/framework repos where the repo IS the documentation.
- All diagrams MUST be Mermaid (no binary: .drawio, .png, .vsdx)
- README.md = project summary + TOC to docs/ (max 150 lines)
- ADRs MUST use MADR template, numbered sequentially
- No documentation duplication — single source of truth, link elsewhere

## Anti-Laziness Rules (CRITICAL)

Documentation updates MUST be substantial. Cosmetic-only changes are rejected.

### BLOCKED — These patterns FAIL the update:
- Changing ONLY "Last updated:", "Date:", or "Version:" without content changes
- CHANGELOG entries using generic phrases: "updated docs", "documentation improvements", "misc updates", "various fixes", "minor changes"
- CHANGELOG entries under 15 words
- Adding a CHANGELOG entry without updating the actual documentation files
- Updating a diagram title/metadata without changing the diagram content

### REQUIRED — Every doc update MUST:
- Tie to a specific code change (not a vague "keeping docs current")
- Have >70% actual content (not metadata/formatting) in the diff
- Include specifics: name the component, endpoint, feature, or config that changed
- For API changes: document endpoint, parameters, response example, error codes
- For architecture changes: update the Mermaid diagram to reflect new/changed components

### Self-Check Before Committing Doc Changes:
1. "Did I change actual content or just dates/versions?"
2. "Could a new developer understand this change from my docs alone?"
3. "Does my CHANGELOG entry name the specific thing that changed?"

## Blocked

- No `.md` files outside `docs/` and root (except AGENTS.md, CHANGELOG, CONTRIBUTING, LICENSE). Exception: standards/framework repos.
- No binary diagram formats committed to repo
- No architectural decisions without an ADR
