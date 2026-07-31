import { describe, expect, it } from "vitest";

import { resolveCombatLogAutoFollow, type CombatLogEntryView } from "./index";

const entry = (id: number): CombatLogEntryView => ({
  id,
  source: `T1 action-${id}`,
  text: `Entry ${id}`,
  severity: "info",
  details: [`Resolution ${id}`],
});

describe("combat log auto-follow", () => {
  it("follows initial and newly published stable entry identities only", () => {
    expect(resolveCombatLogAutoFollow(null, [])).toEqual({
      latestEntryId: null,
      shouldScroll: false,
    });
    expect(resolveCombatLogAutoFollow(null, [entry(1), entry(2)])).toEqual({
      latestEntryId: 2,
      shouldScroll: true,
    });
    expect(resolveCombatLogAutoFollow(2, [entry(1), entry(2)])).toEqual({
      latestEntryId: 2,
      shouldScroll: false,
    });
    expect(resolveCombatLogAutoFollow(2, [entry(2), entry(3)])).toEqual({
      latestEntryId: 3,
      shouldScroll: true,
    });
  });
});
