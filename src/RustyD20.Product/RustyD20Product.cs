using System.Numerics;
using Rusty.Engine;
using Rusty.Engine.Mechanics;
using Rusty.Engine.Persistence;
using RustyD20.Core.Campaign;
using RustyD20.Core.Content;
using RustyD20.Core.Contract;
using RustyD20.Core.Persistence;
using RustyD20.Core.Rules;
using RustyD20.Core.Session;
using RustyD20.Core.Tactical;

namespace RustyD20.Product;

internal static class D20Disposal
{
    public static void DisposeAll(params IDisposable?[] owners)
    {
        List<Exception>? failures = null;
        foreach (IDisposable? owner in owners)
        {
            if (owner is null) continue;
            try { owner.Dispose(); }
            catch (Exception error) { (failures ??= []).Add(error); }
        }

        if (failures is { Count: > 0 }) throw new AggregateException(failures);
    }

    public static void DisposeAfterFailure(Exception primary, params IDisposable?[] owners)
    {
        try { DisposeAll(owners); }
        catch (AggregateException cleanup) { throw new AggregateException(new[] { primary }.Concat(cleanup.InnerExceptions)); }
        throw primary;
    }
}

/// <summary>Named product-owned posture for the retained Engine presentation; no geometry or renderer is duplicated here.</summary>
public sealed record D20PresentationTuning(
    uint MaterialSlot = 1,
    uint PartyMaterialSlot = 2,
    uint OppositionMaterialSlot = 3,
    uint ActiveMaterialSlot = 4,
    uint SelectionMaterialSlot = 5,
    int DungeonFloorLayer = 0,
    int DungeonWallLayer = 1,
    int MarkerLayer = 2,
    int TacticalFloorLayer = 0,
    int TacticalWallLayer = 1,
    float MaterialRed = .55f,
    float MaterialGreen = .25f,
    float MaterialBlue = .08f,
    float MaterialRoughness = .8f,
    float ExplorationEyeHeight = .65f,
    float ExplorationCameraPitch = -8,
    float TacticalCameraHeight = 9,
    float TacticalCameraPitch = -62,
    float CameraFieldOfView = 65,
    float CameraFar = 64)
{
    public void Validate()
    {
        uint[] slots = [MaterialSlot, PartyMaterialSlot, OppositionMaterialSlot, ActiveMaterialSlot, SelectionMaterialSlot];
        if (slots.Any(slot => slot == 0) || slots.Distinct().Count() != slots.Length || DungeonFloorLayer < 0 || DungeonWallLayer <= DungeonFloorLayer || MarkerLayer <= DungeonWallLayer || TacticalFloorLayer < 0 || TacticalWallLayer <= TacticalFloorLayer || MaterialRed is < 0 or > 1 || MaterialGreen is < 0 or > 1 || MaterialBlue is < 0 or > 1 || MaterialRoughness is < 0 or > 1 || ExplorationEyeHeight is <= 0 or > 4 || TacticalCameraHeight is <= 0 or > 32 || CameraFieldOfView is < 1 or > 179 || CameraFar <= 0)
            throw new ArgumentOutOfRangeException(nameof(D20PresentationTuning), "Presentation tuning is outside the admitted product bounds.");
    }

    public string Readout => $"materials=terrain:{MaterialSlot},party:{PartyMaterialSlot},opposition:{OppositionMaterialSlot},active:{ActiveMaterialSlot},selection:{SelectionMaterialSlot};layers=explore:{DungeonFloorLayer}/{DungeonWallLayer}/{MarkerLayer},tactical:{TacticalFloorLayer}/{TacticalWallLayer}/{MarkerLayer};camera=eye:{ExplorationEyeHeight}:{ExplorationCameraPitch},tactical:{TacticalCameraHeight}:{TacticalCameraPitch}/{CameraFieldOfView}/{CameraFar}";
}

/// <summary>Engine-admitted D20 orchestration. Core owns rules/campaign meaning; this class owns only lifecycle, input, retained view, and projection composition.</summary>
public sealed class RustyD20Product : IEngineProduct
{
    private const string SaveScope = "rusty-d20.native-product";
    private const string SaveKey = "campaign/current";
    private readonly IEngineContext _engine;
    private readonly ulong _instanceId;
    private readonly InputContext _inputContext;
    private readonly CompiledD20Content _content;
    private readonly ScopedSeededRollAdapter _rolls;
    private readonly UiStream _ui;
    private readonly D20EngineStateStore _saves;
    private readonly D20Surface _surface;
    private readonly List<string> _log = [];
    private D20CampaignRuntime? _campaign;
    private D20Session? _session;
    private TacticalEncounter? _tactical;
    private D20Id _selection;
    private int _partyCursor;
    private int _actionCursor;
    private int _targetCursor;
    private ulong _revision;
    private ulong _sequence;
    private string _lastReadout = "created";
    private bool _started;
    private bool _paused;
    private bool _shutdown;
    private bool _disposed;

