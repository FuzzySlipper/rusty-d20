import {
  mkdtempSync,
  readFileSync,
  readdirSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, resolve } from "node:path";
import { spawnSync } from "node:child_process";

export const ENGINE_REPOSITORY = "https://github.com/FuzzySlipper/rusty-engine";
export const COMMIT_PATTERN = /^[0-9a-f]{40}$/u;

export const ENGINE_PACKAGES = new Map([
  [
    "@rusty-engine/gameplay-rules-authoring",
    "rules/packages/gameplay-rules-authoring",
  ],
  [
    "@rusty-engine/gameplay-rules-contracts",
    "rules/packages/gameplay-rules-contracts",
  ],
]);

export const ENGINE_CRATES = [
  "core-ids",
  "core-space",
  "core-voxel",
  "entity-state",
  "gameplay-mechanics",
  "gameplay-rules",
  "svc-collision",
  "svc-pathfinding",
  "svc-rng",
  "svc-spatial",
  "svc-volume",
];

export const ACTIVE_CARRIER_PATHS = [
  "engine-source.json",
  "rust/crates/rusty-d20/Cargo.toml",
  "Cargo.lock",
  "rules/packages/d20-authoring/package.json",
  "rules/pnpm-workspace.yaml",
  "rules/pnpm-lock.yaml",
];

export const DERIVED_CONSUMER_PATHS = [
  "rust/crates/rusty-d20/build.rs",
  "rust/crates/rusty-d20/src/lib.rs",
  "rules/scripts/check-boundaries.mjs",
  "tools/scripts/check-product-boundaries.mjs",
];

const REPAIR_COMMAND = "./scripts/engine-revision update <sha>";
const DECLARED_PACKAGE_MANIFESTS = new Set([
  "rules/packages/d20-authoring/package.json",
]);
const DECLARED_CARGO_MANIFESTS = new Set(["rust/crates/rusty-d20/Cargo.toml"]);
const MANIFEST_SCAN_IGNORES = new Set([
  ".git",
  ".nx",
  "dist",
  "node_modules",
  "target",
]);

export function loadEngineSource(repoRoot) {
  const relativePath = "engine-source.json";
  const path = resolve(repoRoot, relativePath);
  let source;
  try {
    source = JSON.parse(readFileSync(path, "utf8"));
  } catch (error) {
    throw new Error(
      `${relativePath}: cannot decode canonical Engine source: ${error.message}`,
    );
  }
  if (source === null || Array.isArray(source) || typeof source !== "object") {
    throw new Error(`${relativePath}: expected one JSON object`);
  }
  const keys = Object.keys(source).sort();
  const expectedKeys = ["commit", "repository", "schemaVersion"];
  if (JSON.stringify(keys) !== JSON.stringify(expectedKeys)) {
    throw new Error(
      `${relativePath}: expected exactly ${expectedKeys.join(", ")}; observed ${keys.join(", ")}`,
    );
  }
  if (source.schemaVersion !== 1) {
    throw new Error(
      `${relativePath}: schemaVersion expected 1; observed ${String(source.schemaVersion)}`,
    );
  }
  if (source.repository !== ENGINE_REPOSITORY) {
    throw new Error(
      `${relativePath}: repository expected ${ENGINE_REPOSITORY}; observed ${String(source.repository)}`,
    );
  }
  assertCommit(source.commit, `${relativePath}: commit`);
  return Object.freeze({
    schemaVersion: 1,
    repository: ENGINE_REPOSITORY,
    commit: source.commit,
  });
}

export function checkEngineRevision(repoRoot) {
  const source = loadEngineSource(repoRoot);
  const violations = [];
  checkCargoManifest(repoRoot, source, violations);
  checkCargoLock(repoRoot, source, violations);
  checkPackageManifest(
    repoRoot,
    "rules/packages/d20-authoring/package.json",
    new Set(ENGINE_PACKAGES.keys()),
    source,
    violations,
  );
  checkPnpmWorkspace(repoRoot, source, violations);
  checkPnpmLock(repoRoot, source, violations);
  checkAdjacentDependencyManifests(repoRoot, violations);
  checkDerivedConsumers(repoRoot, source, violations);
  if (violations.length > 0) {
    throw new Error(
      `Engine revision check failed:\n${violations
        .map((violation) => `- ${violation}`)
        .join("\n")}\nRepair with: ${REPAIR_COMMAND}`,
    );
  }
  return source;
}

