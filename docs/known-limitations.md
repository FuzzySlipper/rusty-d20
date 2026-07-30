# Known limitations

The interactive product now proves a small Gold Box-style game loop with two
selectable authored paths: camp, Rust-owned first-person grid exploration,
hidden encounter triggers, a rendered overhead combat scene with overlay HUD,
outcomes, and
complete saves. Warden's Gate is the first complete bounded Ruleweaver-derived
adventure with three ordered encounters, treasure, a locked door, a durable
checkpoint, and a terminal ending; Ember's Wake has one encounter.

The browser now mounts one permanent full-window Rusty Engine renderer and
places phase UI in named overlay regions. Encounter and outcome modes now render
the Rust-projected tactical board, occupancy, movement routes, and combat states
through that shared surface; exploration uses the bounded corridor frame.
Catalog, camp, terminal, loading, and failure modes retain abstract
presentation-only backdrops. The remaining renderer-first campaign gaps are
recorded in Den rather than implied complete: task 6429 adds brief camera
tweens, task 6430 replaces the temporary target dropdown with action-then-grid
picking, and task 6431 certifies the combined loop. A native window/input host
remains a later downstream consumer; this browser composition does not claim
to be that host.

- The live slice has an authored four-person Warden company and four
  Ruleweaver/Asha-derived opposition roles,
  canonical initiative and activation budgets, and a bounded overhead tactical
  board. Warden's Gate uses
  Steel Guard armor, canonical Training Blade/Field Bow implements, Guard
  reactions, physical/control actions, and its Warden reward; Ember's Wake
  uses Nerve equipment, Focus reactions, energy/resolve actions, and its Ash
  Seer reward. Both expose vitality, attributed affinities,
  scheduled effects, explicit player/opposition turns, deterministic opponent
  policy, Engine-routed combat movement, range and line of effect, conditions,
  forced movement, terminal outcomes, bounded defeat recovery, ordered
  terminal adventure completion, and exactly-once rewards. It does not yet define
  spellcasting, advancement, branching/repeatable campaign graphs, generated
  rewards, or broader Ruleweaver content.
- Each path has one bounded floor with step-forward/backward and turn-left/right
  movement, collision, hidden triggers, landmarks, a three-depth corridor
  projection rendered as a retained Rusty Engine Three/WebGL scene, compass,
  and discovered-cell map. Warden's Gate adds one claimed treasure, one
  treasure-gated door, and a safe-return checkpoint. The current visual scene
  uses untextured floor, ceiling, and wall cuboids. Although the pinned Engine
  renderer supports sprites and billboard hosts, Rust does not yet project an
  exploration-visible actor or prop appearance, so the game does not invent a
  browser-only sprite to exercise them. There are no stairs, multiple floors,
  traps, roaming encounters, authored lighting, navigation audio, or
  exploration-time party selection yet.
- The selected Ruleweaver foundation is translated: six attributes, four
  defenses, Standard/Bonus/Reaction/Movement budget definitions, shaped
  actions, Held/Unsettled condition clauses, and explicit armor/implement
  binding. This remains deliberately bounded. There is no bulk-imported class,
  talent, power, monster, item, scenario, or D&D 4e catalog. Warden's Gate
  adapts only the bounded Crosswind role shape and independently authored
  values/content described in source provenance.
- Selection admits only the chosen exact package closure. Warden's Gate does
  not silently load Ember rules, Ember's Wake does not silently load Steel
  rules, and the non-selectable catalog probe never appears as a product path.
- There is no content publication service, watch mode, browser editor, or
  ruleset migration policy. Artifact generation remains an explicit build-time
  command.
- The host uses one explicit local save path, defaulting to
  `target/rusty-d20/save.json`. There is no save-slot UI, cloud/storage policy,
  authentication, TLS, multi-user coordination, or general migration between
  Engine or ruleset revisions. Product schemas 1 through 9 and session schemas
  before 5 are rejected; this is not a general migration framework. Schema 10
  binds saves to the exact authored adventure package composition, dungeon
  events/progress, tactical positions, completed encounter prefix, and terminal
  ending. The host can
  discard one malformed or obsolete save through a guarded visible reset, but
  it cannot repair arbitrary content.
- Opaque reaction prompts are process-local and intentionally not saved. Save
  is unavailable until the player chooses or declines that reaction; either
  command then resolves the roll atomically. Completed entity state, turn,
  seeded/static roll-source configuration and position, operation/log
  identities, and bounded receipt explanations persist.
- The default camp inventory/equipment path proves a four-item active-character
  pack, a projected 24-slot shared inventory seeded with four authored spares,
  drag/drop and keyboard/click/touch-compatible preparation, one opponent
  reward, one dungeon treasure, capacity rejection without mutation,
  equipment attribution/binding, and save/reopen. The same party loadout opens
  over exploration without changing phase, but is intentionally read-only
  because Rust admits loadout placement only in camp. It does not yet define
  consumables, shops, loot generation, item comparison, crafting, or a
  substantial authored item catalog.
- The retained compass and minimap are now real product-neutral consumers of
  Rust-projected facing and discovered cells. The complete dungeon topology and
  hidden trigger coordinates never enter browser state.

Later work expands this complete bounded adventure rather than treating it as a
sprawling Ruleweaver campaign.