    public RustyD20Product(ProductCreateContext context)
    {
        ArgumentNullException.ThrowIfNull(context);
        _engine = context.Engine;
        _instanceId = context.Input.Binding.InstanceId;
        _inputContext = context.Input.Context;
        _content = D20ContentCatalog.Compile();
        _selection = _content.Adventures.Values.Single(value => value.IsDefault).Id;
        UiStream? ui = null;
        D20EngineStateStore? saves = null;
        D20Surface? surface = null;
        try
        {
            ui = _engine.Ui.OpenStream(new UiStreamRequest("rusty-d20", "rusty-d20.workbench.v1"));
            saves = new D20EngineStateStore(_engine, SaveScope);
            surface = D20Surface.Create(_engine, _content.Adventures[_selection]);
            _ui = ui;
            _saves = saves;
            _surface = surface;
            _rolls = new ScopedSeededRollAdapter(_engine.Random);
        }
        catch (Exception error)
        {
            D20Disposal.DisposeAfterFailure(error, surface, saves, ui);
            throw;
        }
    }

    public void Start()
    {
        ThrowIfDisposed();
        _started = true; _paused = false; _shutdown = false;
        EnsureCandidate();
        Note("lifecycle:start");
        Publish();
    }

    public void Attach()
    {
        ThrowIfDisposed();
        Note("lifecycle:attached");
        Publish();
    }

    public ProductUpdateResult Update(ProductUpdate update)
    {
        ThrowIfDisposed();
        if (!_started || _paused || _shutdown) return ProductUpdateResult.None;
        InputBinding binding = new(_instanceId, update.Facts.Generation, update.Facts.ControlRevision);
        foreach (ProductInputEvent input in update.Input)
        {
            if (D20InputClaims.TryClaim(input, binding, _inputContext, out D20Command command))
            {
                Apply(command);
            }
            else
            {
                _lastReadout = "rejected:stale-or-inactive-input";
            }
        }
        Publish();
        return ProductUpdateResult.None;
    }

    public void Pause() { ThrowIfDisposed(); _paused = true; Note("lifecycle:paused"); Publish(); }
    public void Resume() { ThrowIfDisposed(); if (!_shutdown) { _paused = false; Note("lifecycle:resumed"); Publish(); } }
    public void Restart() { ThrowIfDisposed(); _started = true; _paused = false; _shutdown = false; ReplaceCandidate(_selection); Note("lifecycle:restarted"); Publish(); }
    public void Shutdown() { if (_disposed) return; _shutdown = true; Note("lifecycle:shutdown"); Publish(); }

    public void Dispose()
    {
        if (_disposed) return;
        _disposed = true;
        _shutdown = true; _started = false; _paused = false;
        List<Exception>? failures = null;
        try { DisposeCandidate(); }
        catch (Exception error) { (failures ??= []).Add(error); }
        foreach (IDisposable owner in new IDisposable[] { _surface, _saves, _ui })
        {
            try { owner.Dispose(); }
            catch (Exception error) { (failures ??= []).Add(error); }
        }
        if (failures is { Count: > 0 }) throw new AggregateException(failures);
    }

    private void Apply(D20Command command)
    {
        try
        {
            switch (command)
            {
                case D20Command.SelectWarden: ReplaceCandidate(D20Id.Parse("wardens-gate")); break;
                case D20Command.SelectEmber: ReplaceCandidate(D20Id.Parse("embers-wake")); break;
                case D20Command.Begin: RequireCampaign().BeginExploration(); Note("campaign:begin"); break;
                case D20Command.Forward: Explore(ExplorationCommand.StepForward); break;
                case D20Command.Back: Explore(ExplorationCommand.StepBackward); break;
                case D20Command.Left: Explore(ExplorationCommand.TurnLeft); break;
                case D20Command.Right: Explore(ExplorationCommand.TurnRight); break;
                case D20Command.Interact: Explore(ExplorationCommand.Interact); break;
                case D20Command.PartyNext: CycleParty(); break;
                case D20Command.ActionNext: _actionCursor++; Note("selection:action"); break;
                case D20Command.TargetNext: _targetCursor++; Note("selection:target"); break;
                case D20Command.TacticalMoveNorth: MoveTactical(0, -1); break;
                case D20Command.TacticalMoveSouth: MoveTactical(0, 1); break;
                case D20Command.TacticalMoveWest: MoveTactical(-1, 0); break;
                case D20Command.TacticalMoveEast: MoveTactical(1, 0); break;
                case D20Command.CommitAction: CommitAction(); break;
                case D20Command.React: ResolveReaction(true); break;
                case D20Command.Decline: ResolveReaction(false); break;
                case D20Command.Continue: ContinueOutcome(); break;
                case D20Command.Save: Save(); break;
                case D20Command.Load: Load(); break;
                case D20Command.Reset: ReplaceCandidate(_selection); Note("campaign:reset"); break;
            }
            _lastReadout = $"accepted:{command}";
            _revision++;
        }
        catch (Exception error) when (error is CampaignException or TacticalException or D20SessionException or InvalidOperationException)
        {
            _lastReadout = $"rejected:{command}:{Bound(error.Message)}";
            Note(_lastReadout);
        }
    }

