import { readFileSync, readdirSync, statSync } from "node:fs";
import { join, relative } from "node:path";

import { containsStaleEngineClaim } from "./dependency-doc-policy.mjs";

const root = process.cwd();
const packageJson = JSON.parse(
  readFileSync(join(root, "package.json"), "utf8"),
);
const scripts = new Set(Object.keys(packageJson.scripts ?? {}));
const docs = collectDocs([
  "README.md",
  "AGENTS.md",
  "docs",
  "apps/app-e2e/src/live/docs",
]);
const failures = [];
const currentDependencyDocs = new Set([
  "AGENTS.md",
  "README.md",
  "docs/adjacent-engine-dependency.md",
  "docs/design.md",
  "docs/known-limitations.md",
  "docs/rules-authoring.md",
  "docs/verification.md",
]);

for (const docPath of docs) {
  const rel = relative(root, docPath);
  const text = readFileSync(docPath, "utf8");
  for (const match of text.matchAll(/\b(?:pnpm|npm) run ([a-zA-Z0-9:_-]+)/g)) {
    const script = match[1];
    if (!scripts.has(script)) {
      failures.push(`${rel} cites missing package script: ${script}`);
    }
  }
  const dependencyText =
    rel === "docs/source-provenance.md"
      ? text.split("\n## Rusty Engine UI donor", 1)[0]
      : currentDependencyDocs.has(rel)
        ? text
        : "";
  if (containsStaleEngineClaim(dependencyText)) {
    failures.push(
      `${rel} contains a stale Engine pin or revision-carrier claim`,
    );
  }
}

if (failures.length > 0) {
  console.error(failures.join("\n"));
  process.exit(1);
}

console.log("docs command references ok");

function collectDocs(entries) {
  const docs = [];
  for (const entry of entries) {
    const path = join(root, entry);
    try {
      const stats = statSync(path);
      if (stats.isDirectory()) {
        docs.push(...collectMarkdown(path));
      } else {
        docs.push(path);
      }
    } catch {
      // Optional docs paths are allowed to be absent.
    }
  }
  return docs;
}

function collectMarkdown(directory) {
  const files = [];
  for (const entry of readdirSync(directory)) {
    const path = join(directory, entry);
    const stats = statSync(path);
    if (stats.isDirectory()) {
      files.push(...collectMarkdown(path));
    } else if (entry.endsWith(".md")) {
      files.push(path);
    }
  }
  return files;
}
