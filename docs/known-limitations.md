# Known limitations

The bootstrap proves the permanent Rust/Angular product path, not a playable
d20 game.

- No d20 candidate schema, semantic compiler, action resolution, or ruleset
  content exists yet.
- No authored TypeScript rules SDK exists yet.
- The authoritative runtime contains one bootstrap entity and exposes only
  health/version/readout operations.
- Complete saves, fresh-process reopen, encounter flow, turns, effects,
  inventory, equipment, and combat UI are later milestones.
- The host is local/trusted and has no authentication, TLS, rate limiting, or
  multi-user policy.
- The retained presentational widgets are not product proof until connected to
  Rust-owned projections through real features.

Do not hide these boundaries with fake product data. Extend the real Rust
contract and production store when a milestone needs new behavior.
