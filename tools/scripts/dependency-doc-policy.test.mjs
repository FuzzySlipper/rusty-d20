import assert from "node:assert/strict";
import test from "node:test";

import { containsStaleEngineClaim } from "./dependency-doc-policy.mjs";

test("rejects current Engine pin and revision-carrier claims", () => {
  for (const claim of [
    "Although the pinned Engine renderer supports sprites",
    "Packages come from the reviewed Engine revision.",
    "engine-source.json selects the provider.",
    "Run scripts/engine-revision update.",
    "Cargo.lock records the exact Engine revision.",
    "There is no sibling path fallback.",
  ]) {
    assert.equal(containsStaleEngineClaim(claim), true, claim);
  }
});

test("allows adjacent-provider and negative revision-identity guidance", () => {
  assert.equal(
    containsStaleEngineClaim(
      "The adjacent Engine checkout is consumed as-is; Engine revision identity does not enter saves.",
    ),
    false,
  );
});
