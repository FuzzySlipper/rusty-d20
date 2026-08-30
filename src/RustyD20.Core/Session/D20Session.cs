using System.Collections.Immutable;
using Rusty.Engine;
using Rusty.Engine.Entities;
using Rusty.Engine.Mechanics;
using RustyD20.Core.Contract;
using RustyD20.Core.Rules;

namespace RustyD20.Core.Session;

public sealed record SessionTuning(int AbilityBaseline = 10, int AbilityModifierDivisor = 2, int MaximumStaticRolls = 4096, int MaximumReceiptCount = 128);
public enum RollSourceKind { Seeded, Static }
public sealed record StaticActionRoll(byte D20, ImmutableArray<ushort> Damage)
{
    public StaticActionRoll(byte d20, IReadOnlyList<ushort> damage) : this(d20, damage.ToImmutableArray()) { }
    public void Validate(DamageDefinition damage) { if (D20 is < 1 or > 20 || Damage.Length != damage.Dice || Damage.Any(value => value is 0 || value > damage.Sides)) throw new D20SessionException("The static action roll does not match the authored damage shape."); }
}
public sealed record RollSourceState(RollSourceKind Kind, ulong Seed, ImmutableArray<StaticActionRoll> StaticRolls, ulong Position)
{
    public static RollSourceState Seeded(ulong seed, ulong position = 0) => new(RollSourceKind.Seeded, seed, [], position);
    public static RollSourceState Static(IReadOnlyList<StaticActionRoll> rolls, ulong position = 0) => new(RollSourceKind.Static, 0, rolls.ToImmutableArray(), position);
}
/// <summary>The sole seeded-random boundary: this adapter only reaches generated Engine random services.</summary>
public sealed class ScopedSeededRollAdapter
{
    private readonly IRandomService _random;
    public ScopedSeededRollAdapter(IRandomService random) => _random = random ?? throw new ArgumentNullException(nameof(random));
    public StaticActionRoll Draw(ulong seed, ulong position, DamageDefinition damage)
    {
        Rng stream = _random.CreateScoped(new ScopedRngCreateRequest(seed, $"rusty-d20.action.{position}"));
        try { var results = ImmutableArray.CreateBuilder<ushort>(damage.Dice); byte d20 = checked((byte)(_random.NextBoundedU32(new ScopedRngBoundedRequest(stream, 20)).Value + 1)); for (int index = 0; index < damage.Dice; index++) results.Add(checked((ushort)(_random.NextBoundedU32(new ScopedRngBoundedRequest(stream, checked((uint)damage.Sides))).Value + 1))); return new StaticActionRoll(d20, results.MoveToImmutable()); }
        finally { stream.Dispose(); }
    }
}

