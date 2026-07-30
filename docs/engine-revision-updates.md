# Rusty Engine revision updates

`engine-source.json` is the only hand-edited Rusty Engine source declaration.
It selects one exact lowercase public commit for every Engine Rust crate and
rules package in this repository. Cargo manifests, package manifests, build
policy, and both lockfiles are active carriers of that same value; runtime
provenance and boundary checks derive it from the canonical manifest.

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
carriers, regenerates both lockfiles with pinned tools, and validates the
candidate before applying its scoped diff. It rechecks the caller head and
carrier cleanliness immediately before applying. Failure and dry-run paths
remove the temporary worktree and leave the caller carriers unchanged.

After an applied update, inspect the diff, run the aggregate verification, and
commit all registered carrier and lockfile changes together:

```bash
./scripts/verify.sh
git diff -- engine-source.json rust/crates/rusty-d20/Cargo.toml Cargo.lock \
  rules/packages/d20-authoring/package.json rules/pnpm-workspace.yaml \
  rules/pnpm-lock.yaml
```

Rollback uses the same command with the last reviewed public commit. Do not
hand-edit individual carriers, substitute a branch/tag/floating dependency, or
add a sibling path fallback.
