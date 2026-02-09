---
globs: ["**/*.ts", "**/*.tsx", "**/*.js", "**/*.jsx", "**/*.py", "**/*.rs", "**/*.go", "**/*.java", "**/*.env", "**/*.env.*", "**/docker-compose*.yml", "**/Dockerfile*"]
alwaysApply: false
description: "Enforces security rules: no secrets in code, parameterized queries, input validation"
---

<MANDATORY_RULE severity="CRITICAL" priority="HIGHEST">

# Security — Non-Negotiable Rules

## NEVER commit
- API keys, tokens, passwords, certificates, private keys
- .env files with real credentials
- Connection strings with embedded passwords

## ALWAYS
- Use parameterized queries (never string interpolation in SQL/NoSQL)
- Validate ALL user input with schema validation before processing
- Check authorization on the server for every protected operation
- Hash passwords with bcrypt/argon2 (cost 12+), NEVER md5/sha1
- Return generic error messages to clients (no stack traces, no file paths)

## Before installing ANY package
- Verify it exists on the official registry (npmjs.com, pypi.org, crates.io)
- Check download count (legitimate packages have >1000 weekly downloads on npm)
- Check publisher and repository link — no repo = don't install
- If in doubt, ask the user before installing

## Check before committing
- `git diff` — are there any hardcoded secrets?
- Is `.env` in `.gitignore`?
- Are all queries parameterized?

</MANDATORY_RULE>
