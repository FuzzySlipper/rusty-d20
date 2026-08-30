using System.Collections.Frozen;
using System.Collections.ObjectModel;
using System.Security.Cryptography;
using System.Text;
using RustyD20.Core.Contract;

namespace RustyD20.Core.Rules;

/// <summary>Strictly admits the closed authored D20 catalog. C# modules are its only input.</summary>
public sealed class D20SemanticCompiler
{
    public CompiledD20Content Compile(IEnumerable<D20ContentModule> input)
    {
        // Admission owns a complete snapshot. No caller-owned collection remains reachable after Compile returns.
        var modules = input.Select(Snapshot).OrderBy(module => module.Id.Value, StringComparer.Ordinal).ToArray();
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

            Distinct(module.Dependencies, module.Source, "D20_DUPLICATE_MODULE_DEPENDENCY", "module dependency");

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
        var sources = ReadOnly(modules.Select(module => module.Source).OrderBy(source => source.SourcePath, StringComparer.Ordinal));
        var definitionCount = abilities.Count + defenses.Count + budgets.Count + damageTypes.Count + resources.Count + armors.Count + implements.Count + effects.Count + reactions.Count + actions.Count + features.Count + characters.Count + storage.Count + items.Count + encounters.Count;
        var catalog = new D20DefinitionCatalog(
            Frozen(abilities), Frozen(defenses), Frozen(budgets), Frozen(damageTypes), Frozen(resources), Frozen(armors),
            Frozen(implements), Frozen(effects), Frozen(reactions), Frozen(actions), Frozen(features), Frozen(characters),
            Frozen(storage), Frozen(items), Frozen(encounters), Frozen(adventures));
        return new CompiledD20Content(fingerprint, catalog.Adventures, catalog.Characters, catalog.Items, catalog, ReadOnly(modules), new CompilationReceipt(fingerprint, sources, definitionCount, adventures.Count));

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
                Distinct(defense.Abilities, defense.Source, "D20_DUPLICATE_DEFENSE_ABILITY", "defense ability");
            }

            foreach (var armor in armors.Values)
            {
                Require(defenses, armor.Defense, armor.Source, "D20_UNKNOWN_DEFENSE", "armor defense");
                if (armor.Bonus < 0) Add("D20_ARMOR_BONUS_LIMIT", "armor bonus is negative", armor.Source);
            }

            foreach (var ability in abilities.Values)
            {
                if (ability.Minimum < 0 || ability.Maximum < ability.Minimum) Add("D20_ABILITY_BOUND_LIMIT", "ability bounds are negative or inverted", ability.Source);
            }

            foreach (var budget in budgets.Values)
            {
                if (budget.Initial < 0) Add("D20_BUDGET_LIMIT", "activation budget is negative", budget.Source);
            }

            foreach (var resource in resources.Values)
            {
                if (resource.Maximum < 0) Add("D20_RESOURCE_LIMIT", "resource maximum is negative", resource.Source);
            }

            foreach (var implement in implements.Values)
            {
                Require(abilities, implement.Ability, implement.Source, "D20_UNKNOWN_ABILITY", "implement ability");
                Require(defenses, implement.Defense, implement.Source, "D20_UNKNOWN_DEFENSE", "implement defense");
                Require(damageTypes, implement.Damage.Kind, implement.Source, "D20_UNKNOWN_DAMAGE_TYPE", "implement damage");
                Bound(implement.Tags.Count, D20Limits.ImplementTags, implement.Source, "D20_IMPLEMENT_TAG_QUOTA");
                Bound(implement.Range, D20Limits.TacticalRange, implement.Source, "D20_TACTICAL_RANGE");
                Distinct(implement.Tags, implement.Source, "D20_DUPLICATE_IMPLEMENT_TAG", "implement tag");
                ValidateDamage(implement.Damage, implement.Source);
            }

