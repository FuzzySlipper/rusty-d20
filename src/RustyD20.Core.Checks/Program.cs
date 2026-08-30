using System.Reflection;
using Rusty.Engine;
using RustyD20.Core.Content;
using RustyD20.Core.Contract;
using RustyD20.Core.Rules;
using RustyD20.Core.Session;
using RustyD20.Core.Campaign;
using RustyD20.Core.Checks;
using RustyD20.Core.Persistence;
using RustyD20.Core.Tactical;
using Rusty.Engine.Mechanics;

var checks = new (string Name, Action Run)[]
{
    ("strict id and schema", StrictIdentityAndSchema),
    ("duplicate, quota, and reference admission", DuplicateQuotaAndReference),
    ("topology, placement, dependency, and reachability admission", TopologyPlacementAndDependency),
    ("stable composition fingerprint", StableFingerprint),
    ("immutable normalized content catalog", ImmutableNormalizedCatalog),
    ("named Ember's Wake authored semantics", NamedAdventures),
    ("strict scalar, text, and repeated-identity rejection", StrictScalarTextAndRepeatedIdentity),
    ("retained spatial grid identities fit Engine u32", RetainedGridIds),
    ("persistence cleanup aggregates every owner", PersistenceCleanup),
    ("d20 session floor, static order, and choice", SessionFloorStaticAndChoice),
    ("d20 session action effects and late-failure atomicity", SessionActionAndAtomicity),
    ("d20 session stale reaction and canonical equipment", SessionStaleReactionAndEquipment),
    ("d20 session engine-state admission fences", SessionEngineStateAdmissionFences),
    ("d20 session preview outcome authority", SessionAuthorityProbe.AssertPreviewOutcomeAuthority),
    ("d20 session reaction defense boundary", SessionAuthorityProbe.AssertReactionDefenseBoundary),
    ("d20 session fresh restore and strict save boundary", SessionFreshRestore),
    ("campaign hidden topology and strict current save", CampaignHiddenTopologyAndSave),
    ("tactical initiative, dead skip, opposition reaction", TacticalInitiativeAndReaction),
    ("tactical post-resolution continuation retry", TacticalPostResolutionContinuationRetry),
    ("tactical movement, range, opposition, and outcome authority", TacticalReviewFindings),
    ("retained Engine spatial and adventure loadout admission", RetainedSpatialAndAdventureLoadout),
    ("authored defeat recovery transaction", DefeatRecoveryTransaction),
};

foreach (var check in checks)
{
    check.Run();
    Console.WriteLine($"passed: {check.Name}");
}

return;

static void StrictIdentityAndSchema()
{
    Assert(!D20Id.TryParse("Uppercase", out _, out _), "uppercase identifiers must reject");
    Assert(!D20Id.TryParse("", out _, out _), "empty identifiers must reject");
    Assert(D20Id.TryParse("warden.sigil_1", out var id, out _), "stable lowercase id must admit");
    Assert(id.ToString() == "warden.sigil_1", "id must retain exact identity");
    Expect("D20_UNSUPPORTED_CONTENT_SCHEMA", () => new D20SemanticCompiler().Compile([new D20ContentModule(D20Id.Parse("wrong-schema"), "rusty-d20.candidate/6", [], Source("wrong-schema"))]));
}

static void DuplicateQuotaAndReference()
{
    var source = Source("bad-content");
    var duplicate = new D20ContentModule(D20Id.Parse("duplicate-content"), D20Schemas.Content, [], source,
        Abilities: [new(D20Id.Parse("might"), 1, 20, source), new(D20Id.Parse("might"), 1, 20, source)]);
    Expect("D20_DUPLICATE_ID", () => new D20SemanticCompiler().Compile([duplicate]));

    var tooMany = new D20ContentModule(D20Id.Parse("quota-content"), D20Schemas.Content, [], source,
        Abilities: Enumerable.Range(0, D20Limits.DefinitionsPerKind + 1).Select(index => new AbilityDefinition(D20Id.Parse($"ability-{index}"), 1, 20, source)).ToArray());
    Expect("D20_DEFINITION_QUOTA", () => new D20SemanticCompiler().Compile([tooMany]));

    var dangling = new D20ContentModule(D20Id.Parse("reference-content"), D20Schemas.Content, [], source,
        Armors: [new(D20Id.Parse("ghost-armor"), D20Id.Parse("unknown-defense"), 1, D20Id.Parse("body"), source)]);
    Expect("D20_UNKNOWN_DEFENSE", () => new D20SemanticCompiler().Compile([dangling]));
}

static void TopologyPlacementAndDependency()
{
    var source = Source("topology-content");
    var module = new D20ContentModule(D20Id.Parse("topology-content"), D20Schemas.Content, [D20Id.Parse("missing-dependency")], source,
        Adventures:
        [
            new(D20Id.Parse("broken-adventure"), "Broken", true, true, [], [], D20Id.Parse("missing-storage"), [], [], [], new DungeonDefinition("Broken", D20Id.Parse("stone"), 3, 3, ["###", "#.#", "##x"], new(1, 1), D20Id.Parse("missing-checkpoint"), DungeonFacing.East, [], [], [], [], []), new AdventureOutcome("test", "won", "won", "lost", "lost", []), source),
        ]);
    ExpectAny(["D20_UNKNOWN_MODULE_DEPENDENCY", "D20_INVALID_DUNGEON_TOPOLOGY", "D20_UNKNOWN_STORAGE"], () => new D20SemanticCompiler().Compile([module]));

    var cycleA = new D20ContentModule(D20Id.Parse("cycle-a"), D20Schemas.Content, [D20Id.Parse("cycle-b")], Source("cycle-a"));
    var cycleB = new D20ContentModule(D20Id.Parse("cycle-b"), D20Schemas.Content, [D20Id.Parse("cycle-a")], Source("cycle-b"));
    Expect("D20_MODULE_DEPENDENCY_CYCLE", () => new D20SemanticCompiler().Compile([cycleA, cycleB]));

    var catalog = D20ContentCatalog.Modules.ToArray();
    var wardenIndex = Array.FindIndex(catalog, module => module.Id == D20Id.Parse("wardens-gate-adventure"));
    var warden = catalog[wardenIndex];
    var adventure = warden.AdventuresOrEmpty.Single();
    var isolatedRows = adventure.Dungeon.Rows.Select((row, index) => index is 1 or 5 ? "#.#######.#" : row).ToArray();
    catalog[wardenIndex] = warden with { Adventures = [adventure with { Dungeon = adventure.Dungeon with { Rows = isolatedRows } }] };
    Expect("D20_UNREACHABLE_ENCOUNTER_PLACEMENT", () => new D20SemanticCompiler().Compile(catalog));
}

