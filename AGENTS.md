# Notes for coding agents

## Prose style

Use the `asd-ste100` skill for every new or changed piece of prose. This rule
covers code comments, documentation, commit messages, PR descriptions, manuals
and docs.

- Use STE-flavored mode for explanatory prose. Keep sentences at or below 25
  words. Use active voice and simple tenses. Do not use semicolons.
- Use Strict mode for error messages, tool descriptions and instructions another
  agent must parse.
- Preserve facts, conditions, hedges and technical terms. Do not add a claim the
  source did not make.
- Apply this rule when you touch text. Do not rewrite existing prose only to
  apply this rule.

## The prose gate

The gate enforces the rule on `.md` and `.mdx` files a change touches, outside
`vendor/`, nested `*/vendor/` trees, `target/` and `node_modules/`. The linter
is `scripts/ste-lint.py` and needs `python3` on PATH.

- `cargo xtask check` runs it against `origin/main...HEAD` plus untracked
  prose files. `cargo xtask ste [base]` runs it alone.
- CI runs it as the `prose` job on pull requests and pushes to `main`.
- `prek install` installs the commit hook, which lints the staged files.
- The gate lints each touched file in full. A dirty file stays green until
  you touch it, then it must come out clean.
- Code comments and catalog `summary:` fields follow the same rule but the
  gate does not lint them mechanically. They stay advisory.