    private void EnsureCandidate() { if (_campaign is null) ReplaceCandidate(_selection); }
    private void ReplaceCandidate(D20Id adventure)
    {
        if (!_content.Adventures.TryGetValue(adventure, out AdventureDefinition? definition) || !definition.Selectable)
            throw new InvalidOperationException("unknown-or-unselectable-adventure");
        D20Session? session = null; D20CampaignRuntime? campaign = null; EngineCampaignSpatialGateway? spatial = null;
        try
        {
            session = new D20Session(_content, RollSourceState.Seeded(0xD20UL), seededRolls: _rolls);
            session.AdmitAdventureLoadout(definition);
            spatial = new EngineCampaignSpatialGateway(_engine.Spatial, definition.Dungeon);
            campaign = new D20CampaignRuntime(_content, definition.Id, spatial, session: session, ownsSpatialGateway: true);
            spatial = null; // D20CampaignRuntime now owns this successfully constructed Engine gateway.
            DisposeCandidate();
            _selection = definition.Id; _session = session; _campaign = campaign; _tactical = null;
            _partyCursor = _actionCursor = _targetCursor = 0;
            session = null; campaign = null;
            Note($"selection:{definition.Id}");
        }
        finally { D20Disposal.DisposeAll(campaign, spatial, session); }
    }

    private void Explore(ExplorationCommand command)
    {
        D20CampaignRuntime campaign = RequireCampaign();
        campaign.Explore(command);
        TryComposeTactical();
        Note($"explore:{command}");
    }

    private void TryComposeTactical()
    {
        D20CampaignRuntime campaign = RequireCampaign();
        CampaignSnapshot snapshot = campaign.Snapshot();
        if (snapshot.Phase != CampaignPhase.Encounter || snapshot.ActiveEncounter is not D20Id active || _tactical is not null) return;
        EncounterDefinition encounter = _content.Catalog.Encounters[active];
        var participants = encounter.Roster.Select(row => new TacticalParticipant(row.Character, RequireSession().OwnerEntity(row.Character), Initiative(row.Character), encounter.Board.Placements.Single(place => place.Character == row.Character).Position));
        _tactical = new TacticalEncounter(RequireSession(), (ITacticalSpatialGateway)campaign.Spatial, participants, encounter.Board);
        _partyCursor = _actionCursor = _targetCursor = 0;
        Note($"encounter:{active}");
    }

    private void CycleParty()
    {
        TacticalEncounter? tactical = _tactical;
        if (tactical is null) throw new CampaignException("No tactical party actor is active.");
        _partyCursor++;
        Note("selection:party");
    }

    private void CommitAction()
    {
        TacticalEncounter tactical = _tactical ?? throw new CampaignException("No tactical action is active.");
        if (tactical.PendingReaction is not null) throw new TacticalException("Choose or decline the current reaction.");
        EncounterDefinition encounter = CurrentEncounter();
        D20Id actor = tactical.CurrentActor;
        if (RequireSession().FactionOf(RequireSession().OwnerEntity(actor)) != EncounterFaction.Party) throw new TacticalException("The Engine-admitted opposition is resolving automatically.");
        CharacterDefinition character = _content.Characters[actor];
        D20Id[] actions = character.Actions.ToArray();
        D20Id[] targets = encounter.Roster.Where(value => value.Faction == EncounterFaction.Opposition).Select(value => value.Character).ToArray();
        D20Id action = actions[Modulo(_actionCursor, actions.Length)];
        D20Id target = targets[Modulo(_targetCursor, targets.Length)];
        tactical.PartyAction(actor, target, action, OperationId.Parse($"d20-{_revision + 1}"));
        Note($"action:{actor}:{action}:{target}");
        ResolveCampaignIfSettled();
    }

    private void MoveTactical(int deltaX, int deltaY)
    {
        TacticalEncounter tactical = _tactical ?? throw new CampaignException("No tactical movement is active.");
        D20Id actor = tactical.CurrentActor;
        if (RequireSession().FactionOf(RequireSession().OwnerEntity(actor)) != EncounterFaction.Party) throw new TacticalException("The Engine-admitted opposition is resolving automatically.");
        TacticalParticipant current = tactical.Participants.Single(value => value.Id == actor);
        tactical.PartyMove(actor, new GridPosition(current.Position.X + deltaX, current.Position.Y + deltaY));
        Note($"move:{actor}:{deltaX},{deltaY}");
    }