static void StableFingerprint()
{
    var modules = D20ContentCatalog.Modules;
    var first = D20SemanticCompiler.Fingerprint(modules);
    var second = D20SemanticCompiler.Fingerprint(modules.Reverse());
    Assert(first == second, "helper/module ordering must not alter a content fingerprint");

    var steel = modules.Single(module => module.Id == D20Id.Parse("steel-guard-content"));
    var mutatedEffect = steel.EffectsOrEmpty.Select(effect => effect.Id == D20Id.Parse("bleeding") ? effect with { DurationTurns = effect.DurationTurns + 1 } : effect).ToArray();
    var mutatedModules = modules.Select(module => module.Id == steel.Id ? module with { Effects = mutatedEffect } : module).ToArray();
    Assert(first != D20SemanticCompiler.Fingerprint(mutatedModules), "a gameplay-semantic effect mutation must change the fingerprint");

    var reorderedEffects = modules.Select(module => module.Id == steel.Id ? module with { Effects = steel.EffectsOrEmpty.Reverse().ToArray() } : module).ToArray();
    Assert(first == D20SemanticCompiler.Fingerprint(reorderedEffects), "unordered definition helper ordering must be canonical");
}

static void NamedAdventures()
{
    var compiled = D20ContentCatalog.Compile();
    Assert(compiled.Adventures.ContainsKey(D20Id.Parse("wardens-gate")), "Warden's Gate must compile");
    Assert(compiled.Adventures.ContainsKey(D20Id.Parse("embers-wake")), "Ember's Wake must compile");
    Assert(compiled.Adventures[D20Id.Parse("wardens-gate")].Party.Count == 4, "Warden's Gate preserves its four-person party");
    Assert(compiled.Adventures[D20Id.Parse("embers-wake")].Party.Count == 1, "Ember's Wake preserves Sera's solo party");
    Assert(compiled.ContentFingerprint.Length == 64, "receipt exposes a SHA-256 content fingerprint");

    var emberWard = compiled.Catalog.Effects[Id("ember-ward")];
    var mindwardCharm = compiled.Catalog.Armors[Id("mindward-charm")];
    var wardFlare = compiled.Catalog.Reactions[Id("ward-flare")];
    Assert(mindwardCharm.Defense == Id("nerve") && mindwardCharm.Bonus == 1 && mindwardCharm.Slot == Id("neck"), "mindward charm preserves Nerve +1 neck semantics");
    Assert(emberWard.Defense == Id("nerve") && emberWard.DefenseBonus == 3 && emberWard.DurationTurns == 1, "ember ward is the Nerve +3 defensive effect");
    Assert(wardFlare.Bonus == 3 && wardFlare.Effect == Id("ember-ward"), "ward flare applies ember ward with its authored +3");

    var fireBolt = compiled.Catalog.Actions[Id("fire-bolt")];
    var mindSpike = compiled.Catalog.Actions[Id("mind-spike")];
    Assert(fireBolt.Attack.Ability == Id("acuity") && fireBolt.Attack.Defense == Id("wits") && fireBolt.Attack.Damage == new DamageDefinition(Id("energy"), 2, 6, 0), "fire bolt preserves Acuity versus Wits energy 2d6");
    Assert(mindSpike.Attack.Ability == Id("conviction") && mindSpike.Attack.Defense == Id("nerve") && mindSpike.Attack.Damage == new DamageDefinition(Id("resolve"), 1, 8, 1), "mind spike preserves Conviction versus Nerve resolve 1d8+1");

    var sera = compiled.Characters[Id("sera-vale")];
    var seer = compiled.Characters[Id("ash-seer")];
    Assert(sera.Experience == 840 && sera.Vitality == 22 && sera.Abilities[Id("acuity")] == 18 && sera.Abilities[Id("conviction")] == 18 && sera.ResourcesOrEmpty[Id("focus")] == 3 && sera.AffinitiesOrEmpty.Single() == new DamageAffinity(Id("energy"), DamageAffinityKind.Resistant) && sera.Features.SequenceEqual([Id("arcane-composure"), Id("ember-attunement")]), "Sera preserves authored stats, XP, vitality, resources, energy affinity, and features");
    Assert(seer.Vitality == 22 && seer.Abilities[Id("acuity")] == 16 && seer.Abilities[Id("conviction")] == 16 && seer.ResourcesOrEmpty[Id("resolve-points")] == 2 && seer.AffinitiesOrEmpty.Single() == new DamageAffinity(Id("resolve"), DamageAffinityKind.Resistant) && seer.Features.SequenceEqual([Id("reliquary-sense")]), "Ash Seer preserves authored stats, vitality, resources, resolve affinity, and feature");

    var ashSeer = compiled.Catalog.Encounters[Id("ash-seer")];
    Assert(ashSeer.Summary == "Break the psychic ward around the ember reliquary." && ashSeer.Board.Rows.SequenceEqual(["##########", "#........#", "#..#.....#", "#........#", "#.....#..#", "#........#", "##########"]) && ashSeer.Board.Placements.SequenceEqual([new TacticalPlacement(Id("sera-vale"), new(1, 3)), new TacticalPlacement(Id("ash-seer"), new(8, 3))]) && ashSeer.Victory.RewardItem == Id("seer-charm") && ashSeer.Defeat.RecoveryVitality == 11, "Ash Seer encounter preserves authored board, placements, reward, and recovery facts");
}