export async function updateEngineRevision({
  repoRoot,
  commit,
  dryRun = false,
  provePublic = provePublicCommit,
  regenerate = regenerateLocks,
  validate = validateCandidate,
}) {
  assertCommit(commit, "update commit");
  const before = checkEngineRevision(repoRoot);
  assertCarrierFilesClean(repoRoot);
  await provePublic(before.repository, commit);

  const head = git(repoRoot, ["rev-parse", "HEAD"]).trim();
  const temporaryRoot = mkdtempSync(
    resolve(tmpdir(), "rusty-d20-engine-revision-"),
  );
  const candidate = resolve(temporaryRoot, "candidate");
  let worktreeAdded = false;
  try {
    git(repoRoot, ["worktree", "add", "--detach", candidate, head]);
    worktreeAdded = true;
    rewriteActiveCarriers(candidate, before.commit, commit);
    await regenerate(candidate, before.commit, commit);
    await validate(candidate);
    const diff = scopedDiff(candidate);

    if (dryRun) return Object.freeze({ before, commit, diff, dryRun: true });

    if (git(repoRoot, ["rev-parse", "HEAD"]).trim() !== head) {
      throw new Error(
        `caller HEAD changed during update; expected ${head}. No update was applied.`,
      );
    }
    assertCarrierFilesClean(repoRoot);
    if (diff.length > 0) {
      run("git", ["apply", "--whitespace=nowarn", "-"], {
        cwd: repoRoot,
        input: diff,
      });
    }
    checkEngineRevision(repoRoot);
    return Object.freeze({
      before,
      commit,
      diff: scopedDiff(repoRoot),
      dryRun: false,
    });
  } finally {
    if (worktreeAdded) {
      run("git", ["worktree", "remove", "--force", candidate], {
        cwd: repoRoot,
        allowFailure: true,
      });
    }
    rmSync(temporaryRoot, { recursive: true, force: true });
    run("git", ["worktree", "prune"], { cwd: repoRoot, allowFailure: true });
  }
}

export function rewriteActiveCarriers(repoRoot, previousCommit, commit) {
  assertCommit(previousCommit, "previous commit");
  assertCommit(commit, "replacement commit");

  writeFileSync(
    resolve(repoRoot, "engine-source.json"),
    `${JSON.stringify(
      {
        schemaVersion: 1,
        repository: ENGINE_REPOSITORY,
        commit,
      },
      null,
      2,
    )}\n`,
  );

  replaceRequiredCommit(
    resolve(repoRoot, "rust/crates/rusty-d20/Cargo.toml"),
    previousCommit,
    commit,
  );
  rewritePackageManifest(
    resolve(repoRoot, "rules/packages/d20-authoring/package.json"),
    new Set(ENGINE_PACKAGES.keys()),
    previousCommit,
    commit,
  );
  rewriteWorkspacePolicy(
    resolve(repoRoot, "rules/pnpm-workspace.yaml"),
    previousCommit,
    commit,
  );
}

export async function provePublicCommit(repository, commit) {
  const temporaryRoot = mkdtempSync(
    resolve(tmpdir(), "rusty-engine-public-commit-"),
  );
  try {
    run("git", ["init", "--bare", "--quiet"], { cwd: temporaryRoot });
    run(
      "git",
      [
        "-c",
        "protocol.version=2",
        "fetch",
        "--quiet",
        "--no-tags",
        "--depth=1",
        `${repository}.git`,
        commit,
      ],
      { cwd: temporaryRoot },
    );
    const fetched = git(temporaryRoot, ["rev-parse", "FETCH_HEAD"]).trim();
    if (fetched !== commit) {
      throw new Error(
        `public fetch resolved ${fetched}; expected exact commit ${commit}`,
      );
    }
  } catch (error) {
    throw new Error(
      `Engine commit ${commit} is not publicly fetchable from ${repository}: ${error.message}`,
    );
  } finally {
    rmSync(temporaryRoot, { recursive: true, force: true });
  }
}