            foreach (var effect in effects.Values)
            {
                if (effect.Defense is { } defense) Require(defenses, defense, effect.Source, "D20_UNKNOWN_DEFENSE", "effect defense");
                Bound(effect.DurationTurns, D20Limits.EffectDurationTurns, effect.Source, "D20_EFFECT_DURATION");
                Bound(effect.Conditions.Count, D20Limits.ConditionClauses, effect.Source, "D20_CONDITION_QUOTA");
                Distinct(effect.Conditions.Select(condition => $"{condition.Kind}:{condition.Tag}:{condition.Amount}"), effect.Source, "D20_DUPLICATE_CONDITION", "condition clause");
            }

            foreach (var reaction in reactions.Values)
            {
                Require(defenses, reaction.Defense, reaction.Source, "D20_UNKNOWN_DEFENSE", "reaction defense");
                Require(resources, reaction.Resource, reaction.Source, "D20_UNKNOWN_RESOURCE", "reaction resource");
                Require(effects, reaction.Effect, reaction.Source, "D20_UNKNOWN_EFFECT", "reaction effect");
                if (reaction.Cost < 0 || (resources.TryGetValue(reaction.Resource, out var resource) && reaction.Cost > resource.Maximum)) Add("D20_REACTION_COST_LIMIT", "reaction resource cost is outside the admitted bound", reaction.Source);
                ValidateCosts(reaction.Costs, ActivationTiming.Reaction, reaction.Source);
            }

            foreach (var action in actions.Values)
            {
                Bound(action.Tags.Count, D20Limits.ActionTags, action.Source, "D20_ACTION_TAG_QUOTA");
                Bound(action.Costs.Count, D20Limits.ActivationCosts, action.Source, "D20_ACTIVATION_COST_QUOTA");
                Bound(action.Target.MaximumTargets, D20Limits.ActionTargets, action.Source, "D20_ACTION_TARGET_QUOTA");
                Bound(action.ForcedMovement, D20Limits.ForcedMovement, action.Source, "D20_FORCED_MOVEMENT");
                Distinct(action.Tags, action.Source, "D20_DUPLICATE_ACTION_TAG", "action tag");
                ValidateCosts(action.Costs, ActivationTiming.Action, action.Source);
                ValidateAttack(action.Attack, action.Source);

                if (action.Effect is { } effect) Require(effects, effect, action.Source, "D20_UNKNOWN_EFFECT", "action effect");
            }

            foreach (var character in characters.Values)
            {
                if (character.Experience is < 0 or > D20Limits.Experience) Add("D20_EXPERIENCE_LIMIT", "character experience is outside the admitted bound", character.Source);
                if (character.Level < 0 || character.Vitality < 0) Add("D20_CHARACTER_SCALAR_LIMIT", "character level or vitality is negative", character.Source);
                RequireAll(abilities, character.Abilities.Keys, character.Source, "D20_UNKNOWN_ABILITY", "character ability");
                RequireAll(actions, character.Actions, character.Source, "D20_UNKNOWN_ACTION", "character action");
                RequireAll(reactions, character.Reactions, character.Source, "D20_UNKNOWN_REACTION", "character reaction");
                RequireAll(features, character.Features, character.Source, "D20_UNKNOWN_FEATURE", "character feature");
                RequireAll(resources, character.ResourcesOrEmpty.Keys, character.Source, "D20_UNKNOWN_RESOURCE", "character resource");
                foreach (var resource in character.ResourcesOrEmpty)
                {
                    if (resource.Value < 0 || (resources.TryGetValue(resource.Key, out var definition) && resource.Value > definition.Maximum)) Add("D20_CHARACTER_RESOURCE_LIMIT", "character resource value is outside the admitted bound", character.Source);
                }
                foreach (var affinity in character.AffinitiesOrEmpty) Require(damageTypes, affinity.DamageType, character.Source, "D20_UNKNOWN_DAMAGE_TYPE", "character affinity");
                Distinct(character.Actions, character.Source, "D20_DUPLICATE_CHARACTER_ACTION", "character action");
                Distinct(character.Reactions, character.Source, "D20_DUPLICATE_CHARACTER_REACTION", "character reaction");
                Distinct(character.Features, character.Source, "D20_DUPLICATE_CHARACTER_FEATURE", "character feature");
                Distinct(character.AffinitiesOrEmpty.Select(affinity => affinity.DamageType), character.Source, "D20_DUPLICATE_CHARACTER_AFFINITY", "character affinity");
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
                Text(item.Name, item.Source);
            }