static void ImmutableNormalizedCatalog()
{
    var modules = D20ContentCatalog.Modules.ToArray();
    var steelIndex = Array.FindIndex(modules, module => module.Id == Id("steel-guard-content"));
    var steel = modules[steelIndex];
    var action = steel.ActionsOrEmpty.Single(value => value.Id == Id("disrupt"));
    var callerOwnedTags = action.Tags.ToList();
    modules[steelIndex] = steel with { Actions = steel.ActionsOrEmpty.Select(value => value.Id == action.Id ? value with { Tags = callerOwnedTags } : value).ToArray() };
    var compiled = new D20SemanticCompiler().Compile(modules);
    var fingerprint = compiled.ContentFingerprint;
    callerOwnedTags.Clear();

    Assert(compiled.Catalog.Actions[Id("disrupt")].Tags.SequenceEqual([Id("martial")]), "compiled definitions do not retain caller-owned nested lists");
    Assert(D20SemanticCompiler.Fingerprint(compiled.Modules) == fingerprint && compiled.ContentFingerprint == compiled.Receipt.ContentFingerprint, "compiled fingerprint remains bound to the admitted immutable snapshot");
    Assert(compiled.Modules is not D20ContentModule[], "compiled modules do not expose a mutable array");
    ExpectUnsupported(() => ((IList<D20Id>)compiled.Catalog.Actions[Id("disrupt")].Tags).Add(Id("injected")));
    Assert(compiled.Catalog.Abilities.Count != 0 && compiled.Catalog.Defenses.Count != 0 && compiled.Catalog.Budgets.Count != 0 && compiled.Catalog.DamageTypes.Count != 0 && compiled.Catalog.Resources.Count != 0 && compiled.Catalog.Armors.Count != 0 && compiled.Catalog.Implements.Count != 0 && compiled.Catalog.Effects.Count != 0 && compiled.Catalog.Reactions.Count != 0 && compiled.Catalog.Actions.Count != 0 && compiled.Catalog.Features.Count != 0 && compiled.Catalog.Characters.Count != 0 && compiled.Catalog.Storage.Count != 0 && compiled.Catalog.Items.Count != 0 && compiled.Catalog.Encounters.Count != 0 && compiled.Catalog.Adventures.Count != 0, "closed normalized catalog publishes every admitted definition kind");
}

static void StrictScalarTextAndRepeatedIdentity()
{
    var modules = D20ContentCatalog.Modules.ToArray();
    var steel = modules.Single(module => module.Id == Id("steel-guard-content"));
    var negativeRange = steel with { Implements = steel.ImplementsOrEmpty.Select(value => value.Id == Id("training-blade") ? value with { Range = -1 } : value).ToArray() };
    Expect("D20_TACTICAL_RANGE", () => new D20SemanticCompiler().Compile(modules.Select(module => module.Id == steel.Id ? negativeRange : module)));

    var warden = modules.Single(module => module.Id == Id("wardens-gate-adventure"));
    var oversizedText = warden with { Features = warden.FeaturesOrEmpty.Select(value => value with { Description = new string('é', 257) }).ToArray() };
    Expect("D20_AUTHORED_TEXT_LIMIT", () => new D20SemanticCompiler().Compile(modules.Select(module => module.Id == warden.Id ? oversizedText : module)));

    var repeatedTags = steel with { Actions = steel.ActionsOrEmpty.Select(value => value.Id == Id("disrupt") ? value with { Tags = [Id("martial"), Id("martial")] } : value).ToArray() };
    Expect("D20_DUPLICATE_ACTION_TAG", () => new D20SemanticCompiler().Compile(modules.Select(module => module.Id == steel.Id ? repeatedTags : module)));

    var adventure = warden.AdventuresOrEmpty.Single();
    var repeatedParty = warden with { Adventures = [adventure with { Party = [Id("mara-venn"), Id("mara-venn")] }] };
    Expect("D20_DUPLICATE_PARTY_MEMBER", () => new D20SemanticCompiler().Compile(modules.Select(module => module.Id == warden.Id ? repeatedParty : module)));
}

static void RetainedGridIds()
{
    var content = D20ContentCatalog.Compile();
    MethodInfo dungeonIdentity = typeof(EngineCampaignSpatialGateway).GetMethod("StableGridId", BindingFlags.NonPublic | BindingFlags.Static, [typeof(DungeonDefinition)]) ?? throw new InvalidOperationException("Dungeon grid identity helper is missing.");
    MethodInfo tacticalIdentity = typeof(EngineCampaignSpatialGateway).GetMethod("StableGridId", BindingFlags.NonPublic | BindingFlags.Static, [typeof(TacticalBoard)]) ?? throw new InvalidOperationException("Tactical grid identity helper is missing.");
    foreach (AdventureDefinition adventure in content.Adventures.Values)
    {
        uint dungeon = (uint)(dungeonIdentity.Invoke(null, [adventure.Dungeon]) ?? 0U);
        Assert(dungeon != 0, "retained dungeon grid identity must be a nonzero Engine u32.");
        foreach (EncounterDefinition encounter in content.Catalog.Encounters.Values)
        {
            uint tactical = (uint)(tacticalIdentity.Invoke(null, [encounter.Board]) ?? 0U);
            Assert(tactical != 0, "retained tactical grid identity must be a nonzero Engine u32.");
        }
    }
}

static void PersistenceCleanup()
{
    var restoredCampaign = new RecordingOwner("restored-campaign", throws: false);
    var restoredSession = new RecordingOwner("restored-session", throws: false);
    try
    {
        throw D20PersistenceDisposal.DisposeAfterFailure(new CampaignException("injected restore failure"), restoredCampaign, restoredSession);
    }
    catch (CampaignException error)
    {
        Assert(error.Message == "injected restore failure", "successful fresh-candidate cleanup preserves the original CampaignException");
    }

    Assert(restoredCampaign.Disposed && restoredSession.Disposed, "successful fresh-candidate cleanup still attempts campaign and session");

    var campaign = new RecordingOwner("campaign", throws: true);
    var session = new RecordingOwner("session", throws: true);
    try
    {
        D20PersistenceDisposal.DisposeAll(campaign, session);
        throw new InvalidOperationException("A failing cleanup must be reported after every owner is attempted.");
    }
    catch (AggregateException error)
    {
        Assert(error.InnerExceptions.Count == 2, "both failing candidate owners are retained in the aggregate");
    }

    Assert(campaign.Disposed && session.Disposed, "persistence cleanup attempts campaign and session even after either throws");
}

