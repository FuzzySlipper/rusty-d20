# Known limitations

The interactive product now proves a small Gold Box-style game loop with two
selectable authored paths: camp, Rust-owned first-person grid exploration,
hidden encounter triggers, the existing modal combat screen, outcomes, and
complete saves. Warden's Gate has two ordered encounters and Ember's Wake has
one. This is the exploration foundation for the planned Ruleweaver game, not
yet that complete game.

- The live slice has one player and one target per path. Warden's Gate uses
  Steel Guard armor, canonical Training Blade/Field Bow implements, Guard
  reactions, physical/control actions, and its Warden reward; Ember's Wake
  uses Nerve equipment, Focus reactions, energy/resolve actions, and its Ash
  Seer reward. Both expose vitality, attributed affinities,
  scheduled effects, explicit player/opposition turns, deterministic opponent
  policy, terminal outcomes, bounded defeat recovery, ordered campaign
  completion, and exactly-once rewards. Combat is still the earlier
  non-spatial one-hero/one-target presentation; it does not yet define the
  overhead tactical grid, multiple-combatant initiative and live activation
  budget consumption/reset,
  combat movement, spellcasting, advancement, branching/repeatable campaign
  graphs, generated rewards, or broader Ruleweaver content.
- Each path has one bounded floor with step-forward/backward and turn-left/right
  movement, collision, one or two hidden triggers, one landmark, a three-depth
  corridor projection, compass, and discovered-cell map. There are no doors,
  keys, stairs, multiple floors, traps, roaming encounters, lighting,
  navigation audio, or exploration-time party selection yet.
- The selected Ruleweaver foundation is translated: six attributes, four
  defenses, Standard/Bonus/Reaction/Movement budget definitions, shaped
  actions, Held/Unsettled condition clauses, and explicit armor/implement
  binding. This remains deliberately bounded. There is no bulk-imported class,
  talent, power, monster, item, scenario, or D&D 4e catalog. Party semantics,
  live per-actor budget reset/consumption, tactical-grid range/line-of-effect,
  forced movement, and the first bounded Ruleweaver adventure remain owned by
  the following milestones.
- Selection admits only the chosen exact package closure. Warden's Gate does
  not silently load Ember rules, Ember's Wake does not silently load Steel
  rules, and the non-selectable catalog probe never appears as a product path.
- There is no content publication service, watch mode, browser editor, or
  ruleset migration policy. Artifact generation remains an explicit build-time
  command.
- The host uses one explicit local save path, defaulting to
  `target/rusty-d20/save.json`. There is no save-slot UI, cloud/storage policy,
  authentication, TLS, multi-user coordination, or general migration between
  Engine or ruleset revisions. Product schemas 1 through 6 have explicit
  migrations into schema 7, including deterministic installation of the
  starter loadout and a
  defined legacy encounter turn or vitality-derived terminal outcome; this is
  not a general migration framework. Schema 7 binds saves to the exact authored
  adventure package composition, dungeon progress, and completed encounter
  prefix. The host can
  discard one malformed or obsolete save through a guarded visible reset, but
  it cannot repair arbitrary content.
- Opaque action previews are process-local and intentionally not saved. Save is
  unavailable until the pending action is resolved, including after a reaction
  commits its cost and effect. Completed entity state, turn, RNG position,
  operation/log identities, and bounded receipt explanations persist.
- The default camp inventory/equipment path proves four carried items
  (two armor pieces and two implements), one stash spare, one opponent reward,
  capacity rejection, equipment attribution/binding, and persistence. It does
  not yet define consumables, shops, loot generation, item comparison,
  crafting, or a substantial authored item catalog.
- The retained compass and minimap are now real product-neutral consumers of
  Rust-projected facing and discovered cells. The complete dungeon topology and
  hidden trigger coordinates never enter browser state.

The authoritative party/initiative model, overhead combat view, and first
bounded Ruleweaver adventure are tracked as downstream milestones rather than
being implied complete by this foundation translation.