public readonly record struct AbilityScoreEntry(D20Id Id, int Value);
public readonly record struct ResourceEntry(D20Id Id, int Value);
public readonly record struct BudgetEntry(D20Id Id, int Value);
public readonly record struct AbilityScoresFact(ImmutableArray<AbilityScoreEntry> Values);
public readonly record struct ActionResourcesFact(ImmutableArray<ResourceEntry> Values);
public readonly record struct ActivationBudgetsFact(ImmutableArray<BudgetEntry> Values);
public readonly record struct EncounterParticipationFact(EncounterFaction Faction, bool Living);
/// <summary>Duration/condition projection only; Engine EffectState is the canonical active-effect collection.</summary>
public readonly record struct EffectProjectionFact(ImmutableArray<ScheduledEffectProjection> Values);
public readonly record struct ScheduledEffectProjection(EffectInstanceId Instance, D20Id Effect, ulong ExpiresAtTurn);
public static class D20ComponentTypes
{
    public static readonly ComponentType<AbilityScoresFact> Abilities = ComponentType<AbilityScoresFact>.Create(ProductComponentKeys.Create(1));
    public static readonly ComponentType<ActionResourcesFact> Resources = ComponentType<ActionResourcesFact>.Create(ProductComponentKeys.Create(2));
    public static readonly ComponentType<ActivationBudgetsFact> Budgets = ComponentType<ActivationBudgetsFact>.Create(ProductComponentKeys.Create(3));
    public static readonly ComponentType<EncounterParticipationFact> Participation = ComponentType<EncounterParticipationFact>.Create(ProductComponentKeys.Create(4));
    public static readonly ComponentType<EffectProjectionFact> Effects = ComponentType<EffectProjectionFact>.Create(ProductComponentKeys.Create(5));
}
public sealed record ActionPreview(EntityId Actor, EntityId Target, D20Id Action, OperationId Operation, ulong Turn, ulong RollPosition, int AbilityModifier, int Defense, DamageDefinition Damage, int Range, ulong ActorAbilitiesRevision, ulong ActorBudgetRevision, ulong ActorEquipmentRevision, ulong ActorEffectsRevision, ulong TargetResourcesRevision, ulong TargetBudgetRevision, ulong TargetVitalityRevision, ulong TargetEffectsRevision, ulong InventoryRevision);
public sealed record ActionReceipt(OperationId Operation, EntityId Actor, EntityId Target, D20Id Action, ulong RollPosition, byte D20, int Total, int Defense, bool Hit, int Damage, D20Id? Effect, int ForcedMovementIntent, ulong Turn, ulong SessionRevision);
public sealed record ReactionReceipt(D20Id Reaction, EntityId Target, D20Id Resource, int Before, int After, D20Id Effect, ulong ExpiresAtTurn, ulong SessionRevision);
public sealed record VitalityProjection(EntityId Entity, ExactValue Current, ExactTrackBounds Bounds, ulong Revision);
public sealed class D20SessionException : InvalidOperationException { public D20SessionException(string message) : base(message) { } }

public sealed class D20Session : IDisposable
{
    private const ulong FirstInventoryOnlyEntity = 1UL << 63;
    private readonly IReadOnlyDictionary<D20Id, ActionDefinition> _actions;
    private readonly IReadOnlyDictionary<D20Id, DefenseDefinition> _defenses;
    private readonly IReadOnlyDictionary<D20Id, ImplementDefinition> _implements;
    private readonly IReadOnlyDictionary<D20Id, RustyD20.Core.Rules.EffectDefinition> _effects;
    private readonly IReadOnlyDictionary<D20Id, ReactionDefinition> _reactions;
    private readonly IReadOnlyDictionary<D20Id, Rusty.Engine.Mechanics.EffectDefinition> _engineEffects;
    private readonly ScopedSeededRollAdapter? _seededRolls;
    private readonly Dictionary<EntityId, ExactStatTrackState> _vitalityTracks = [];
    private readonly Dictionary<EntityId, EffectState> _effectStates = [];
    private readonly List<ActionReceipt> _receipts = [];
    private ulong _nextInventoryOnlyEntity = FirstInventoryOnlyEntity;
    private bool _disposed;

