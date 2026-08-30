using System.Security.Cryptography;
using System.Text;
using RustyD20.Core.Contract;

namespace RustyD20.Core.Rules;

/// <summary>Strictly admits the closed authored D20 catalog. C# modules are its only input.</summary>
public sealed class D20SemanticCompiler
{
    public CompiledD20Content Compile(IEnumerable<D20ContentModule> input)
    {
        var modules = input.OrderBy(module => module.Id.Value, StringComparer.Ordinal).ToArray();
        var diagnostics = new List<D20Diagnostic>();
        var moduleIds = modules.Select(module => module.Id).ToHashSet();

        foreach (var module in modules)
        {
            if (!D20Schemas.IsCurrentContent(module.ContentSchema))
            {
                Add("D20_UNSUPPORTED_CONTENT_SCHEMA", $"module uses unsupported content schema '{module.ContentSchema}'", module.Source);
            }

            foreach (var dependency in module.Dependencies)
            {
                if (!moduleIds.Contains(dependency))
                {
                    Add("D20_UNKNOWN_MODULE_DEPENDENCY", $"module dependency '{dependency}' is not composed", module.Source);
                }
            }

            if (module.AdventuresOrEmpty.Count > D20Limits.AdventuresPerPackage)
            {
                Add("D20_ADVENTURE_QUOTA", $"module contains more than {D20Limits.AdventuresPerPackage} adventures", module.Source);
            }
        }

        ValidateModuleGraph();

        var abilities = Collect(modules, module => module.AbilitiesOrEmpty, definition => definition.Id, "ability");
        var defenses = Collect(modules, module => module.DefensesOrEmpty, definition => definition.Id, "defense");
        var budgets = Collect(modules, module => module.BudgetsOrEmpty, definition => definition.Id, "activation budget");
        var damageTypes = Collect(modules, module => module.DamageTypesOrEmpty, definition => definition.Id, "damage type");
        var resources = Collect(modules, module => module.ResourcesOrEmpty, definition => definition.Id, "resource");
        var armors = Collect(modules, module => module.ArmorsOrEmpty, definition => definition.Id, "armor");
        var implements = Collect(modules, module => module.ImplementsOrEmpty, definition => definition.Id, "implement");
        var effects = Collect(modules, module => module.EffectsOrEmpty, definition => definition.Id, "effect");
        var reactions = Collect(modules, module => module.ReactionsOrEmpty, definition => definition.Id, "reaction");
        var actions = Collect(modules, module => module.ActionsOrEmpty, definition => definition.Id, "action");
        var features = Collect(modules, module => module.FeaturesOrEmpty, definition => definition.Id, "feature");
        var characters = Collect(modules, module => module.CharactersOrEmpty, definition => definition.Id, "character");
        var storage = Collect(modules, module => module.StorageOrEmpty, definition => definition.Id, "storage");
        var items = Collect(modules, module => module.ItemsOrEmpty, definition => definition.Id, "item");
        var encounters = Collect(modules, module => module.EncountersOrEmpty, definition => definition.Id, "encounter");
        var adventures = Collect(modules, module => module.AdventuresOrEmpty, definition => definition.Id, "adventure");

        ValidateReferences();
        ValidateAdventures();
        ValidateDefaults();

        if (diagnostics.Count != 0)
        {
            throw new D20CompilationException(diagnostics.OrderBy(diagnostic => diagnostic.Code, StringComparer.Ordinal).ThenBy(diagnostic => diagnostic.CorrelationId, StringComparer.Ordinal).ToArray());
        }

        var fingerprint = Fingerprint(modules);
        var sources = modules.Select(module => module.Source).OrderBy(source => source.SourcePath, StringComparer.Ordinal).ToArray();
        var definitionCount = abilities.Count + defenses.Count + budgets.Count + damageTypes.Count + resources.Count + armors.Count + implements.Count + effects.Count + reactions.Count + actions.Count + features.Count + characters.Count + storage.Count + items.Count + encounters.Count;
        return new CompiledD20Content(fingerprint, adventures, characters, items, modules, new CompilationReceipt(fingerprint, sources, definitionCount, adventures.Count));

        Dictionary<D20Id, T> Collect<T>(IEnumerable<D20ContentModule> sourceModules, Func<D20ContentModule, IReadOnlyList<T>> select, Func<T, D20Id> identity, string kind) where T : notnull
        {
            var result = new Dictionary<D20Id, T>();
            foreach (var module in sourceModules)
            {
                var definitions = select(module);
                if (definitions.Count > D20Limits.DefinitionsPerKind)
                {
                    Add("D20_DEFINITION_QUOTA", $"module contains more than {D20Limits.DefinitionsPerKind} {kind} definitions", module.Source);
                }

                foreach (var definition in definitions)
                {
                    var id = identity(definition);
                    if (!result.TryAdd(id, definition))
                    {
                        Add("D20_DUPLICATE_ID", $"duplicate {kind} identity '{id}'", module.Source, id.Value);
                    }
                }
            }

            return result;
        }

        void ValidateReferences()
        {
            foreach (var defense in defenses.Values)
            {
                RequireAll(abilities, defense.Abilities, defense.Source, "D20_UNKNOWN_ABILITY", "defense ability");
            }

            foreach (var armor in armors.Values)
            {
                Require(defenses, armor.Defense, armor.Source, "D20_UNKNOWN_DEFENSE", "armor defense");
            }

            foreach (var implement in implements.Values)
            {
                Require(abilities, implement.Ability, implement.Source, "D20_UNKNOWN_ABILITY", "implement ability");
                Require(defenses, implement.Defense, implement.Source, "D20_UNKNOWN_DEFENSE", "implement defense");
                Require(damageTypes, implement.Damage.Kind, implement.Source, "D20_UNKNOWN_DAMAGE_TYPE", "implement damage");
                Bound(implement.Tags.Count, D20Limits.ImplementTags, implement.Source, "D20_IMPLEMENT_TAG_QUOTA");
                Bound(implement.Range, D20Limits.TacticalRange, implement.Source, "D20_TACTICAL_RANGE");
                ValidateDamage(implement.Damage, implement.Source);
            }

            foreach (var effect in effects.Values)
            {
                if (effect.Defense is { } defense) Require(defenses, defense, effect.Source, "D20_UNKNOWN_DEFENSE", "effect defense");
                Bound(effect.DurationTurns, D20Limits.EffectDurationTurns, effect.Source, "D20_EFFECT_DURATION");
                Bound(effect.Conditions.Count, D20Limits.ConditionClauses, effect.Source, "D20_CONDITION_QUOTA");
            }

            foreach (var reaction in reactions.Values)
            {
                Require(defenses, reaction.Defense, reaction.Source, "D20_UNKNOWN_DEFENSE", "reaction defense");
                Require(resources, reaction.Resource, reaction.Source, "D20_UNKNOWN_RESOURCE", "reaction resource");
                Require(effects, reaction.Effect, reaction.Source, "D20_UNKNOWN_EFFECT", "reaction effect");
                ValidateCosts(reaction.Costs, ActivationTiming.Reaction, reaction.Source);
            }

            foreach (var action in actions.Values)
            {
                Bound(action.Tags.Count, D20Limits.ActionTags, action.Source, "D20_ACTION_TAG_QUOTA");
                Bound(action.Costs.Count, D20Limits.ActivationCosts, action.Source, "D20_ACTIVATION_COST_QUOTA");
                Bound(action.Target.MaximumTargets, D20Limits.ActionTargets, action.Source, "D20_ACTION_TARGET_QUOTA");
                Bound(action.ForcedMovement, D20Limits.ForcedMovement, action.Source, "D20_FORCED_MOVEMENT");
                ValidateCosts(action.Costs, ActivationTiming.Action, action.Source);
                if (action.Attack.Implement is { } implement) Require(implements, implement, action.Source, "D20_UNKNOWN_IMPLEMENT", "action implement");
                if (action.Attack.Ability is { } ability) Require(abilities, ability, action.Source, "D20_UNKNOWN_ABILITY", "action ability");
                if (action.Attack.Defense is { } defense) Require(defenses, defense, action.Source, "D20_UNKNOWN_DEFENSE", "action defense");
                if (action.Attack.Damage is { } damage)
                {
                    Require(damageTypes, damage.Kind, action.Source, "D20_UNKNOWN_DAMAGE_TYPE", "action damage");
                    ValidateDamage(damage, action.Source);
                }

                if (action.Effect is { } effect) Require(effects, effect, action.Source, "D20_UNKNOWN_EFFECT", "action effect");
            }

            foreach (var character in characters.Values)
            {
                if (character.Experience is < 0 or > D20Limits.Experience) Add("D20_EXPERIENCE_LIMIT", "character experience is outside the admitted bound", character.Source);
                RequireAll(abilities, character.Abilities.Keys, character.Source, "D20_UNKNOWN_ABILITY", "character ability");
                RequireAll(actions, character.Actions, character.Source, "D20_UNKNOWN_ACTION", "character action");
                RequireAll(reactions, character.Reactions, character.Source, "D20_UNKNOWN_REACTION", "character reaction");
                RequireAll(features, character.Features, character.Source, "D20_UNKNOWN_FEATURE", "character feature");
                Text(character.Name, character.Source);
                Text(character.Title, character.Source);
            }

            foreach (var item in items.Values)
            {
                if (!characters.ContainsKey(item.Owner) && !storage.ContainsKey(item.Owner))
                {
                    Add("D20_UNKNOWN_ITEM_OWNER", $"item owner references unknown identity '{item.Owner}'", item.Source, item.Owner.Value);
                }
                if (item.EquipmentKind == EquipmentKind.Armor) Require(armors, item.Equipment, item.Source, "D20_UNKNOWN_ARMOR", "item armor");
                else Require(implements, item.Equipment, item.Source, "D20_UNKNOWN_IMPLEMENT", "item implement");
            }

            foreach (var encounter in encounters.Values)
            {
                if (encounter.Roster.Count is 0 or > D20Limits.EncounterParticipants) Add("D20_ENCOUNTER_PARTICIPANT_QUOTA", "encounter roster is outside the admitted bound", encounter.Source);
                RequireAll(characters, encounter.Roster.Select(participant => participant.Character), encounter.Source, "D20_UNKNOWN_CHARACTER", "encounter character");
                ValidateBoard(encounter.Board, encounter.Source, "tactical");
            }
        }

        void ValidateModuleGraph()
        {
            foreach (var duplicate in modules.GroupBy(module => module.Id).Where(group => group.Count() > 1))
            {
                Add("D20_DUPLICATE_MODULE_ID", $"duplicate content module identity '{duplicate.Key}'", duplicate.First().Source, duplicate.Key.Value);
            }

            var byId = modules.GroupBy(module => module.Id).ToDictionary(group => group.Key, group => group.First());
            var state = new Dictionary<D20Id, VisitState>();
            var reportedCycles = new HashSet<D20Id>();
            foreach (var module in modules)
            {
                Visit(module.Id);
            }

            return;

            void Visit(D20Id id)
            {
                if (state.TryGetValue(id, out var seen))
                {
                    if (seen == VisitState.Visiting && reportedCycles.Add(id))
                    {
                        var source = byId[id].Source;
                        Add("D20_MODULE_DEPENDENCY_CYCLE", $"content module dependency cycle includes '{id}'", source, id.Value);
                    }

                    return;
                }

                state[id] = VisitState.Visiting;
                foreach (var dependency in byId[id].Dependencies)
                {
                    if (byId.ContainsKey(dependency)) Visit(dependency);
                }

                state[id] = VisitState.Complete;
            }
        }

        void ValidateAdventures()
        {
            foreach (var adventure in adventures.Values)
            {
                if (adventure.Party.Count is 0 or > D20Limits.PartyMembers) Add("D20_PARTY_QUOTA", "adventure party is outside the admitted bound", adventure.Source);
                if (adventure.Characters.Count > D20Limits.AdventureEntries || adventure.Items.Count > D20Limits.AdventureEntries || adventure.Encounters.Count > D20Limits.AdventureEntries) Add("D20_ADVENTURE_ENTRY_QUOTA", "adventure collection is outside the admitted bound", adventure.Source);
                RequireAll(characters, adventure.Party, adventure.Source, "D20_UNKNOWN_CHARACTER", "party character");
                RequireAll(characters, adventure.Characters, adventure.Source, "D20_UNKNOWN_CHARACTER", "adventure character");
                Require(storage, adventure.CampStorage, adventure.Source, "D20_UNKNOWN_STORAGE", "camp storage");
                RequireAll(storage, adventure.Storage, adventure.Source, "D20_UNKNOWN_STORAGE", "adventure storage");
                RequireAll(items, adventure.Items, adventure.Source, "D20_UNKNOWN_ITEM", "adventure item");
                RequireAll(encounters, adventure.Encounters, adventure.Source, "D20_UNKNOWN_ENCOUNTER", "adventure encounter");
                ValidateDungeon(adventure.Dungeon, adventure, adventure.Source);
            }
        }

        void ValidateDefaults()
        {
            var source = modules.FirstOrDefault()?.Source ?? new SourceProvenance("<composition>", "catalog", "compiler");
            if (adventures.Values.Count(adventure => adventure.IsDefault) != 1) Add("D20_DEFAULT_ADVENTURE", "exactly one admitted adventure must be the default", source);
            if (!adventures.Values.Any(adventure => adventure.Selectable)) Add("D20_NO_SELECTABLE_ADVENTURE", "at least one admitted adventure must be selectable", source);
        }

        void ValidateDungeon(DungeonDefinition dungeon, AdventureDefinition adventure, SourceProvenance source)
        {
            if (dungeon.Width <= 0 || dungeon.Width > D20Limits.DungeonWidth || dungeon.Height <= 0 || dungeon.Height > D20Limits.DungeonHeight) Add("D20_DUNGEON_SIZE", "dungeon dimensions are outside the admitted bound", source);
            ValidateBoard(new TacticalBoard(dungeon.Width, dungeon.Height, dungeon.Rows, []), source, "dungeon");
            if (!Floor(dungeon.Rows, dungeon.Start)) Add("D20_INVALID_START", "dungeon start must be a floor cell", source);
            if (!dungeon.Checkpoints.Any(value => value.Id == dungeon.StartCheckpoint)) Add("D20_UNKNOWN_CHECKPOINT", $"dungeon start checkpoint '{dungeon.StartCheckpoint}' is missing", source);

            foreach (var encounter in dungeon.Encounters)
            {
                Require(encounters, encounter.Encounter, source, "D20_UNKNOWN_ENCOUNTER", "dungeon encounter");
                Position(encounter.Position, "encounter");
            }

            foreach (var landmark in dungeon.Landmarks) Position(landmark.Position, "landmark");
            foreach (var treasure in dungeon.Treasures)
            {
                Require(items, treasure.Item, source, "D20_UNKNOWN_ITEM", "treasure item");
                Position(treasure.Position, "treasure");
            }

            foreach (var checkpoint in dungeon.Checkpoints) Position(checkpoint.Position, "checkpoint");
            foreach (var door in dungeon.Doors)
            {
                if (door.RequiresTreasure is { } treasure && !dungeon.Treasures.Any(value => value.Id == treasure)) Add("D20_UNKNOWN_TREASURE", $"door requires unknown treasure '{treasure}'", source);
                Position(door.Position, "door");
            }

            if (!dungeon.Encounters.Select(value => value.Encounter).SequenceEqual(adventure.Encounters)) Add("D20_ENCOUNTER_TOPOLOGY_ORDER", "dungeon encounter order must match the adventure order", source);
            var reachable = ReachableFloorCells(dungeon.Rows, dungeon.Start);
            foreach (var encounter in dungeon.Encounters) Reachable(encounter.Position, "encounter");
            foreach (var landmark in dungeon.Landmarks) Reachable(landmark.Position, "landmark");
            foreach (var door in dungeon.Doors) Reachable(door.Position, "door");
            foreach (var treasure in dungeon.Treasures) Reachable(treasure.Position, "treasure");
            foreach (var checkpoint in dungeon.Checkpoints) Reachable(checkpoint.Position, "checkpoint");
            void Position(GridPosition position, string kind)
            {
                if (!Floor(dungeon.Rows, position)) Add($"D20_INVALID_{kind.ToUpperInvariant()}_PLACEMENT", $"{kind} placement must be a floor cell", source);
            }

            void Reachable(GridPosition position, string kind)
            {
                if (Floor(dungeon.Rows, position) && !reachable.Contains(position))
                {
                    Add($"D20_UNREACHABLE_{kind.ToUpperInvariant()}_PLACEMENT", $"{kind} placement is not reachable from the dungeon start", source);
                }
            }
        }

        void ValidateBoard(TacticalBoard board, SourceProvenance source, string kind)
        {
            var maxWidth = kind == "tactical" ? D20Limits.TacticalBoardWidth : D20Limits.DungeonWidth;
            var maxHeight = kind == "tactical" ? D20Limits.TacticalBoardHeight : D20Limits.DungeonHeight;
            if (board.Width <= 0 || board.Width > maxWidth || board.Height <= 0 || board.Height > maxHeight || board.Rows.Count != board.Height || board.Rows.Any(row => row.Length != board.Width || row.Any(cell => cell is not '#' and not '.')))
            {
                Add($"D20_INVALID_{kind.ToUpperInvariant()}_TOPOLOGY", $"{kind} rows must be bounded rectangular '#' and '.' topology", source);
            }

            var occupied = new HashSet<GridPosition>();
            foreach (var placement in board.Placements)
            {
                if (!Floor(board.Rows, placement.Position)) Add($"D20_INVALID_{kind.ToUpperInvariant()}_PLACEMENT", $"{kind} placement must be a floor cell", source);
                if (!occupied.Add(placement.Position)) Add($"D20_DUPLICATE_{kind.ToUpperInvariant()}_PLACEMENT", $"{kind} placements cannot overlap", source);
            }
        }

        void ValidateCosts(IReadOnlyList<ActivationCost> costs, ActivationTiming expected, SourceProvenance source)
        {
            foreach (var cost in costs)
            {
                if (!budgets.TryGetValue(cost.Budget, out var budget)) Add("D20_UNKNOWN_ACTIVATION_BUDGET", $"unknown activation budget '{cost.Budget}'", source);
                else if (budget.Timing != expected || cost.Amount > budget.Initial) Add("D20_INCOMPATIBLE_ACTIVATION_COST", "activation cost has wrong timing or exceeds the authored initial budget", source);
            }
        }

        void ValidateDamage(DamageDefinition damage, SourceProvenance source)
        {
            if (damage.Dice <= 0 || damage.Dice > D20Limits.DamageDice || damage.Sides <= 0 || damage.Sides > D20Limits.DamageDieSides) Add("D20_DAMAGE_LIMIT", "damage dice or sides are outside the admitted bound", source);
        }

        void Require<T>(IReadOnlyDictionary<D20Id, T> table, D20Id id, SourceProvenance source, string code, string role) where T : notnull
        {
            if (!table.ContainsKey(id)) Add(code, $"{role} references unknown identity '{id}'", source, id.Value);
        }

        void RequireAll<T>(IReadOnlyDictionary<D20Id, T> table, IEnumerable<D20Id> ids, SourceProvenance source, string code, string role) where T : notnull
        {
            foreach (var id in ids) Require(table, id, source, code, role);
        }

        void Bound(int value, int maximum, SourceProvenance source, string code)
        {
            if (value > maximum) Add(code, $"value exceeds {maximum}", source);
        }

        void Text(string value, SourceProvenance source)
        {
            if (Encoding.UTF8.GetByteCount(value) > D20Limits.AuthoredTextBytes) Add("D20_AUTHORED_TEXT_LIMIT", $"authored text exceeds {D20Limits.AuthoredTextBytes} UTF-8 bytes", source);
        }

        void Add(string code, string message, SourceProvenance source, string? detail = null) => diagnostics.Add(new D20Diagnostic(code, message, source, source.Subject, detail));
    }

