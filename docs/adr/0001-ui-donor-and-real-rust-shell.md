# ADR 0001: One durable UI donor and a real Rust shell

Status: accepted

Rusty D20 uses the reviewed `rusty-engine-ui` copy as its only UI foundation.
It preserves the donor's layered package boundaries and product-neutral
widgets, while replacing fake product wiring before the first runnable commit.

The normal application is served by `rusty-d20-host` and obtains its initial
readout through a strict same-origin transport. Fake transport exists only in
`libs/testing-fixtures`. The donor's fixture-driven HUD/inventory screens were
excluded rather than establishing placeholder behavior that later gameplay
work would need to remove.

Consequences:

- GM7 and later product work extend one permanent shell.
- Browser proof reaches a Rust process from the start.
- Reusable widgets remain available without carrying demo authority.
- D20 semantics, content, and save behavior remain explicitly unimplemented
  until their owning milestones.