    public D20Session(CompiledD20Content rules, RollSourceState rollSource, SessionTuning? tuning = null, ScopedSeededRollAdapter? seededRolls = null)
    {
        ArgumentNullException.ThrowIfNull(rules);
        _actions = rules.Modules.SelectMany(module => module.ActionsOrEmpty).ToDictionary(value => value.Id);
        _defenses = rules.Modules.SelectMany(module => module.DefensesOrEmpty).ToDictionary(value => value.Id);
        _implements = rules.Modules.SelectMany(module => module.ImplementsOrEmpty).ToDictionary(value => value.Id);
        _effects = rules.Modules.SelectMany(module => module.EffectsOrEmpty).ToDictionary(value => value.Id);
        _reactions = rules.Modules.SelectMany(module => module.ReactionsOrEmpty).ToDictionary(value => value.Id);
        _engineEffects = _effects.Values.ToDictionary(effect => effect.Id, effect => new Rusty.Engine.Mechanics.EffectDefinition(EffectDefinitionId.Parse($"d20.effect.{effect.Id.Value}"), StackingGroupId.Parse($"d20.effect.{effect.Id.Value}"), EffectStackingPolicy.Refresh, 1, 1));
        Tuning = tuning ?? new SessionTuning();
        if (rollSource.StaticRolls.Length > Tuning.MaximumStaticRolls) throw new D20SessionException("The static action roll tape exceeds the named bound.");
        if (rollSource.Kind == RollSourceKind.Seeded && seededRolls is null) throw new ArgumentNullException(nameof(seededRolls), "Seeded sessions require the Engine random adapter.");
        RollSource = rollSource; _seededRolls = seededRolls;
        Entities = new EntityWorld([D20ComponentTypes.Abilities, D20ComponentTypes.Resources, D20ComponentTypes.Budgets, D20ComponentTypes.Participation, D20ComponentTypes.Effects]); Inventory = new InventoryWorld();
    }
    public EntityWorld Entities { get; }
    public InventoryWorld Inventory { get; }
    public SessionTuning Tuning { get; }
    public RollSourceState RollSource { get; private set; }
    public ulong Turn { get; private set; }
    public ulong Revision { get; private set; }
    public IReadOnlyList<ActionReceipt> Receipts => _receipts.AsReadOnly();
    public static int AbilityModifier(int score, SessionTuning? tuning = null) { var policy = tuning ?? new SessionTuning(); return Math.DivRem(score - policy.AbilityBaseline, policy.AbilityModifierDivisor, out int remainder) is int quotient && remainder < 0 ? quotient - 1 : quotient; }

    /// <summary>Participant admission mutates only a prepared EntityWorld batch; inventory ownership is an explicit later operation.</summary>
    public EntityId AddParticipant(CharacterDefinition character, EncounterFaction faction, int? vitality = null)
    {
        ThrowIfDisposed(); ArgumentNullException.ThrowIfNull(character); int maximum = vitality ?? character.Vitality;
        if (maximum <= 0) throw new D20SessionException("Participant vitality must be positive.");
        EntityId entity = new(Entities.NextEntityValue); ExactStatTrackState track = CreateVitalityTrack(entity, maximum);
        AbilityScoresFact abilities = new(character.Abilities.OrderBy(pair => pair.Key.Value, StringComparer.Ordinal).Select(pair => new AbilityScoreEntry(pair.Key, pair.Value)).ToImmutableArray());
        EntityBatch batch = new EntityBatch().Mutate(world => { EntityId created = world.Create(); if (created != entity) throw new D20SessionException("Entity identity changed while staging participant admission."); world.Set(created, D20ComponentTypes.Abilities, abilities); world.Set(created, D20ComponentTypes.Resources, new ActionResourcesFact([])); world.Set(created, D20ComponentTypes.Budgets, new ActivationBudgetsFact([])); world.Set(created, D20ComponentTypes.Participation, new EncounterParticipationFact(faction, true)); world.Set(created, D20ComponentTypes.Effects, new EffectProjectionFact([])); });
        Entities.PrepareBatch(batch, Entities.Revision).Publish(); _vitalityTracks.Add(entity, track); _effectStates.Add(entity, new EffectState(entity)); Revision++; return entity;
    }
    public void RegisterLoadoutOwner(EntityId owner)
    {
        ThrowIfDisposed(); RequireParticipant(owner); if (Inventory.TryGetInventory(owner, out _) || Inventory.TryGetEquipment(owner, out _)) throw new D20SessionException("Participant already has a loadout owner.");
        Inventory.RegisterInventory(new InventoryState(owner)); Inventory.RegisterEquipment(new EquipmentState(owner)); Revision++;
    }
    public VitalityProjection ReadVitality(EntityId entity) { ExactStatTrackSnapshot value = RequireTrack(entity).Read(); return new(entity, value.TrackCurrent, value.TrackBounds, value.Revision); }
    public void SetActionResource(EntityId entity, D20Id resource, int amount) => Replace(entity, D20ComponentTypes.Resources, value => new ActionResourcesFact(ReplaceResource(value.Values, resource, amount)));
    public void SetActivationBudget(EntityId entity, D20Id budget, int amount) => Replace(entity, D20ComponentTypes.Budgets, value => new ActivationBudgetsFact(ReplaceBudget(value.Values, budget, amount)));