            foreach (var storageDefinition in storage.Values)
            {
                if (storageDefinition.Capacity < 0) Add("D20_STORAGE_CAPACITY_LIMIT", "storage capacity is negative", storageDefinition.Source);
                Text(storageDefinition.Name, storageDefinition.Source);
            }

            foreach (var feature in features.Values)
            {
                Text(feature.Label, feature.Source);
                Text(feature.Description, feature.Source);
            }

            foreach (var encounter in encounters.Values)
            {
                if (encounter.Roster.Count is 0 or > D20Limits.EncounterParticipants) Add("D20_ENCOUNTER_PARTICIPANT_QUOTA", "encounter roster is outside the admitted bound", encounter.Source);
                RequireAll(characters, encounter.Roster.Select(participant => participant.Character), encounter.Source, "D20_UNKNOWN_CHARACTER", "encounter character");
                Distinct(encounter.Roster.Select(participant => participant.Character), encounter.Source, "D20_DUPLICATE_ENCOUNTER_ROSTER", "encounter roster character");
                Text(encounter.Title, encounter.Source);
                Text(encounter.Summary, encounter.Source);
                Text(encounter.Victory.Title, encounter.Source);
                Text(encounter.Victory.Summary, encounter.Source);
                Text(encounter.Defeat.Title, encounter.Source);
                Text(encounter.Defeat.Summary, encounter.Source);
                if (encounter.Victory.RecoveryVitality is < 0 || encounter.Defeat.RecoveryVitality is < 0) Add("D20_ENCOUNTER_RECOVERY_LIMIT", "encounter recovery vitality is negative", encounter.Source);
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
                Distinct(adventure.Party, adventure.Source, "D20_DUPLICATE_PARTY_MEMBER", "party member");
                Distinct(adventure.Characters, adventure.Source, "D20_DUPLICATE_ADVENTURE_CHARACTER", "adventure character");
                Distinct(adventure.Storage, adventure.Source, "D20_DUPLICATE_ADVENTURE_STORAGE", "adventure storage");
                Distinct(adventure.Items, adventure.Source, "D20_DUPLICATE_ADVENTURE_ITEM", "adventure item");
                Distinct(adventure.Encounters, adventure.Source, "D20_DUPLICATE_ADVENTURE_ENCOUNTER", "adventure encounter");
                RequireAll(characters, adventure.Party, adventure.Source, "D20_UNKNOWN_CHARACTER", "party character");
                RequireAll(characters, adventure.Characters, adventure.Source, "D20_UNKNOWN_CHARACTER", "adventure character");
                Require(storage, adventure.CampStorage, adventure.Source, "D20_UNKNOWN_STORAGE", "camp storage");
                RequireAll(storage, adventure.Storage, adventure.Source, "D20_UNKNOWN_STORAGE", "adventure storage");
                RequireAll(items, adventure.Items, adventure.Source, "D20_UNKNOWN_ITEM", "adventure item");
                RequireAll(encounters, adventure.Encounters, adventure.Source, "D20_UNKNOWN_ENCOUNTER", "adventure encounter");
                Text(adventure.Title, adventure.Source);
                Text(adventure.Completion.Source, adventure.Source);
                Text(adventure.Completion.VictoryTitle, adventure.Source);
                Text(adventure.Completion.VictoryText, adventure.Source);
                Text(adventure.Completion.DefeatTitle, adventure.Source);
                Text(adventure.Completion.DefeatText, adventure.Source);
                foreach (var detail in adventure.Completion.Details) Text(detail, adventure.Source);
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
            Distinct(dungeon.Encounters.Select(value => value.Encounter), source, "D20_DUPLICATE_DUNGEON_ENCOUNTER", "dungeon encounter");
            Distinct(dungeon.Landmarks.Select(value => value.Id), source, "D20_DUPLICATE_DUNGEON_LANDMARK", "dungeon landmark");
            Distinct(dungeon.Doors.Select(value => value.Id), source, "D20_DUPLICATE_DUNGEON_DOOR", "dungeon door");
            Distinct(dungeon.Treasures.Select(value => value.Id), source, "D20_DUPLICATE_DUNGEON_TREASURE", "dungeon treasure");
            Distinct(dungeon.Checkpoints.Select(value => value.Id), source, "D20_DUPLICATE_DUNGEON_CHECKPOINT", "dungeon checkpoint");
            Text(dungeon.Title, source);

            foreach (var encounter in dungeon.Encounters)
            {
                Require(encounters, encounter.Encounter, source, "D20_UNKNOWN_ENCOUNTER", "dungeon encounter");
                Position(encounter.Position, "encounter");
            }

            foreach (var landmark in dungeon.Landmarks)
            {
                Position(landmark.Position, "landmark");
                Text(landmark.Title, source);
                Text(landmark.Text, source);
            }
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
                Text(door.Title, source);
                Text(door.Text, source);
            }

