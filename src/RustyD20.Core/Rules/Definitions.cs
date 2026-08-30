using RustyD20.Core.Contract;

namespace RustyD20.Core.Rules;

public sealed record AbilityDefinition(D20Id Id, int Minimum, int Maximum, SourceProvenance Source);
public sealed record DefenseDefinition(D20Id Id, int Base, IReadOnlyList<D20Id> Abilities, SourceProvenance Source);
public sealed record ActivationBudgetDefinition(D20Id Id, ActivationTiming Timing, int Initial, SourceProvenance Source);
public sealed record DamageTypeDefinition(D20Id Id, SourceProvenance Source);
public sealed record ResourceDefinition(D20Id Id, int Maximum, SourceProvenance Source);
public sealed record ArmorDefinition(D20Id Id, D20Id Defense, int Bonus, D20Id Slot, SourceProvenance Source);
public sealed record ImplementDefinition(D20Id Id, D20Id Slot, IReadOnlyList<D20Id> Tags, D20Id Ability, D20Id Defense, DamageDefinition Damage, int Range, SourceProvenance Source);
public sealed record DamageDefinition(D20Id Kind, int Dice, int Sides, int Bonus);
public sealed record EffectDefinition(D20Id Id, D20Id? Defense, int DefenseBonus, int DurationTurns, IReadOnlyList<ConditionClause> Conditions, SourceProvenance Source);
public sealed record ReactionDefinition(D20Id Id, D20Id Defense, int Bonus, D20Id Resource, int Cost, IReadOnlyList<ActivationCost> Costs, D20Id Effect, SourceProvenance Source);
public sealed record ActionDefinition(D20Id Id, IReadOnlyList<D20Id> Tags, IReadOnlyList<ActivationCost> Costs, ActionTarget Target, ActionAttack Attack, D20Id? Effect, int ForcedMovement, SourceProvenance Source);
public sealed record FeatureDefinition(D20Id Id, string Label, string Description, SourceProvenance Source);
public sealed record CharacterDefinition(D20Id Id, string Name, string Title, int Level, int Experience, int Vitality, IReadOnlyDictionary<D20Id, int> Abilities, IReadOnlyList<D20Id> Actions, IReadOnlyList<D20Id> Reactions, IReadOnlyList<D20Id> Features, SourceProvenance Source, IReadOnlyDictionary<D20Id, int>? Resources = null, IReadOnlyList<DamageAffinity>? Affinities = null)
{
    public IReadOnlyDictionary<D20Id, int> ResourcesOrEmpty => Resources ?? new Dictionary<D20Id, int>();
    public IReadOnlyList<DamageAffinity> AffinitiesOrEmpty => Affinities ?? [];
}
public sealed record StorageDefinition(D20Id Id, string Name, int Capacity, SourceProvenance Source);
public sealed record ItemDefinition(D20Id Id, string Name, EquipmentKind EquipmentKind, D20Id Equipment, D20Id Owner, bool Equipped, SourceProvenance Source);
public sealed record EncounterDefinition(D20Id Id, string Title, IReadOnlyList<EncounterParticipant> Roster, TacticalBoard Board, EncounterOutcome Victory, EncounterOutcome Defeat, SourceProvenance Source, string Summary = "");
public sealed record AdventureDefinition(D20Id Id, string Title, bool IsDefault, bool Selectable, IReadOnlyList<D20Id> Party, IReadOnlyList<D20Id> Characters, D20Id CampStorage, IReadOnlyList<D20Id> Storage, IReadOnlyList<D20Id> Items, IReadOnlyList<D20Id> Encounters, DungeonDefinition Dungeon, AdventureOutcome Completion, SourceProvenance Source);

public enum ActivationTiming { Action, Reaction, Movement }
public enum EquipmentKind { Armor, Implement }
public enum EncounterFaction { Party, Opposition }
public enum DungeonFacing { North, East, South, West }
public sealed record ActivationCost(D20Id Budget, int Amount);
public sealed record ConditionClause(ConditionKind Kind, D20Id? Tag = null, int Amount = 0);
public enum ConditionKind { ForbidMovement, ForbidActionTag, AttackPenalty }
public sealed record DamageAffinity(D20Id DamageType, DamageAffinityKind Affinity);
public enum DamageAffinityKind { Resistant }
public sealed record ActionTarget(TargetKind Kind, TargetTeam Team, int MaximumTargets, bool RequiresLineOfEffect);
public enum TargetKind { Participant, Cell }
public enum TargetTeam { Hostile, Ally, SelfOnly, Any }
public sealed record ActionAttack(D20Id? Ability, D20Id? Defense, DamageDefinition? Damage, D20Id? Implement, int Range = 0);
public sealed record EncounterParticipant(D20Id Character, EncounterFaction Faction);
public sealed record GridPosition(int X, int Y);
public sealed record TacticalPlacement(D20Id Character, GridPosition Position);
public sealed record TacticalBoard(int Width, int Height, IReadOnlyList<string> Rows, IReadOnlyList<TacticalPlacement> Placements);
public sealed record EncounterOutcome(string Title, string Summary, D20Id? RewardItem, int? RecoveryVitality);
public sealed record DungeonEncounter(D20Id Encounter, GridPosition Position);
public sealed record DungeonLandmark(D20Id Id, GridPosition Position, string Title, string Text);
public sealed record DungeonDoor(D20Id Id, GridPosition Position, DungeonFacing Facing, string Title, string Text, D20Id? RequiresTreasure);
public sealed record DungeonTreasure(D20Id Id, GridPosition Position, D20Id Item, string Title, string Text);
public sealed record DungeonCheckpoint(D20Id Id, GridPosition Position, string Title, string Text);
public sealed record DungeonDefinition(string Title, D20Id WallStyle, int Width, int Height, IReadOnlyList<string> Rows, GridPosition Start, D20Id StartCheckpoint, DungeonFacing StartFacing, IReadOnlyList<DungeonEncounter> Encounters, IReadOnlyList<DungeonLandmark> Landmarks, IReadOnlyList<DungeonDoor> Doors, IReadOnlyList<DungeonTreasure> Treasures, IReadOnlyList<DungeonCheckpoint> Checkpoints);
public sealed record AdventureOutcome(string Source, string VictoryTitle, string VictoryText, string DefeatTitle, string DefeatText, IReadOnlyList<string> Details);
