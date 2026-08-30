using System.Collections.Immutable;
using Rusty.Engine;
using Rusty.Engine.Entities;
using Rusty.Engine.Mechanics;
using RustyD20.Core.Contract;
using RustyD20.Core.Rules;

namespace RustyD20.Core.Session;

public sealed record SessionTuning(int AbilityBaseline = 10, int AbilityModifierDivisor = 2, int MaximumStaticRolls = 4096, int MaximumReceiptCount = 128, int ParticipantInventoryCapacity = 4);
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
/// <summary>
/// A session-issued action preview. Consumers can observe it for tactical
/// admission, but only this assembly can construct or alter its fence facts.
/// Apply paths still rebind every outcome fact from the live catalog/state.
/// </summary>
public sealed record ActionPreview
{
    internal ActionPreview(EntityId actor, EntityId target, D20Id action, OperationId operation, ulong turn, ulong rollPosition, int abilityModifier, int defense, DamageDefinition damage, int range, ulong actorAbilitiesRevision, ulong actorBudgetRevision, ulong actorEquipmentRevision, ulong actorEffectsRevision, ulong targetResourcesRevision, ulong targetBudgetRevision, ulong targetVitalityRevision, ulong targetEffectsRevision, ulong inventoryRevision)
    {
        Actor = actor; Target = target; Action = action; Operation = operation; Turn = turn; RollPosition = rollPosition;
        AbilityModifier = abilityModifier; Defense = defense; Damage = damage; Range = range;
        ActorAbilitiesRevision = actorAbilitiesRevision; ActorBudgetRevision = actorBudgetRevision; ActorEquipmentRevision = actorEquipmentRevision; ActorEffectsRevision = actorEffectsRevision;
        TargetResourcesRevision = targetResourcesRevision; TargetBudgetRevision = targetBudgetRevision; TargetVitalityRevision = targetVitalityRevision; TargetEffectsRevision = targetEffectsRevision; InventoryRevision = inventoryRevision;
    }

    public EntityId Actor { get; internal init; }
    public EntityId Target { get; internal init; }
    public D20Id Action { get; internal init; }
    public OperationId Operation { get; internal init; }
    public ulong Turn { get; internal init; }
    public ulong RollPosition { get; internal init; }
    public int AbilityModifier { get; internal init; }
    public int Defense { get; internal init; }
    public DamageDefinition Damage { get; internal init; }
    public int Range { get; internal init; }
    public ulong ActorAbilitiesRevision { get; internal init; }
    public ulong ActorBudgetRevision { get; internal init; }
    public ulong ActorEquipmentRevision { get; internal init; }
    public ulong ActorEffectsRevision { get; internal init; }
    public ulong TargetResourcesRevision { get; internal init; }
    public ulong TargetBudgetRevision { get; internal init; }
    public ulong TargetVitalityRevision { get; internal init; }
    public ulong TargetEffectsRevision { get; internal init; }
    public ulong InventoryRevision { get; internal init; }
}
public sealed record ActionReceipt(OperationId Operation, EntityId Actor, EntityId Target, D20Id Action, ulong RollPosition, byte D20, int Total, int Defense, bool Hit, int Damage, D20Id? Effect, int ForcedMovementIntent, ulong Turn, ulong SessionRevision);
public sealed record ReactionReceipt(D20Id Reaction, EntityId Target, D20Id Resource, int Before, int After, D20Id Effect, ulong ExpiresAtTurn, ulong SessionRevision);
public sealed record ReactionResolutionReceipt(ReactionReceipt? Reaction, ActionReceipt Action);
public sealed record VitalityProjection(EntityId Entity, ExactValue Current, ExactTrackBounds Bounds, ulong Revision);
public sealed record DefeatRecoveryReceipt(EntityId Entity, ExactValue Before, ExactValue After, ExactValue AppliedAmount, ulong TrackRevision, ulong SessionRevision);
public enum SessionOwnerKind { Participant, Storage }
public sealed record SessionOwnerSave(ulong Entity, D20Id Owner, SessionOwnerKind Kind, int Capacity, bool HasInventory);
public sealed record SessionParticipantSave(ulong Entity, D20Id Character, EncounterFaction Faction, bool Living, int Vitality, int MaximumVitality, IReadOnlyList<AbilityScoreEntry> Abilities, IReadOnlyList<ResourceEntry> Resources, IReadOnlyList<BudgetEntry> Budgets, IReadOnlyList<ScheduledEffectProjection> Effects);
public sealed record SessionItemSave(ulong Entity, ulong Owner, D20Id Implement, IReadOnlyList<string> EquippedSlots, D20Id? Item = null, EquipmentKind Kind = EquipmentKind.Implement, D20Id? Equipment = null);
public sealed record D20InventoryTransferReceipt(D20Id Item, D20Id FromOwner, D20Id ToOwner, ulong EngineRevisionBefore, ulong EngineRevisionAfter, ulong SessionRevision);
public sealed record AdventureLoadoutAdmission(D20Id Adventure, IReadOnlyDictionary<D20Id, EntityId> Owners, IReadOnlyDictionary<D20Id, EntityId> Items, ulong InventoryRevision, ulong SessionRevision);
public sealed record D20SessionSave(RollSourceState RollSource, ulong Turn, ulong Revision, IReadOnlyList<SessionParticipantSave> Participants, IReadOnlyList<SessionItemSave> Items, IReadOnlyList<ActionReceipt> Receipts, IReadOnlyList<SessionOwnerSave>? Owners = null, IReadOnlyList<D20InventoryTransferReceipt>? InventoryTransfers = null, D20Id? Adventure = null);
public sealed class D20SessionException : InvalidOperationException { public D20SessionException(string message) : base(message) { } }

public sealed class D20Session : IDisposable
{
    private const ulong FirstInventoryOnlyEntity = 1UL << 63;
    private readonly D20DefinitionCatalog _catalog;
    private readonly IReadOnlyDictionary<D20Id, ActionDefinition> _actions;
    private readonly IReadOnlyDictionary<D20Id, AbilityDefinition> _abilities;
    private readonly IReadOnlyDictionary<D20Id, DefenseDefinition> _defenses;
    private readonly IReadOnlyDictionary<D20Id, ArmorDefinition> _armors;
    private readonly IReadOnlyDictionary<D20Id, ImplementDefinition> _implements;
    private readonly IReadOnlyDictionary<D20Id, CharacterDefinition> _characters;
    private readonly IReadOnlyDictionary<D20Id, RustyD20.Core.Rules.ItemDefinition> _authoredItems;
    private readonly IReadOnlyDictionary<D20Id, StorageDefinition> _storage;
    private readonly IReadOnlyDictionary<D20Id, ResourceDefinition> _resources;
    private readonly IReadOnlyDictionary<D20Id, ActivationBudgetDefinition> _budgets;
    private readonly IReadOnlyDictionary<D20Id, RustyD20.Core.Rules.EffectDefinition> _effects;
    private readonly IReadOnlyDictionary<D20Id, ReactionDefinition> _reactions;
    private readonly IReadOnlyDictionary<D20Id, Rusty.Engine.Mechanics.EffectDefinition> _engineEffects;
    private readonly ScopedSeededRollAdapter? _seededRolls;
    private readonly Dictionary<EntityId, ExactStatTrackState> _vitalityTracks = [];
    private readonly Dictionary<EntityId, EffectState> _effectStates = [];
    private readonly Dictionary<EntityId, D20Id> _participantCharacters = [];
    private readonly Dictionary<D20Id, EntityId> _ownerEntities = [];
    private readonly Dictionary<EntityId, D20Id> _ownerIds = [];
    private readonly Dictionary<D20Id, EntityId> _itemEntities = [];
    private readonly Dictionary<EntityId, D20Id> _itemIds = [];
    private readonly Dictionary<EntityId, (EquipmentKind Kind, D20Id Equipment)> _itemEquipment = [];
    private readonly Dictionary<D20Id, SessionOwnerKind> _ownerKinds = [];
    private readonly Dictionary<D20Id, int> _ownerCapacities = [];
    private readonly List<D20InventoryTransferReceipt> _inventoryTransfers = [];
    private D20Id? _admittedAdventure;
    private readonly List<ActionReceipt> _receipts = [];
    private ulong _nextInventoryOnlyEntity = FirstInventoryOnlyEntity;
    private bool _disposed;