    public static string Fingerprint(IEnumerable<D20ContentModule> modules)
    {
        var lines = modules.OrderBy(module => module.Id.Value, StringComparer.Ordinal).SelectMany(CanonicalLines);
        return Convert.ToHexString(SHA256.HashData(Encoding.UTF8.GetBytes(string.Join('\n', lines)))).ToLowerInvariant();
    }

    private static IEnumerable<string> CanonicalLines(D20ContentModule module)
    {
        yield return $"module:{module.Id}|schema:{module.ContentSchema}|dependencies:{Ids(module.Dependencies)}|source:{Source(module.Source)}";
        foreach (var definition in module.AbilitiesOrEmpty.OrderBy(value => value.Id.Value, StringComparer.Ordinal)) yield return $"ability:{definition.Id}:{definition.Minimum}:{definition.Maximum}:{Source(definition.Source)}";
        foreach (var definition in module.DefensesOrEmpty.OrderBy(value => value.Id.Value, StringComparer.Ordinal)) yield return $"defense:{definition.Id}:{definition.Base}:{Ids(definition.Abilities)}:{Source(definition.Source)}";
        foreach (var definition in module.BudgetsOrEmpty.OrderBy(value => value.Id.Value, StringComparer.Ordinal)) yield return $"budget:{definition.Id}:{definition.Timing}:{definition.Initial}:{Source(definition.Source)}";
        foreach (var definition in module.DamageTypesOrEmpty.OrderBy(value => value.Id.Value, StringComparer.Ordinal)) yield return $"damage-type:{definition.Id}:{Source(definition.Source)}";
        foreach (var definition in module.ResourcesOrEmpty.OrderBy(value => value.Id.Value, StringComparer.Ordinal)) yield return $"resource:{definition.Id}:{definition.Maximum}:{Source(definition.Source)}";
        foreach (var definition in module.ArmorsOrEmpty.OrderBy(value => value.Id.Value, StringComparer.Ordinal)) yield return $"armor:{definition.Id}:{definition.Defense}:{definition.Bonus}:{definition.Slot}:{Source(definition.Source)}";
        foreach (var definition in module.ImplementsOrEmpty.OrderBy(value => value.Id.Value, StringComparer.Ordinal)) yield return $"implement:{definition.Id}:{definition.Slot}:{Ids(definition.Tags)}:{definition.Ability}:{definition.Defense}:{Damage(definition.Damage)}:{definition.Range}:{Source(definition.Source)}";
        foreach (var definition in module.EffectsOrEmpty.OrderBy(value => value.Id.Value, StringComparer.Ordinal)) yield return $"effect:{definition.Id}:{definition.Defense}:{definition.DefenseBonus}:{definition.DurationTurns}:{string.Join(',', definition.Conditions.OrderBy(value => value.Kind).ThenBy(value => value.Tag?.Value, StringComparer.Ordinal).ThenBy(value => value.Amount).Select(Condition))}:{Source(definition.Source)}";
        foreach (var definition in module.ReactionsOrEmpty.OrderBy(value => value.Id.Value, StringComparer.Ordinal)) yield return $"reaction:{definition.Id}:{definition.Defense}:{definition.Bonus}:{definition.Resource}:{definition.Cost}:{Costs(definition.Costs)}:{definition.Effect}:{Source(definition.Source)}";
        foreach (var definition in module.ActionsOrEmpty.OrderBy(value => value.Id.Value, StringComparer.Ordinal)) yield return $"action:{definition.Id}:{Ids(definition.Tags)}:{Costs(definition.Costs)}:{Target(definition.Target)}:{Attack(definition.Attack)}:{definition.Effect}:{definition.ForcedMovement}:{Source(definition.Source)}";
        foreach (var definition in module.FeaturesOrEmpty.OrderBy(value => value.Id.Value, StringComparer.Ordinal)) yield return $"feature:{definition.Id}:{definition.Label}:{definition.Description}:{Source(definition.Source)}";
        foreach (var definition in module.CharactersOrEmpty.OrderBy(value => value.Id.Value, StringComparer.Ordinal)) yield return $"character:{definition.Id}:{definition.Name}:{definition.Title}:{definition.Level}:{definition.Experience}:{definition.Vitality}:{string.Join(',', definition.Abilities.OrderBy(value => value.Key.Value, StringComparer.Ordinal).Select(value => $"{value.Key}={value.Value}"))}:{Ids(definition.Actions)}:{Ids(definition.Reactions)}:{Ids(definition.Features)}:{Source(definition.Source)}";
        foreach (var definition in module.StorageOrEmpty.OrderBy(value => value.Id.Value, StringComparer.Ordinal)) yield return $"storage:{definition.Id}:{definition.Name}:{definition.Capacity}:{Source(definition.Source)}";
        foreach (var definition in module.ItemsOrEmpty.OrderBy(value => value.Id.Value, StringComparer.Ordinal)) yield return $"item:{definition.Id}:{definition.Name}:{definition.EquipmentKind}:{definition.Equipment}:{definition.Owner}:{definition.Equipped}:{Source(definition.Source)}";
        foreach (var definition in module.EncountersOrEmpty.OrderBy(value => value.Id.Value, StringComparer.Ordinal)) yield return $"encounter:{definition.Id}:{definition.Title}:{string.Join(',', definition.Roster.OrderBy(value => value.Character.Value, StringComparer.Ordinal).ThenBy(value => value.Faction).Select(value => $"{value.Character}:{value.Faction}"))}:{Board(definition.Board)}:{Outcome(definition.Victory)}:{Outcome(definition.Defeat)}:{Source(definition.Source)}";
        foreach (var definition in module.AdventuresOrEmpty.OrderBy(value => value.Id.Value, StringComparer.Ordinal)) yield return $"adventure:{definition.Id}:{definition.Title}:{definition.IsDefault}:{definition.Selectable}:{Ids(definition.Party)}:{Ids(definition.Characters)}:{definition.CampStorage}:{Ids(definition.Storage)}:{Ids(definition.Items)}:{string.Join(',', definition.Encounters)}:{Dungeon(definition.Dungeon)}:{AdventureOutcome(definition.Completion)}:{Source(definition.Source)}";

        static string Ids(IEnumerable<D20Id> ids) => string.Join(',', ids.OrderBy(value => value.Value, StringComparer.Ordinal));
        static string Source(SourceProvenance source) => $"{source.SourcePath}:{source.Subject}:{source.Adaptation}:{source.DonorPath}";
        static string Damage(DamageDefinition damage) => $"{damage.Kind}:{damage.Dice}:{damage.Sides}:{damage.Bonus}";
        static string Condition(ConditionClause clause) => $"{clause.Kind}:{clause.Tag}:{clause.Amount}";
        static string Costs(IEnumerable<ActivationCost> costs) => string.Join(',', costs.OrderBy(value => value.Budget.Value, StringComparer.Ordinal).ThenBy(value => value.Amount).Select(value => $"{value.Budget}:{value.Amount}"));
        static string Target(ActionTarget target) => $"{target.Kind}:{target.Team}:{target.MaximumTargets}:{target.RequiresLineOfEffect}";
        static string Attack(ActionAttack attack) => $"{attack.Ability}:{attack.Defense}:{(attack.Damage is null ? string.Empty : Damage(attack.Damage))}:{attack.Implement}";
        static string Board(TacticalBoard board) => $"{board.Width}:{board.Height}:{string.Join('/', board.Rows)}:{string.Join(',', board.Placements.OrderBy(value => value.Character.Value, StringComparer.Ordinal).ThenBy(value => value.Position.X).ThenBy(value => value.Position.Y).Select(value => $"{value.Character}@{value.Position.X},{value.Position.Y}"))}";
        static string Outcome(EncounterOutcome outcome) => $"{outcome.Title}:{outcome.Summary}:{outcome.RewardItem}:{outcome.RecoveryVitality}";
        static string Dungeon(DungeonDefinition dungeon) => $"{dungeon.Title}:{dungeon.WallStyle}:{dungeon.Width}:{dungeon.Height}:{string.Join('/', dungeon.Rows)}:{dungeon.Start.X},{dungeon.Start.Y}:{dungeon.StartCheckpoint}:{dungeon.StartFacing}:{string.Join(',', dungeon.Encounters.Select(value => $"{value.Encounter}@{value.Position.X},{value.Position.Y}"))}:{string.Join(',', dungeon.Landmarks.OrderBy(value => value.Id.Value, StringComparer.Ordinal).Select(value => $"{value.Id}@{value.Position.X},{value.Position.Y}:{value.Title}:{value.Text}"))}:{string.Join(',', dungeon.Doors.OrderBy(value => value.Id.Value, StringComparer.Ordinal).Select(value => $"{value.Id}@{value.Position.X},{value.Position.Y}:{value.Facing}:{value.Title}:{value.Text}:{value.RequiresTreasure}"))}:{string.Join(',', dungeon.Treasures.OrderBy(value => value.Id.Value, StringComparer.Ordinal).Select(value => $"{value.Id}@{value.Position.X},{value.Position.Y}:{value.Item}:{value.Title}:{value.Text}"))}:{string.Join(',', dungeon.Checkpoints.OrderBy(value => value.Id.Value, StringComparer.Ordinal).Select(value => $"{value.Id}@{value.Position.X},{value.Position.Y}:{value.Title}:{value.Text}"))}";
        static string AdventureOutcome(AdventureOutcome outcome) => $"{outcome.Source}:{outcome.VictoryTitle}:{outcome.VictoryText}:{outcome.DefeatTitle}:{outcome.DefeatText}:{string.Join('|', outcome.Details.Select((value, index) => $"{index}:{value}"))}";
    }

    private static bool Floor(IReadOnlyList<string> rows, GridPosition position) => position.Y >= 0 && position.Y < rows.Count && position.X >= 0 && position.X < rows[position.Y].Length && rows[position.Y][position.X] == '.';

    private static HashSet<GridPosition> ReachableFloorCells(IReadOnlyList<string> rows, GridPosition start)
    {
        var reachable = new HashSet<GridPosition>();
        if (!Floor(rows, start)) return reachable;

        var pending = new Queue<GridPosition>();
        pending.Enqueue(start);
        reachable.Add(start);
        while (pending.TryDequeue(out var current))
        {
            foreach (var next in new[] { new GridPosition(current.X + 1, current.Y), new GridPosition(current.X - 1, current.Y), new GridPosition(current.X, current.Y + 1), new GridPosition(current.X, current.Y - 1) })
            {
                if (Floor(rows, next) && reachable.Add(next)) pending.Enqueue(next);
            }
        }

        return reachable;
    }

    private enum VisitState { Visiting, Complete }
}
