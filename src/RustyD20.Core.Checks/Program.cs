using System.Reflection;
using Rusty.Engine;
using RustyD20.Core.Content;
using RustyD20.Core.Contract;
using RustyD20.Core.Rules;
using RustyD20.Core.Session;
using RustyD20.Core.Campaign;
using RustyD20.Core.Tactical;
using Rusty.Engine.Mechanics;

var checks = new (string Name, Action Run)[]
{
    ("strict id and schema", StrictIdentityAndSchema),
    ("duplicate, quota, and reference admission", DuplicateQuotaAndReference),
    ("topology, placement, dependency, and reachability admission", TopologyPlacementAndDependency),
    ("stable composition fingerprint", StableFingerprint),
    ("named adventure compilation", NamedAdventures),
    ("d20 session floor, static order, and choice", SessionFloorStaticAndChoice),
    ("d20 session action effects and late-failure atomicity", SessionActionAndAtomicity),
    ("d20 session stale reaction and canonical equipment", SessionStaleReactionAndEquipment),
    ("d20 session engine-state admission fences", SessionEngineStateAdmissionFences),
    ("d20 session fresh restore and strict save boundary", SessionFreshRestore),
    ("campaign hidden topology and strict current save", CampaignHiddenTopologyAndSave),
    ("tactical initiative, dead skip, opposition reaction", TacticalInitiativeAndReaction),
    ("tactical post-resolution continuation retry", TacticalPostResolutionContinuationRetry),
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
    var blade = content.Modules.SelectMany(module => module.ImplementsOrEmpty).Single(implement => implement.Id == Id("training-blade"));
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
    var bow = content.Modules.SelectMany(module => module.ImplementsOrEmpty).Single(implement => implement.Id == Id("field-bow"));
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
    var blade = content.Modules.SelectMany(module => module.ImplementsOrEmpty).Single(implement => implement.Id == Id("training-blade"));
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
    session.RegisterLoadoutOwner(actor);
    var blade = D20ContentCatalog.Compile().Modules.SelectMany(module => module.ImplementsOrEmpty).Single(value => value.Id == Id("training-blade"));
    var item = session.EquipImplement(actor, blade);
    session.ApplyAction(session.PreviewAction(actor, target, Id("disrupt"), OperationId.Parse("save-fresh")));
    var save = session.CaptureSave();
    using var restored = D20Session.Restore(D20ContentCatalog.Compile(), save);
    Assert(restored.CaptureSave().Participants.Count == save.Participants.Count && restored.RollSource.Position == save.RollSource.Position && restored.Inventory.TryGetItem(item, out _), "session restore builds a fresh Engine-backed candidate with roll position, effects, and canonical loadout item");
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
    stale.EquipImplement(staleActor, D20ContentCatalog.Compile().Modules.SelectMany(module => module.ImplementsOrEmpty).Single(value => value.Id == Id("training-blade")));
    var staleEncounter = new TacticalEncounter(stale, new CheckTacticalSpatial(), [new(Id("mara-venn"), staleActor, 10, new(1, 1)), new(Id("gate-skirmisher"), staleTarget, 10, new(2, 1))]);
    staleEncounter.PartyAction(Id("mara-venn"), Id("gate-skirmisher"), Id("longsword-strike"), OperationId.Parse("stale-reaction-custody"));
    Assert(staleEncounter.PendingReaction is not null, "tactical reaction custody retains the original action");
    stale.SetActivationBudget(staleActor, Id("standard-action"), 0);
    ExpectSession(() => staleEncounter.ResolveReaction(Id("parry"), true), "stale");
    Assert(staleEncounter.PendingReaction is not null && stale.RollSource.Position == 0, "a failed combined reaction leaves prompt custody and session roll position unchanged");

    using var combined = NewSession([new(20, [4]), new(1, [1])], out var combinedActor, out var combinedTarget);
    combined.RegisterLoadoutOwner(combinedActor);
    combined.EquipImplement(combinedActor, D20ContentCatalog.Compile().Modules.SelectMany(module => module.ImplementsOrEmpty).Single(value => value.Id == Id("training-blade")));
    combined.SetActionResource(combinedActor, Id("guard"), 0);
    combined.SetActivationBudget(combinedActor, Id("reaction"), 0);
    var combinedEncounter = new TacticalEncounter(combined, new CheckTacticalSpatial(), [new(Id("mara-venn"), combinedActor, 10, new(1, 1)), new(Id("gate-skirmisher"), combinedTarget, 10, new(2, 1))]);
    combinedEncounter.PartyAction(Id("mara-venn"), Id("gate-skirmisher"), Id("longsword-strike"), OperationId.Parse("combined-reaction-action"));
    combinedEncounter.ResolveReaction(Id("parry"), true);
    Assert(combinedEncounter.PendingReaction is null && combined.RollSource.Position == 2 && combined.Inventory.Revision > 0, "accepted reaction commits the fresh action in one progression");
}