    public D20Session(CompiledD20Content rules, RollSourceState rollSource, SessionTuning? tuning = null, ScopedSeededRollAdapter? seededRolls = null)
    {
        ArgumentNullException.ThrowIfNull(rules);
        _catalog = rules.Catalog ?? throw new D20SessionException("Compiled content does not provide its normalized definition catalog.");
        _actions = _catalog.Actions;
        _abilities = _catalog.Abilities;
        _defenses = _catalog.Defenses;
        _armors = _catalog.Armors;
        _implements = _catalog.Implements;
        _characters = _catalog.Characters;
        _authoredItems = _catalog.Items;
        _storage = _catalog.Storage;
        _resources = _catalog.Resources;
        _budgets = _catalog.Budgets;
        _effects = _catalog.Effects;
        _reactions = _catalog.Reactions;
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
        if (_ownerEntities.ContainsKey(character.Id)) throw new D20SessionException($"Participant character {character.Id} is already admitted.");
        if (_participantCharacters.Count >= D20Limits.EncounterParticipants) throw new D20SessionException("Participant bound.");
        EntityId entity = new(Entities.NextEntityValue); ExactStatTrackState track = CreateVitalityTrack(entity, maximum);
        AbilityScoresFact abilities = new(character.Abilities.OrderBy(pair => pair.Key.Value, StringComparer.Ordinal).Select(pair => new AbilityScoreEntry(pair.Key, pair.Value)).ToImmutableArray());
        ActionResourcesFact resources = new(_resources.Values.OrderBy(value => value.Id.Value, StringComparer.Ordinal).Select(value => new ResourceEntry(value.Id, value.Maximum)).ToImmutableArray());
        ActivationBudgetsFact budgets = new(_budgets.Values.OrderBy(value => value.Id.Value, StringComparer.Ordinal).Select(value => new BudgetEntry(value.Id, value.Initial)).ToImmutableArray());
        EntityBatch batch = new EntityBatch().Mutate(world => { EntityId created = world.Create(); if (created != entity) throw new D20SessionException("Entity identity changed while staging participant admission."); world.Set(created, D20ComponentTypes.Abilities, abilities); world.Set(created, D20ComponentTypes.Resources, resources); world.Set(created, D20ComponentTypes.Budgets, budgets); world.Set(created, D20ComponentTypes.Participation, new EncounterParticipationFact(faction, true)); world.Set(created, D20ComponentTypes.Effects, new EffectProjectionFact([])); });
        Entities.PrepareBatch(batch, Entities.Revision).Publish(); _vitalityTracks.Add(entity, track); _effectStates.Add(entity, new EffectState(entity)); _participantCharacters.Add(entity, character.Id); _ownerEntities.Add(character.Id, entity); _ownerIds.Add(entity, character.Id); _ownerKinds[character.Id] = SessionOwnerKind.Participant; _ownerCapacities[character.Id] = Tuning.ParticipantInventoryCapacity; Revision++; return entity;
    }
    public void RegisterLoadoutOwner(EntityId owner)
    {
        ThrowIfDisposed(); RequireParticipant(owner); D20Id ownerId = _ownerIds[owner]; RegisterLoadoutOwner(ownerId, Tuning.ParticipantInventoryCapacity);
    }
    public VitalityProjection ReadVitality(EntityId entity) { ExactStatTrackSnapshot value = RequireTrack(entity).Read(); return new(entity, value.TrackCurrent, value.TrackBounds, value.Revision); }
    public bool IsParticipant(EntityId entity) => _vitalityTracks.ContainsKey(entity);
    public EncounterFaction FactionOf(EntityId entity) => Entities.Get(entity, D20ComponentTypes.Participation).Faction;
    public bool IsLiving(EntityId entity) => Entities.Get(entity, D20ComponentTypes.Participation).Living && ReadVitality(entity).Current.Raw > 0;
    public bool TryGetOwnerEntity(D20Id owner, out EntityId entity) => _ownerEntities.TryGetValue(owner, out entity);
    public EntityId OwnerEntity(D20Id owner) => _ownerEntities.TryGetValue(owner, out EntityId entity) ? entity : throw new D20SessionException($"Unknown D20 owner {owner}.");
    public bool TryGetItemEntity(D20Id item, out EntityId entity) => _itemEntities.TryGetValue(item, out entity);
    public EntityId ItemEntity(D20Id item) => _itemEntities.TryGetValue(item, out EntityId entity) ? entity : throw new D20SessionException($"Unknown D20 item {item}.");
    /// <summary>Returns this admitted actor's authored action closure from the immutable compiled catalog.</summary>
    public IReadOnlyList<ActionDefinition> AdmittedActions(EntityId actor)
    {
        ThrowIfDisposed(); RequireParticipant(actor);
        CharacterDefinition character = _catalog.Characters[_participantCharacters[actor]];
        return character.Actions.Select(action => _catalog.Actions[action]).ToImmutableArray();
    }
    public IReadOnlyList<D20InventoryTransferReceipt> InventoryTransfers => _inventoryTransfers.AsReadOnly();
    public void SetActionResource(EntityId entity, D20Id resource, int amount) => Replace(entity, D20ComponentTypes.Resources, value => new ActionResourcesFact(ReplaceResource(value.Values, resource, amount)));
    public void SetActivationBudget(EntityId entity, D20Id budget, int amount) => Replace(entity, D20ComponentTypes.Budgets, value => new ActivationBudgetsFact(ReplaceBudget(value.Values, budget, amount)));

    /// <summary>
    /// Admits one authored adventure's complete owner/item closure through one detached
    /// InventoryWorld candidate. Character and storage identities are stable D20 mappings;
    /// Engine owns containment and equipment relationships.
    /// </summary>
    public AdventureLoadoutAdmission AdmitAdventureLoadout(AdventureDefinition adventure)
    {
        ThrowIfDisposed();
        ArgumentNullException.ThrowIfNull(adventure);
        if (_admittedAdventure is D20Id admitted)
        {
            if (admitted == adventure.Id) throw new D20SessionException($"Adventure loadout {adventure.Id} is already admitted.");
            throw new D20SessionException($"Session already owns adventure loadout {admitted}.");
        }

        ValidateAdventureLoadout(adventure);
        if (adventure.Characters.Count > D20Limits.AdventureEntries || adventure.Storage.Count > D20Limits.AdventureEntries || adventure.Items.Count > D20Limits.AdventureEntries)
        {
            throw new D20SessionException("Adventure loadout exceeds the authored entry bound.");
        }

        // Admit any encounter-only character owners that the caller has not explicitly added.
        // Their faction is authored by party membership; all remaining adventure characters are
        // opposition owners. This keeps the owner closure complete and deterministic.
        foreach (D20Id characterId in adventure.Characters.OrderBy(value => value.Value, StringComparer.Ordinal))
        {
            if (_ownerEntities.ContainsKey(characterId)) continue;
            if (!_characters.TryGetValue(characterId, out CharacterDefinition? character)) throw new D20SessionException($"Adventure references unknown character owner {characterId}.");
            EncounterFaction faction = adventure.Party.Contains(characterId) ? EncounterFaction.Party : EncounterFaction.Opposition;
            AddParticipant(character, faction);
        }

        var storageEntities = new Dictionary<D20Id, EntityId>();
        ulong nextEntity = _nextInventoryOnlyEntity;
        foreach (D20Id storageId in adventure.Storage.OrderBy(value => value.Value, StringComparer.Ordinal))
        {
            if (!_storage.TryGetValue(storageId, out StorageDefinition? definition)) throw new D20SessionException($"Adventure references unknown storage owner {storageId}.");
            if (_ownerEntities.TryGetValue(storageId, out EntityId existing))
            {
                storageEntities.Add(storageId, existing);
                continue;
            }

            if (nextEntity == ulong.MaxValue) throw new D20SessionException("Inventory owner identity is exhausted.");
            EntityId entity = new(nextEntity++);
            storageEntities.Add(storageId, entity);
        }

        foreach (D20Id ownerId in adventure.Characters.Concat(adventure.Storage).Distinct().OrderBy(value => value.Value, StringComparer.Ordinal))
        {
            if (!_ownerEntities.TryGetValue(ownerId, out EntityId owner))
            {
                owner = storageEntities[ownerId];
                _ownerEntities.Add(ownerId, owner);
                _ownerIds.Add(owner, ownerId);
                _ownerKinds[ownerId] = SessionOwnerKind.Storage;
                _ownerCapacities[ownerId] = _storage[ownerId].Capacity;
            }

            int capacity = _ownerKinds[ownerId] == SessionOwnerKind.Storage ? _storage[ownerId].Capacity : Tuning.ParticipantInventoryCapacity;
            RegisterLoadoutOwner(ownerId, capacity);
        }

        var itemEntities = new Dictionary<D20Id, EntityId>();
        var itemMechanisms = new Dictionary<D20Id, Rusty.Engine.Mechanics.ItemDefinition>();
        var initialEquipmentSlots = new Dictionary<D20Id, EquipmentSlotId>();
        var occupiedEquipmentSlots = new Dictionary<EntityId, HashSet<EquipmentSlotId>>();
        foreach ((D20Id ownerId, EntityId owner) in _ownerEntities.Where(entry => adventure.Characters.Contains(entry.Key) || adventure.Storage.Contains(entry.Key)))
        {
            EquipmentState equipment = Inventory.TryGetEquipment(owner, out EquipmentState? state) && state is not null ? state : new EquipmentState(owner);
            occupiedEquipmentSlots[owner] = equipment.Assignments.Select(value => value.Slot).ToHashSet();
        }
        foreach (D20Id itemId in adventure.Items.OrderBy(value => value.Value, StringComparer.Ordinal))
        {
            if (!_authoredItems.TryGetValue(itemId, out RustyD20.Core.Rules.ItemDefinition? authored)) throw new D20SessionException($"Adventure references unknown item {itemId}.");
            if (!_ownerEntities.TryGetValue(authored.Owner, out EntityId owner)) throw new D20SessionException($"Item {itemId} has no admitted owner.");
            if (itemEntities.ContainsKey(itemId)) throw new D20SessionException($"Adventure item {itemId} is duplicated.");
            if (nextEntity == ulong.MaxValue) throw new D20SessionException("Inventory item identity is exhausted.");
            EntityId itemEntity = new(nextEntity++);
            D20Id equipmentId = authored.Equipment;
            Rusty.Engine.Mechanics.ItemDefinition mechanism = ToEngineItemDefinition(itemId, authored, equipmentId);
            itemEntities.Add(itemId, itemEntity);
            itemMechanisms.Add(itemId, mechanism);
            if (authored.Equipped)
            {
                EquipmentSlotId authoredSlot = EquipmentSlotId.Parse((_armors.TryGetValue(equipmentId, out ArmorDefinition? armor) ? armor.Slot : _implements[equipmentId].Slot).Value);
                EquipmentSlotId selectedSlot = authoredSlot;
                if (!occupiedEquipmentSlots[owner].Add(selectedSlot))
                {
                    selectedSlot = EquipmentSlotId.Parse($"d20.initial.{itemId.Value}");
                    if (!occupiedEquipmentSlots[owner].Add(selectedSlot)) throw new D20SessionException($"Authored initial equipment slot for {itemId} is not unique.");
                }
                initialEquipmentSlots[itemId] = selectedSlot;
            }
        }

        InventoryWorldCandidate candidate = Inventory.Prepare();
        foreach (D20Id itemId in adventure.Items.OrderBy(value => value.Value, StringComparer.Ordinal))
        {
            RustyD20.Core.Rules.ItemDefinition authored = _authoredItems[itemId];
            EntityId owner = _ownerEntities[authored.Owner];
            EntityId itemEntity = itemEntities[itemId];
            candidate.MaterializeUnique(new ItemState(itemEntity, itemMechanisms[itemId]), owner);
            if (authored.Equipped)
            {
                candidate.Equip(owner, itemEntity, [new EquipmentSlotDefinition(initialEquipmentSlots[itemId])]);
            }
        }
        candidate.Publish();

        foreach ((D20Id storageId, EntityId entity) in storageEntities)
        {
            _nextInventoryOnlyEntity = Math.Max(_nextInventoryOnlyEntity, checked(entity.Value + 1));
        }
        foreach ((D20Id itemId, EntityId entity) in itemEntities)
        {
            _itemEntities.Add(itemId, entity);
            _itemIds.Add(entity, itemId);
            RustyD20.Core.Rules.ItemDefinition authored = _authoredItems[itemId];
            _itemEquipment[entity] = (authored.EquipmentKind, authored.Equipment);
            _nextInventoryOnlyEntity = Math.Max(_nextInventoryOnlyEntity, checked(entity.Value + 1));
        }
        _admittedAdventure = adventure.Id;
        Revision++;
        return new AdventureLoadoutAdmission(adventure.Id, _ownerEntities.Where(entry => adventure.Characters.Contains(entry.Key) || adventure.Storage.Contains(entry.Key)).ToDictionary(entry => entry.Key, entry => entry.Value), itemEntities, Inventory.Revision, Revision);
    }