    /// <summary>Items are inventory-only Engine identities, so one prepared InventoryWorld candidate is the complete mutation.</summary>
    public EntityId EquipImplement(EntityId owner, ImplementDefinition implement)
    {
        ThrowIfDisposed(); RequireParticipant(owner); ArgumentNullException.ThrowIfNull(implement);
        if (!Inventory.TryGetInventory(owner, out _) || !Inventory.TryGetEquipment(owner, out EquipmentState? equipment) || equipment is null) throw new D20SessionException("Participant must register a loadout owner before equipping.");
        EquipmentSlotId slot = EquipmentSlotId.Parse(implement.Slot.Value); if (equipment.Assignments.Any(assignment => assignment.Slot == slot)) throw new D20SessionException("The canonical Engine equipment slot is occupied.");
        if (_nextInventoryOnlyEntity == ulong.MaxValue) throw new D20SessionException("Inventory-only entity identity is exhausted."); EntityId item = new(_nextInventoryOnlyEntity);
        var definition = new Rusty.Engine.Mechanics.ItemDefinition(ItemDefinitionId.Parse(implement.Id.Value), ItemKind.Unique, 1, equipment: new ItemEquipmentPolicy(1));
        InventoryWorldCandidate candidate = Inventory.Prepare(); candidate.MaterializeUnique(new ItemState(item, definition), owner); candidate.Equip(owner, item, [new EquipmentSlotDefinition(slot)]); candidate.Publish(); _nextInventoryOnlyEntity++; Revision++; return item;
    }
    public void TransferImplementLoadout(EntityId item, EntityId fromOwner, EntityId toOwner, ImplementDefinition implement)
    {
        ThrowIfDisposed(); RequireParticipant(fromOwner); RequireParticipant(toOwner); ArgumentNullException.ThrowIfNull(implement);
        if (!Inventory.TryGetItem(item, out ItemState? currentItem) || currentItem is null || currentItem.Definition.Id != ItemDefinitionId.Parse(implement.Id.Value)) throw new D20SessionException("Transferred Engine item does not match the authored implement definition.");
        if (!Inventory.TryGetEquipment(toOwner, out EquipmentState? target) || target is null || target.Assignments.Any(assignment => assignment.Slot == EquipmentSlotId.Parse(implement.Slot.Value))) throw new D20SessionException("Target canonical Engine equipment slot is occupied.");
        InventoryWorldCandidate candidate = Inventory.Prepare(); candidate.Unequip(fromOwner, item); candidate.TransferUnique(item, fromOwner, toOwner); candidate.Equip(toOwner, item, [new EquipmentSlotDefinition(EquipmentSlotId.Parse(implement.Slot.Value))]); candidate.Publish(); Revision++;
    }
    public int ChoiceIndex(int choiceIndex, int choiceCount) { ThrowIfDisposed(); if (choiceCount <= 0 || choiceIndex < 0 || choiceIndex >= choiceCount) throw new D20SessionException("Choice index is outside the authored target set."); return choiceIndex; }

