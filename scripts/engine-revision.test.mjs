import assert from "node:assert/strict";
import {
  cpSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  unlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, resolve } from "node:path";
import { spawnSync } from "node:child_process";
import { afterEach, test } from "node:test";
import { fileURLToPath } from "node:url";

import {
  ACTIVE_CARRIER_PATHS,
  DERIVED_CONSUMER_PATHS,
  checkEngineRevision,
  updateEngineRevision,
} from "./engine-revision-lib.mjs";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const CURRENT = JSON.parse(
  readFileSync(resolve(repoRoot, "engine-source.json"), "utf8"),
).commit;
const NEXT = "1111111111111111111111111111111111111111";
const HISTORICAL = "2222222222222222222222222222222222222222";
const temporaryRoots = [];

afterEach(() => {
  for (const path of temporaryRoots.splice(0)) {
    rmSync(path, { recursive: true, force: true });
  }
});

test("check accepts the complete current carrier set", () => {
  const fixture = copyFixture();
  assert.equal(checkEngineRevision(fixture).commit, CURRENT);
});

test("check rejects malformed canonical source identity", () => {
  const fixture = copyFixture();
  writeJson(fixture, "engine-source.json", {
    schemaVersion: 1,
    repository: "https://example.invalid/private-engine",
    commit: CURRENT.toUpperCase(),
    branch: "main",
  });
  assert.throws(
    () => checkEngineRevision(fixture),
    /repository expected/u,
  );
});

test("check rejects a renamed Rust facade carrier", () => {
  const mutations = [
    (content) => content.replace("rusty-engine =", "renamed-rusty-engine ="),
  ];
  for (const mutate of mutations) {
    const fixture = copyFixture();
    mutateFile(fixture, "Cargo.toml", mutate);
    assert.throws(() => checkEngineRevision(fixture), /Cargo\.toml/u);
  }
});

test("check rejects rules package and allow-build drift", () => {
  const packageFixture = copyFixture();
  mutateFile(
    packageFixture,
    "rules/packages/d20-authoring/package.json",
    (content) => content.replace(CURRENT, NEXT),
  );
  assert.throws(
    () => checkEngineRevision(packageFixture),
    /d20-authoring\/package\.json/u,
  );

  const workspaceFixture = copyFixture();
  mutateFile(workspaceFixture, "rules/pnpm-workspace.yaml", (content) =>
    content.replace(
      '"@rusty-engine/gameplay-rules-authoring@',
      '"@rusty-engine/renamed-authoring@',
    ),
  );
  assert.throws(
    () => checkEngineRevision(workspaceFixture),
    /pnpm-workspace\.yaml/u,
  );

  const unexpectedFixture = copyFixture();
  const relativePath = "rules/packages/d20-authoring/package.json";
  const manifest = JSON.parse(
    readFileSync(resolve(unexpectedFixture, relativePath), "utf8"),
  );
  manifest.devDependencies = {
    "@rusty-engine/unexpected": `github:FuzzySlipper/rusty-engine#${CURRENT}&path:tools/unexpected`,
  };
  writeJson(unexpectedFixture, relativePath, manifest);
  assert.throws(
    () => checkEngineRevision(unexpectedFixture),
    /unexpected Engine package @rusty-engine\/unexpected/u,
  );

});

test("check rejects Engine packages in the application workspace", () => {
  const fixture = copyFixture();
  mutateFile(fixture, "pnpm-workspace.yaml", (content) =>
    `${content}\n  "@rusty-engine/renderer-host@https://example.invalid": true\n`,
  );
  assert.throws(
    () => checkEngineRevision(fixture),
    /application workspace must not carry Engine renderer packages/u,
  );
});