    /// <summary>Transfers an authored item exactly once to another admitted owner.</summary>
    public D20InventoryTransferReceipt TransferAdventureItem(D20Id item, D20Id toOwner)
    {
        ThrowIfDisposed();
        EntityId itemEntity = ItemEntity(item);
        EntityId destination = OwnerEntity(toOwner);
        if (!Inventory.TryGetContainer(itemEntity, out EntityId source)) throw new D20SessionException($"Adventure item {item} has no Engine inventory owner.");
        if (source == destination) throw new D20SessionException($"Adventure item {item} is already owned by {toOwner}.");
        if (!_ownerIds.TryGetValue(source, out D20Id fromOwner)) throw new D20SessionException("Adventure item source owner is outside the admitted owner mapping.");
        if (!Inventory.TryGetEquipment(source, out EquipmentState? sourceEquipment) || sourceEquipment is null) throw new D20SessionException("Adventure item source lacks canonical Engine equipment state.");

        InventoryWorldCandidate candidate = Inventory.Prepare();
        if (sourceEquipment.ContainsItem(itemEntity)) candidate.Unequip(source, itemEntity);
        ItemTransferReceipt transfer = candidate.TransferUnique(itemEntity, source, destination);
        candidate.Publish();
        Revision++;
        var receipt = new D20InventoryTransferReceipt(item, fromOwner, toOwner, transfer.WorldRevisionBefore, transfer.WorldRevisionAfter, Revision);
        _inventoryTransfers.Add(receipt);
        if (_inventoryTransfers.Count > Tuning.MaximumReceiptCount) _inventoryTransfers.RemoveAt(0);
        return receipt;
    }

    public void RequireAdventureItemOwner(D20Id item, D20Id owner)
    {
        EntityId itemEntity = ItemEntity(item);
        EntityId ownerEntity = OwnerEntity(owner);
        if (!Inventory.TryGetContainer(itemEntity, out EntityId actual) || actual != ownerEntity) throw new D20SessionException($"Adventure item {item} is not contained by {owner}.");
    }

    /// <summary>Restores authored defeat vitality through detached Engine track candidates.</summary>
    public IReadOnlyList<DefeatRecoveryReceipt> ApplyDefeatRecovery(IEnumerable<EntityId> partyMembers, int amount)
    {
        ThrowIfDisposed();
        ArgumentNullException.ThrowIfNull(partyMembers);
        if (amount <= 0) throw new D20SessionException("Defeat recovery vitality must be positive.");
        EntityId[] entities = partyMembers.ToArray();
        if (entities.Length is < 1 or > D20Limits.PartyMembers || entities.Distinct().Count() != entities.Length) throw new D20SessionException("Defeat recovery party closure is invalid.");
        ulong nextRevision = checked(Revision + 1);
        var candidates = new Dictionary<EntityId, ExactStatTrackState>(entities.Length);
        var mutations = new Dictionary<EntityId, ExactStatTrackCurrentMutationPreview>(entities.Length);
        foreach (EntityId entity in entities)
        {
            if (!_vitalityTracks.ContainsKey(entity) || Entities.Get(entity, D20ComponentTypes.Participation).Faction != EncounterFaction.Party) throw new D20SessionException("Defeat recovery may target only admitted party participants.");
            ExactStatTrackState candidate = CloneTrack(entity);
            ExactStatTrackCurrentMutationCandidate mutation = candidate.PrepareRestore(new ExactValue(amount), candidate.Revision);
            mutations.Add(entity, mutation.Preview);
            mutation.Publish();
            candidates.Add(entity, candidate);
        }

        EntityBatch batch = new();
        foreach ((EntityId entity, ExactStatTrackState candidate) in candidates)
        {
            EncounterParticipationFact participation = Entities.Get(entity, D20ComponentTypes.Participation);
            bool living = candidate.Read().TrackCurrent.Raw > 0;
            if (participation.Living != living)
            {
                batch.Mutate(world => world.Set(entity, D20ComponentTypes.Participation, participation with { Living = living }, world.GetComponentRevision(entity, D20ComponentTypes.Participation)));
            }
        }
        EntityWorldBatchCandidate prepared = Entities.PrepareBatch(batch, Entities.Revision);
        prepared.Publish();
        foreach ((EntityId entity, ExactStatTrackState candidate) in candidates) _vitalityTracks[entity] = candidate;
        Revision = nextRevision;
        return mutations.Select(entry => new DefeatRecoveryReceipt(entry.Key, entry.Value.Before.TrackCurrent, entry.Value.After.TrackCurrent, ((ExactStatTrackCurrentMutation.Restore)entry.Value.Mutation).AppliedAmount, entry.Value.After.Revision, Revision)).OrderBy(value => value.Entity.Value).ToArray();
    }

    private void RegisterLoadoutOwner(D20Id ownerId, int capacity)
    {
        if (!_ownerEntities.TryGetValue(ownerId, out EntityId owner)) throw new D20SessionException($"Unknown D20 owner {ownerId}.");
        if (capacity < 0) throw new D20SessionException("Inventory capacity cannot be negative.");
        bool hasInventory = Inventory.TryGetInventory(owner, out InventoryState? inventory);
        bool hasEquipment = Inventory.TryGetEquipment(owner, out _);
        if (hasInventory != hasEquipment) throw new D20SessionException($"Owner {ownerId} has a partial Engine loadout registration.");
        if (hasInventory)
        {
            if (inventory is null || !inventory.CapacityLimits.Any(limit => limit.Metric == CapacityMetricId.Parse("d20.carried-items") && limit.Maximum == checked((ulong)capacity))) throw new D20SessionException($"Owner {ownerId} has a mismatched Engine inventory capacity.");
            _ownerCapacities[ownerId] = capacity;
            return;
        }

        CapacityMetricId metric = CapacityMetricId.Parse("d20.carried-items");
        Inventory.RegisterInventory(new InventoryState(owner, [new InventoryCapacityLimit(metric, checked((ulong)capacity))]));
        Inventory.RegisterEquipment(new EquipmentState(owner));
        _ownerCapacities[ownerId] = capacity;
        Revision++;
    }

    private void ValidateAdventureLoadout(AdventureDefinition adventure)
    {
        if (adventure.Party is null || adventure.Characters is null || adventure.Storage is null || adventure.Items is null || adventure.Party.Count > D20Limits.PartyMembers || adventure.Characters.Count > D20Limits.EncounterParticipants) throw new D20SessionException("Adventure owner closure exceeds the admitted bound.");
        EnsureDistinct(adventure.Party, "party"); EnsureDistinct(adventure.Characters, "character owners"); EnsureDistinct(adventure.Storage, "storage owners"); EnsureDistinct(adventure.Items, "items");
        if (!adventure.Party.All(adventure.Characters.Contains) || !adventure.Storage.Contains(adventure.CampStorage) || adventure.Characters.Intersect(adventure.Storage).Any()) throw new D20SessionException("Adventure owner closure is not disjoint and complete.");
        if (_participantCharacters.Count + adventure.Characters.Count(value => !_ownerEntities.ContainsKey(value)) > D20Limits.EncounterParticipants) throw new D20SessionException("Adventure participant admission exceeds the Engine entity bound.");
        foreach (D20Id characterId in adventure.Characters)
        {
            if (!_characters.ContainsKey(characterId)) throw new D20SessionException($"Adventure references unknown character owner {characterId}.");
            if (_ownerEntities.TryGetValue(characterId, out EntityId existing) && (!_ownerKinds.TryGetValue(characterId, out SessionOwnerKind kind) || kind != SessionOwnerKind.Participant || !_participantCharacters.ContainsKey(existing))) throw new D20SessionException($"Adventure character owner {characterId} collides with a non-participant owner.");
        }
        foreach (D20Id storageId in adventure.Storage)
        {
            if (!_storage.TryGetValue(storageId, out StorageDefinition? storage) || storage.Capacity < 0) throw new D20SessionException($"Adventure references invalid storage owner {storageId}.");
            if (_ownerEntities.ContainsKey(storageId)) throw new D20SessionException($"Adventure storage owner {storageId} is already claimed by another owner.");
        }
        var itemCountByOwner = adventure.Items.Select(itemId =>
        {
            if (!_authoredItems.TryGetValue(itemId, out RustyD20.Core.Rules.ItemDefinition? item) || !adventure.Characters.Concat(adventure.Storage).Contains(item.Owner)) throw new D20SessionException($"Adventure item {itemId} has an invalid authored owner.");
            ToEngineItemDefinition(itemId, item, item.Equipment);
            return item.Owner;
        }).GroupBy(owner => owner).ToDictionary(group => group.Key, group => group.Count());
        foreach ((D20Id owner, int count) in itemCountByOwner)
        {
            int capacity = adventure.Characters.Contains(owner) ? Tuning.ParticipantInventoryCapacity : _storage[owner].Capacity;
            if (_ownerEntities.TryGetValue(owner, out EntityId existingOwner)
                && Inventory.TryGetInventory(existingOwner, out _))
            {
                CapacityUsage used = Inventory.View(existingOwner).Capacity
                    .SingleOrDefault(value => value.Metric == CapacityMetricId.Parse("d20.carried-items"));
                if (used.Maximum is not ulong maximum || used.Used > maximum || checked(used.Used + checked((ulong)count)) > maximum)
                {
                    throw new D20SessionException($"Adventure owner {owner} cannot contain its authored item closure.");
                }
            }
            else if (count > capacity)
            {
                throw new D20SessionException($"Adventure owner {owner} cannot contain its authored item closure.");
            }
        }
    }

