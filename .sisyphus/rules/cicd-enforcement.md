---
globs: [".gitlab-ci.yml", ".github/workflows/**", "Dockerfile*", "docker-compose*.yml", "Jenkinsfile", ".circleci/**", "Makefile", "scripts/deploy*", "scripts/ci*", "k8s/**", "helm/**", "terraform/**", "*.tf"]
alwaysApply: false
description: "Enforces CI/CD pipeline standards: stage order, security scanning, deployment gates, rollback mechanisms"
---

<MANDATORY_RULE severity="CRITICAL" priority="HIGHEST">

# CI/CD — Non-Negotiable Rules

## Pipeline Structure
- **Stage order MUST be**: preflight → lint → test → security → build → package → publish → deploy → verify
- **Security stage NEVER skippable** — `[skip ci]` markers disabled by policy on ALL branches, no manual skip of security scans
- **Lint stage MUST include**: formatter check (fail if unformatted), linter with errors-as-failures, type checking
- **Test stage MUST include**: unit tests (always), integration tests (on MR/PR), coverage gate (minimum 60%, no decrease allowed)
- **Pipeline creation**: Do NOT use `workflow:rules` to suppress pipeline creation based on commit type or files - use job-level `rules:changes` for smart skip instead

## Security Scanning (MANDATORY)
- **Dependency audit**: cargo audit / npm audit --audit-level=high / pip-audit / govulncheck
- **Secrets detection**: gitleaks or trufflehog — scan full MR/PR diff, FAIL on ANY detected secret
- **SAST**: clippy / eslint-plugin-security / bandit / gosec — FAIL on HIGH, warn on MEDIUM
- **Threshold**: FAIL on HIGH or CRITICAL vulnerabilities (72h grace for newly disclosed CVEs in transitive deps)

## Deployment Gates
- **Production deployment REQUIRES**: manual approval gate + tag (vX.Y.Z format) + full pipeline pass
- **Rollback mechanism REQUIRED**: auto-rollback on health check failure >30s, error rate >5x baseline >60s, OOM kills, crash loops
- **Smoke tests post-deploy**: MUST run and pass before deployment considered successful
- **Environment promotion**: dev (auto) → staging (auto) → production (manual + tag only)

## Merge Requirements
- **Coverage ratchet**: Coverage MUST NOT decrease on any MR/PR — new code minimum 80% coverage for changed lines
- **Conventional commits**: `type(scope): subject` — imperative, lowercase, no period, max 72 chars
- **Merge strategy**: ALWAYS `--no-ff` (merge commits required), squash only as pre-merge cleanup on the branch itself, NEVER rebase to protected branches

## Forbidden Patterns
- Hardcoded secrets in CI config (use CI variables / vault)
- Manual skip of test or lint stages via pipeline UI
- Deploying without passing security scans
- Releasing without CHANGELOG.md update
- Force push to protected branches

</MANDATORY_RULE>
