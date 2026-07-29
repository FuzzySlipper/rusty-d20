# Known limitations

The interactive product proves one durable adventure shell and one bounded
authored d20 encounter. It is not yet a broad d20 game.

- The live slice has one player, one target, two Steel Guard actions, one armor
  reaction, vitality, attributed resistance, scheduled effects, explicit
  player/opposition turn ownership, a deterministic opponent policy, terminal
  outcomes, one fixed reward, and bounded defeat recovery. Camp exposes one
  authored encounter choice. It does not yet define multiple-combatant
  initiative, movement, spellcasting, advancement, encounter selection beyond
  this one resolved encounter, generated rewards, or broader content.
- The checked authoring catalog also contains the Ember Ward rules package and
  a second content-only adventure composition. The interactive default selects
  only the exact Starter Core, Steel Guard, and Warden's Gate dependency
  closure; Ember Ward is not silently loaded.
- There is no content publication service, watch mode, browser editor, or
  ruleset migration policy. Artifact generation remains an explicit build-time
  command.
- The host uses one explicit local save path, defaulting to
  `target/rusty-d20/save.json`. There is no save-slot UI, cloud/storage policy,
  authentication, TLS, multi-user coordination, or migration between Engine or
  ruleset revisions. Product schemas 1 through 4 have explicit migrations into
  schema 5, including deterministic installation of the starter loadout and a
  defined legacy encounter turn or vitality-derived terminal outcome; this is
  not a general migration framework. Schema 5 binds saves to the exact authored
  adventure package composition.
- Opaque action previews are process-local and intentionally not saved. Save is
  unavailable until the pending action is resolved, including after a reaction
  commits its cost and effect. Completed entity state, turn, RNG position,
  operation/log identities, and bounded receipt explanations persist.
- Camp inventory/equipment proves two carried armor items, one stash spare,
  one opponent armor reward, capacity rejection, equipment attribution, and
  persistence. It does not yet define consumables, stacking, shops, loot
  generation, item comparison, crafting, or a substantial authored item
  catalog.
- The retained compass and minimap remain product-neutral building blocks.
  Rusty D20 does not connect them until it owns real navigation facts.

Broader Rusty D20 product and UI expansion is downstream follow-up, not an
acceptance requirement for the Rusty Engine GM7 mechanism campaign.