            foreach (var treasure in dungeon.Treasures)
            {
                Text(treasure.Title, source);
                Text(treasure.Text, source);
            }

            foreach (var checkpoint in dungeon.Checkpoints)
            {
                Text(checkpoint.Title, source);
                Text(checkpoint.Text, source);
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
            var placedCharacters = new HashSet<D20Id>();
            foreach (var placement in board.Placements)
            {
                if (!Floor(board.Rows, placement.Position)) Add($"D20_INVALID_{kind.ToUpperInvariant()}_PLACEMENT", $"{kind} placement must be a floor cell", source);
                if (!occupied.Add(placement.Position)) Add($"D20_DUPLICATE_{kind.ToUpperInvariant()}_PLACEMENT", $"{kind} placements cannot overlap", source);
                if (!placedCharacters.Add(placement.Character)) Add($"D20_DUPLICATE_{kind.ToUpperInvariant()}_PLACEMENT_CHARACTER", $"{kind} character has multiple placements", source);
            }
        }

        void ValidateCosts(IReadOnlyList<ActivationCost> costs, ActivationTiming expected, SourceProvenance source)
        {
            foreach (var cost in costs)
            {
                if (!budgets.TryGetValue(cost.Budget, out var budget)) Add("D20_UNKNOWN_ACTIVATION_BUDGET", $"unknown activation budget '{cost.Budget}'", source);
                else if (cost.Amount <= 0 || budget.Timing != expected || cost.Amount > budget.Initial) Add("D20_INCOMPATIBLE_ACTIVATION_COST", "activation cost is nonpositive, has wrong timing, or exceeds the authored initial budget", source);
            }
            Distinct(costs.Select(cost => cost.Budget), source, "D20_DUPLICATE_ACTIVATION_COST", "activation cost budget");
        }

        void ValidateDamage(DamageDefinition damage, SourceProvenance source)
        {
            if (damage.Dice <= 0 || damage.Dice > D20Limits.DamageDice || damage.Sides <= 0 || damage.Sides > D20Limits.DamageDieSides) Add("D20_DAMAGE_LIMIT", "damage dice or sides are outside the admitted bound", source);
        }

        void ValidateAttack(ActionAttack attack, SourceProvenance source)
        {
            var hasFixedField = attack.Ability is not null || attack.Defense is not null || attack.Damage is not null;
            if (attack.Implement is { } implement)
            {
                if (hasFixedField) Add("D20_INVALID_ATTACK_SHAPE", "an action attack cannot combine fixed and implement forms", source);
                if (attack.Range != 0) Add("D20_TACTICAL_RANGE", "implement-bound action range must come from the admitted implement", source);
                Require(implements, implement, source, "D20_UNKNOWN_IMPLEMENT", "action implement");
                return;
            }

            if (attack.Ability is not { } ability || attack.Defense is not { } defense || attack.Damage is not { } damage)
            {
                Add("D20_INVALID_ATTACK_SHAPE", "an action attack must be a complete fixed attack or an implement-only attack", source);
                return;
            }

            Require(abilities, ability, source, "D20_UNKNOWN_ABILITY", "action ability");
            Require(defenses, defense, source, "D20_UNKNOWN_DEFENSE", "action defense");
            Require(damageTypes, damage.Kind, source, "D20_UNKNOWN_DAMAGE_TYPE", "action damage");
            ValidateDamage(damage, source);
            if (attack.Range < 1 || attack.Range > D20Limits.TacticalRange) Add("D20_TACTICAL_RANGE", "fixed action range is outside the admitted tactical bound", source);
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
            if (value < 0 || value > maximum) Add(code, $"value is outside 0 through {maximum}", source);
        }