async function regenerateLocks(candidate) {
  for (const relativePath of ["package.json", "rules/package.json"]) {
    const packageManager = JSON.parse(
      readFileSync(resolve(candidate, relativePath), "utf8"),
    ).packageManager;
    if (packageManager !== "pnpm@11.7.0") {
      throw new Error(
        `${relativePath}: expected repository-pinned packageManager pnpm@11.7.0; observed ${String(packageManager)}`,
      );
    }
  }
  const pnpmVersion = run("pnpm", ["--version"], { cwd: candidate }).trim();
  if (pnpmVersion !== "11.7.0") {
    throw new Error(
      `pnpm version expected 11.7.0 from packageManager; observed ${pnpmVersion}`,
    );
  }
  run("cargo", ["metadata", "--format-version", "1"], {
    cwd: candidate,
  });
  run(
    "pnpm",
    [
      "install",
      "--lockfile-only",
      "--ignore-scripts",
      "--frozen-lockfile=false",
    ],
    { cwd: resolve(candidate, "rules") },
  );
}

async function validateCandidate(candidate) {
  checkEngineRevision(candidate);
  run("node", ["tools/scripts/check-product-boundaries.mjs"], {
    cwd: candidate,
  });
  run("node", ["rules/scripts/check-boundaries.mjs"], { cwd: candidate });
  run("cargo", ["metadata", "--format-version", "1", "--locked", "--no-deps"], {
    cwd: candidate,
  });
}

function checkCargoManifest(repoRoot, source, violations) {
  const relativePath = "rust/crates/rusty-d20/Cargo.toml";
  const content = readFile(repoRoot, relativePath, violations);
  if (content === null) return;
  const dependencySection =
    content.match(/\[dependencies\]([\s\S]*?)(?=\n\[|$)/u)?.[1] ?? "";
  for (const crate of ENGINE_CRATES) {
    const match = dependencySection.match(
      new RegExp(`^${escapeRegExp(crate)}\\s*=\\s*\\{([^\\n]+)\\}$`, "mu"),
    );
    if (match === null) {
      violations.push(`${relativePath}: missing Engine dependency ${crate}`);
      continue;
    }
    const expected = `git = "${ENGINE_REPOSITORY}", rev = "${source.commit}"`;
    if (match[1].trim() !== expected) {
      violations.push(
        `${relativePath}: ${crate} expected { ${expected} }; observed { ${match[1].trim()} }`,
      );
    }
  }
  for (const line of dependencySection.split("\n")) {
    if (
      /rusty-engine|@rusty-engine/iu.test(line) &&
      !ENGINE_CRATES.some((crate) => line.startsWith(`${crate} `))
    ) {
      violations.push(
        `${relativePath}: unexpected Engine dependency carrier ${line.trim()}`,
      );
    }
  }
  const hasSiblingPath = [...content.matchAll(/path\s*=\s*"([^"]*)"/gmu)].some(
    (match) => isEngineRepositoryReference(match[1]),
  );
  const hasNonCanonicalEngineGit = [
    ...content.matchAll(/git\s*=\s*"([^"]*)"/gmu),
  ].some(
    (match) =>
      isEngineRepositoryReference(match[1]) && match[1] !== ENGINE_REPOSITORY,
  );
  if (hasSiblingPath || hasNonCanonicalEngineGit) {
    violations.push(
      `${relativePath}: path, sibling, or non-canonical Engine source is forbidden`,
    );
  }
}

function checkCargoLock(repoRoot, source, violations) {
  const relativePath = "Cargo.lock";
  const content = readFile(repoRoot, relativePath, violations);
  if (content === null) return;
  const sources = [
    ...content.matchAll(/^source = "(git\+[^"]*rusty-engine[^"]*)"$/gimu),
  ].map((match) => match[1]);
  if (sources.length === 0) {
    violations.push(`${relativePath}: missing locked Engine sources`);
    return;
  }
  const expected = `git+${ENGINE_REPOSITORY}?rev=${source.commit}#${source.commit}`;
  for (const observed of new Set(sources)) {
    if (observed !== expected) {
      violations.push(
        `${relativePath}: Engine source expected ${expected}; observed ${observed}`,
      );
    }
  }
}