    public ActionPreview PreviewAction(EntityId actor, EntityId target, D20Id action, OperationId operation)
    {
        ThrowIfDisposed(); ActionDefinition definition = RequireAction(action); var actorParticipation = Entities.Get(actor, D20ComponentTypes.Participation); var targetParticipation = Entities.Get(target, D20ComponentTypes.Participation);
        if (!actorParticipation.Living || !targetParticipation.Living) throw new D20SessionException("Actions require living encounter participants."); EnsureTarget(actor, target, actorParticipation, targetParticipation, definition.Target); EnsureCosts(Entities.Get(actor, D20ComponentTypes.Budgets).Values, definition.Costs);
        ResolvedAttack resolved = ResolveAttack(actor, definition); AbilityScoresFact abilities = Entities.Get(actor, D20ComponentTypes.Abilities); if (!TryValue(abilities.Values, resolved.Ability, out int score)) throw new D20SessionException("The actor lacks the authored ability.");
        var active = ActiveEffects(actor); int penalty = active.Select(effect => _effects[effect.Effect]).SelectMany(effect => effect.Conditions).Where(clause => clause.Kind == ConditionKind.AttackPenalty).Sum(clause => clause.Amount);
        if (active.Any(effect => _effects[effect.Effect].Conditions.Any(clause => clause.Kind == ConditionKind.ForbidActionTag && clause.Tag is D20Id tag && definition.Tags.Any(actionTag => actionTag == tag)))) throw new D20SessionException("An active scheduled effect forbids this action tag.");
        return new(actor, target, action, operation, Turn, RollSource.Position, AbilityModifier(score, Tuning) + penalty, Defense(target, resolved.Defense), resolved.Damage, resolved.Range, ComponentRevision(actor, D20ComponentTypes.Abilities), ComponentRevision(actor, D20ComponentTypes.Budgets), EquipmentRevision(actor), ComponentRevision(actor, D20ComponentTypes.Effects), ComponentRevision(target, D20ComponentTypes.Resources), ComponentRevision(target, D20ComponentTypes.Budgets), RequireTrack(target).Revision, ComponentRevision(target, D20ComponentTypes.Effects), Inventory.Revision);
    }
    public ReactionReceipt ApplyReaction(ActionPreview preview, D20Id reaction)
    {
        EnsureFresh(preview); ReactionDefinition definition = _reactions.TryGetValue(reaction, out ReactionDefinition? value) ? value : throw new D20SessionException("Unknown reaction.");
        if (definition.Defense != ResolveAttack(preview.Actor, RequireAction(preview.Action)).Defense) throw new D20SessionException("Reaction does not defend this action's authored defense.");
        ActionResourcesFact resources = Entities.Get(preview.Target, D20ComponentTypes.Resources); if (!TryValue(resources.Values, definition.Resource, out int before) || before < definition.Cost) throw new D20SessionException("Reaction resource is unavailable.");
        ActivationBudgetsFact budgets = Entities.Get(preview.Target, D20ComponentTypes.Budgets); EnsureCosts(budgets.Values, definition.Costs); ulong expires = checked(Turn + (ulong)_effects[definition.Effect].DurationTurns); EffectProjectionFact afterProjection = NextEffectProjection(preview.Target, definition.Effect, expires);
        EntityBatch batch = new EntityBatch().Mutate(world => world.Set(preview.Target, D20ComponentTypes.Resources, new ActionResourcesFact(ReplaceResource(resources.Values, definition.Resource, before - definition.Cost)), world.GetComponentRevision(preview.Target, D20ComponentTypes.Resources))).Mutate(world => world.Set(preview.Target, D20ComponentTypes.Budgets, new ActivationBudgetsFact(SpendCosts(budgets.Values, definition.Costs)), world.GetComponentRevision(preview.Target, D20ComponentTypes.Budgets))).Mutate(world => world.Set(preview.Target, D20ComponentTypes.Effects, afterProjection, world.GetComponentRevision(preview.Target, D20ComponentTypes.Effects)));
        EntityWorldBatchCandidate prepared = Entities.PrepareBatch(batch, Entities.Revision); ApplyOrRefreshEffect(preview.Target, definition.Effect, preview.Operation); prepared.Publish(); Revision++; return new(reaction, preview.Target, definition.Resource, before, before - definition.Cost, definition.Effect, expires, Revision);
    }
    public ActionReceipt ApplyAction(ActionPreview preview)
    {
        EnsureFresh(preview); ActionDefinition definition = RequireAction(preview.Action); if (RollSource.Position == ulong.MaxValue) throw new D20SessionException("Roll-source position is exhausted."); ulong nextPosition = RollSource.Position + 1; StaticActionRoll roll = ReadActionRoll(preview.Damage);
        int total = roll.D20 + preview.AbilityModifier; bool hit = total >= preview.Defense; int damage = hit ? Math.Max(0, checked(roll.Damage.Sum(value => (int)value) + preview.Damage.Bonus)) : 0;
        ActivationBudgetsFact afterBudgets = new(SpendCosts(Entities.Get(preview.Actor, D20ComponentTypes.Budgets).Values, definition.Costs)); D20Id? effect = hit ? definition.Effect : null; EffectProjectionFact effectsAfter = effect is D20Id effectId ? NextEffectProjection(preview.Target, effectId, checked(Turn + (ulong)_effects[effectId].DurationTurns)) : Entities.Get(preview.Target, D20ComponentTypes.Effects);
        ExactStatTrackCurrentMutationCandidate? vitality = hit && damage != 0 ? RequireTrack(preview.Target).PrepareSpend(new ExactValue(damage), preview.TargetVitalityRevision) : null;
        EntityBatch batch = new EntityBatch().Mutate(world => world.Set(preview.Actor, D20ComponentTypes.Budgets, afterBudgets, world.GetComponentRevision(preview.Actor, D20ComponentTypes.Budgets))).Mutate(world => world.Set(preview.Target, D20ComponentTypes.Effects, effectsAfter, world.GetComponentRevision(preview.Target, D20ComponentTypes.Effects)));
        EntityWorldBatchCandidate prepared = Entities.PrepareBatch(batch, Entities.Revision); if (effect is D20Id appliedEffect) ApplyOrRefreshEffect(preview.Target, appliedEffect, preview.Operation); vitality?.Publish(); prepared.Publish(); RollSource = RollSource with { Position = nextPosition }; Revision++;
        var receipt = new ActionReceipt(preview.Operation, preview.Actor, preview.Target, preview.Action, preview.RollPosition, roll.D20, total, preview.Defense, hit, damage, effect, hit ? definition.ForcedMovement : 0, Turn, Revision); _receipts.Add(receipt); if (_receipts.Count > Tuning.MaximumReceiptCount) _receipts.RemoveAt(0); return receipt;
    }
    public void AdvanceTurn()
    {
        ThrowIfDisposed(); ulong nextTurn = checked(Turn + 1); var mutations = new EntityBatch(); var expiring = new List<(EntityId Entity, EffectInstanceId Instance)>();
        foreach (EntityComponent<EffectProjectionFact> component in Entities.Query(D20ComponentTypes.Effects)) { var after = new EffectProjectionFact(component.Value.Values.Where(value => value.ExpiresAtTurn > nextTurn).ToImmutableArray()); foreach (ScheduledEffectProjection value in component.Value.Values.Where(value => value.ExpiresAtTurn <= nextTurn)) expiring.Add((component.Entity, value.Instance)); if (!after.Equals(component.Value)) mutations.Mutate(world => world.Set(component.Entity, D20ComponentTypes.Effects, after, world.GetComponentRevision(component.Entity, D20ComponentTypes.Effects))); }
        EntityWorldBatchCandidate prepared = Entities.PrepareBatch(mutations, Entities.Revision); foreach ((EntityId entity, EffectInstanceId instance) in expiring) _effectStates[entity].Expire(instance); prepared.Publish(); Turn = nextTurn; Revision++;
    }