    private void ResolveReaction(bool choose)
    {
        TacticalEncounter tactical = _tactical ?? throw new TacticalException("No pending reaction.");
        ReactionPrompt prompt = tactical.PendingReaction ?? throw new TacticalException("No pending reaction.");
        D20Id reaction = _content.Characters[prompt.Defender].Reactions.FirstOrDefault();
        tactical.ResolveReaction(reaction, choose);
        Note(choose ? "reaction:chosen" : "reaction:declined");
        ResolveCampaignIfSettled();
    }

    private void ResolveCampaignIfSettled()
    {
        TacticalEncounter? tactical = _tactical;
        if (tactical?.PendingReaction is not null) return;
        if (tactical is not null && tactical.TryGetTerminalResult(out EncounterResult result))
        {
            RequireCampaign().ResolveEncounter(tactical);
            _tactical = null;
            Note(result == EncounterResult.Victory ? "outcome:victory" : "outcome:defeat");
        }
    }

    private void ContinueOutcome() { RequireCampaign().ContinueOutcome(); _tactical = null; Note("outcome:continue"); }
    private void Save() => _saves.Save(SaveKey, _content.ContentFingerprint, RequireCampaign(), RequireSession(), _tactical, _log, _revision, _tactical?.PendingReaction is not null);
    private void Load()
    {
        ProductStateLoad<D20RestoreCandidate> loaded = _saves.Load(SaveKey, _content, dungeon => new EngineCampaignSpatialGateway(_engine.Spatial, dungeon), seededRolls: _rolls);
        if (!loaded.Present || loaded.State is null) { Note("load:absent"); return; }
        D20RestoreCandidate? candidate = loaded.State;
        try
        {
            CampaignSnapshot restored = candidate.Campaign.Snapshot();
            DisposeCandidate();
            _selection = restored.Adventure; _campaign = candidate.Campaign; _session = candidate.Session; _tactical = candidate.Tactical; _revision = candidate.Revision;
            candidate = null; // The product now owns the fully admitted replacement aggregate.
            _log.Clear(); _log.AddRange(loaded.State.Log);
            Note("load:accepted");
        }
        catch (Exception error)
        {
            if (candidate is not null) D20Disposal.DisposeAfterFailure(error, candidate);
            throw;
        }
    }

