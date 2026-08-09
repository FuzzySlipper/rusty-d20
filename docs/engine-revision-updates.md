# Rusty Engine revision updates

Cargo declares one complete `rusty-engine` facade tracking public branch
`main`. `engine-source.json` records the exact lowercase commit currently
resolved by `Cargo.lock`; isolated rules-authoring packages use that same
commit. Runtime provenance and boundary checks derive from the canonical file.

Check the current revision and every carrier:

```bash
./scripts/engine-revision check
```

Preview an update without changing the caller checkout:

```bash
./scripts/engine-revision update <40-character-public-sha> --dry-run
```

Apply an update:

```bash
./scripts/engine-revision update <40-character-public-sha>
```

The updater proves the exact commit is publicly fetchable, rejects dirty
carrier files and undeclared adjacent Engine sources, creates a detached
candidate worktree at the caller's exact head, rewrites only registered
carriers, updates the complete Cargo lock closure and rules lockfile, and validates the
candidate before applying its scoped diff. It rechecks the caller head and
carrier cleanliness immediately before applying. Failure and dry-run paths
remove the temporary worktree and leave the caller carriers unchanged.

After an applied update, inspect the diff, run the aggregate verification, and
commit all registered carrier and lockfile changes together:

```bash
./scripts/verify.sh
git diff -- engine-source.json Cargo.toml Cargo.lock \
  rules/packages/d20-authoring/package.json rules/pnpm-workspace.yaml \
  rules/pnpm-lock.yaml
```

Rollback uses the same command with the last compatible public commit. Do not
hand-edit individual carriers, replace the complete facade with a crate menu,
or add a sibling path fallback.