function checkPackageManifest(
  repoRoot,
  relativePath,
  expectedNames,
  source,
  violations,
) {
  let manifest;
  try {
    manifest = JSON.parse(
      readFileSync(resolve(repoRoot, relativePath), "utf8"),
    );
  } catch (error) {
    violations.push(`${relativePath}: cannot decode JSON: ${error.message}`);
    return;
  }
  const dependencies = manifest.dependencies ?? {};
  for (const name of expectedNames) {
    const packagePath = ENGINE_PACKAGES.get(name);
    const expected = packageSpecifier(source.commit, packagePath);
    if (dependencies[name] !== expected) {
      violations.push(
        `${relativePath}: ${name} expected ${expected}; observed ${String(dependencies[name])}`,
      );
    }
  }
  for (const sectionName of dependencySectionNames()) {
    for (const [name, observed] of Object.entries(
      manifest[sectionName] ?? {},
    )) {
      if (
        isEnginePackageReference(name, observed) &&
        (sectionName !== "dependencies" || !expectedNames.has(name))
      ) {
        violations.push(
          `${relativePath}: unexpected Engine package ${name} in ${sectionName} (${String(observed)})`,
        );
      }
    }
  }
}

function checkPnpmWorkspace(repoRoot, source, violations) {
  const relativePath = "rules/pnpm-workspace.yaml";
  const content = readFile(repoRoot, relativePath, violations);
  if (content === null) return;
  const observed = [
    ...content.matchAll(
      /^\s+"([^"]*(?:@rusty-engine\/|FuzzySlipper\/rusty-engine)[^"]*)":\s+true$/gimu,
    ),
  ].map((match) => match[1]);
  const expected = [...ENGINE_PACKAGES.entries()].map(
    ([name, path]) => `${name}@${codeloadSpecifier(source.commit, path)}`,
  );
  compareSets(relativePath, expected, observed, violations);
}