    private EncounterDefinition CurrentEncounter()
    {
        D20Id id = RequireCampaign().Snapshot().ActiveEncounter ?? throw new CampaignException("No active encounter identity.");
        return _content.Catalog.Encounters[id];
    }
    private int Initiative(D20Id id) => _content.Characters[id].Abilities.Values.Sum();
    private D20CampaignRuntime RequireCampaign() => _campaign ?? throw new InvalidOperationException("Campaign candidate was not created.");
    private D20Session RequireSession() => _session ?? throw new InvalidOperationException("Session candidate was not created.");
    private void DisposeCandidate()
    {
        D20CampaignRuntime? campaign = _campaign; D20Session? session = _session;
        _tactical = null; _campaign = null; _session = null;
        D20Disposal.DisposeAll(campaign, session);
    }
    private void Note(string message) { _log.Add(message); if (_log.Count > 64) _log.RemoveAt(0); }
    private void Publish()
    {
        List<UiField> fields = [UiField.TextValue("status.lifecycle", _shutdown ? "shutdown" : _paused ? "paused" : _started ? "running" : "created"), UiField.TextValue("content.fingerprint", _content.ContentFingerprint), UiField.TextValue("content.source", string.Join(',', _content.Receipt.Sources.Select(source => source.Subject))), UiField.TextValue("selection.adventure", _selection.Value), UiField.NumberValue("revision", _revision), UiField.TextValue("readout.last", _lastReadout), UiField.NumberValue("tuning.viewDepth", 3), UiField.NumberValue("tuning.partyLimit", 4), UiField.NumberValue("tuning.targetLimit", 12)];
        if (_campaign is { } campaign)
        {
            CampaignSnapshot snapshot = campaign.Snapshot();
            AdventureDefinition adventure = _content.Adventures[snapshot.Adventure];
            EncounterDefinition? encounter = snapshot.ActiveEncounter is D20Id active ? _content.Catalog.Encounters[active] : null;
            _surface.Refresh(adventure, snapshot, encounter, _tactical, _session, _targetCursor);
            AddCampaign(fields, snapshot);
        }
        fields.Add(UiField.NumberValue("presentation.chunks", _surface.ChunkCount)); fields.Add(UiField.TextValue("presentation.source", _surface.Source)); fields.Add(UiField.TextValue("presentation.adventure", _surface.Adventure)); fields.Add(UiField.TextValue("presentation.board", _surface.Board)); fields.Add(UiField.TextValue("presentation.mode", _surface.Mode)); fields.Add(UiField.NumberValue("presentation.visibleDepth", _surface.VisibleDepth)); fields.Add(UiField.TextValue("presentation.tuning", _surface.Tuning.Readout));
        if (_tactical is { } tactical) AddTactical(fields, tactical);
        foreach ((string line, int index) in _log.TakeLast(12).Select((line, index) => (line, index))) fields.Add(UiField.TextValue($"log.{index}", line));
        _engine.Ui.PublishProjection(new UiProjection(_ui, ++_sequence, StructuredUiProjection.Object(fields)));
    }
    private void AddCampaign(List<UiField> fields, CampaignSnapshot state)
    {
        fields.Add(UiField.TextValue("campaign.phase", state.Phase.ToString())); fields.Add(UiField.NumberValue("campaign.revision", state.Revision)); fields.Add(UiField.TextValue("campaign.activeEncounter", state.ActiveEncounter?.Value ?? string.Empty)); fields.Add(UiField.TextValue("campaign.outcome", state.Outcome?.ToString() ?? string.Empty));
        if (state.Exploration is { } view) { fields.Add(UiField.TextValue("exploration.location", $"{view.Position.X},{view.Position.Y}")); fields.Add(UiField.TextValue("exploration.facing", view.Facing.ToString())); fields.Add(UiField.NumberValue("exploration.visibleDepth", view.View.Count)); fields.Add(UiField.TextValue("exploration.landmarks", string.Join(',', view.Landmarks.Select(value => value.Value)))); }
        if (_session is { } session) { fields.Add(UiField.TextValue("party.vitality", string.Join(';', _content.Adventures[_selection].Party.Select(id => $"{id}:{session.ReadVitality(session.OwnerEntity(id)).Current.Raw}")))); fields.Add(UiField.TextValue("party.loadout", string.Join(',', _content.Adventures[_selection].Items.Take(12).Select(value => value.Value)))); fields.Add(UiField.TextValue("rolls", string.Join(';', session.Receipts.TakeLast(4).Select(value => $"{value.D20}/{value.Damage}")))); }
    }
    private void AddTactical(List<UiField> fields, TacticalEncounter tactical)
    {
        TacticalMovementReadout movement = tactical.Movement;
        EncounterDefinition encounter = CurrentEncounter();
        D20Session session = RequireSession();
        EncounterFaction actorFaction = session.FactionOf(session.OwnerEntity(tactical.CurrentActor));
        D20Id[] actions = _content.Characters[tactical.CurrentActor].Actions.ToArray();
        D20Id[] targets = encounter.Roster.Where(value => value.Faction != actorFaction).Select(value => value.Character).ToArray();
        D20Id? selectedAction = actions.Length == 0 ? null : actions[Modulo(_actionCursor, actions.Length)];
        D20Id? selectedTarget = targets.Length == 0 ? null : targets[Modulo(_targetCursor, targets.Length)];
        fields.Add(UiField.TextValue("tactical.actor", tactical.CurrentActor.Value)); fields.Add(UiField.TextValue("tactical.actorFaction", actorFaction.ToString())); fields.Add(UiField.NumberValue("tactical.initiativeProgress", tactical.OppositionProgress)); fields.Add(UiField.TextValue("tactical.pendingReaction", tactical.PendingReaction is null ? string.Empty : tactical.PendingReaction.Defender.Value)); fields.Add(UiField.NumberValue("tactical.movement.budget", movement.Budget)); fields.Add(UiField.NumberValue("tactical.movement.remaining", movement.Remaining)); fields.Add(UiField.TextValue("tactical.positions", string.Join(';', movement.Participants.Select(value => $"{value.Id}:{session.FactionOf(value.Entity)}@{value.Position.X},{value.Position.Y}")))); fields.Add(UiField.TextValue("tactical.participants", string.Join(';', movement.Participants.Select(value => $"{value.Id}:{session.FactionOf(value.Entity)}:vitality={session.ReadVitality(value.Entity).Current.Raw}@{value.Position.X},{value.Position.Y}")))); fields.Add(UiField.TextValue("tactical.actions", string.Join(',', actions.Select(value => value.Value)))); fields.Add(UiField.TextValue("tactical.targets", string.Join(',', targets.Select(value => value.Value)))); fields.Add(UiField.TextValue("tactical.selectedAction", selectedAction?.Value ?? string.Empty)); fields.Add(UiField.TextValue("tactical.selectedTarget", selectedTarget?.Value ?? string.Empty)); fields.Add(UiField.TextValue("tactical.selectionReadout", tactical.PendingReaction is not null ? "reaction choice is required before progression" : actorFaction == EncounterFaction.Party ? "selected action and target are resolver-admitted on commit" : "Engine-admitted opposition is resolving automatically")); fields.Add(UiField.NumberValue("selection.party", _partyCursor)); fields.Add(UiField.NumberValue("selection.action", _actionCursor)); fields.Add(UiField.NumberValue("selection.target", _targetCursor));
    }
    private static int Modulo(int value, int length) => length == 0 ? throw new InvalidOperationException("No authored choice.") : ((value % length) + length) % length;
    private static string Bound(string value) => value.Length <= 120 ? value : value[..120];
    private void ThrowIfDisposed() => ObjectDisposedException.ThrowIf(_disposed, this);
}