static void SessionFloorStaticAndChoice()
{
    Assert(D20Session.AbilityModifier(9) == -1 && D20Session.AbilityModifier(8) == -1 && D20Session.AbilityModifier(7) == -2, "ability modifiers use mathematical floor division");
    using var session = NewSession([new(20, [4]), new(1, [1])], out var actor, out var target);
    session.SetActivationBudget(actor, Id("bonus-action"), 3);
    var before = session.RollSource.Position;
    Assert(session.ChoiceIndex(0, 1) == 0 && session.RollSource.Position == before, "choice index does not consume an action roll");
    var first = session.ApplyAction(session.PreviewAction(actor, target, Id("disrupt"), OperationId.Parse("check-static-one")));
    Assert(first.Hit && first.Damage == 4 && session.RollSource.Position == 1, "static tape consumes one authored action draw");
    var second = session.ApplyAction(session.PreviewAction(actor, target, Id("disrupt"), OperationId.Parse("check-static-two")));
    Assert(!second.Hit && session.RollSource.Position == 2, "static tape preserves authored hit/miss order");
    var exhausted = session.PreviewAction(actor, target, Id("disrupt"), OperationId.Parse("check-static-three"));
    ExpectSession(() => session.ApplyAction(exhausted), "exhausted");
    Assert(session.RollSource.Position == 2 && session.Receipts.Count == 2, "static tape exhaustion is atomic");
}

static void SessionActionAndAtomicity()
{
    using var session = NewSession([new(20, [4])], out var actor, out var target);
    var preview = session.PreviewAction(actor, target, Id("disrupt"), OperationId.Parse("check-effect"));
    var receipt = session.ApplyAction(preview);
    Assert(receipt.Hit && receipt.Effect == Id("unsettled"), "hit commits authored damage/effect intent");

    using var failed = NewSession([new(20, [4])], out var failingActor, out var failingTarget);
    var failingPreview = failed.PreviewAction(failingActor, failingTarget, Id("disrupt"), OperationId.Parse("check-failed-late"));
    failed.SetActivationBudget(failingActor, Id("bonus-action"), 0);
    var revision = failed.Revision;
    var position = failed.RollSource.Position;
    ExpectSession(() => failed.ApplyAction(failingPreview), "stale");
    Assert(failed.Revision == revision && failed.RollSource.Position == position && failed.Receipts.Count == 0, "failed validation leaves receipts, revisions, and roll position unchanged");
}

static void SessionStaleReactionAndEquipment()
{
    using var session = NewSession([new(20, [4])], out var actor, out var target);
    session.SetActivationBudget(actor, Id("standard-action"), 1);
    session.RegisterLoadoutOwner(actor);
    var content = D20ContentCatalog.Compile();
    var blade = content.Catalog.Implements[Id("training-blade")];
    session.EquipImplement(actor, blade);
    session.SetActionResource(target, Id("guard"), 2);
    session.SetActivationBudget(target, Id("reaction"), 1);
    var preview = session.PreviewAction(actor, target, Id("longsword-strike"), OperationId.Parse("check-reaction"));
    var reaction = session.ApplyReaction(preview, Id("parry"));
    Assert(reaction.After == 1 && session.RollSource.Position == 0, "reaction atomically spends resource/budget without consuming action rolls");
    ExpectSession(() => session.ApplyAction(preview), "stale");
    session.AdvanceTurn();
    Assert(session.Entities.Get(target, D20ComponentTypes.Effects).Values.Length == 0, "turn advance expires scheduled effects");

    using var equipment = NewSession([new(20, [8])], out var equipmentActor, out var equipmentTarget);
    equipment.SetActivationBudget(equipmentActor, Id("standard-action"), 1);
    equipment.RegisterLoadoutOwner(equipmentActor);
    equipment.RegisterLoadoutOwner(equipmentTarget);
    ExpectSession(() => equipment.PreviewAction(equipmentActor, equipmentTarget, Id("longsword-strike"), OperationId.Parse("check-no-implement")), "canonical Engine implement");
    var equippedBlade = equipment.EquipImplement(equipmentActor, blade);
    var armed = equipment.PreviewAction(equipmentActor, equipmentTarget, Id("longsword-strike"), OperationId.Parse("check-implement"));
    Assert(armed.Damage.Sides == 8 && armed.Range == 1, "implement-bound action derives its damage and range from the canonical equipped Engine item");
    var bow = content.Catalog.Implements[Id("field-bow")];
    var transferRevision = equipment.Inventory.Revision;
    ExpectSession(() => equipment.TransferImplementLoadout(equippedBlade, equipmentActor, equipmentTarget, bow), "does not match");
    Assert(equipment.Inventory.Revision == transferRevision, "mismatched authored implement leaves canonical inventory unchanged");
    equipment.TransferImplementLoadout(equippedBlade, equipmentActor, equipmentTarget, blade);
    Assert(equipment.Inventory.TryGetEquipment(equipmentTarget, out var targetLoadout) && targetLoadout!.ContainsItem(equippedBlade), "loadout transfer uses canonical Engine containment and equipment state");
}

static void SessionEngineStateAdmissionFences()
{
    var content = D20ContentCatalog.Compile();
    var character = content.Characters[Id("mara-venn")];
    using var session = new D20Session(content, RollSourceState.Static([]));
    var entityCount = session.Entities.CaptureEntities().Count;
    ExpectSession(() => session.AddParticipant(character, EncounterFaction.Party, 0), "vitality");
    Assert(session.Entities.CaptureEntities().Count == entityCount, "invalid vitality leaves EntityWorld unchanged");
    var actor = session.AddParticipant(character, EncounterFaction.Party);
    var immutableFact = session.Entities.Get(actor, D20ComponentTypes.Abilities);
    session.SetActionResource(actor, Id("guard"), 1);
    Assert(immutableFact.Values.Length == character.Abilities.Count && immutableFact.Values.All(value => character.Abilities[value.Id] == value.Value), "component facts keep immutable copied values rather than caller aliases");
    var blade = content.Catalog.Implements[Id("training-blade")];
    var inventoryRevision = session.Inventory.Revision;
    ExpectSession(() => session.EquipImplement(actor, blade), "register a loadout");
    Assert(session.Inventory.Revision == inventoryRevision, "unregistered owner leaves inventory unchanged");
    session.RegisterLoadoutOwner(actor);
    session.EquipImplement(actor, blade);
    inventoryRevision = session.Inventory.Revision;
    ExpectSession(() => session.EquipImplement(actor, blade), "occupied");
    Assert(session.Inventory.Revision == inventoryRevision, "occupied slot leaves canonical inventory unchanged");

    using var repeated = NewSession([new(20, [4]), new(20, [4])], out var repeatedActor, out var repeatedTarget);
    var first = repeated.ApplyAction(repeated.PreviewAction(repeatedActor, repeatedTarget, Id("disrupt"), OperationId.Parse("check-refresh-one")));
    repeated.SetActivationBudget(repeatedActor, Id("bonus-action"), 1);
    var second = repeated.ApplyAction(repeated.PreviewAction(repeatedActor, repeatedTarget, Id("disrupt"), OperationId.Parse("check-refresh-two")));
    Assert(first.Effect == Id("unsettled") && second.Effect == Id("unsettled") && repeated.Entities.Get(repeatedTarget, D20ComponentTypes.Effects).Values.Length == 1, "repeated D20 effect refreshes one Engine EffectState instance");

    using var overflow = NewSession([new(20, [4])], out var overflowActor, out var overflowTarget, ulong.MaxValue);
    var overflowPreview = overflow.PreviewAction(overflowActor, overflowTarget, Id("disrupt"), OperationId.Parse("check-roll-overflow"));
    var overflowRevision = overflow.Revision;
    ExpectSession(() => overflow.ApplyAction(overflowPreview), "position");
    Assert(overflow.RollSource.Position == ulong.MaxValue && overflow.Revision == overflowRevision, "roll-position overflow rejects before any mutation");
}

