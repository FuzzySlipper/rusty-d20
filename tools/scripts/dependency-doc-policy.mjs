const staleEngineClaims = [
  /\bpinned Engine\b/iu,
  /\breviewed Engine revision\b/iu,
  /\bengine-source\.json\b/iu,
  /\bscripts\/engine-revision\b/iu,
  /\bCargo\.lock records the exact (?:Engine|resolved) revision\b/iu,
  /\bno sibling path fallback/iu,
];

export function containsStaleEngineClaim(text) {
  return staleEngineClaims.some((pattern) => pattern.test(text));
}