internal sealed class D20Surface : IDisposable
{
    private readonly IEngineContext _engine; private readonly SpatialSession _spatial; private readonly IReadOnlyDictionary<uint, Material> _materials; private readonly VoxelScenePresentation _presentation; private readonly Camera _camera; private readonly Dictionary<VoxelAddress, uint> _occupied = [];
    private bool _disposed;
    private D20Surface(IEngineContext engine, SpatialSession spatial, IReadOnlyDictionary<uint, Material> materials, VoxelScenePresentation presentation, Camera camera, D20PresentationTuning tuning, IReadOnlyDictionary<VoxelAddress, uint> occupied, AdventureDefinition initial) { _engine = engine; _spatial = spatial; _materials = materials; _presentation = presentation; _camera = camera; Tuning = tuning; foreach ((VoxelAddress address, uint material) in occupied) _occupied.Add(address, material); Adventure = initial.Id.Value; Source = initial.Source.SourcePath; }
    public ulong ChunkCount { get; private set; }
    public string Source { get; private set; } = "unprojected";
    public string Adventure { get; private set; } = "";
    public string Board { get; private set; } = "";
    public string Mode { get; private set; } = "camp";
    public int VisibleDepth { get; private set; }
    public D20PresentationTuning Tuning { get; }
    public static D20Surface Create(IEngineContext engine, AdventureDefinition initial, D20PresentationTuning? tuning = null)
    {
        ArgumentNullException.ThrowIfNull(initial);
        D20PresentationTuning posture = tuning ?? new D20PresentationTuning(); posture.Validate();
        SpatialSession? spatial = null; var materials = new Dictionary<uint, Material>(); VoxelScenePresentation? presentation = null; Camera? camera = null;
        try
        {
            spatial = engine.Spatial.CreateSession(new SpatialSessionConfig(1.0, 16, VoxelSurfaceMode.GreedyCubes));
            materials.Add(posture.MaterialSlot, CreateMaterial(engine, new Color(posture.MaterialRed, posture.MaterialGreen, posture.MaterialBlue, 1), posture.MaterialRoughness));
            materials.Add(posture.PartyMaterialSlot, CreateMaterial(engine, new Color(.18f, .62f, .92f, 1), .5f));
            materials.Add(posture.OppositionMaterialSlot, CreateMaterial(engine, new Color(.82f, .18f, .2f, 1), .5f));
            materials.Add(posture.ActiveMaterialSlot, CreateMaterial(engine, new Color(.28f, .9f, .36f, 1), .35f));
            materials.Add(posture.SelectionMaterialSlot, CreateMaterial(engine, new Color(.95f, .78f, .18f, 1), .35f));
            var occupied = new Dictionary<VoxelAddress, uint>(); AddCamp(occupied, initial.Dungeon.Start, posture);
            VoxelSceneReadout scene = engine.Voxel.ReadScene(new VoxelSceneReadRequest(spatial));
            engine.Voxel.ApplyEdits(new VoxelEditTransaction(spatial, scene.SourceRevision, occupied.Select(value => new VoxelEdit(VoxelEditKind.Set, value.Key, value.Value)).ToArray()));
            presentation = engine.VoxelScenePresentation.ProjectScene(new ProjectVoxelSceneRequest(spatial, Bindings(materials, occupied)));
            camera = engine.CameraView.CreateCamera(ExplorationCamera(posture, initial.Dungeon.Start, initial.Dungeon.StartFacing));
            engine.CameraView.SetActiveCamera(camera);
            D20Surface result = new(engine, spatial, materials, presentation, camera, posture, occupied, initial); result.ChunkCount = engine.VoxelScenePresentation.RefreshScene(presentation).ChunkCount; spatial = null; materials = []; presentation = null; camera = null; return result;
        }
        catch (Exception error) { D20Disposal.DisposeAfterFailure(error, new IDisposable?[] { camera, presentation }.Concat(materials.Values).Append(spatial).ToArray()); throw; }
    }
    public void Refresh(AdventureDefinition adventure, CampaignSnapshot state, EncounterDefinition? encounter, TacticalEncounter? tactical, D20Session? session, int targetCursor)
    {
        ObjectDisposedException.ThrowIf(_disposed, this);
        ArgumentNullException.ThrowIfNull(adventure);
        var next = new Dictionary<VoxelAddress, uint>();
        if (state.Phase == CampaignPhase.Encounter && encounter is not null && tactical is not null && session is not null)
        {
            AddTactical(next, encounter, tactical, session, targetCursor, Tuning);
            SetCamera(TacticalCamera(Tuning, encounter.Board));
            Mode = "tactical-modal"; VisibleDepth = 0;
        }
        else if (state.Exploration is { } exploration)
        {
            AddExploration(next, exploration, Tuning);
            SetCamera(ExplorationCamera(Tuning, exploration.Position, exploration.Facing));
            Mode = "exploration-bounded"; VisibleDepth = exploration.View.Count;
        }
        else
        {
            AddCamp(next, adventure.Dungeon.Start, Tuning);
            SetCamera(ExplorationCamera(Tuning, adventure.Dungeon.Start, adventure.Dungeon.StartFacing));
            Mode = "camp"; VisibleDepth = 0;
        }
        VoxelEdit[] edits = _occupied.Keys.Except(next.Keys).Select(address => new VoxelEdit(VoxelEditKind.Clear, address, 0)).Concat(next.Where(value => !_occupied.TryGetValue(value.Key, out uint material) || material != value.Value).Select(value => new VoxelEdit(VoxelEditKind.Set, value.Key, value.Value))).ToArray();
        if (edits.Length != 0)
        {
            VoxelSceneReadout scene = _engine.Voxel.ReadScene(new VoxelSceneReadRequest(_spatial));
            _engine.Voxel.ApplyEdits(new VoxelEditTransaction(_spatial, scene.SourceRevision, edits));
            _occupied.Clear(); foreach ((VoxelAddress address, uint material) in next) _occupied.Add(address, material);
        }
        VoxelScenePresentationReadout view = _engine.VoxelScenePresentation.UpdateScene(new UpdateVoxelScenePresentationRequest(_presentation, Bindings(_materials, next)));
        ChunkCount = view.ChunkCount; Adventure = adventure.Id.Value; Board = encounter?.Id.Value ?? ""; Source = encounter is null ? adventure.Source.SourcePath : $"{adventure.Source.SourcePath};{encounter.Source.SourcePath}";
    }
    public void Dispose()
    {
        if (_disposed) return;
        _disposed = true;
        D20Disposal.DisposeAll(new IDisposable?[] { _camera, _presentation }.Concat(_materials.Values).Append(_spatial).ToArray());
    }