static void CampaignHiddenTopologyAndSave()
{
    var content = D20ContentCatalog.Compile();
    var spatial = new CheckSpatial();
    var campaign = new D20CampaignRuntime(content, Id("wardens-gate"), spatial);
    campaign.BeginExploration();
    var snapshot = campaign.Snapshot();
    Assert(snapshot.Exploration!.View.Count == 3, "exploration readout fixes the visible depth at three");
    Assert(snapshot.Exploration.View.Skip(1).All(value => value.FrontBlocked && value.LeftBlocked && value.RightBlocked), "occluded depths are neutral all-wall records");
    var saved = campaign.EncodeSave();
    var restored = D20CampaignRuntime.Restore(saved, content, _ => new CheckSpatial());
    Assert(restored.Snapshot().Phase == CampaignPhase.Exploration, "current strict save restores into a fresh campaign candidate");
    ExpectCampaign(() => D20CampaignRuntime.Restore(saved.Replace(content.ContentFingerprint, new string('0', 64), StringComparison.Ordinal), content, _ => new CheckSpatial()), "fingerprint");
    ExpectCampaign(() => D20CampaignRuntime.Restore("{\"schema\":0}", content, _ => new CheckSpatial()), "schema");
}

static void SessionFreshRestore()
{
    using var session = NewSession([new(20, [4])], out var actor, out var target);
    session.SetActionResource(actor, Id("guard"), 2);
    session.SetActivationBudget(actor, Id("bonus-action"), 1);
    session.ApplyAction(session.PreviewAction(actor, target, Id("disrupt"), OperationId.Parse("save-fresh")));
    var save = session.CaptureSave();
    using var restored = D20Session.Restore(D20ContentCatalog.Compile(), save);
    Assert(restored.CaptureSave().Participants.Count == save.Participants.Count && restored.RollSource.Position == save.RollSource.Position, "session restore builds a fresh Engine-backed candidate with roll position and effects");
    var malformed = save with { Participants = [save.Participants[0] with { Entity = 99 }] };
    ExpectSession(() => D20Session.Restore(D20ContentCatalog.Compile(), malformed), "identity");
}

static void TacticalInitiativeAndReaction()
{
    using var session = NewSession([new(20, [4]), new(20, [4])], out var party, out var opposition);
    session.SetActivationBudget(opposition, Id("bonus-action"), 1);
    var encounter = new TacticalEncounter(session, new CheckTacticalSpatial(), [
        new(Id("mara-venn"), party, 10, new(1, 1)), new(Id("gate-skirmisher"), opposition, 10, new(2, 1))]);
    Assert(encounter.CurrentActor == Id("mara-venn"), "initiative ties use stable Engine entity identity ordering");
    encounter.PartyAction(Id("mara-venn"), Id("gate-skirmisher"), Id("disrupt"), OperationId.Parse("tactical-session"));
    if (encounter.PendingReaction is not null) encounter.ResolveReaction(Id("parry"), false);
    Assert(session.ReadVitality(opposition).Current.Raw < 24, "tactical action delegates damage to canonical D20 session tracks");

    using var stale = NewSession([new(20, [4])], out var staleActor, out var staleTarget);
    stale.RegisterLoadoutOwner(staleActor);
    stale.EquipImplement(staleActor, D20ContentCatalog.Compile().Catalog.Implements[Id("training-blade")]);
    var staleEncounter = new TacticalEncounter(stale, new CheckTacticalSpatial(), [new(Id("mara-venn"), staleActor, 10, new(1, 1)), new(Id("gate-skirmisher"), staleTarget, 10, new(2, 1))]);
    staleEncounter.PartyAction(Id("mara-venn"), Id("gate-skirmisher"), Id("longsword-strike"), OperationId.Parse("stale-reaction-custody"));
    Assert(staleEncounter.PendingReaction is not null, "tactical reaction custody retains the original action");
    stale.SetActivationBudget(staleActor, Id("standard-action"), 0);
    ExpectSession(() => staleEncounter.ResolveReaction(Id("parry"), true), "stale");
    Assert(staleEncounter.PendingReaction is not null && stale.RollSource.Position == 0, "a failed combined reaction leaves prompt custody and session roll position unchanged");

    using var combined = NewSession([new(20, [4]), new(1, [1])], out var combinedActor, out var combinedTarget);
    combined.RegisterLoadoutOwner(combinedActor);
    combined.EquipImplement(combinedActor, D20ContentCatalog.Compile().Catalog.Implements[Id("training-blade")]);
    combined.SetActionResource(combinedActor, Id("guard"), 0);
    combined.SetActivationBudget(combinedActor, Id("reaction"), 0);
    var combinedEncounter = new TacticalEncounter(combined, new CheckTacticalSpatial(), [new(Id("mara-venn"), combinedActor, 10, new(1, 1)), new(Id("gate-skirmisher"), combinedTarget, 10, new(2, 1))]);
    combinedEncounter.PartyAction(Id("mara-venn"), Id("gate-skirmisher"), Id("longsword-strike"), OperationId.Parse("combined-reaction-action"));
    combinedEncounter.ResolveReaction(Id("parry"), true);
    Assert(combinedEncounter.PendingReaction is null && combined.RollSource.Position >= 1 && combined.Receipts.Count >= 1 && combined.Inventory.Revision > 0, "accepted reaction commits the fresh action before deterministic opposition settlement");
}