test("check rejects adjacent and case-variant Engine sources", () => {
  const packageFixture = copyFixture();
  writeJson(
    packageFixture,
    "libs/adjacent/package.json",
    {
      name: "@rusty-d20/adjacent",
      dependencies: {
        unexpected: `github:FuzzySlipper/Rusty-Engine#${NEXT}&path:tools/unexpected`,
      },
    },
    true,
  );
  assert.throws(
    () => checkEngineRevision(packageFixture),
    /libs\/adjacent\/package\.json: unexpected Engine source/u,
  );

  const cargoFixture = copyFixture();
  writeText(
    cargoFixture,
    "rust/crates/adjacent/Cargo.toml",
    `[package]
name = "adjacent"
version = "0.1.0"

[dependencies]
unexpected = { git = "https://github.com/FuzzySlipper/Rusty-Engine", rev = "${NEXT}" }
`,
  );
  assert.throws(
    () => checkEngineRevision(cargoFixture),
    /rust\/crates\/adjacent\/Cargo\.toml: unexpected direct Engine dependency/u,
  );
});

test("check rejects stale missing and sibling lock sources", () => {
  for (const relativePath of [
    "Cargo.lock",
    "rules/pnpm-lock.yaml",
  ]) {
    const staleFixture = copyFixture();
    mutateFile(staleFixture, relativePath, (content) =>
      content.replace(CURRENT, NEXT),
    );
    assert.throws(() => checkEngineRevision(staleFixture), /lock/u);

    const missingFixture = copyFixture();
    unlinkSync(resolve(missingFixture, relativePath));
    assert.throws(() => checkEngineRevision(missingFixture), /missing/u);
  }

  const pathFixture = copyFixture();
  mutateFile(pathFixture, "rules/pnpm-lock.yaml", (content) =>
    content.replace(
      `github:FuzzySlipper/rusty-engine#${CURRENT}`,
      "file:../../../rusty-engine",
    ),
  );
  assert.throws(() => checkEngineRevision(pathFixture), /pnpm-lock\.yaml/u);

  const missingCrateFixture = copyFixture();
  mutateFile(missingCrateFixture, "Cargo.lock", (content) =>
    removeCargoPackage(content, "rusty-engine"),
  );
  assert.throws(
    () => checkEngineRevision(missingCrateFixture),
    /Cargo\.lock: expected exactly one locked package rusty-engine; observed 0/u,
  );
});

test("check requires derived runtime and boundary consumers without copied commits", () => {
  const runtimeFixture = copyFixture();
  mutateFile(runtimeFixture, "rust/crates/rusty-d20/src/lib.rs", (content) =>
    content.replace('env!("RUSTY_D20_ENGINE_REVISION")', `"${CURRENT}"`),
  );
  assert.throws(
    () => checkEngineRevision(runtimeFixture),
    /must derive the Engine revision/u,
  );

  const boundaryFixture = copyFixture();
  mutateFile(
    boundaryFixture,
    "tools/scripts/check-product-boundaries.mjs",
    (content) => `${content}\n// ${CURRENT}\n`,
  );
  assert.throws(
    () => checkEngineRevision(boundaryFixture),
    /must not duplicate the canonical Engine commit/u,
  );
});

test("update validates shape and public reachability before worktree creation", async () => {
  const fixture = gitFixture();
  await assert.rejects(
    updateEngineRevision({ repoRoot: fixture, commit: "main" }),
    /lowercase 40-character/u,
  );
  await assert.rejects(
    updateEngineRevision({
      repoRoot: fixture,
      commit: NEXT,
      provePublic: async () => {
        throw new Error("not public");
      },
    }),
    /not public/u,
  );
  assert.equal(worktreeCount(fixture), 1);
});

test("update rejects adjacent sources before public fetch", async () => {
  const fixture = gitFixture();
  const before = carrierSnapshot(fixture);
  writeJson(
    fixture,
    "libs/adjacent/package.json",
    {
      name: "@rusty-d20/adjacent",
      dependencies: {
        unexpected: `github:FuzzySlipper/Rusty-Engine#${NEXT}&path:tools/unexpected`,
      },
    },
    true,
  );
  let publicFetchCalled = false;
  await assert.rejects(
    updateEngineRevision({
      repoRoot: fixture,
      commit: NEXT,
      provePublic: async () => {
        publicFetchCalled = true;
      },
    }),
    /unexpected Engine source/u,
  );
  assert.equal(publicFetchCalled, false);
  assert.deepEqual(carrierSnapshot(fixture), before);
  assert.equal(worktreeCount(fixture), 1);
});