    private static Material CreateMaterial(IEngineContext engine, Color color, float roughness) => engine.Graphics.CreateMaterial(new MaterialRequest(color, new RenderResourceHandle(0), roughness, new Color(1, 1, 1, 1), Vector3.Zero, 0, false));
    private static VoxelSceneMaterialBinding[] Bindings(IReadOnlyDictionary<uint, Material> materials, IReadOnlyDictionary<VoxelAddress, uint> occupied) => occupied.Values.Distinct().OrderBy(slot => slot).Select(slot => new VoxelSceneMaterialBinding(slot, materials[slot])).ToArray();
    private void SetCamera(CameraDescriptor camera) { _engine.CameraView.UpdateCamera(new CameraUpdateRequest(_camera, camera)); _engine.CameraView.SetActiveCamera(_camera); }
    private static CameraDescriptor ExplorationCamera(D20PresentationTuning tuning, GridPosition position, DungeonFacing facing)
    {
        Vector3 forward = FacingVector(facing);
        return new CameraDescriptor(new CameraPose(new Vector3(position.X + .5f, tuning.ExplorationEyeHeight, position.Y + .5f), tuning.ExplorationCameraPitch, FacingYaw(facing)), CameraBasisMode.Explicit, new CameraBasis(Vector3.Normalize(forward + new Vector3(0, -.14f, 0)), Vector3.UnitX, Vector3.UnitY), new CameraProjection(CameraProjectionKind.Perspective, tuning.CameraFieldOfView, 0, .1f, tuning.CameraFar), new CameraViewport(0, 0, 1, 1));
    }
    private static CameraDescriptor TacticalCamera(D20PresentationTuning tuning, TacticalBoard board)
    {
        float extent = Math.Max(board.Width, board.Height) + 2;
        return new CameraDescriptor(new CameraPose(new Vector3(board.Width / 2f, tuning.TacticalCameraHeight, board.Height / 2f), tuning.TacticalCameraPitch, 0), CameraBasisMode.Explicit, new CameraBasis(Vector3.Normalize(new Vector3(0, -1, -.15f)), Vector3.UnitX, Vector3.UnitY), new CameraProjection(CameraProjectionKind.Orthographic, 0, extent, .1f, tuning.CameraFar), new CameraViewport(0, 0, 1, 1));
    }
    private static void AddCamp(IDictionary<VoxelAddress, uint> target, GridPosition start, D20PresentationTuning tuning)
    {
        for (int z = -1; z <= 1; z++) for (int x = -1; x <= 1; x++) Set(target, new VoxelAddress(start.X + x, tuning.DungeonFloorLayer, start.Y + z), tuning.MaterialSlot);
    }
    private static void AddExploration(IDictionary<VoxelAddress, uint> target, ExplorationReadout view, D20PresentationTuning tuning)
    {
        GridPosition cell = view.Position;
        foreach (VisibleDepth depth in view.View.OrderBy(value => value.Depth))
        {
            Set(target, new VoxelAddress(cell.X, tuning.DungeonFloorLayer, cell.Y), tuning.MaterialSlot);
            GridPosition left = Offset(cell, view.Facing, -1, 0); GridPosition right = Offset(cell, view.Facing, 1, 0); GridPosition front = Offset(cell, view.Facing, 0, 1);
            if (depth.LeftBlocked) Set(target, new VoxelAddress(left.X, tuning.DungeonWallLayer, left.Y), tuning.MaterialSlot);
            if (depth.RightBlocked) Set(target, new VoxelAddress(right.X, tuning.DungeonWallLayer, right.Y), tuning.MaterialSlot);
            if (depth.FrontBlocked) { Set(target, new VoxelAddress(front.X, tuning.DungeonWallLayer, front.Y), tuning.MaterialSlot); break; }
            cell = front;
        }
        Set(target, new VoxelAddress(view.Position.X, tuning.MarkerLayer, view.Position.Y), tuning.PartyMaterialSlot);
    }
    private static void AddTactical(IDictionary<VoxelAddress, uint> target, EncounterDefinition encounter, TacticalEncounter tactical, D20Session session, int targetCursor, D20PresentationTuning tuning)
    {
        AddRows(target, encounter.Board.Rows, tuning.TacticalFloorLayer, tuning.TacticalWallLayer, tuning.MaterialSlot);
        EncounterFaction actorFaction = session.FactionOf(session.OwnerEntity(tactical.CurrentActor));
        D20Id[] selectableTargets = encounter.Roster.Where(value => value.Faction != actorFaction).Select(value => value.Character).ToArray();
        D20Id? selectedTarget = selectableTargets.Length == 0 ? null : selectableTargets[Modulo(targetCursor, selectableTargets.Length)];
        foreach (TacticalParticipant participant in tactical.Participants)
        {
            uint material = participant.Id == tactical.CurrentActor ? tuning.ActiveMaterialSlot : participant.Id == selectedTarget ? tuning.SelectionMaterialSlot : session.FactionOf(participant.Entity) == EncounterFaction.Party ? tuning.PartyMaterialSlot : tuning.OppositionMaterialSlot;
            Set(target, new VoxelAddress(participant.Position.X, tuning.MarkerLayer, participant.Position.Y), material);
        }
    }
    private static void AddRows(IDictionary<VoxelAddress, uint> target, IReadOnlyList<string> rows, int floorLayer, int wallLayer, uint material)
    {
        for (int z = 0; z < rows.Count; z++) for (int x = 0; x < rows[z].Length; x++) { if (rows[z][x] is not ('.' or '#')) continue; Set(target, new VoxelAddress(x, floorLayer, z), material); if (rows[z][x] == '#') Set(target, new VoxelAddress(x, wallLayer, z), material); }
    }
    private static void Set(IDictionary<VoxelAddress, uint> target, VoxelAddress address, uint material) => target[address] = material;
    private static GridPosition Offset(GridPosition point, DungeonFacing facing, int lateral, int forward)
    {
        return facing switch { DungeonFacing.North => new(point.X + lateral, point.Y - forward), DungeonFacing.East => new(point.X + forward, point.Y + lateral), DungeonFacing.South => new(point.X - lateral, point.Y + forward), _ => new(point.X - forward, point.Y - lateral) };
    }
    private static Vector3 FacingVector(DungeonFacing facing) => facing switch { DungeonFacing.North => new(0, 0, -1), DungeonFacing.East => new(1, 0, 0), DungeonFacing.South => new(0, 0, 1), _ => new(-1, 0, 0) };
    private static float FacingYaw(DungeonFacing facing) => facing switch { DungeonFacing.North => 0, DungeonFacing.East => 90, DungeonFacing.South => 180, _ => -90 };
    private static int Modulo(int value, int length) => ((value % length) + length) % length;
}
