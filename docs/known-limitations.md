# Known limitations

The interactive product proves one durable adventure shell with two selectable
authored paths and one bounded d20 encounter in each. It is not yet a broad
d20 game.

- The live slice has one player and one target per path. Warden's Gate uses
  Steel Guard armor, Guard reactions, physical actions, and its Warden reward;
  Ember's Wake uses Resolve equipment, Focus reactions, fire/psychic actions,
  and its Ash Seer reward. Both expose vitality, attributed affinities,
  scheduled effects, explicit player/opposition turns, deterministic opponent
  policy, terminal outcomes, and bounded defeat recovery. It does not yet
  define multiple-combatant initiative, movement, spellcasting, advancement,
  branching or multi-encounter progression, generated rewards, or broader
  content.
- Selection admits only the chosen exact package closure. Warden's Gate does
  not silently load Ember rules, Ember's Wake does not silently load Steel
  rules, and the non-selectable catalog probe never appears as a product path.
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
- Each camp inventory/equipment path proves two carried items, one stash
  spare, one opponent reward, capacity rejection, equipment attribution, and
  persistence. It does not yet define consumables, shops, loot generation,
  item comparison, crafting, or a substantial authored item catalog.
- The retained compass and minimap remain product-neutral building blocks.
  Rusty D20 does not connect them until it owns real navigation facts.

Broader Rusty D20 product and UI expansion is downstream follow-up, not an
acceptance requirement for the Rusty Engine GM7 mechanism campaign.
