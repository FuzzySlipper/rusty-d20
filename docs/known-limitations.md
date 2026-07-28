# Known limitations

The headless semantic kernel proves a bounded d20 rules and action slice. The
interactive product is still a bootstrap, not a playable d20 game.

- The Rust host still exposes one bootstrap entity and only
  health/version/readout operations; it does not expose `D20Session` commands.
- The kernel covers ability modifiers, one attack/check shape, typed damage
  affinities, armor, reactions, ongoing effects, explicit turns, and complete
  reopen. It does not yet define initiative, movement, spellcasting,
  advancement, encounters, or broader d20 content.
- TypeScript now authors checked starter artifacts, but there is no content
  publication service, watch mode, browser editor, or ruleset migration policy.
  Artifact generation remains an explicit build-time command.
- Save bytes are an in-process product format with exact Engine and ruleset
  identity checks. Filesystem layout, migration between rulesets, and user save
  management are not implemented.
- The host is local/trusted and has no authentication, TLS, rate limiting, or
  multi-user policy.
- The retained presentational widgets are not product proof until connected to
  Rust-owned projections through real features.

Do not hide these boundaries with fake product data. Extend the real Rust
contract and production store when a milestone needs new behavior.