function checkPnpmLock(repoRoot, source, violations) {
  const relativePath = "rules/pnpm-lock.yaml";
  const content = readFile(repoRoot, relativePath, violations);
  if (content === null) return;
  const references = [
    ...content.matchAll(
      /(?:github:FuzzySlipper\/rusty-engine#|codeload\.github\.com\/FuzzySlipper\/rusty-engine\/tar\.gz\/)([^&/#\s'")}\]]+)/gimu,
    ),
  ].map((match) => match[1]);
  if (references.length === 0) {
    violations.push(`${relativePath}: missing locked Engine package sources`);
  }
  for (const observed of new Set(references)) {
    if (observed !== source.commit) {
      violations.push(
        `${relativePath}: Engine package commit expected ${source.commit}; observed ${observed}`,
      );
    }
  }
  for (const [name, path] of ENGINE_PACKAGES) {
    if (
      !content.includes(`${name}:`) &&
      !content.includes(`${name}@`) &&
      !content.includes(`'${name}@`)
    ) {
      violations.push(`${relativePath}: missing locked package ${name}`);
    }
    if (!content.includes(`#path:${path}`)) {
      violations.push(`${relativePath}: missing locked Engine path ${path}`);
    }
  }
  const enginePaths = [
    ...content.matchAll(
      /(?:github:FuzzySlipper\/rusty-engine#[^&\s'")}\]]+&path:|codeload\.github\.com\/FuzzySlipper\/rusty-engine\/tar\.gz\/[^#\s'")}\]]+#path:)([A-Za-z0-9_./-]+)/gimu,
    ),
  ].map((match) => match[1]);
  for (const observed of new Set(enginePaths)) {
    if (![...ENGINE_PACKAGES.values()].includes(observed)) {
      violations.push(
        `${relativePath}: unexpected Engine package path ${observed}`,
      );
    }
  }
  const repositorySpellings = [
    ...content.matchAll(
      /(?:github:|codeload\.github\.com\/)([^/\s]+\/rusty-engine)(?=[/#])/gimu,
    ),
  ].map((match) => match[1]);
  for (const observed of new Set(repositorySpellings)) {
    if (observed !== "FuzzySlipper/rusty-engine") {
      violations.push(
        `${relativePath}: Engine repository identity must use canonical spelling FuzzySlipper/rusty-engine; observed ${observed}`,
      );
    }
  }
  if (
    /(@rusty-engine\/[^\s]+)(?:file:|link:)|(?:file:|link:)[^\s]*rusty-engine/iu.test(
      content,
    )
  ) {
    violations.push(
      `${relativePath}: path, link, or sibling Engine package source is forbidden`,
    );
  }
}

function checkAdjacentDependencyManifests(repoRoot, violations) {
  for (const relativePath of discoverManifests(repoRoot, "package.json")) {
    if (DECLARED_PACKAGE_MANIFESTS.has(relativePath)) continue;
    let manifest;
    try {
      manifest = JSON.parse(
        readFileSync(resolve(repoRoot, relativePath), "utf8"),
      );
    } catch (error) {
      violations.push(
        `${relativePath}: cannot decode discovered package manifest: ${error.message}`,
      );
      continue;
    }
    for (const sectionName of dependencySectionNames()) {
      for (const [name, observed] of Object.entries(
        manifest[sectionName] ?? {},
      )) {
        if (isEnginePackageReference(name, observed)) {
          violations.push(
            `${relativePath}: unexpected Engine source ${name} in ${sectionName} (${String(observed)}); Engine packages are allowed only in declared carrier manifests`,
          );
        }
      }
    }
  }

  for (const relativePath of discoverManifests(repoRoot, "Cargo.toml")) {
    if (
      relativePath === "Cargo.toml" ||
      DECLARED_CARGO_MANIFESTS.has(relativePath)
    ) {
      continue;
    }
    const content = readFile(repoRoot, relativePath, violations);
    if (content === null) continue;
    for (const line of content.split("\n")) {
      const trimmed = line.trim();
      const directSource = trimmed.match(/(?:git|path)\s*=\s*"([^"]*)"/u)?.[1];
      if (
        (directSource !== undefined &&
          isEngineRepositoryReference(directSource)) ||
        ENGINE_CRATES.some((crate) => {
          if (trimmed === `${crate}.workspace = true`) return false;
          return (
            trimmed.startsWith(`${crate} =`) ||
            new RegExp(`\\bpackage\\s*=\\s*"${escapeRegExp(crate)}"`, "u").test(
              trimmed,
            )
          );
        })
      ) {
        violations.push(
          `${relativePath}: unexpected direct Engine dependency carrier ${trimmed}; Engine crates are allowed only in declared carrier manifests`,
        );
      }
    }
  }
}

function checkDerivedConsumers(repoRoot, source, violations) {
  const expectations = new Map([
    [
      "rust/crates/rusty-d20/build.rs",
      ["engine-source.json", "RUSTY_D20_ENGINE_REVISION"],
    ],
    ["rust/crates/rusty-d20/src/lib.rs", ['env!("RUSTY_D20_ENGINE_REVISION")']],
    ["rules/scripts/check-boundaries.mjs", ["loadEngineSource"]],
    ["tools/scripts/check-product-boundaries.mjs", ["loadEngineSource"]],
  ]);

  for (const relativePath of DERIVED_CONSUMER_PATHS) {
    const content = readFile(repoRoot, relativePath, violations);
    if (content === null) continue;
    for (const expected of expectations.get(relativePath) ?? []) {
      if (!content.includes(expected)) {
        violations.push(
          `${relativePath}: must derive the Engine revision through ${expected}`,
        );
      }
    }
    if (content.includes(source.commit)) {
      violations.push(
        `${relativePath}: must not duplicate the canonical Engine commit`,
      );
    }
  }
}

function rewritePackageManifest(path, expectedNames, previousCommit, commit) {
  const manifest = JSON.parse(readFileSync(path, "utf8"));
  for (const name of expectedNames) {
    const packagePath = ENGINE_PACKAGES.get(name);
    const expected = packageSpecifier(previousCommit, packagePath);
    if (manifest.dependencies?.[name] !== expected) {
      throw new Error(
        `${path}: ${name} changed before rewrite; expected ${expected}`,
      );
    }
    manifest.dependencies[name] = packageSpecifier(commit, packagePath);
  }
  writeFileSync(path, `${JSON.stringify(manifest, null, 2)}\n`);
}

function rewriteWorkspacePolicy(path, previousCommit, commit) {
  let content = readFileSync(path, "utf8");
  for (const [name, packagePath] of ENGINE_PACKAGES) {
    const before = `"${name}@${codeloadSpecifier(previousCommit, packagePath)}": true`;
    const after = `"${name}@${codeloadSpecifier(commit, packagePath)}": true`;
    if (!content.includes(before)) {
      throw new Error(`${path}: missing active allowBuilds carrier ${before}`);
    }
    content = content.replace(before, after);
  }
  writeFileSync(path, content);
}

function replaceRequiredCommit(path, previousCommit, commit) {
  const content = readFileSync(path, "utf8");
  const occurrences = content.split(previousCommit).length - 1;
  if (occurrences !== ENGINE_CRATES.length) {
    throw new Error(
      `${path}: expected ${String(ENGINE_CRATES.length)} active commit carriers; observed ${String(occurrences)}`,
    );
  }
  writeFileSync(path, content.replaceAll(previousCommit, commit));
}

function assertCarrierFilesClean(repoRoot) {
  const output = git(repoRoot, [
    "status",
    "--porcelain=v1",
    "--",
    ...ACTIVE_CARRIER_PATHS,
  ]);
  if (output.trim().length > 0) {
    throw new Error(
      `active Engine carrier or lock files are dirty; preserve or commit them before update:\n${output.trim()}`,
    );
  }
}

function scopedDiff(repoRoot) {
  return git(repoRoot, ["diff", "--binary", "--", ...ACTIVE_CARRIER_PATHS]);
}

function readFile(repoRoot, relativePath, violations) {
  try {
    return readFileSync(resolve(repoRoot, relativePath), "utf8");
  } catch (error) {
    violations.push(
      `${relativePath}: missing or unreadable (${error.message})`,
    );
    return null;
  }
}

function compareSets(path, expected, observed, violations) {
  const expectedSet = new Set(expected);
  const observedSet = new Set(observed);
  for (const value of expectedSet) {
    if (!observedSet.has(value)) violations.push(`${path}: missing ${value}`);
  }
  for (const value of observedSet) {
    if (!expectedSet.has(value))
      violations.push(`${path}: unexpected ${value}`);
  }
}

function dependencySectionNames() {
  return [
    "dependencies",
    "devDependencies",
    "optionalDependencies",
    "peerDependencies",
  ];
}

function isEnginePackageReference(name, observed) {
  if (name.toLowerCase().startsWith("@rusty-engine/")) return true;
  if (typeof observed !== "string") return false;
  if (observed.toLowerCase().includes("@rusty-engine/")) return true;
  return isEngineRepositoryReference(observed);
}

function isEngineRepositoryReference(value) {
  const normalized = value.toLowerCase();
  return (
    normalized.includes("fuzzyslipper/rusty-engine") ||
    /(?:^|[/])rusty-engine(?:[/#&.]|$)/u.test(normalized)
  );
}

function discoverManifests(repoRoot, fileName) {
  const discovered = [];
  const visit = (directory, relativeDirectory) => {
    for (const entry of readdirSync(directory, { withFileTypes: true })) {
      if (entry.isSymbolicLink()) continue;
      const relativePath =
        relativeDirectory.length === 0
          ? entry.name
          : `${relativeDirectory}/${entry.name}`;
      if (entry.isDirectory()) {
        if (!MANIFEST_SCAN_IGNORES.has(entry.name)) {
          visit(resolve(directory, entry.name), relativePath);
        }
      } else if (entry.isFile() && entry.name === fileName) {
        discovered.push(relativePath);
      }
    }
  };
  visit(repoRoot, "");
  return discovered.sort();
}

function packageSpecifier(commit, path) {
  return `github:FuzzySlipper/rusty-engine#${commit}&path:${path}`;
}

function codeloadSpecifier(commit, path) {
  return `https://codeload.github.com/FuzzySlipper/rusty-engine/tar.gz/${commit}#path:${path}`;
}

function assertCommit(commit, label) {
  if (typeof commit !== "string" || !COMMIT_PATTERN.test(commit)) {
    throw new Error(
      `${label} must be one lowercase 40-character hexadecimal commit; observed ${String(commit)}`,
    );
  }
}

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/gu, "\\$&");
}

function git(cwd, args) {
  return run("git", args, { cwd });
}

function run(command, args, { cwd, input, allowFailure = false } = {}) {
  const result = spawnSync(command, args, {
    cwd,
    input,
    encoding: "utf8",
    env: { ...process.env, NO_COLOR: "1" },
    maxBuffer: 64 * 1024 * 1024,
  });
  if (result.status !== 0 && !allowFailure) {
    throw new Error(
      `${command} ${args.join(" ")} failed (${String(result.status)}):\n${result.stderr || result.stdout}`,
    );
  }
  return result.stdout ?? "";
}