static void TacticalPostResolutionContinuationRetry()
{
    using var session = NewSession([new(20, [4]), new(1, [1])], out var party, out var opposition);
    session.RegisterLoadoutOwner(party);
    session.EquipImplement(party, D20ContentCatalog.Compile().Catalog.Implements[Id("training-blade")]);
    session.SetActionResource(party, Id("guard"), 0);
    session.SetActivationBudget(party, Id("reaction"), 0);
    session.SetActivationBudget(opposition, Id("bonus-action"), 1);
    var spatial = new FailingTacticalSpatial();
    var encounter = new TacticalEncounter(session, spatial, [
        new(Id("mara-venn"), party, 10, new(1, 1)), new(Id("gate-skirmisher"), opposition, 10, new(2, 1))]);

    encounter.PartyAction(Id("mara-venn"), Id("gate-skirmisher"), Id("longsword-strike"), OperationId.Parse("continuation-reaction-action"));
    spatial.FailRoutes = true;
    bool failedAfterResolution = false;
    try { encounter.ResolveReaction(Id("parry"), true); }
    catch (TacticalException) { failedAfterResolution = true; }
    Assert(failedAfterResolution && encounter.PendingReaction is null && encounter.HasCommittedContinuation && encounter.OppositionProgress == 0 && session.RollSource.Position == 1 && session.Receipts.Count == 1, "a post-resolution spatial failure keeps automatic custody after exactly one reaction and action commit");

    spatial.FailRoutes = false;
    encounter.ResumeAutomaticProgression();
    Assert(!encounter.HasCommittedContinuation && encounter.PendingReaction is null && encounter.OppositionProgress == 0 && session.RollSource.Position == 1 && session.Receipts.Count == 1, "retry completes deterministic no-legal-action opposition settlement without replaying the committed party action or reaction");
}

static void RetainedSpatialAndAdventureLoadout()
{
    var content = D20ContentCatalog.Compile();
    var adventure = content.Adventures[Id("wardens-gate")];
    var service = DispatchProxy.Create<ISpatialService, RecordingSpatialService>();
    using (var gateway = new EngineCampaignSpatialGateway(service, adventure.Dungeon))
    {
        var recording = RecordingSpatialService.Instance!;
        Assert(recording.NavigationReplacements == 1 && recording.CollisionReplacements == 1, "campaign construction admits authored dungeon navigation and collision into the retained Engine session");
        gateway.ReplaceTacticalBoard(content.Catalog.Encounters[Id("iron-warden")].Board);
        Assert(recording.NavigationReplacements == 2 && recording.CollisionReplacements == 2, "tactical board composition replaces the same retained spatial projections");
        gateway.ReplaceOpenedDoors(new HashSet<D20Id> { Id("inner-sigil-gate") });
        Assert(recording.NavigationReplacements == 3 && recording.CollisionReplacements == 3 && gateway.CanMove(new(1, 1), new(2, 1), new HashSet<D20Id>()), "door projection updates remain deterministic and route queries use Engine navigation");
    }
    Assert(RecordingSpatialService.Instance!.DisposedSessions > 0, "the gateway disposes its owned retained spatial session candidates");

    using var session = new D20Session(content, RollSourceState.Static([]));
    AdventureLoadoutAdmission admission = session.AdmitAdventureLoadout(adventure);
    Assert(admission.Owners.Count == adventure.Characters.Count + adventure.Storage.Count && admission.Items.Count == adventure.Items.Count, "admission exposes stable owner and authored armor/implement item mappings");
    session.RequireAdventureItemOwner(Id("gate-sigil-buckler"), Id("gate-cache"));
    var restored = D20Session.Restore(content, session.CaptureSave());
    using (restored) Assert(restored.Inventory.ItemEntities.Count == adventure.Items.Count, "the complete Engine-owned adventure loadout restores with empty and populated owners");

    using var campaignSession = TerminalAdventureSession(content, adventure, Id("iron-warden"), EncounterResult.Victory);
    var campaignService = DispatchProxy.Create<ISpatialService, RecordingSpatialService>();
    using var campaignGateway = new EngineCampaignSpatialGateway(campaignService, adventure.Dungeon);
    using var campaign = new D20CampaignRuntime(content, adventure.Id, campaignGateway, session: campaignSession);
    campaign.BeginExploration();
    for (int step = 0; step < 8; step++) campaign.Explore(ExplorationCommand.StepForward);
    Assert(campaign.Snapshot().Phase == CampaignPhase.Encounter, "ordered exploration admission enters the first authored encounter");
    var tactical = BuildTactical(content, campaignSession, Id("iron-warden"), campaignGateway);
    campaign.ResolveEncounter(tactical);
    campaignSession.RequireAdventureItemOwner(Id("warden-chain"), adventure.CampStorage);
    ExpectCampaign(() => campaign.ResolveEncounter(tactical), "requires Encounter");
    campaign.ContinueOutcome();
    ExpectCampaign(() => D20CampaignRuntime.Restore(campaign.EncodeSave(), content, dungeon => new EngineCampaignSpatialGateway(campaignService, dungeon)), "restored D20 session");
    using var restoredCampaign = D20CampaignRuntime.Restore(campaign.EncodeSave(), content, dungeon => new EngineCampaignSpatialGateway(campaignService, dungeon), session: campaignSession);
    Assert(restoredCampaign.Snapshot().Outcome == EncounterResult.Victory, "victory reward transfer precedes durable campaign completion and restores with ownership proof");
}