    private void EnsureFresh(ActionPreview preview) { if (preview.Turn != Turn || preview.RollPosition != RollSource.Position || preview.InventoryRevision != Inventory.Revision || preview.ActorAbilitiesRevision != ComponentRevision(preview.Actor, D20ComponentTypes.Abilities) || preview.ActorBudgetRevision != ComponentRevision(preview.Actor, D20ComponentTypes.Budgets) || preview.ActorEquipmentRevision != EquipmentRevision(preview.Actor) || preview.ActorEffectsRevision != ComponentRevision(preview.Actor, D20ComponentTypes.Effects) || preview.TargetResourcesRevision != ComponentRevision(preview.Target, D20ComponentTypes.Resources) || preview.TargetBudgetRevision != ComponentRevision(preview.Target, D20ComponentTypes.Budgets) || preview.TargetVitalityRevision != RequireTrack(preview.Target).Revision || preview.TargetEffectsRevision != ComponentRevision(preview.Target, D20ComponentTypes.Effects)) throw new D20SessionException("Action preview is stale."); }
    private StaticActionRoll ReadActionRoll(DamageDefinition damage) { StaticActionRoll result = RollSource.Kind switch { RollSourceKind.Static when RollSource.Position < (ulong)RollSource.StaticRolls.Length => RollSource.StaticRolls[checked((int)RollSource.Position)], RollSourceKind.Static => throw new D20SessionException("Static action rolls are exhausted."), RollSourceKind.Seeded => _seededRolls!.Draw(RollSource.Seed, RollSource.Position, damage), _ => throw new D20SessionException("Unknown roll source."), }; result.Validate(damage); return result; }
    private EffectProjectionFact NextEffectProjection(EntityId entity, D20Id effect, ulong expires) { EffectInstanceId instance = EffectInstance(entity, effect); var values = Entities.Get(entity, D20ComponentTypes.Effects).Values.Where(value => value.Instance != instance).Append(new ScheduledEffectProjection(instance, effect, expires)).OrderBy(value => value.Instance.Value, StringComparer.Ordinal).ToImmutableArray(); return new EffectProjectionFact(values); }
    private void ApplyOrRefreshEffect(EntityId entity, D20Id effect, OperationId operation) { EffectState state = _effectStates[entity]; EffectInstanceId instance = EffectInstance(entity, effect); var definition = _engineEffects[effect]; var provenance = new RequestSourceIdentity(operation, SourceInstanceId.Parse($"d20.effect.{effect.Value}")); if (state.Effects.Any(value => value.Instance == instance)) state.Refresh(instance, provenance, 1); else state.Apply(definition, instance, provenance, 1); }
    private IReadOnlyList<ScheduledEffectProjection> ActiveEffects(EntityId entity) { var active = _effectStates[entity].Effects.Select(value => value.Instance).ToHashSet(); return Entities.Get(entity, D20ComponentTypes.Effects).Values.Where(value => active.Contains(value.Instance) && value.ExpiresAtTurn > Turn).ToArray(); }
    private static EffectInstanceId EffectInstance(EntityId entity, D20Id effect) => EffectInstanceId.Parse($"d20.effect.{entity.Value}.{effect.Value}");
    private ResolvedAttack ResolveAttack(EntityId actor, ActionDefinition action) { if (action.Attack.Ability is D20Id ability && action.Attack.Defense is D20Id defense && action.Attack.Damage is DamageDefinition damage) return new(ability, defense, damage, 0); D20Id implementId = action.Attack.Implement ?? throw new D20SessionException("Action lacks a resolved attack."); ImplementDefinition implement = _implements[implementId]; if (!Inventory.TryGetEquipment(actor, out EquipmentState? equipment) || equipment is null || !equipment.Assignments.Any(assignment => Inventory.TryGetItem(assignment.Item, out ItemState? item) && item?.Definition.Id == ItemDefinitionId.Parse(implement.Id.Value))) throw new D20SessionException("The required canonical Engine implement is not equipped."); return new(implement.Ability, implement.Defense, implement.Damage, implement.Range); }
    private int Defense(EntityId target, D20Id defense) { DefenseDefinition definition = _defenses[defense]; AbilityScoresFact abilities = Entities.Get(target, D20ComponentTypes.Abilities); return definition.Base + definition.Abilities.Select(ability => TryValue(abilities.Values, ability, out int score) ? AbilityModifier(score, Tuning) : int.MinValue).Max(); }
    private static void EnsureTarget(EntityId actor, EntityId target, EncounterParticipationFact actorParticipation, EncounterParticipationFact targetParticipation, ActionTarget authored) { if (authored.Kind != TargetKind.Participant || authored.MaximumTargets != 1) throw new D20SessionException("This session action API admits exactly one participant target."); bool allowed = authored.Team switch { TargetTeam.Hostile => actorParticipation.Faction != targetParticipation.Faction, TargetTeam.Ally => actorParticipation.Faction == targetParticipation.Faction && actor != target, TargetTeam.SelfOnly => actor == target, TargetTeam.Any => true, _ => false, }; if (!allowed) throw new D20SessionException("Target does not satisfy the authored target team policy."); }
    private ExactStatTrackState CreateVitalityTrack(EntityId entity, int value) { StatId stat = StatId.Parse($"d20.vitality.{entity.Value}"); return new(new ExactStatDefinition(stat, new ExactValue(0), new ExactValue(value)), new ExactValue(value), [], new ExactTrackDefinition(TrackId.Parse($"d20.vitality.{entity.Value}"), new ExactValue(0), new ExactTrackMaximum.FromStat(stat)), new ExactValue(value)); }
    private ActionDefinition RequireAction(D20Id action) => _actions.TryGetValue(action, out ActionDefinition? value) ? value : throw new D20SessionException("Unknown action.");
    private ExactStatTrackState RequireTrack(EntityId entity) => _vitalityTracks.TryGetValue(entity, out ExactStatTrackState? track) ? track : throw new D20SessionException("Participant has no vitality track.");
    private void RequireParticipant(EntityId entity) { if (!Entities.IsAlive(entity) || !_vitalityTracks.ContainsKey(entity)) throw new D20SessionException("Unknown participant."); }
    private static bool TryValue(ImmutableArray<AbilityScoreEntry> values, D20Id id, out int value) { foreach (AbilityScoreEntry entry in values) if (entry.Id == id) { value = entry.Value; return true; } value = 0; return false; }
    private static bool TryValue(ImmutableArray<ResourceEntry> values, D20Id id, out int value) { foreach (ResourceEntry entry in values) if (entry.Id == id) { value = entry.Value; return true; } value = 0; return false; }
    private static bool TryValue(ImmutableArray<BudgetEntry> values, D20Id id, out int value) { foreach (BudgetEntry entry in values) if (entry.Id == id) { value = entry.Value; return true; } value = 0; return false; }
    private static ImmutableArray<ResourceEntry> ReplaceResource(ImmutableArray<ResourceEntry> values, D20Id id, int amount) => values.Where(value => value.Id != id).Append(new ResourceEntry(id, amount)).OrderBy(value => value.Id.Value, StringComparer.Ordinal).ToImmutableArray();
    private static ImmutableArray<BudgetEntry> ReplaceBudget(ImmutableArray<BudgetEntry> values, D20Id id, int amount) => values.Where(value => value.Id != id).Append(new BudgetEntry(id, amount)).OrderBy(value => value.Id.Value, StringComparer.Ordinal).ToImmutableArray();
    private static ImmutableArray<BudgetEntry> SpendCosts(ImmutableArray<BudgetEntry> current, IReadOnlyList<ActivationCost> costs) { var values = current.ToDictionary(value => value.Id, value => value.Value); foreach (ActivationCost cost in costs) values[cost.Budget] -= cost.Amount; return values.OrderBy(pair => pair.Key.Value, StringComparer.Ordinal).Select(pair => new BudgetEntry(pair.Key, pair.Value)).ToImmutableArray(); }
    private static void EnsureCosts(ImmutableArray<BudgetEntry> current, IReadOnlyList<ActivationCost> costs) { if (costs.Any(cost => !TryValue(current, cost.Budget, out int value) || value < cost.Amount)) throw new D20SessionException("Activation budget is unavailable."); }
    private ulong ComponentRevision<T>(EntityId entity, ComponentType<T> component) where T : struct => Entities.GetComponentRevision(entity, component).Revision;
    private ulong EquipmentRevision(EntityId entity) => Inventory.TryGetEquipment(entity, out EquipmentState? state) && state is not null ? state.Revision : 0;
    private void Replace<T>(EntityId entity, ComponentType<T> component, Func<T, T> mutate) where T : struct { T current = Entities.Get(entity, component); Entities.Set(entity, component, mutate(current), Entities.GetComponentRevision(entity, component)); Revision++; }
    private void ThrowIfDisposed() { if (_disposed) throw new ObjectDisposedException(nameof(D20Session)); }
    public void Dispose() { if (_disposed) return; Entities.Dispose(); _disposed = true; }
    private sealed record ResolvedAttack(D20Id Ability, D20Id Defense, DamageDefinition Damage, int Range);
}