test("dry-run is scoped non-mutating and cleans its worktree", async () => {
  const fixture = gitFixture();
  const before = carrierSnapshot(fixture);
  const result = await updateEngineRevision({
    repoRoot: fixture,
    commit: NEXT,
    dryRun: true,
    provePublic: async () => {},
    regenerate: fakeRegenerate,
    validate: async (candidate) => checkEngineRevision(candidate),
  });
  assert.match(result.diff, /engine-source\.json/u);
  assert.match(result.diff, new RegExp(NEXT, "u"));
  assert.doesNotMatch(result.diff, /docs\/history/u);
  assert.deepEqual(carrierSnapshot(fixture), before);
  assert.equal(
    readFileSync(resolve(fixture, "docs/history.txt"), "utf8"),
    `${HISTORICAL}\n`,
  );
  assert.equal(worktreeCount(fixture), 1);
});

test("same-revision dry-run is formatting-neutral", async () => {
  const fixture = gitFixture();
  const result = await updateEngineRevision({
    repoRoot: fixture,
    commit: CURRENT,
    dryRun: true,
    provePublic: async () => {},
    regenerate: fakeRegenerate,
    validate: async (candidate) => checkEngineRevision(candidate),
  });
  assert.equal(result.diff, "");
  assert.equal(worktreeCount(fixture), 1);
});

test("ordinary update and rollback preserve unrelated and historical values", async () => {
  const fixture = gitFixture();
  writeFileSync(resolve(fixture, "unrelated.txt"), "user change\n");
  await applySyntheticUpdate(fixture, NEXT);
  assert.equal(checkEngineRevision(fixture).commit, NEXT);
  assert.equal(
    readFileSync(resolve(fixture, "unrelated.txt"), "utf8"),
    "user change\n",
  );
  assert.equal(
    readFileSync(resolve(fixture, "docs/history.txt"), "utf8"),
    `${HISTORICAL}\n`,
  );

  git(fixture, ["add", ...ACTIVE_CARRIER_PATHS]);
  git(fixture, ["commit", "--quiet", "-m", "advance fixture"]);
  await applySyntheticUpdate(fixture, CURRENT);
  assert.equal(checkEngineRevision(fixture).commit, CURRENT);
  assert.equal(worktreeCount(fixture), 1);
});

test("update rejects dirty carriers and cleans candidate failures", async () => {
  const dirtyFixture = gitFixture();
  mutateFile(
    dirtyFixture,
    "rules/packages/d20-authoring/package.json",
    (content) => `${content}\n`,
  );
  await assert.rejects(
    updateEngineRevision({
      repoRoot: dirtyFixture,
      commit: NEXT,
      provePublic: async () => {},
    }),
    /carrier or lock files are dirty/u,
  );
  assert.equal(worktreeCount(dirtyFixture), 1);

  const failedFixture = gitFixture();
  const before = carrierSnapshot(failedFixture);
  await assert.rejects(
    updateEngineRevision({
      repoRoot: failedFixture,
      commit: NEXT,
      provePublic: async () => {},
      regenerate: async () => {
        throw new Error("synthetic regeneration failure");
      },
    }),
    /synthetic regeneration failure/u,
  );
  assert.deepEqual(carrierSnapshot(failedFixture), before);
  assert.equal(worktreeCount(failedFixture), 1);
});