    private static void EnsureDistinct(IEnumerable<D20Id> values, string label)
    {
        if (values.Distinct().Count() != values.Count()) throw new D20SessionException($"Adventure {label} contain duplicate identities.");
    }

    private Rusty.Engine.Mechanics.ItemDefinition ToEngineItemDefinition(D20Id itemId, RustyD20.Core.Rules.ItemDefinition authored, D20Id equipmentId)
    {
        if (authored.EquipmentKind == EquipmentKind.Armor && !_armors.ContainsKey(equipmentId)) throw new D20SessionException($"Unknown authored armor {equipmentId} for item {itemId}.");
        if (authored.EquipmentKind == EquipmentKind.Implement && !_implements.ContainsKey(equipmentId)) throw new D20SessionException($"Unknown authored implement {equipmentId} for item {itemId}.");
        IReadOnlyList<ItemClassificationId> classifications = authored.EquipmentKind == EquipmentKind.Implement
            ? _implements[equipmentId].Tags.Select(tag => ItemClassificationId.Parse($"d20.{tag.Value}")).ToArray()
            : [];
        return new Rusty.Engine.Mechanics.ItemDefinition(
            ItemDefinitionId.Parse($"d20.item.{itemId.Value}"),
            ItemKind.Unique,
            1,
            classifications,
            [new ItemCapacityCost(CapacityMetricId.Parse("d20.carried-items"), 1)],
            new ItemEquipmentPolicy(1));
    }

    /// <summary>Items are inventory-only Engine identities, so one prepared InventoryWorld candidate is the complete mutation.</summary>
    public EntityId EquipImplement(EntityId owner, ImplementDefinition implement)
    {
        ThrowIfDisposed(); RequireParticipant(owner); ArgumentNullException.ThrowIfNull(implement);
        if (!Inventory.TryGetInventory(owner, out _) || !Inventory.TryGetEquipment(owner, out EquipmentState? equipment) || equipment is null) throw new D20SessionException("Participant must register a loadout owner before equipping.");
        EquipmentSlotId slot = EquipmentSlotId.Parse(implement.Slot.Value); if (equipment.Assignments.Any(assignment => assignment.Slot == slot)) throw new D20SessionException("The canonical Engine equipment slot is occupied.");
        if (_nextInventoryOnlyEntity == ulong.MaxValue) throw new D20SessionException("Inventory-only entity identity is exhausted."); EntityId item = new(_nextInventoryOnlyEntity);
        var definition = new Rusty.Engine.Mechanics.ItemDefinition(ItemDefinitionId.Parse(implement.Id.Value), ItemKind.Unique, 1, equipment: new ItemEquipmentPolicy(1));
        InventoryWorldCandidate candidate = Inventory.Prepare(); candidate.MaterializeUnique(new ItemState(item, definition), owner); candidate.Equip(owner, item, [new EquipmentSlotDefinition(slot)]); candidate.Publish(); _itemEquipment[item] = (EquipmentKind.Implement, implement.Id); _nextInventoryOnlyEntity++; Revision++; return item;
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
        EnsureFresh(preview); ResolvedAction action = RebindAction(preview); ReactionDefinition definition = RequireReaction(reaction, action.DefendedBy);
        ActionResourcesFact resources = Entities.Get(preview.Target, D20ComponentTypes.Resources); if (!TryValue(resources.Values, definition.Resource, out int before) || before < definition.Cost) throw new D20SessionException("Reaction resource is unavailable.");
        ActivationBudgetsFact budgets = Entities.Get(preview.Target, D20ComponentTypes.Budgets); EnsureCosts(budgets.Values, definition.Costs); ulong expires = checked(Turn + (ulong)_effects[definition.Effect].DurationTurns);
        EffectState candidateEffects = CloneEffectState(preview.Target); ApplyOrRefreshEffect(candidateEffects, preview.Target, definition.Effect, preview.Operation); EffectProjectionFact afterProjection = Projection(Entities.Get(preview.Target, D20ComponentTypes.Effects), preview.Target, expires, definition.Effect);
        EntityBatch batch = new EntityBatch().Mutate(world => world.Set(preview.Target, D20ComponentTypes.Resources, new ActionResourcesFact(ReplaceResource(resources.Values, definition.Resource, before - definition.Cost)), world.GetComponentRevision(preview.Target, D20ComponentTypes.Resources))).Mutate(world => world.Set(preview.Target, D20ComponentTypes.Budgets, new ActivationBudgetsFact(SpendCosts(budgets.Values, definition.Costs)), world.GetComponentRevision(preview.Target, D20ComponentTypes.Budgets))).Mutate(world => world.Set(preview.Target, D20ComponentTypes.Effects, afterProjection, world.GetComponentRevision(preview.Target, D20ComponentTypes.Effects)));
        EntityWorldBatchCandidate prepared = Entities.PrepareBatch(batch, Entities.Revision); prepared.Publish(); _effectStates[preview.Target] = candidateEffects; Revision++; return new(reaction, preview.Target, definition.Resource, before, before - definition.Cost, definition.Effect, expires, Revision);
    }

    /// <summary>
    /// Resolves an opaque tactical reaction choice and its stored action in one
    /// detached Engine/session transition. A null reaction declines and applies
    /// the original action unchanged. The caller retains its prompt until this
    /// method returns successfully.
    /// </summary>
    public ReactionResolutionReceipt ResolveReaction(ActionPreview preview, D20Id? reaction)
    {
        EnsureFresh(preview);
        if (reaction is null)
        {
            return new(null, ApplyAction(preview));
        }

        ResolvedAction action = RebindAction(preview);
        ReactionDefinition reactionDefinition = RequireReaction(reaction.Value, action.DefendedBy);

        ActionResourcesFact liveResources = Entities.Get(preview.Target, D20ComponentTypes.Resources);
        if (!TryValue(liveResources.Values, reactionDefinition.Resource, out int beforeResource) || beforeResource < reactionDefinition.Cost)
        {
            throw new D20SessionException("Reaction resource is unavailable.");
        }

        ActivationBudgetsFact liveTargetBudgets = Entities.Get(preview.Target, D20ComponentTypes.Budgets);
        EnsureCosts(liveTargetBudgets.Values, reactionDefinition.Costs);
        ulong expiresAtTurn = checked(Turn + (ulong)_effects[reactionDefinition.Effect].DurationTurns);
        EffectState candidateEffects = CloneEffectState(preview.Target);
        ApplyOrRefreshEffect(candidateEffects, preview.Target, reactionDefinition.Effect, preview.Operation);
        EffectProjectionFact candidateProjection = Projection(
            Entities.Get(preview.Target, D20ComponentTypes.Effects),
            preview.Target,
            expiresAtTurn,
            reactionDefinition.Effect);
        ActionResourcesFact afterResources = new(ReplaceResource(liveResources.Values, reactionDefinition.Resource, beforeResource - reactionDefinition.Cost));
        ActivationBudgetsFact afterReactionBudgets = new(SpendCosts(liveTargetBudgets.Values, reactionDefinition.Costs));
        ActivationBudgetsFact actorBudgets = preview.Actor == preview.Target
            ? afterReactionBudgets
            : Entities.Get(preview.Actor, D20ComponentTypes.Budgets);

        // Rebuild the action preview against the detached reaction facts. This
        // catches an effect that changes target defense or forbids the action
        // without publishing the reaction first.
        ActionPreview freshPreview = PreviewAfterReaction(
            preview,
            actorBudgets,
            afterReactionBudgets,
            candidateEffects,
            candidateProjection);
        ActionDefinition actionDefinition = RequireAction(freshPreview.Action);
        if (RollSource.Position == ulong.MaxValue) throw new D20SessionException("Roll-source position is exhausted.");
        ulong nextPosition = RollSource.Position + 1;
        StaticActionRoll roll = ReadActionRoll(freshPreview.Damage);
        int total = roll.D20 + freshPreview.AbilityModifier;
        bool hit = total >= freshPreview.Defense;
        int damage = hit ? Math.Max(0, checked(roll.Damage.Sum(value => (int)value) + freshPreview.Damage.Bonus)) : 0;
        ActivationBudgetsFact afterActionBudgets = new(SpendCosts(actorBudgets.Values, actionDefinition.Costs));
        D20Id? effect = hit ? actionDefinition.Effect : null;
        if (effect is D20Id actionEffect)
        {
            ApplyOrRefreshEffect(candidateEffects, freshPreview.Target, actionEffect, freshPreview.Operation);
            candidateProjection = Projection(candidateProjection, freshPreview.Target, checked(Turn + (ulong)_effects[actionEffect].DurationTurns), actionEffect);
        }

        ExactStatTrackState? candidateTrack = null;
        if (hit && damage != 0)
        {
            candidateTrack = CloneTrack(freshPreview.Target);
            ExactStatTrackCurrentMutationCandidate vitality = candidateTrack.PrepareSpend(new ExactValue(damage), freshPreview.TargetVitalityRevision);
            vitality.Publish();
        }
        bool targetDies = hit && candidateTrack is not null && candidateTrack.Read().TrackCurrent.Raw == 0;
        EntityBatch batch = new EntityBatch()
            .Mutate(world => world.Set(freshPreview.Target, D20ComponentTypes.Resources, afterResources, world.GetComponentRevision(freshPreview.Target, D20ComponentTypes.Resources)))
            .Mutate(world => world.Set(freshPreview.Target, D20ComponentTypes.Effects, candidateProjection, world.GetComponentRevision(freshPreview.Target, D20ComponentTypes.Effects)));
        if (freshPreview.Actor == freshPreview.Target)
        {
            batch.Mutate(world => world.Set(freshPreview.Actor, D20ComponentTypes.Budgets, afterActionBudgets, world.GetComponentRevision(freshPreview.Actor, D20ComponentTypes.Budgets)));
        }
        else
        {
            batch.Mutate(world => world.Set(freshPreview.Target, D20ComponentTypes.Budgets, afterReactionBudgets, world.GetComponentRevision(freshPreview.Target, D20ComponentTypes.Budgets)))
                .Mutate(world => world.Set(freshPreview.Actor, D20ComponentTypes.Budgets, afterActionBudgets, world.GetComponentRevision(freshPreview.Actor, D20ComponentTypes.Budgets)));
        }
        if (targetDies)
        {
            batch.Mutate(world => world.Set(freshPreview.Target, D20ComponentTypes.Participation, Entities.Get(freshPreview.Target, D20ComponentTypes.Participation) with { Living = false }, world.GetComponentRevision(freshPreview.Target, D20ComponentTypes.Participation)));
        }

        EntityWorldBatchCandidate prepared = Entities.PrepareBatch(batch, Entities.Revision);
        prepared.Publish();
        _effectStates[freshPreview.Target] = candidateEffects;
        if (candidateTrack is not null) _vitalityTracks[freshPreview.Target] = candidateTrack;
        RollSource = RollSource with { Position = nextPosition };
        Revision = checked(Revision + 1);
        ReactionReceipt reactionReceipt = new(reaction.Value, freshPreview.Target, reactionDefinition.Resource, beforeResource, beforeResource - reactionDefinition.Cost, reactionDefinition.Effect, expiresAtTurn, Revision);
        var actionReceipt = new ActionReceipt(freshPreview.Operation, freshPreview.Actor, freshPreview.Target, freshPreview.Action, freshPreview.RollPosition, roll.D20, total, freshPreview.Defense, hit, damage, effect, hit ? actionDefinition.ForcedMovement : 0, Turn, Revision);
        _receipts.Add(actionReceipt);
        if (_receipts.Count > Tuning.MaximumReceiptCount) _receipts.RemoveAt(0);
        return new(reactionReceipt, actionReceipt);
    }

