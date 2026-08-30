using RustyD20.Core.Contract;

namespace RustyD20.Core.Rules;

/// <summary>One inspectable C# authoring module. Modules are composition input, not runtime files or a generic rules language.</summary>
public sealed record D20ContentModule(
    D20Id Id,
    string ContentSchema,
    IReadOnlyList<D20Id> Dependencies,
    SourceProvenance Source,
    IReadOnlyList<AbilityDefinition>? Abilities = null,
    IReadOnlyList<DefenseDefinition>? Defenses = null,
    IReadOnlyList<ActivationBudgetDefinition>? Budgets = null,
    IReadOnlyList<DamageTypeDefinition>? DamageTypes = null,
    IReadOnlyList<ResourceDefinition>? Resources = null,
    IReadOnlyList<ArmorDefinition>? Armors = null,
    IReadOnlyList<ImplementDefinition>? Implements = null,
    IReadOnlyList<EffectDefinition>? Effects = null,
    IReadOnlyList<ReactionDefinition>? Reactions = null,
    IReadOnlyList<ActionDefinition>? Actions = null,
    IReadOnlyList<FeatureDefinition>? Features = null,
    IReadOnlyList<CharacterDefinition>? Characters = null,
    IReadOnlyList<StorageDefinition>? Storage = null,
    IReadOnlyList<ItemDefinition>? Items = null,
    IReadOnlyList<EncounterDefinition>? Encounters = null,
    IReadOnlyList<AdventureDefinition>? Adventures = null)
{
    public IReadOnlyList<AbilityDefinition> AbilitiesOrEmpty => Abilities ?? [];
    public IReadOnlyList<DefenseDefinition> DefensesOrEmpty => Defenses ?? [];
    public IReadOnlyList<ActivationBudgetDefinition> BudgetsOrEmpty => Budgets ?? [];
    public IReadOnlyList<DamageTypeDefinition> DamageTypesOrEmpty => DamageTypes ?? [];
    public IReadOnlyList<ResourceDefinition> ResourcesOrEmpty => Resources ?? [];
    public IReadOnlyList<ArmorDefinition> ArmorsOrEmpty => Armors ?? [];
    public IReadOnlyList<ImplementDefinition> ImplementsOrEmpty => Implements ?? [];
    public IReadOnlyList<EffectDefinition> EffectsOrEmpty => Effects ?? [];
    public IReadOnlyList<ReactionDefinition> ReactionsOrEmpty => Reactions ?? [];
    public IReadOnlyList<ActionDefinition> ActionsOrEmpty => Actions ?? [];
    public IReadOnlyList<FeatureDefinition> FeaturesOrEmpty => Features ?? [];
    public IReadOnlyList<CharacterDefinition> CharactersOrEmpty => Characters ?? [];
    public IReadOnlyList<StorageDefinition> StorageOrEmpty => Storage ?? [];
    public IReadOnlyList<ItemDefinition> ItemsOrEmpty => Items ?? [];
    public IReadOnlyList<EncounterDefinition> EncountersOrEmpty => Encounters ?? [];
    public IReadOnlyList<AdventureDefinition> AdventuresOrEmpty => Adventures ?? [];
}

public sealed record CompiledD20Content(
    string ContentFingerprint,
    IReadOnlyDictionary<D20Id, AdventureDefinition> Adventures,
    IReadOnlyDictionary<D20Id, CharacterDefinition> Characters,
    IReadOnlyDictionary<D20Id, ItemDefinition> Items,
    D20DefinitionCatalog Catalog,
    IReadOnlyList<D20ContentModule> Modules,
    CompilationReceipt Receipt);

public sealed record CompilationReceipt(string ContentFingerprint, IReadOnlyList<SourceProvenance> Sources, int DefinitionCount, int AdventureCount);

/// <summary>Closed, normalized admitted definitions. Runtime consumers use these tables instead of reopening composition modules.</summary>
public sealed record D20DefinitionCatalog(
    IReadOnlyDictionary<D20Id, AbilityDefinition> Abilities,
    IReadOnlyDictionary<D20Id, DefenseDefinition> Defenses,
    IReadOnlyDictionary<D20Id, ActivationBudgetDefinition> Budgets,
    IReadOnlyDictionary<D20Id, DamageTypeDefinition> DamageTypes,
    IReadOnlyDictionary<D20Id, ResourceDefinition> Resources,
    IReadOnlyDictionary<D20Id, ArmorDefinition> Armors,
    IReadOnlyDictionary<D20Id, ImplementDefinition> Implements,
    IReadOnlyDictionary<D20Id, EffectDefinition> Effects,
    IReadOnlyDictionary<D20Id, ReactionDefinition> Reactions,
    IReadOnlyDictionary<D20Id, ActionDefinition> Actions,
    IReadOnlyDictionary<D20Id, FeatureDefinition> Features,
    IReadOnlyDictionary<D20Id, CharacterDefinition> Characters,
    IReadOnlyDictionary<D20Id, StorageDefinition> Storage,
    IReadOnlyDictionary<D20Id, ItemDefinition> Items,
    IReadOnlyDictionary<D20Id, EncounterDefinition> Encounters,
    IReadOnlyDictionary<D20Id, AdventureDefinition> Adventures);