        void Text(string value, SourceProvenance source)
        {
            if (Encoding.UTF8.GetByteCount(value) > D20Limits.AuthoredTextBytes) Add("D20_AUTHORED_TEXT_LIMIT", $"authored text exceeds {D20Limits.AuthoredTextBytes} UTF-8 bytes", source);
        }

        void Distinct<T>(IEnumerable<T> values, SourceProvenance source, string code, string kind) where T : notnull
        {
            var seen = new HashSet<T>();
            foreach (var value in values)
            {
                if (!seen.Add(value)) Add(code, $"duplicate {kind} '{value}'", source);
            }
        }

        void Add(string code, string message, SourceProvenance source, string? detail = null) => diagnostics.Add(new D20Diagnostic(code, message, source, source.Subject, detail));
    }

    private static FrozenDictionary<D20Id, T> Frozen<T>(IEnumerable<KeyValuePair<D20Id, T>> definitions) where T : notnull =>
        definitions.ToFrozenDictionary(pair => pair.Key, pair => pair.Value);

    private static IReadOnlyList<T> ReadOnly<T>(IEnumerable<T> values) => new ReadOnlyCollection<T>(values.ToArray());

    private static IReadOnlyDictionary<D20Id, int> ReadOnlyMap(IEnumerable<KeyValuePair<D20Id, int>> values) =>
        values.ToFrozenDictionary(pair => pair.Key, pair => pair.Value);

    private static D20ContentModule Snapshot(D20ContentModule module) => new(
        module.Id,
        module.ContentSchema,
        ReadOnly(module.Dependencies),
        module.Source,
        ReadOnly(module.AbilitiesOrEmpty.Select(value => value with { })),
        ReadOnly(module.DefensesOrEmpty.Select(value => value with { Abilities = ReadOnly(value.Abilities) })),
        ReadOnly(module.BudgetsOrEmpty.Select(value => value with { })),
        ReadOnly(module.DamageTypesOrEmpty.Select(value => value with { })),
        ReadOnly(module.ResourcesOrEmpty.Select(value => value with { })),
        ReadOnly(module.ArmorsOrEmpty.Select(value => value with { })),
        ReadOnly(module.ImplementsOrEmpty.Select(value => value with { Tags = ReadOnly(value.Tags), Damage = value.Damage with { } })),
        ReadOnly(module.EffectsOrEmpty.Select(value => value with { Conditions = ReadOnly(value.Conditions.Select(condition => condition with { })) })),
        ReadOnly(module.ReactionsOrEmpty.Select(value => value with { Costs = ReadOnly(value.Costs.Select(cost => cost with { })) })),
        ReadOnly(module.ActionsOrEmpty.Select(value => value with
        {
            Tags = ReadOnly(value.Tags),
            Costs = ReadOnly(value.Costs.Select(cost => cost with { })),
            Target = value.Target with { },
            Attack = value.Attack with { Damage = value.Attack.Damage is null ? null : value.Attack.Damage with { } },
        })),
        ReadOnly(module.FeaturesOrEmpty.Select(value => value with { })),
        ReadOnly(module.CharactersOrEmpty.Select(value => value with
        {
            Abilities = ReadOnlyMap(value.Abilities),
            Actions = ReadOnly(value.Actions),
            Reactions = ReadOnly(value.Reactions),
            Features = ReadOnly(value.Features),
            Resources = ReadOnlyMap(value.ResourcesOrEmpty),
            Affinities = ReadOnly(value.AffinitiesOrEmpty.Select(affinity => affinity with { })),
        })),
        ReadOnly(module.StorageOrEmpty.Select(value => value with { })),
        ReadOnly(module.ItemsOrEmpty.Select(value => value with { })),
        ReadOnly(module.EncountersOrEmpty.Select(value => value with
        {
            Roster = ReadOnly(value.Roster.Select(participant => participant with { })),
            Board = Snapshot(value.Board),
            Victory = value.Victory with { },
            Defeat = value.Defeat with { },
        })),
        ReadOnly(module.AdventuresOrEmpty.Select(value => value with
        {
            Party = ReadOnly(value.Party),
            Characters = ReadOnly(value.Characters),
            Storage = ReadOnly(value.Storage),
            Items = ReadOnly(value.Items),
            Encounters = ReadOnly(value.Encounters),
            Dungeon = Snapshot(value.Dungeon),
            Completion = value.Completion with { Details = ReadOnly(value.Completion.Details) },
        })));

    private static TacticalBoard Snapshot(TacticalBoard board) => new(
        board.Width,
        board.Height,
        ReadOnly(board.Rows),
        ReadOnly(board.Placements.Select(placement => placement with { Position = placement.Position with { } })));

    private static DungeonDefinition Snapshot(DungeonDefinition dungeon) => new(
        dungeon.Title,
        dungeon.WallStyle,
        dungeon.Width,
        dungeon.Height,
        ReadOnly(dungeon.Rows),
        dungeon.Start with { },
        dungeon.StartCheckpoint,
        dungeon.StartFacing,
        ReadOnly(dungeon.Encounters.Select(encounter => encounter with { Position = encounter.Position with { } })),
        ReadOnly(dungeon.Landmarks.Select(landmark => landmark with { Position = landmark.Position with { } })),
        ReadOnly(dungeon.Doors.Select(door => door with { Position = door.Position with { } })),
        ReadOnly(dungeon.Treasures.Select(treasure => treasure with { Position = treasure.Position with { } })),
        ReadOnly(dungeon.Checkpoints.Select(checkpoint => checkpoint with { Position = checkpoint.Position with { } })));

    public static string Fingerprint(IEnumerable<D20ContentModule> modules)
    {
        var lines = modules.OrderBy(module => module.Id.Value, StringComparer.Ordinal).SelectMany(CanonicalLines);
        return Convert.ToHexString(SHA256.HashData(Encoding.UTF8.GetBytes(string.Join('\n', lines)))).ToLowerInvariant();
    }

    private static IEnumerable<string> CanonicalLines(D20ContentModule module)
    {
        yield return Line("module", module.Id.Value, module.ContentSchema, SetIds(module.Dependencies), Source(module.Source));
        foreach (var definition in module.AbilitiesOrEmpty.OrderBy(value => value.Id.Value, StringComparer.Ordinal)) yield return Line("ability", definition.Id.Value, definition.Minimum.ToString(), definition.Maximum.ToString(), Source(definition.Source));
        foreach (var definition in module.DefensesOrEmpty.OrderBy(value => value.Id.Value, StringComparer.Ordinal)) yield return Line("defense", definition.Id.Value, definition.Base.ToString(), SetIds(definition.Abilities), Source(definition.Source));
        foreach (var definition in module.BudgetsOrEmpty.OrderBy(value => value.Id.Value, StringComparer.Ordinal)) yield return Line("budget", definition.Id.Value, definition.Timing.ToString(), definition.Initial.ToString(), Source(definition.Source));
        foreach (var definition in module.DamageTypesOrEmpty.OrderBy(value => value.Id.Value, StringComparer.Ordinal)) yield return Line("damage-type", definition.Id.Value, Source(definition.Source));
        foreach (var definition in module.ResourcesOrEmpty.OrderBy(value => value.Id.Value, StringComparer.Ordinal)) yield return Line("resource", definition.Id.Value, definition.Maximum.ToString(), Source(definition.Source));
        foreach (var definition in module.ArmorsOrEmpty.OrderBy(value => value.Id.Value, StringComparer.Ordinal)) yield return Line("armor", definition.Id.Value, definition.Defense.Value, definition.Bonus.ToString(), definition.Slot.Value, Source(definition.Source));
        foreach (var definition in module.ImplementsOrEmpty.OrderBy(value => value.Id.Value, StringComparer.Ordinal)) yield return Line("implement", definition.Id.Value, definition.Slot.Value, SetIds(definition.Tags), definition.Ability.Value, definition.Defense.Value, Damage(definition.Damage), definition.Range.ToString(), Source(definition.Source));
        foreach (var definition in module.EffectsOrEmpty.OrderBy(value => value.Id.Value, StringComparer.Ordinal)) yield return Line("effect", definition.Id.Value, Id(definition.Defense), definition.DefenseBonus.ToString(), definition.DurationTurns.ToString(), List(definition.Conditions.Select(Condition)), Source(definition.Source));
        foreach (var definition in module.ReactionsOrEmpty.OrderBy(value => value.Id.Value, StringComparer.Ordinal)) yield return Line("reaction", definition.Id.Value, definition.Defense.Value, definition.Bonus.ToString(), definition.Resource.Value, definition.Cost.ToString(), List(definition.Costs.Select(Cost)), definition.Effect.Value, Source(definition.Source));
        foreach (var definition in module.ActionsOrEmpty.OrderBy(value => value.Id.Value, StringComparer.Ordinal)) yield return Line("action", definition.Id.Value, SetIds(definition.Tags), List(definition.Costs.Select(Cost)), Target(definition.Target), Attack(definition.Attack), Id(definition.Effect), definition.ForcedMovement.ToString(), Source(definition.Source));
        foreach (var definition in module.FeaturesOrEmpty.OrderBy(value => value.Id.Value, StringComparer.Ordinal)) yield return Line("feature", definition.Id.Value, definition.Label, definition.Description, Source(definition.Source));
        foreach (var definition in module.CharactersOrEmpty.OrderBy(value => value.Id.Value, StringComparer.Ordinal)) yield return Line("character", definition.Id.Value, definition.Name, definition.Title, definition.Level.ToString(), definition.Experience.ToString(), definition.Vitality.ToString(), Map(definition.Abilities), List(definition.Actions.Select(id => id.Value)), List(definition.Reactions.Select(id => id.Value)), List(definition.Features.Select(id => id.Value)), Map(definition.ResourcesOrEmpty), List(definition.AffinitiesOrEmpty.Select(Affinity)), Source(definition.Source));
        foreach (var definition in module.StorageOrEmpty.OrderBy(value => value.Id.Value, StringComparer.Ordinal)) yield return Line("storage", definition.Id.Value, definition.Name, definition.Capacity.ToString(), Source(definition.Source));
        foreach (var definition in module.ItemsOrEmpty.OrderBy(value => value.Id.Value, StringComparer.Ordinal)) yield return Line("item", definition.Id.Value, definition.Name, definition.EquipmentKind.ToString(), definition.Equipment.Value, definition.Owner.Value, definition.Equipped.ToString(), Source(definition.Source));
        foreach (var definition in module.EncountersOrEmpty.OrderBy(value => value.Id.Value, StringComparer.Ordinal)) yield return Line("encounter", definition.Id.Value, definition.Title, definition.Summary, List(definition.Roster.Select(Roster)), Board(definition.Board), Outcome(definition.Victory), Outcome(definition.Defeat), Source(definition.Source));
        foreach (var definition in module.AdventuresOrEmpty.OrderBy(value => value.Id.Value, StringComparer.Ordinal)) yield return Line("adventure", definition.Id.Value, definition.Title, definition.IsDefault.ToString(), definition.Selectable.ToString(), List(definition.Party.Select(id => id.Value)), List(definition.Characters.Select(id => id.Value)), definition.CampStorage.Value, List(definition.Storage.Select(id => id.Value)), List(definition.Items.Select(id => id.Value)), List(definition.Encounters.Select(id => id.Value)), Dungeon(definition.Dungeon), AdventureOutcome(definition.Completion), Source(definition.Source));

        static string Line(string kind, params string?[] fields) => Frame([kind, .. fields]);
        static string Frame(IEnumerable<string?> fields) => string.Concat(fields.Select(field => field is null ? "-1:" : $"{Encoding.UTF8.GetByteCount(field)}:{field}"));
        static string List(IEnumerable<string?> values) => Frame(values);
        static string SetIds(IEnumerable<D20Id> ids) => List(ids.OrderBy(value => value.Value, StringComparer.Ordinal).Select(value => value.Value));
        static string Id(D20Id? id) => id?.Value ?? null!;
        static string Map(IEnumerable<KeyValuePair<D20Id, int>> values) => List(values.OrderBy(value => value.Key.Value, StringComparer.Ordinal).Select(value => Line(value.Key.Value, value.Value.ToString())));
        static string Source(SourceProvenance source) => Line(source.SourcePath, source.Subject, source.Adaptation, source.DonorPath);
        static string Damage(DamageDefinition damage) => Line(damage.Kind.Value, damage.Dice.ToString(), damage.Sides.ToString(), damage.Bonus.ToString());
        static string Condition(ConditionClause clause) => Line(clause.Kind.ToString(), Id(clause.Tag), clause.Amount.ToString());
        static string Cost(ActivationCost cost) => Line(cost.Budget.Value, cost.Amount.ToString());
        static string Target(ActionTarget target) => Line(target.Kind.ToString(), target.Team.ToString(), target.MaximumTargets.ToString(), target.RequiresLineOfEffect.ToString());
        static string Attack(ActionAttack attack) => Line(Id(attack.Ability), Id(attack.Defense), attack.Damage is null ? null : Damage(attack.Damage), Id(attack.Implement), attack.Range.ToString());
        static string Affinity(DamageAffinity affinity) => Line(affinity.DamageType.Value, affinity.Affinity.ToString());
        static string Roster(EncounterParticipant participant) => Line(participant.Character.Value, participant.Faction.ToString());
        static string Board(TacticalBoard board) => Line(board.Width.ToString(), board.Height.ToString(), List(board.Rows), List(board.Placements.Select(value => Line(value.Character.Value, value.Position.X.ToString(), value.Position.Y.ToString()))));
        static string Outcome(EncounterOutcome outcome) => Line(outcome.Title, outcome.Summary, Id(outcome.RewardItem), outcome.RecoveryVitality?.ToString());
        static string Dungeon(DungeonDefinition dungeon) => Line(dungeon.Title, dungeon.WallStyle.Value, dungeon.Width.ToString(), dungeon.Height.ToString(), List(dungeon.Rows), Line(dungeon.Start.X.ToString(), dungeon.Start.Y.ToString()), dungeon.StartCheckpoint.Value, dungeon.StartFacing.ToString(), List(dungeon.Encounters.Select(value => Line(value.Encounter.Value, value.Position.X.ToString(), value.Position.Y.ToString()))), List(dungeon.Landmarks.Select(value => Line(value.Id.Value, value.Position.X.ToString(), value.Position.Y.ToString(), value.Title, value.Text))), List(dungeon.Doors.Select(value => Line(value.Id.Value, value.Position.X.ToString(), value.Position.Y.ToString(), value.Facing.ToString(), value.Title, value.Text, Id(value.RequiresTreasure)))), List(dungeon.Treasures.Select(value => Line(value.Id.Value, value.Position.X.ToString(), value.Position.Y.ToString(), value.Item.Value, value.Title, value.Text))), List(dungeon.Checkpoints.Select(value => Line(value.Id.Value, value.Position.X.ToString(), value.Position.Y.ToString(), value.Title, value.Text))));
        static string AdventureOutcome(AdventureOutcome outcome) => Line(outcome.Source, outcome.VictoryTitle, outcome.VictoryText, outcome.DefeatTitle, outcome.DefeatText, List(outcome.Details));
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
