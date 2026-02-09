# Claude Code Compatibility Pointer

> OpenCode loads both `AGENTS.md` and `CLAUDE.md`. This file ensures correct path resolution.

**All project standards are in [`AGENTS.md`](./AGENTS.md).** This file exists solely for
OpenCode's Claude Code compatibility layer, which injects `CLAUDE.md` paths into the system
prompt. Without it, the runtime may report a stale or incorrect working directory.

Do NOT add rules here. Edit `AGENTS.md` instead.