static void TacticalPostResolutionContinuationRetry()
{
    using var session = NewSession([new(20, [4]), new(1, [1])], out var party, out var opposition);
    session.RegisterLoadoutOwner(party);
    session.EquipImplement(party, D20ContentCatalog.Compile().Modules.SelectMany(module => module.ImplementsOrEmpty).Single(value => value.Id == Id("training-blade")));
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
    Assert(!encounter.HasCommittedContinuation && encounter.PendingReaction is null && encounter.OppositionProgress == 1 && session.RollSource.Position == 2 && session.Receipts.Count == 2, "retry resumes opposition once, preserving progress without replaying the committed action or reaction");
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
        gateway.ReplaceTacticalBoard(content.Modules.SelectMany(module => module.EncountersOrEmpty).Single(value => value.Id == Id("iron-warden")).Board);
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

    var campaignService = DispatchProxy.Create<ISpatialService, RecordingSpatialService>();
    using var campaignGateway = new EngineCampaignSpatialGateway(campaignService, adventure.Dungeon);
    using var campaign = new D20CampaignRuntime(content, adventure.Id, campaignGateway, session: session);
    campaign.BeginExploration();
    for (int step = 0; step < 8; step++) campaign.Explore(ExplorationCommand.StepForward);
    Assert(campaign.Snapshot().Phase == CampaignPhase.Encounter, "ordered exploration admission enters the first authored encounter");
    campaign.ResolveEncounter(EncounterResult.Victory);
    session.RequireAdventureItemOwner(Id("warden-chain"), adventure.CampStorage);
    ExpectCampaign(() => campaign.ResolveEncounter(EncounterResult.Victory), "requires Encounter");
    campaign.ContinueOutcome();
    campaign.Explore(ExplorationCommand.TurnRight);
    campaign.Explore(ExplorationCommand.StepForward);
    campaign.Explore(ExplorationCommand.Interact);
    campaign.Explore(ExplorationCommand.StepBackward);
    campaign.Explore(ExplorationCommand.TurnRight);
    campaign.Explore(ExplorationCommand.StepForward);
    campaign.Explore(ExplorationCommand.StepForward);
    campaign.Explore(ExplorationCommand.TurnLeft);
    campaign.Explore(ExplorationCommand.StepForward);
    campaign.Explore(ExplorationCommand.StepForward);
    campaign.Explore(ExplorationCommand.StepForward);
    campaign.Explore(ExplorationCommand.StepForward);
    campaign.Explore(ExplorationCommand.TurnLeft);
    campaign.Explore(ExplorationCommand.StepForward);
    campaign.Explore(ExplorationCommand.StepForward);
    if (campaign.Phase == CampaignPhase.Encounter)
    {
        campaign.ResolveEncounter(EncounterResult.Victory);
        campaign.ContinueOutcome();
    }
    campaign.Explore(ExplorationCommand.TurnLeft);
    ExpectCampaign(() => campaign.Explore(ExplorationCommand.StepForward), "navigation rejects");
    campaign.Explore(ExplorationCommand.Interact);
    campaign.Explore(ExplorationCommand.StepForward);
    Assert(campaign.Snapshot().Exploration!.Position == new GridPosition(9, 4), "Warden's Gate opens the adjacent closed door before Engine traversal");
    session.RequireAdventureItemOwner(Id("gate-sigil-buckler"), adventure.CampStorage);
    ExpectSession(() => session.TransferAdventureItem(Id("gate-sigil-buckler"), adventure.CampStorage), "already owned");
    ExpectCampaign(() => D20CampaignRuntime.Restore(campaign.EncodeSave(), content, dungeon => new EngineCampaignSpatialGateway(campaignService, dungeon)), "restored D20 session");
    using var restoredCampaign = D20CampaignRuntime.Restore(campaign.EncodeSave(), content, dungeon => new EngineCampaignSpatialGateway(campaignService, dungeon), session: session);
    Assert(restoredCampaign.Snapshot().Outcome == EncounterResult.Victory, "victory reward transfer precedes durable campaign completion and restores with ownership proof");
}

static void DefeatRecoveryTransaction()
{
    var content = D20ContentCatalog.Compile();
    var adventure = content.Adventures[Id("wardens-gate")];
    using var session = new D20Session(content, RollSourceState.Static([new(20, [4])]));
    session.AdmitAdventureLoadout(adventure);
    var defeatedPartyMember = session.OwnerEntity(Id("mara-venn"));
    var opposition = session.OwnerEntity(Id("iron-warden"));
    session.ApplyAction(session.PreviewAction(opposition, defeatedPartyMember, Id("disrupt"), OperationId.Parse("prepare-defeat")));
    Assert(session.ReadVitality(defeatedPartyMember).Current.Raw == 20, "focused defeat setup lowers one living party member through Engine vitality");
    ulong beforeRecoveryRevision = session.Revision;
    var service = DispatchProxy.Create<ISpatialService, RecordingSpatialService>();
    using var gateway = new EngineCampaignSpatialGateway(service, adventure.Dungeon);
    using var campaign = new D20CampaignRuntime(content, adventure.Id, gateway, session: session);
    campaign.BeginExploration();
    for (int step = 0; step < 8; step++) campaign.Explore(ExplorationCommand.StepForward);
    campaign.ResolveEncounter(EncounterResult.Defeat);
    Assert(campaign.Snapshot().Outcome == EncounterResult.Defeat && session.ReadVitality(defeatedPartyMember).Current.Raw == 24 && session.Revision == beforeRecoveryRevision + 1, "authored defeat recovery restores party vitality through one detached session commit");
    ulong afterRecoveryRevision = session.Revision;
    int afterRecoveryVitality = checked((int)session.ReadVitality(defeatedPartyMember).Current.Raw);
    ExpectCampaign(() => campaign.ResolveEncounter(EncounterResult.Defeat), "requires Encounter");
    Assert(session.Revision == afterRecoveryRevision && session.ReadVitality(defeatedPartyMember).Current.Raw == afterRecoveryVitality, "defeat recovery is exactly once at the campaign outcome boundary");
    ExpectSession(() => session.ApplyDefeatRecovery([opposition], 0), "positive");
    Assert(session.Revision == afterRecoveryRevision && session.ReadVitality(defeatedPartyMember).Current.Raw == afterRecoveryVitality, "invalid recovery leaves the live session unchanged");
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