    public ActionReceipt ApplyAction(ActionPreview preview)
    {
        EnsureFresh(preview); ResolvedAction action = RebindAction(preview); if (RollSource.Position == ulong.MaxValue) throw new D20SessionException("Roll-source position is exhausted."); ulong nextPosition = RollSource.Position + 1; StaticActionRoll roll = ReadActionRoll(action.Damage);
        int total = roll.D20 + action.AbilityModifier; bool hit = total >= action.Defense; int damage = hit ? Math.Max(0, checked(roll.Damage.Sum(value => (int)value) + action.Damage.Bonus)) : 0;
        ActivationBudgetsFact afterBudgets = new(SpendCosts(Entities.Get(preview.Actor, D20ComponentTypes.Budgets).Values, action.Definition.Costs)); D20Id? effect = hit ? action.Definition.Effect : null;
        EffectState? candidateEffects = null; EffectProjectionFact effectsAfter = Entities.Get(preview.Target, D20ComponentTypes.Effects); if (effect is D20Id effectId) { candidateEffects = CloneEffectState(preview.Target); ApplyOrRefreshEffect(candidateEffects, preview.Target, effectId, preview.Operation); effectsAfter = Projection(effectsAfter, preview.Target, checked(Turn + (ulong)_effects[effectId].DurationTurns), effectId); }
        ExactStatTrackState? candidateTrack = null; ExactStatTrackCurrentMutationCandidate? vitality = null; if (hit && damage != 0) { candidateTrack = CloneTrack(preview.Target); vitality = candidateTrack.PrepareSpend(new ExactValue(damage), preview.TargetVitalityRevision); vitality.Publish(); }
        bool targetDies = hit && candidateTrack is not null && candidateTrack.Read().TrackCurrent.Raw == 0;
        EntityBatch batch = new EntityBatch().Mutate(world => world.Set(preview.Actor, D20ComponentTypes.Budgets, afterBudgets, world.GetComponentRevision(preview.Actor, D20ComponentTypes.Budgets))).Mutate(world => world.Set(preview.Target, D20ComponentTypes.Effects, effectsAfter, world.GetComponentRevision(preview.Target, D20ComponentTypes.Effects)));
        if (targetDies) batch.Mutate(world => world.Set(preview.Target, D20ComponentTypes.Participation, Entities.Get(preview.Target, D20ComponentTypes.Participation) with { Living = false }, world.GetComponentRevision(preview.Target, D20ComponentTypes.Participation)));
        EntityWorldBatchCandidate prepared = Entities.PrepareBatch(batch, Entities.Revision); prepared.Publish(); if (candidateEffects is not null) _effectStates[preview.Target] = candidateEffects; if (candidateTrack is not null) _vitalityTracks[preview.Target] = candidateTrack; RollSource = RollSource with { Position = nextPosition }; Revision++;
        var receipt = new ActionReceipt(preview.Operation, preview.Actor, preview.Target, preview.Action, preview.RollPosition, roll.D20, total, action.Defense, hit, damage, effect, hit ? action.Definition.ForcedMovement : 0, Turn, Revision); _receipts.Add(receipt); if (_receipts.Count > Tuning.MaximumReceiptCount) _receipts.RemoveAt(0); return receipt;
    }
    public void AdvanceTurn()
    {
        ThrowIfDisposed(); ulong nextTurn = checked(Turn + 1); var mutations = new EntityBatch(); var candidates = new Dictionary<EntityId, EffectState>();
        foreach (EntityComponent<EffectProjectionFact> component in Entities.Query(D20ComponentTypes.Effects)) { EffectState candidate = CloneEffectState(component.Entity); foreach (ScheduledEffectProjection value in component.Value.Values.Where(value => value.ExpiresAtTurn <= nextTurn)) { if (candidate.Effects.Any(effect => effect.Instance == value.Instance)) candidate.Expire(value.Instance); } EffectProjectionFact after = new(component.Value.Values.Where(value => value.ExpiresAtTurn > nextTurn).ToImmutableArray()); if (!after.Equals(component.Value)) mutations.Mutate(world => world.Set(component.Entity, D20ComponentTypes.Effects, after, world.GetComponentRevision(component.Entity, D20ComponentTypes.Effects))); candidates[component.Entity] = candidate; }
        EntityWorldBatchCandidate prepared = Entities.PrepareBatch(mutations, Entities.Revision); prepared.Publish(); foreach ((EntityId entity, EffectState candidate) in candidates) _effectStates[entity] = candidate; Turn = nextTurn; Revision++;
    }
    /// <summary>Closed product save facts; Engine state is reconstructed through normal managed APIs on restore.</summary>
    public D20SessionSave CaptureSave()
    {
        ThrowIfDisposed();
        var participants = _participantCharacters.OrderBy(pair => pair.Key.Value).Select(pair =>
        {
            EntityId entity = pair.Key; ExactStatTrackSnapshot vitality = RequireTrack(entity).Read();
            EncounterParticipationFact participation = Entities.Get(entity, D20ComponentTypes.Participation);
            return new SessionParticipantSave(entity.Value, pair.Value, participation.Faction, participation.Living, checked((int)vitality.TrackCurrent.Raw), checked((int)vitality.TrackBounds.Maximum.Raw), Entities.Get(entity, D20ComponentTypes.Abilities).Values.ToArray(), Entities.Get(entity, D20ComponentTypes.Resources).Values.ToArray(), Entities.Get(entity, D20ComponentTypes.Budgets).Values.ToArray(), Entities.Get(entity, D20ComponentTypes.Effects).Values.ToArray());
        }).ToArray();
        var owners = _ownerEntities.OrderBy(pair => pair.Value.Value).Select(pair => new SessionOwnerSave(pair.Value.Value, pair.Key, _ownerKinds[pair.Key], _ownerCapacities.TryGetValue(pair.Key, out int capacity) ? capacity : Tuning.ParticipantInventoryCapacity, Inventory.TryGetInventory(pair.Value, out _))).ToArray();
        var items = Inventory.ItemEntities.Select(item =>
        {
            if (!Inventory.TryGetItem(item, out ItemState? state) || state is null || !Inventory.TryGetContainer(item, out EntityId owner) || !_ownerIds.TryGetValue(owner, out _)) throw new D20SessionException("Inventory contains an item outside the admitted owner loadout.");
            if (!_itemEquipment.TryGetValue(item, out (EquipmentKind Kind, D20Id Equipment) mapping)) throw new D20SessionException("Inventory contains an item without a stable armor/implement mapping.");
            string[] slots = Inventory.TryGetEquipment(owner, out EquipmentState? equipment) && equipment is not null ? equipment.Assignments.Where(value => value.Item == item).Select(value => value.Slot.Value).OrderBy(value => value, StringComparer.Ordinal).ToArray() : [];
            return new SessionItemSave(item.Value, owner.Value, mapping.Equipment, slots, _itemIds.TryGetValue(item, out D20Id authored) ? authored : null, mapping.Kind, mapping.Equipment);
        }).OrderBy(value => value.Entity).ToArray();
        return new D20SessionSave(RollSource, Turn, Revision, participants, items, _receipts.ToArray(), owners, _inventoryTransfers.ToArray(), _admittedAdventure);
    }
    public static D20Session Restore(CompiledD20Content rules, D20SessionSave save, SessionTuning? tuning = null, ScopedSeededRollAdapter? seededRolls = null)
    {
        ArgumentNullException.ThrowIfNull(rules);
        ArgumentNullException.ThrowIfNull(save);
        SessionTuning policy = tuning ?? new SessionTuning();
        if (save.RollSource is null || save.Participants is null || save.Participants.Count is < 1 or > D20Limits.EncounterParticipants || save.Items is null || save.Items.Count > D20Limits.AdventureEntries * 2 || save.Receipts is null || save.Receipts.Count > policy.MaximumReceiptCount || save.RollSource.Kind is not (RollSourceKind.Static or RollSourceKind.Seeded) || save.RollSource.Kind == RollSourceKind.Static && (save.RollSource.Position > (ulong)save.RollSource.StaticRolls.Length || save.RollSource.StaticRolls.Any(roll => roll.D20 is < 1 or > 20 || roll.Damage.Length > D20Limits.DamageDice || roll.Damage.Any(value => value is 0 or > D20Limits.DamageDieSides))) || save.RollSource.Kind == RollSourceKind.Seeded && !save.RollSource.StaticRolls.IsDefaultOrEmpty) throw new D20SessionException("Saved session facts are impossible.");
        if (save.InventoryTransfers is null) throw new D20SessionException("Saved inventory transfer receipts are missing.");
        D20Session? candidate = null;
        try
        {
            candidate = new D20Session(rules, save.RollSource, policy, seededRolls);
            foreach (SessionParticipantSave entry in save.Participants.OrderBy(value => value.Entity))
            {
                if (!Enum.IsDefined(entry.Faction) || !rules.Characters.TryGetValue(entry.Character, out CharacterDefinition? character) || entry.Entity != candidate.Entities.NextEntityValue || entry.MaximumVitality <= 0 || entry.Vitality < 0 || entry.Vitality > entry.MaximumVitality || entry.Abilities is null || entry.Resources is null || entry.Budgets is null || entry.Effects is null) throw new D20SessionException("Saved participant identity or vitality is invalid.");
                ValidateAbilities(entry.Abilities, character, candidate);
                ValidateResources(entry.Resources, candidate);
                ValidateBudgets(entry.Budgets, candidate);
                EntityId entity = candidate.AddParticipant(character, entry.Faction, entry.MaximumVitality);
                if (!entry.Living && entry.Vitality != 0 || entry.Living && entry.Vitality == 0) throw new D20SessionException("Saved participant life state disagrees with vitality.");
                foreach (ResourceEntry resource in entry.Resources) candidate.SetActionResource(entity, resource.Id, resource.Value);
                foreach (BudgetEntry budget in entry.Budgets) candidate.SetActivationBudget(entity, budget.Id, budget.Value);
                if (entry.Vitality != entry.MaximumVitality) candidate.RequireTrack(entity).Spend(new ExactValue(entry.MaximumVitality - entry.Vitality));
                if (!entry.Living) candidate.Replace(entity, D20ComponentTypes.Participation, value => value with { Living = false });
                var seenEffects = new HashSet<EffectInstanceId>();
                foreach (ScheduledEffectProjection effect in entry.Effects.OrderBy(value => value.Instance.Value, StringComparer.Ordinal))
                {
                    if (!seenEffects.Add(effect.Instance) || effect.ExpiresAtTurn <= save.Turn || !candidate._effects.TryGetValue(effect.Effect, out RustyD20.Core.Rules.EffectDefinition? authoredEffect) || effect.Instance != EffectInstance(entity, effect.Effect) || effect.ExpiresAtTurn > checked(save.Turn + (ulong)authoredEffect.DurationTurns)) throw new D20SessionException("Saved effect projection is invalid.");
                    candidate.ApplyOrRefreshEffect(entity, effect.Effect, OperationId.Parse($"restore-effect-{entity.Value}-{effect.Effect.Value}"));
                }
                candidate.Replace(entity, D20ComponentTypes.Effects, _ => new EffectProjectionFact(entry.Effects.OrderBy(value => value.Instance.Value, StringComparer.Ordinal).ToImmutableArray()));
            }

            SessionOwnerSave[] owners = save.Owners?.ToArray() ?? throw new D20SessionException("Saved owner mappings are missing.");
            if (owners.Length > D20Limits.AdventureEntries * 2 || owners.Select(value => value.Owner).Distinct().Count() != owners.Length || owners.Select(value => value.Entity).Distinct().Count() != owners.Length) throw new D20SessionException("Saved owner mappings are duplicated.");
            foreach (SessionOwnerSave owner in owners.OrderBy(value => value.Entity))
            {
                if (!Enum.IsDefined(owner.Kind) || owner.Capacity < 0 || owner.Capacity > 256 || owner.Entity == 0 && owner.Kind == SessionOwnerKind.Storage) throw new D20SessionException("Saved owner mapping is invalid.");
                if (owner.Kind == SessionOwnerKind.Participant)
                {
                    if (!candidate._ownerEntities.TryGetValue(owner.Owner, out EntityId expected) || expected.Value != owner.Entity) throw new D20SessionException("Saved participant owner mapping is invalid.");
                }
                else
                {
                    if (!candidate._storage.ContainsKey(owner.Owner) || candidate._ownerEntities.ContainsKey(owner.Owner) || owner.Entity < FirstInventoryOnlyEntity) throw new D20SessionException("Saved storage owner mapping is invalid.");
                    EntityId storageEntity = new(owner.Entity); candidate._ownerEntities.Add(owner.Owner, storageEntity); candidate._ownerIds.Add(storageEntity, owner.Owner); candidate._ownerKinds[owner.Owner] = SessionOwnerKind.Storage; candidate._ownerCapacities[owner.Owner] = owner.Capacity; candidate._nextInventoryOnlyEntity = Math.Max(candidate._nextInventoryOnlyEntity, checked(owner.Entity + 1));
                }
                if (owner.HasInventory) candidate.RegisterLoadoutOwner(owner.Owner, owner.Capacity);
            }
            if (owners.Where(value => value.Kind == SessionOwnerKind.Participant).Select(value => value.Owner).ToHashSet().Count != candidate._participantCharacters.Count) throw new D20SessionException("Saved participant owner mappings omit an empty loadout owner.");

            var itemEntities = new HashSet<EntityId>();
            foreach (SessionItemSave savedItem in save.Items.OrderBy(value => value.Entity))
            {
                EntityId owner = new(savedItem.Owner); EntityId entity = new(savedItem.Entity);
                if (!itemEntities.Add(entity) || savedItem.Entity < FirstInventoryOnlyEntity || savedItem.EquippedSlots is null || savedItem.EquippedSlots.Count > 1 || !candidate._ownerIds.ContainsKey(owner) || !candidate.Inventory.TryGetInventory(owner, out _)) throw new D20SessionException("Saved inventory/loadout identity is invalid.");
                Rusty.Engine.Mechanics.ItemDefinition engineDefinition;
                EquipmentKind kind;
                D20Id equipmentId;
                D20Id authoredId = savedItem.Item ?? throw new D20SessionException("Saved session items must belong to the current authored adventure closure.");
                if (!candidate._authoredItems.TryGetValue(authoredId, out RustyD20.Core.Rules.ItemDefinition? authored) || savedItem.Kind != authored.EquipmentKind || savedItem.Equipment is not D20Id savedEquipment || savedEquipment != authored.Equipment || savedItem.Implement != authored.Equipment) throw new D20SessionException("Saved authored item mapping is invalid.");
                kind = authored.EquipmentKind; equipmentId = authored.Equipment; engineDefinition = candidate.ToEngineItemDefinition(authoredId, authored, equipmentId); candidate._itemEntities.Add(authoredId, entity); candidate._itemIds.Add(entity, authoredId);
                candidate._itemEquipment.Add(entity, (kind, equipmentId));
                candidate.Inventory.MaterializeUnique(new ItemState(entity, engineDefinition), owner);
                if (savedItem.EquippedSlots.Count == 1)
                {
                    D20Id expectedSlot = kind == EquipmentKind.Armor ? candidate._armors[equipmentId].Slot : candidate._implements[equipmentId].Slot;
                    string savedSlot = savedItem.EquippedSlots[0];
                    if (savedSlot != expectedSlot.Value && savedSlot != $"d20.initial.{authoredId.Value}") throw new D20SessionException("Saved item equipment slot does not match its authored equipment.");
                    EquipmentService.Equip(candidate.Inventory, owner, entity, [new EquipmentSlotDefinition(EquipmentSlotId.Parse(savedItem.EquippedSlots[0]))]);
                }
                candidate._nextInventoryOnlyEntity = Math.Max(candidate._nextInventoryOnlyEntity, checked(savedItem.Entity + 1));
            }
            if (save.Adventure is D20Id admittedAdventure)
            {
                if (!rules.Adventures.TryGetValue(admittedAdventure, out AdventureDefinition? adventure)) throw new D20SessionException("Saved adventure identity is unknown.");
                var expectedOwners = adventure.Characters.Concat(adventure.Storage).Distinct().ToHashSet();
                var ownerById = owners.ToDictionary(value => value.Owner);
                if (!ownerById.Keys.ToHashSet().SetEquals(expectedOwners) || ownerById.Any(entry => entry.Value.Kind != (adventure.Characters.Contains(entry.Key) ? SessionOwnerKind.Participant : SessionOwnerKind.Storage) || !entry.Value.HasInventory || entry.Value.Capacity != (adventure.Characters.Contains(entry.Key) ? policy.ParticipantInventoryCapacity : candidate._storage[entry.Key].Capacity))) throw new D20SessionException("Saved adventure owner closure is incomplete or inconsistent.");
                if (save.Items.Any(value => value.Item is not D20Id) || save.Items.Select(value => value.Item!.Value).ToHashSet().SetEquals(adventure.Items) is false) throw new D20SessionException("Saved adventure item closure is incomplete or inconsistent.");
                var itemOwners = adventure.Items.ToDictionary(itemId => itemId, itemId => candidate._authoredItems[itemId].Owner);
                foreach (D20InventoryTransferReceipt transfer in save.InventoryTransfers.OrderBy(value => value.SessionRevision))
                {
                    if (!itemOwners.TryGetValue(transfer.Item, out D20Id currentOwner) || currentOwner != transfer.FromOwner) throw new D20SessionException("Saved adventure transfer history has an invalid source owner.");
                    itemOwners[transfer.Item] = transfer.ToOwner;
                }
                foreach (D20Id itemId in adventure.Items)
                {
                    RustyD20.Core.Rules.ItemDefinition authored = candidate._authoredItems[itemId];
                    SessionItemSave savedItem = save.Items.Single(value => value.Item == itemId);
                    if (savedItem.Owner != candidate.OwnerEntity(itemOwners[itemId]).Value || savedItem.EquippedSlots.Count != (authored.Equipped && itemOwners[itemId] == authored.Owner ? 1 : 0)) throw new D20SessionException("Saved adventure item ownership or equipment disagrees with authored content.");
                }
            }
            else if (save.Items.Any(value => value.Item is not null) || owners.Any(value => value.Kind == SessionOwnerKind.Storage)) throw new D20SessionException("Saved non-adventure session contains authored adventure ownership facts.");

            if (save.InventoryTransfers.Any(receipt => !candidate._itemEntities.ContainsKey(receipt.Item) || !candidate._ownerEntities.ContainsKey(receipt.FromOwner) || !candidate._ownerEntities.ContainsKey(receipt.ToOwner) || receipt.FromOwner == receipt.ToOwner || receipt.EngineRevisionAfter <= receipt.EngineRevisionBefore || receipt.SessionRevision > save.Revision)) throw new D20SessionException("Saved inventory transfer receipt is inconsistent.");
            if (save.InventoryTransfers.Zip(save.InventoryTransfers.Skip(1)).Any(pair => pair.First.SessionRevision >= pair.Second.SessionRevision || pair.First.EngineRevisionAfter > pair.Second.EngineRevisionBefore)) throw new D20SessionException("Saved inventory transfer history is not monotonic.");
            if (save.Receipts.Select(receipt => receipt.Operation).Distinct().Count() != save.Receipts.Count || save.Receipts.Zip(save.Receipts.Skip(1)).Any(pair => pair.First.SessionRevision >= pair.Second.SessionRevision) || save.Receipts.Any(receipt => receipt.SessionRevision > save.Revision || !candidate._participantCharacters.ContainsKey(receipt.Actor) || !candidate._participantCharacters.ContainsKey(receipt.Target) || !candidate._actions.ContainsKey(receipt.Action) || receipt.RollPosition >= save.RollSource.Position || receipt.D20 is < 1 or > 20 || receipt.Damage < 0)) throw new D20SessionException("Saved action receipts are inconsistent.");
            candidate._inventoryTransfers.AddRange(save.InventoryTransfers); candidate._receipts.AddRange(save.Receipts); candidate.Turn = save.Turn; candidate.Revision = save.Revision; candidate._admittedAdventure = save.Adventure;
            return candidate;
        }
        catch (ArgumentException)
        {
            candidate?.Dispose();
            throw new D20SessionException("Saved session identity is malformed.");
        }
        catch
        {
            candidate?.Dispose();
            throw;
        }
    }