test("update detects caller head and carrier races", async () => {
  const headFixture = gitFixture();
  await assert.rejects(
    updateEngineRevision({
      repoRoot: headFixture,
      commit: NEXT,
      provePublic: async () => {},
      regenerate: fakeRegenerate,
      validate: async (candidate) => {
        checkEngineRevision(candidate);
        writeFileSync(resolve(headFixture, "race.txt"), "race\n");
        git(headFixture, ["add", "race.txt"]);
        git(headFixture, ["commit", "--quiet", "-m", "race"]);
      },
    }),
    /caller HEAD changed/u,
  );
  assert.equal(checkEngineRevision(headFixture).commit, CURRENT);
  assert.equal(worktreeCount(headFixture), 1);

  const carrierFixture = gitFixture();
  await assert.rejects(
    updateEngineRevision({
      repoRoot: carrierFixture,
      commit: NEXT,
      provePublic: async () => {},
      regenerate: fakeRegenerate,
      validate: async (candidate) => {
        checkEngineRevision(candidate);
        mutateFile(
          carrierFixture,
          "engine-source.json",
          (content) => `${content}\n`,
        );
      },
    }),
    /carrier or lock files are dirty/u,
  );
  assert.equal(worktreeCount(carrierFixture), 1);
});

function copyFixture() {
  const root = temporaryRoot();
  for (const relativePath of [
    ...ACTIVE_CARRIER_PATHS,
    ...DERIVED_CONSUMER_PATHS,
  ]) {
    const destination = resolve(root, relativePath);
    mkdirSync(dirname(destination), { recursive: true });
    cpSync(resolve(repoRoot, relativePath), destination);
  }
  return root;
}

function gitFixture() {
  const root = copyFixture();
  writeText(root, "docs/history.txt", `${HISTORICAL}\n`);
  git(root, ["init", "--quiet"]);
  git(root, ["config", "user.email", "engine-revision-test@example.invalid"]);
  git(root, ["config", "user.name", "Engine Revision Test"]);
  git(root, ["add", "."]);
  git(root, ["commit", "--quiet", "-m", "fixture"]);
  return root;
}

async function applySyntheticUpdate(fixture, commit) {
  return updateEngineRevision({
    repoRoot: fixture,
    commit,
    provePublic: async () => {},
    regenerate: fakeRegenerate,
    validate: async (candidate) => checkEngineRevision(candidate),
  });
}

async function fakeRegenerate(candidate, previousCommit, commit) {
  for (const relativePath of [
    "Cargo.lock",
    "rules/pnpm-lock.yaml",
  ]) {
    mutateFile(candidate, relativePath, (content) =>
      content.replaceAll(previousCommit, commit),
    );
  }
}

function carrierSnapshot(root) {
  return Object.fromEntries(
    ACTIVE_CARRIER_PATHS.map((relativePath) => [
      relativePath,
      readFileSync(resolve(root, relativePath), "utf8"),
    ]),
  );
}

function writeJson(root, relativePath, value, createParent = false) {
  writeText(
    root,
    relativePath,
    `${JSON.stringify(value, null, 2)}\n`,
    createParent,
  );
}

function writeText(root, relativePath, value, createParent = true) {
  const path = resolve(root, relativePath);
  if (createParent) mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, value);
}

function mutateFile(root, relativePath, mutate) {
  const path = resolve(root, relativePath);
  writeFileSync(path, mutate(readFileSync(path, "utf8")));
}

function removeCargoPackage(content, packageName) {
  const marker = `[[package]]\nname = "${packageName}"\n`;
  const start = content.indexOf(marker);
  assert.notEqual(start, -1, `fixture contains ${packageName}`);
  const next = content.indexOf("\n[[package]]\n", start + marker.length);
  return next === -1
    ? content.slice(0, start)
    : `${content.slice(0, start)}${content.slice(next + 1)}`;
}

function worktreeCount(root) {
  return git(root, ["worktree", "list", "--porcelain"])
    .split("\n")
    .filter((line) => line.startsWith("worktree ")).length;
}

function git(cwd, args) {
  const result = spawnSync("git", args, { cwd, encoding: "utf8" });
  if (result.status !== 0) {
    throw new Error(result.stderr || result.stdout);
  }
  return result.stdout;
}

function temporaryRoot() {
  const root = mkdtempSync(resolve(tmpdir(), "engine-revision-test-"));
  temporaryRoots.push(root);
  return root;
}