static void DefeatRecoveryTransaction()
{
    var content = D20ContentCatalog.Compile();
    var adventure = content.Adventures[Id("wardens-gate")];
    using var session = TerminalAdventureSession(content, adventure, Id("iron-warden"), EncounterResult.Defeat);
    var defeatedPartyMember = session.OwnerEntity(Id("mara-venn"));
    var opposition = session.OwnerEntity(Id("iron-warden"));
    Assert(session.ReadVitality(defeatedPartyMember).Current.Raw == 0, "focused defeat setup is derived from strict saved vitality facts");
    ulong beforeRecoveryRevision = session.Revision;
    var service = DispatchProxy.Create<ISpatialService, RecordingSpatialService>();
    using var gateway = new EngineCampaignSpatialGateway(service, adventure.Dungeon);
    using var campaign = new D20CampaignRuntime(content, adventure.Id, gateway, session: session);
    campaign.BeginExploration();
    for (int step = 0; step < 8; step++) campaign.Explore(ExplorationCommand.StepForward);
    var tactical = BuildTactical(content, session, Id("iron-warden"), gateway);
    campaign.ResolveEncounter(tactical);
    int recovery = content.Catalog.Encounters[Id("iron-warden")].Defeat.RecoveryVitality!.Value;
    Assert(campaign.Snapshot().Outcome == EncounterResult.Defeat && session.ReadVitality(defeatedPartyMember).Current.Raw == recovery && session.Revision == beforeRecoveryRevision + 1, "authored defeat recovery restores party vitality through one detached session commit");
    ulong afterRecoveryRevision = session.Revision;
    int afterRecoveryVitality = checked((int)session.ReadVitality(defeatedPartyMember).Current.Raw);
    ExpectCampaign(() => campaign.ResolveEncounter(tactical), "requires Encounter");
    Assert(session.Revision == afterRecoveryRevision && session.ReadVitality(defeatedPartyMember).Current.Raw == afterRecoveryVitality, "defeat recovery is exactly once at the campaign outcome boundary");
    ExpectSession(() => session.ApplyDefeatRecovery([opposition], 0), "positive");
    Assert(session.Revision == afterRecoveryRevision && session.ReadVitality(defeatedPartyMember).Current.Raw == afterRecoveryVitality, "invalid recovery leaves the live session unchanged");
}

static void TacticalReviewFindings()
{
    var content = D20ContentCatalog.Compile();
    using (var session = NewSession([], out var party, out var opposition))
    {
        session.SetActivationBudget(party, Id("standard-action"), 1);
        session.RegisterLoadoutOwner(party);
        session.EquipImplement(party, content.Catalog.Implements[Id("training-blade")]);
        var spatial = new CheckTacticalSpatial();
        var encounter = new TacticalEncounter(session, spatial,
        [
            new(Id("mara-venn"), party, 20, new(1, 1)),
            new(Id("gate-skirmisher"), opposition, 10, new(5, 1)),
        ]);
        TacticalReviewProbes.AssertOutOfRangeRejected(encounter, Id("mara-venn"), Id("gate-skirmisher"), Id("longsword-strike"), OperationId.Parse("review-range"));
    }

    using (var session = NewSession([], out var party, out var opposition))
    {
        var spatial = new CheckTacticalSpatial();
        var encounter = new TacticalEncounter(session, spatial,
        [
            new(Id("mara-venn"), party, 20, new(1, 1)),
            new(Id("gate-skirmisher"), opposition, 10, new(3, 1)),
        ]);
        TacticalReviewProbes.AssertMovementSurvivesRestore(encounter, session, spatial,
            new TacticalBoard(6, 4, ["######", "#....#", "#....#", "######"],
            [new(Id("mara-venn"), new(1, 1)), new(Id("gate-skirmisher"), new(3, 1))]),
            Id("mara-venn"), new(1, 2));
    }

    using (var session = NewSession([new(20, [2])], out var party, out var opposition))
    {
        session.RegisterLoadoutOwner(opposition);
        session.EquipImplement(opposition, content.Catalog.Implements[Id("field-bow")]);
        session.ApplyAction(session.PreviewAction(opposition, party, Id("pin-in-place"), OperationId.Parse("review-held")));
        var encounter = new TacticalEncounter(session, new CheckTacticalSpatial(),
        [
            new(Id("mara-venn"), party, 20, new(1, 1)),
            new(Id("gate-skirmisher"), opposition, 10, new(3, 1)),
        ]);
        TacticalReviewProbes.AssertMovementConditionBoundary(encounter, session, Id("mara-venn"), new(1, 2));
    }

    var adventure = content.Adventures[Id("embers-wake")];
    using (var session = new D20Session(content, RollSourceState.Static([new(20, [4, 4])])) )
    {
        session.AdmitAdventureLoadout(adventure);
        var sera = session.OwnerEntity(Id("sera-vale"));
        session.SetActionResource(sera, Id("focus"), 0);
        session.SetActivationBudget(sera, Id("reaction"), 0);
        var encounter = BuildTactical(content, session, Id("ash-seer"), new CheckTacticalSpatial(), oppositionFirst: true);
        TacticalReviewProbes.AssertNoHardcodedOppositionFallback(encounter, session);
    }

    var outcomeAdventure = content.Adventures[Id("wardens-gate")];
    using (var session = new D20Session(content, RollSourceState.Static([])))
    {
        session.AdmitAdventureLoadout(outcomeAdventure);
        var spatial = new CheckSpatial();
        using var campaign = new D20CampaignRuntime(content, outcomeAdventure.Id, spatial, session: session);
        campaign.BeginExploration();
        for (int step = 0; step < 8; step++) campaign.Explore(ExplorationCommand.StepForward);
        Assert(campaign.Phase == CampaignPhase.Encounter, "focused outcome probe reaches the authored encounter");
        var untouched = BuildTactical(content, session, Id("iron-warden"), new CheckTacticalSpatial());
        TacticalReviewProbes.AssertUntouchedOutcomeRejected(campaign, untouched);
    }
}

static D20Session TerminalAdventureSession(CompiledD20Content content, AdventureDefinition adventure, D20Id encounterId, EncounterResult result)
{
    using var source = new D20Session(content, RollSourceState.Static([]));
    source.AdmitAdventureLoadout(adventure);
    D20SessionSave save = source.CaptureSave();
    EncounterDefinition encounter = content.Catalog.Encounters[encounterId];
    EncounterFaction losingFaction = result == EncounterResult.Victory ? EncounterFaction.Opposition : EncounterFaction.Party;
    HashSet<D20Id> losers = encounter.Roster.Where(value => value.Faction == losingFaction).Select(value => value.Character).ToHashSet();
    SessionParticipantSave[] participants = save.Participants
        .Select(value => losers.Contains(value.Character) ? value with { Living = false, Vitality = 0 } : value)
        .ToArray();
    return D20Session.Restore(content, save with { Participants = participants });
}