    private void EnsureFresh(ActionPreview preview) { if (preview.Turn != Turn || preview.RollPosition != RollSource.Position || preview.InventoryRevision != Inventory.Revision || preview.ActorAbilitiesRevision != ComponentRevision(preview.Actor, D20ComponentTypes.Abilities) || preview.ActorBudgetRevision != ComponentRevision(preview.Actor, D20ComponentTypes.Budgets) || preview.ActorEquipmentRevision != EquipmentRevision(preview.Actor) || preview.ActorEffectsRevision != ComponentRevision(preview.Actor, D20ComponentTypes.Effects) || preview.TargetResourcesRevision != ComponentRevision(preview.Target, D20ComponentTypes.Resources) || preview.TargetBudgetRevision != ComponentRevision(preview.Target, D20ComponentTypes.Budgets) || preview.TargetVitalityRevision != RequireTrack(preview.Target).Revision || preview.TargetEffectsRevision != ComponentRevision(preview.Target, D20ComponentTypes.Effects)) throw new D20SessionException("Action preview is stale."); }
    /// <summary>Rebinds all gameplay outcome facts rather than trusting an observed preview projection.</summary>
    private ResolvedAction RebindAction(ActionPreview preview)
    {
        RequireParticipant(preview.Actor); RequireParticipant(preview.Target);
        ActionDefinition definition = RequireAction(preview.Action);
        EncounterParticipationFact actorParticipation = Entities.Get(preview.Actor, D20ComponentTypes.Participation);
        EncounterParticipationFact targetParticipation = Entities.Get(preview.Target, D20ComponentTypes.Participation);
        if (!actorParticipation.Living || !targetParticipation.Living) throw new D20SessionException("Actions require living encounter participants.");
        EnsureTarget(preview.Actor, preview.Target, actorParticipation, targetParticipation, definition.Target);
        EnsureCosts(Entities.Get(preview.Actor, D20ComponentTypes.Budgets).Values, definition.Costs);
        ResolvedAttack attack = ResolveAttack(preview.Actor, definition);
        AbilityScoresFact abilities = Entities.Get(preview.Actor, D20ComponentTypes.Abilities);
        if (!TryValue(abilities.Values, attack.Ability, out int score)) throw new D20SessionException("The actor lacks the authored ability.");
        IReadOnlyList<ScheduledEffectProjection> active = ActiveEffects(preview.Actor);
        int penalty = active.Select(effect => _effects[effect.Effect]).SelectMany(effect => effect.Conditions).Where(clause => clause.Kind == ConditionKind.AttackPenalty).Sum(clause => clause.Amount);
        if (active.Any(effect => _effects[effect.Effect].Conditions.Any(clause => clause.Kind == ConditionKind.ForbidActionTag && clause.Tag is D20Id tag && definition.Tags.Any(actionTag => actionTag == tag)))) throw new D20SessionException("An active scheduled effect forbids this action tag.");
        return new(definition, AbilityModifier(score, Tuning) + penalty, Defense(preview.Target, attack.Defense), attack.Damage, attack.Range, attack.Defense);
    }
    private ReactionDefinition RequireReaction(D20Id reaction, D20Id defended)
    {
        ReactionDefinition definition = _reactions.TryGetValue(reaction, out ReactionDefinition? value) ? value : throw new D20SessionException("Unknown reaction.");
        if (definition.Defense != defended) throw new D20SessionException("Reaction does not defend this action's authored defense.");
        if (!_effects.TryGetValue(definition.Effect, out RustyD20.Core.Rules.EffectDefinition? effect) || effect.Defense != definition.Defense || effect.DefenseBonus != definition.Bonus) throw new D20SessionException("Reaction effect does not match its authored defense bonus.");
        return definition;
    }
    private StaticActionRoll ReadActionRoll(DamageDefinition damage) { StaticActionRoll result = RollSource.Kind switch { RollSourceKind.Static when RollSource.Position < (ulong)RollSource.StaticRolls.Length => RollSource.StaticRolls[checked((int)RollSource.Position)], RollSourceKind.Static => throw new D20SessionException("Static action rolls are exhausted."), RollSourceKind.Seeded => _seededRolls!.Draw(RollSource.Seed, RollSource.Position, damage), _ => throw new D20SessionException("Unknown roll source."), }; result.Validate(damage); return result; }
    private EffectProjectionFact NextEffectProjection(EntityId entity, D20Id effect, ulong expires) => Projection(Entities.Get(entity, D20ComponentTypes.Effects), entity, expires, effect);
    private static EffectProjectionFact Projection(EffectProjectionFact current, EntityId entity, ulong expires, D20Id justApplied)
    {
        EffectInstanceId instance = EffectInstance(entity, justApplied);
        var values = current.Values.Where(value => value.Instance != instance).Append(new ScheduledEffectProjection(instance, justApplied, expires)).OrderBy(value => value.Instance.Value, StringComparer.Ordinal).ToImmutableArray();
        return new EffectProjectionFact(values);
    }
    private EffectState CloneEffectState(EntityId entity)
    {
        var clone = new EffectState(entity);
        foreach (ActiveEffect effect in _effectStates[entity].Effects.OrderBy(value => value.Instance.Value, StringComparer.Ordinal))
        {
            if (effect.Definition.Stacking == EffectStackingPolicy.Refresh && clone.Effects.Any(value => value.Definition.StackingGroup == effect.Definition.StackingGroup)) clone.Refresh(effect.Instance, effect.Provenance, effect.Stacks);
            else if (effect.Definition.Stacking == EffectStackingPolicy.Replace && clone.Effects.Any(value => value.Definition.StackingGroup == effect.Definition.StackingGroup)) clone.Replace(effect.Definition, effect.Instance, effect.Provenance, effect.Stacks);
            else clone.Apply(effect.Definition, effect.Instance, effect.Provenance, effect.Stacks);
        }
        return clone;
    }
    private ExactStatTrackState CloneTrack(EntityId entity)
    {
        ExactStatTrackState source = _vitalityTracks[entity]; ExactStatTrackSnapshot snapshot = source.Read();
        return new ExactStatTrackState(source.StatDefinition, source.Base, source.Sources, source.TrackDefinition, snapshot.TrackCurrent, snapshot.Revision);
    }
    private void ApplyOrRefreshEffect(EntityId entity, D20Id effect, OperationId operation) => ApplyOrRefreshEffect(_effectStates[entity], entity, effect, operation);
    private void ApplyOrRefreshEffect(EffectState state, EntityId entity, D20Id effect, OperationId operation) { EffectInstanceId instance = EffectInstance(entity, effect); var definition = _engineEffects[effect]; var provenance = new RequestSourceIdentity(operation, SourceInstanceId.Parse($"d20.effect.{effect.Value}")); if (state.Effects.Any(value => value.Instance == instance)) state.Refresh(instance, provenance, 1); else state.Apply(definition, instance, provenance, 1); }
    private IReadOnlyList<ScheduledEffectProjection> ActiveEffects(EntityId entity) => ActiveEffects(entity, null, null);
    private IReadOnlyList<ScheduledEffectProjection> ActiveEffects(EntityId entity, EffectState? effectOverride, EffectProjectionFact? projectionOverride)
    {
        EffectState effects = effectOverride ?? _effectStates[entity];
        EffectProjectionFact projection = projectionOverride ?? Entities.Get(entity, D20ComponentTypes.Effects);
        var active = effects.Effects.Select(value => value.Instance).ToHashSet();
        return projection.Values.Where(value => active.Contains(value.Instance) && value.ExpiresAtTurn > Turn).ToArray();
    }
    private ActionPreview PreviewAfterReaction(ActionPreview source, ActivationBudgetsFact actorBudgets, ActivationBudgetsFact targetBudgets, EffectState targetEffects, EffectProjectionFact targetProjection)
    {
        ActionDefinition definition = RequireAction(source.Action);
        EncounterParticipationFact actorParticipation = Entities.Get(source.Actor, D20ComponentTypes.Participation);
        EncounterParticipationFact targetParticipation = Entities.Get(source.Target, D20ComponentTypes.Participation);
        if (!actorParticipation.Living || !targetParticipation.Living) throw new D20SessionException("Actions require living encounter participants.");
        EnsureTarget(source.Actor, source.Target, actorParticipation, targetParticipation, definition.Target);
        EnsureCosts(actorBudgets.Values, definition.Costs);
        ResolvedAttack resolved = ResolveAttack(source.Actor, definition);
        AbilityScoresFact abilities = Entities.Get(source.Actor, D20ComponentTypes.Abilities);
        if (!TryValue(abilities.Values, resolved.Ability, out int score)) throw new D20SessionException("The actor lacks the authored ability.");
        EffectState? actorEffects = source.Actor == source.Target ? targetEffects : null;
        EffectProjectionFact? actorProjection = source.Actor == source.Target ? targetProjection : null;
        IReadOnlyList<ScheduledEffectProjection> active = ActiveEffects(source.Actor, actorEffects, actorProjection);
        int penalty = active.Select(effect => _effects[effect.Effect]).SelectMany(effect => effect.Conditions).Where(clause => clause.Kind == ConditionKind.AttackPenalty).Sum(clause => clause.Amount);
        if (active.Any(effect => _effects[effect.Effect].Conditions.Any(clause => clause.Kind == ConditionKind.ForbidActionTag && clause.Tag is D20Id tag && definition.Tags.Any(actionTag => actionTag == tag)))) throw new D20SessionException("An active scheduled effect forbids this action tag.");
        ulong actorBudgetRevision = checked(ComponentRevision(source.Actor, D20ComponentTypes.Budgets) + (source.Actor == source.Target ? 1UL : 0UL));
        ulong actorEffectRevision = checked(ComponentRevision(source.Actor, D20ComponentTypes.Effects) + (source.Actor == source.Target ? 1UL : 0UL));
        return source with
        {
            AbilityModifier = AbilityModifier(score, Tuning) + penalty,
            Defense = Defense(source.Target, resolved.Defense, targetEffects, targetProjection),
            Damage = resolved.Damage,
            Range = resolved.Range,
            ActorAbilitiesRevision = ComponentRevision(source.Actor, D20ComponentTypes.Abilities),
            ActorBudgetRevision = actorBudgetRevision,
            ActorEquipmentRevision = EquipmentRevision(source.Actor),
            ActorEffectsRevision = actorEffectRevision,
            TargetResourcesRevision = checked(ComponentRevision(source.Target, D20ComponentTypes.Resources) + 1),
            TargetBudgetRevision = checked(ComponentRevision(source.Target, D20ComponentTypes.Budgets) + 1),
            TargetVitalityRevision = RequireTrack(source.Target).Revision,
            TargetEffectsRevision = checked(ComponentRevision(source.Target, D20ComponentTypes.Effects) + 1),
            InventoryRevision = Inventory.Revision,
        };
    }
    private static EffectInstanceId EffectInstance(EntityId entity, D20Id effect) => EffectInstanceId.Parse($"d20.effect.{entity.Value}.{effect.Value}");
    private ResolvedAttack ResolveAttack(EntityId actor, ActionDefinition action) { if (action.Attack.Ability is D20Id ability && action.Attack.Defense is D20Id defense && action.Attack.Damage is DamageDefinition damage) return new(ability, defense, damage, action.Attack.Range); D20Id implementId = action.Attack.Implement ?? throw new D20SessionException("Action lacks a resolved attack."); ImplementDefinition implement = _implements[implementId]; if (!Inventory.TryGetEquipment(actor, out EquipmentState? equipment) || equipment is null || !equipment.Assignments.Any(assignment => _itemEquipment.TryGetValue(assignment.Item, out (EquipmentKind Kind, D20Id Equipment) value) && value.Kind == EquipmentKind.Implement && value.Equipment == implementId)) throw new D20SessionException("The required canonical Engine implement is not equipped."); return new(implement.Ability, implement.Defense, implement.Damage, implement.Range); }
    private int Defense(EntityId target, D20Id defense) => Defense(target, defense, null, null);
    private int Defense(EntityId target, D20Id defense, EffectState? effectOverride, EffectProjectionFact? projectionOverride) { DefenseDefinition definition = _defenses[defense]; AbilityScoresFact abilities = Entities.Get(target, D20ComponentTypes.Abilities); int effects = ActiveEffects(target, effectOverride, projectionOverride).Select(value => _effects[value.Effect]).Where(value => value.Defense == defense).Sum(value => value.DefenseBonus); int armor = 0; if (Inventory.TryGetEquipment(target, out EquipmentState? equipment) && equipment is not null) { armor = equipment.Assignments.Where(assignment => _itemEquipment.TryGetValue(assignment.Item, out (EquipmentKind Kind, D20Id Equipment) value) && value.Kind == EquipmentKind.Armor && _armors.TryGetValue(value.Equipment, out ArmorDefinition? authored) && authored.Defense == defense).Select(assignment => _itemEquipment[assignment.Item].Equipment).Select(id => _armors[id].Bonus).Sum(); } return definition.Base + definition.Abilities.Select(ability => TryValue(abilities.Values, ability, out int score) ? AbilityModifier(score, Tuning) : int.MinValue).Max() + armor + effects; }
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
    private static void ValidateAbilities(IReadOnlyList<AbilityScoreEntry> values, CharacterDefinition character, D20Session candidate)
    {
        if (values.Count != character.Abilities.Count || values.Select(value => value.Id).Distinct().Count() != values.Count || !values.OrderBy(value => value.Id.Value, StringComparer.Ordinal).Select(value => value.Id).SequenceEqual(character.Abilities.Keys.OrderBy(value => value.Value, StringComparer.Ordinal))) throw new D20SessionException("Saved ability facts are not the exact authored character values.");
        foreach (AbilityScoreEntry value in values)
        {
            if (!character.Abilities.TryGetValue(value.Id, out int expected) || expected != value.Value || !candidate._abilities.TryGetValue(value.Id, out AbilityDefinition? definition) || value.Value < definition.Minimum || value.Value > definition.Maximum) throw new D20SessionException("Saved ability facts are outside the authored bounds.");
        }
    }
    private static void ValidateResources(IReadOnlyList<ResourceEntry> values, D20Session candidate)
    {
        if (values.Count != candidate._resources.Count || values.Select(value => value.Id).Distinct().Count() != values.Count || !values.Select(value => value.Id).ToHashSet().SetEquals(candidate._resources.Keys) || values.Any(value => !candidate._resources.TryGetValue(value.Id, out ResourceDefinition? definition) || value.Value < 0 || value.Value > definition.Maximum)) throw new D20SessionException("Saved resource facts are outside the authored bounds or closure.");
    }
    private static void ValidateBudgets(IReadOnlyList<BudgetEntry> values, D20Session candidate)
    {
        if (values.Count != candidate._budgets.Count || values.Select(value => value.Id).Distinct().Count() != values.Count || !values.Select(value => value.Id).ToHashSet().SetEquals(candidate._budgets.Keys) || values.Any(value => !candidate._budgets.TryGetValue(value.Id, out ActivationBudgetDefinition? definition) || value.Value < 0 || value.Value > definition.Initial)) throw new D20SessionException("Saved activation budgets are outside the authored bounds or closure.");
    }
    private AbilityDefinition AbilityDefinition(D20Id id) => _abilities.TryGetValue(id, out AbilityDefinition? definition) ? definition : throw new D20SessionException($"Unknown ability {id}.");
    private ulong ComponentRevision<T>(EntityId entity, ComponentType<T> component) where T : struct => Entities.GetComponentRevision(entity, component).Revision;
    private ulong EquipmentRevision(EntityId entity) => Inventory.TryGetEquipment(entity, out EquipmentState? state) && state is not null ? state.Revision : 0;
    private void Replace<T>(EntityId entity, ComponentType<T> component, Func<T, T> mutate) where T : struct { T current = Entities.Get(entity, component); Entities.Set(entity, component, mutate(current), Entities.GetComponentRevision(entity, component)); Revision++; }
    private void ThrowIfDisposed() { if (_disposed) throw new ObjectDisposedException(nameof(D20Session)); }
    public void Dispose() { if (_disposed) return; Entities.Dispose(); _disposed = true; }
    private sealed record ResolvedAttack(D20Id Ability, D20Id Defense, DamageDefinition Damage, int Range);
    private sealed record ResolvedAction(ActionDefinition Definition, int AbilityModifier, int Defense, DamageDefinition Damage, int Range, D20Id DefendedBy);
}
