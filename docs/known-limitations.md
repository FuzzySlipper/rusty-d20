# Known limitations

The interactive product proves one bounded authored d20 encounter. It is not
yet a broad d20 game.

- The live slice has one player, one target, two Steel Guard actions, one armor
  reaction, vitality, attributed resistance, scheduled effects, and explicit
  turn advancement. It does not yet define initiative, movement, spellcasting,
  advancement, encounter selection, defeat consequences, or broader content.
- The checked authoring catalog also contains the Ember Ward composition, but
  the first interactive encounter intentionally loads Starter Core plus Steel
  Guard only.
- There is no content publication service, watch mode, browser editor, or
  ruleset migration policy. Artifact generation remains an explicit build-time
  command.
- The host uses one explicit local save path, defaulting to
  `target/rusty-d20/save.json`. There is no save-slot UI, cloud/storage policy,
  authentication, TLS, multi-user coordination, or migration between Engine or
  ruleset revisions.
- Opaque action previews are process-local and intentionally not saved. Save is
  unavailable until the pending action is resolved, including after a reaction
  commits its cost and effect. Completed entity state, turn, RNG position,
  operation/log identities, and bounded receipt explanations persist.
- The retained inventory/equipment and world-navigation widgets remain
  product-neutral building blocks. They are not Rusty D20 product proof until a
  later downstream feature connects them to Rust-owned projections.

Broader Rusty D20 product and UI expansion is downstream follow-up, not an
acceptance requirement for the Rusty Engine GM7 mechanism campaign.