static TacticalEncounter BuildTactical(CompiledD20Content content, D20Session session, D20Id encounterId, ITacticalSpatialGateway spatial, bool oppositionFirst = false)
{
    EncounterDefinition encounter = content.Catalog.Encounters[encounterId];
    TacticalParticipant[] participants = encounter.Roster.Select(row => new TacticalParticipant(
        row.Character,
        session.OwnerEntity(row.Character),
        oppositionFirst && row.Faction == EncounterFaction.Opposition ? 20 : 10,
        encounter.Board.Placements.Single(value => value.Character == row.Character).Position)).ToArray();
    return new TacticalEncounter(session, spatial, participants, encounter.Board);
}

static D20Session NewSession(IReadOnlyList<StaticActionRoll> rolls, out Rusty.Engine.Entities.EntityId actor, out Rusty.Engine.Entities.EntityId target, ulong position = 0)
{
    var content = D20ContentCatalog.Compile();
    var characters = content.Characters;
    var session = new D20Session(content, RollSourceState.Static(rolls, position));
    actor = session.AddParticipant(characters[Id("mara-venn")], EncounterFaction.Party);
    target = session.AddParticipant(characters[Id("gate-skirmisher")], EncounterFaction.Opposition);
    session.SetActivationBudget(actor, Id("bonus-action"), 2);
    return session;
}

static D20Id Id(string value) => D20Id.Parse(value);

static void ExpectSession(Action action, string message)
{
    try { action(); throw new InvalidOperationException("expected D20 session failure"); }
    catch (D20SessionException exception) { Assert(exception.Message.Contains(message, StringComparison.OrdinalIgnoreCase), $"expected session failure containing '{message}'"); }
}

static void ExpectCampaign(Action action, string message)
{
    try { action(); throw new InvalidOperationException("expected campaign failure"); }
    catch (CampaignException exception) { Assert(exception.Message.Contains(message, StringComparison.OrdinalIgnoreCase), $"expected campaign failure containing {message}"); }
}

static SourceProvenance Source(string subject) => new($"checks/{subject}.cs", subject, "focused check");

static void Expect(string code, Action action)
{
    try
    {
        action();
        throw new InvalidOperationException($"expected {code}");
    }
    catch (D20CompilationException exception)
    {
        Assert(exception.Diagnostics.Any(diagnostic => diagnostic.Code == code), $"expected diagnostic {code}");
    }
}

static void ExpectAny(IEnumerable<string> codes, Action action)
{
    try
    {
        action();
        throw new InvalidOperationException("expected admission failure");
    }
    catch (D20CompilationException exception)
    {
        Assert(exception.Diagnostics.Any(diagnostic => codes.Contains(diagnostic.Code, StringComparer.Ordinal)), "expected one relevant topology/dependency diagnostic");
    }
}

static void ExpectUnsupported(Action action)
{
    try
    {
        action();
        throw new InvalidOperationException("expected immutable collection rejection");
    }
    catch (NotSupportedException) { }
}

static void Assert(bool condition, string message)
{
    if (!condition) throw new InvalidOperationException(message);
}

sealed class CheckSpatial : ICampaignSpatialGateway
{
    public bool CanMove(GridPosition from, GridPosition to, IReadOnlySet<D20Id> openedDoors) => to.X >= 0 && to.Y >= 0;
    public bool IsOccluded(GridPosition from, GridPosition to, IReadOnlySet<D20Id> openedDoors) => to.X > 1;
}

sealed class CheckTacticalSpatial : ITacticalSpatialGateway
{
    public bool HasLineOfEffect(GridPosition from, GridPosition to) => true;
    public bool HasLegalRoute(GridPosition from, GridPosition to) => true;
}

sealed class FailingTacticalSpatial : ITacticalSpatialGateway
{
    public bool FailRoutes { get; set; }
    public bool HasLineOfEffect(GridPosition from, GridPosition to) => true;
    public bool HasLegalRoute(GridPosition from, GridPosition to) => !FailRoutes ? true : throw new TacticalException("injected post-resolution spatial failure");
}

sealed class RecordingOwner(string name, bool throws) : IDisposable
{
    public bool Disposed { get; private set; }
    public void Dispose()
    {
        Disposed = true;
        if (throws) throw new InvalidOperationException(name);
    }
}

class RecordingSpatialService : DispatchProxy
{
    public static RecordingSpatialService? Instance { get; private set; }
    public int NavigationReplacements { get; private set; }
    public int CollisionReplacements { get; private set; }
    public int DisposedSessions { get; private set; }
    private HashSet<PlanarNavCell> _walkable = [];

    protected override object? Invoke(MethodInfo? targetMethod, object?[]? args)
    {
        Instance = this;
        if (targetMethod is null) throw new InvalidOperationException("Generated spatial proxy did not provide a method.");
        if (targetMethod.Name == nameof(ISpatialService.ReplaceNavigation)) NavigationReplacements++;
        if (targetMethod.Name == nameof(ISpatialService.ReplaceCollision)) CollisionReplacements++;
        if (targetMethod.Name == nameof(ISpatialService.ReplaceNavigation) && args is { Length: > 0 } && args[0] is NavigationReplaceRequest navigation) _walkable = navigation.Cells.ToArray().ToHashSet();
        if (targetMethod.Name == nameof(ISpatialService.CreateSession)) return new SpatialSession(new SpatialSessionHandle(7), () => DisposedSessions++);
        if (targetMethod.Name == nameof(ISpatialService.RequestNavigationPath) && args is { Length: > 0 } && args[0] is NavigationPathRequest path) { bool reached = _walkable.Contains(path.Start) && _walkable.Contains(path.Goal); return new NavigationPathReadout(reached ? NavigationPathOutcome.Reached : NavigationPathOutcome.NoPath, NavigationProjectionKind.HostWalkableCells, 1, reached ? 1u : 0u, 1, 1, 1); }
        return targetMethod.ReturnType.IsValueType ? Activator.CreateInstance(targetMethod.ReturnType) : null;
    }
}
