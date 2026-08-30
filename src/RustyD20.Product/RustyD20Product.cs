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
    int DungeonFloorLayer = 0,
    int DungeonWallLayer = 1,
    int TacticalFloorLayer = 0,
    int TacticalWallLayer = 1,
    int TacticalOffsetX = 32,
    float MaterialRed = .55f,
    float MaterialGreen = .25f,
    float MaterialBlue = .08f,
    float MaterialRoughness = .8f,
    float CameraX = 6,
    float CameraY = 9,
    float CameraZ = 10,
    float CameraPitch = -25,
    float CameraFieldOfView = 65,
    float CameraFar = 64)
{
    public void Validate()
    {
        if (MaterialSlot == 0 || DungeonFloorLayer < 0 || DungeonWallLayer <= DungeonFloorLayer || TacticalFloorLayer < 0 || TacticalWallLayer <= TacticalFloorLayer || TacticalOffsetX < 1 || MaterialRed is < 0 or > 1 || MaterialGreen is < 0 or > 1 || MaterialBlue is < 0 or > 1 || MaterialRoughness is < 0 or > 1 || CameraFieldOfView is < 1 or > 179 || CameraFar <= 0)
            throw new ArgumentOutOfRangeException(nameof(D20PresentationTuning), "Presentation tuning is outside the admitted product bounds.");
    }

    public string Readout => $"material={MaterialSlot};dungeon={DungeonFloorLayer}/{DungeonWallLayer};tactical={TacticalFloorLayer}/{TacticalWallLayer}@{TacticalOffsetX};camera={CameraX},{CameraY},{CameraZ}:{CameraPitch}/{CameraFieldOfView}/{CameraFar}";
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
        EncounterDefinition encounter = _content.Modules.SelectMany(module => module.EncountersOrEmpty).Single(value => value.Id == active);
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
        EncounterDefinition encounter = CurrentEncounter();
        D20Session session = RequireSession();
        bool oppositionAlive = encounter.Roster.Where(row => row.Faction == EncounterFaction.Opposition).Any(row => session.IsLiving(session.OwnerEntity(row.Character)));
        bool partyAlive = encounter.Roster.Where(row => row.Faction == EncounterFaction.Party).Any(row => session.IsLiving(session.OwnerEntity(row.Character)));
        if (!oppositionAlive || !partyAlive)
        {
            RequireCampaign().ResolveEncounter(oppositionAlive ? EncounterResult.Defeat : EncounterResult.Victory);
            _tactical = null;
            Note(oppositionAlive ? "outcome:defeat" : "outcome:victory");
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
        return _content.Modules.SelectMany(module => module.EncountersOrEmpty).Single(value => value.Id == id);
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
            EncounterDefinition? encounter = snapshot.ActiveEncounter is D20Id active ? _content.Modules.SelectMany(module => module.EncountersOrEmpty).Single(value => value.Id == active) : null;
            _surface.Refresh(adventure, encounter);
            AddCampaign(fields, snapshot);
        }
        fields.Add(UiField.NumberValue("presentation.chunks", _surface.ChunkCount)); fields.Add(UiField.TextValue("presentation.source", _surface.Source)); fields.Add(UiField.TextValue("presentation.adventure", _surface.Adventure)); fields.Add(UiField.TextValue("presentation.board", _surface.Board)); fields.Add(UiField.TextValue("presentation.tuning", _surface.Tuning.Readout));
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
        fields.Add(UiField.TextValue("tactical.actor", tactical.CurrentActor.Value)); fields.Add(UiField.NumberValue("tactical.initiativeProgress", tactical.OppositionProgress)); fields.Add(UiField.TextValue("tactical.pendingReaction", tactical.PendingReaction is null ? string.Empty : tactical.PendingReaction.Defender.Value)); fields.Add(UiField.NumberValue("selection.party", _partyCursor)); fields.Add(UiField.NumberValue("selection.action", _actionCursor)); fields.Add(UiField.NumberValue("selection.target", _targetCursor));
    }
    private static int Modulo(int value, int length) => length == 0 ? throw new InvalidOperationException("No authored choice.") : ((value % length) + length) % length;
    private static string Bound(string value) => value.Length <= 120 ? value : value[..120];
    private void ThrowIfDisposed() => ObjectDisposedException.ThrowIf(_disposed, this);
}

internal sealed class D20Surface : IDisposable
{
    private readonly IEngineContext _engine; private readonly SpatialSession _spatial; private readonly Material _material; private readonly VoxelScenePresentation _presentation; private readonly Camera _camera; private readonly HashSet<VoxelAddress> _occupied = [];
    private bool _disposed;
    private D20Surface(IEngineContext engine, SpatialSession spatial, Material material, VoxelScenePresentation presentation, Camera camera, D20PresentationTuning tuning, HashSet<VoxelAddress> occupied, AdventureDefinition initial) { _engine = engine; _spatial = spatial; _material = material; _presentation = presentation; _camera = camera; Tuning = tuning; _occupied.UnionWith(occupied); Adventure = initial.Id.Value; Source = initial.Source.SourcePath; }
    public ulong ChunkCount { get; private set; }
    public string Source { get; private set; } = "unprojected";
    public string Adventure { get; private set; } = "";
    public string Board { get; private set; } = "";
    public D20PresentationTuning Tuning { get; }
    public static D20Surface Create(IEngineContext engine, AdventureDefinition initial, D20PresentationTuning? tuning = null)
    {
        ArgumentNullException.ThrowIfNull(initial);
        D20PresentationTuning posture = tuning ?? new D20PresentationTuning(); posture.Validate();
        SpatialSession? spatial = null; Material? material = null; VoxelScenePresentation? presentation = null; Camera? camera = null;
        try
        {
            spatial = engine.Spatial.CreateSession(new SpatialSessionConfig(1.0, 16, VoxelSurfaceMode.GreedyCubes));
            material = engine.Appearance.CreateMaterial(new MaterialRequest(new Color(posture.MaterialRed, posture.MaterialGreen, posture.MaterialBlue, 1), new RenderResourceHandle(0), posture.MaterialRoughness, new Color(1, 1, 1, 1), Vector3.Zero, 0, false));
            var occupied = new HashSet<VoxelAddress>(); AddRows(occupied, initial.Dungeon.Rows, 0, posture.DungeonFloorLayer, posture.DungeonWallLayer);
            VoxelSceneReadout scene = engine.Voxel.ReadScene(new VoxelSceneReadRequest(spatial));
            engine.Voxel.ApplyEdits(new VoxelEditTransaction(spatial, scene.SourceRevision, occupied.Select(address => new VoxelEdit(VoxelEditKind.Set, address, posture.MaterialSlot)).ToArray()));
            presentation = engine.VoxelScenePresentation.ProjectScene(new ProjectVoxelSceneRequest(spatial, new VoxelSceneMaterialBinding[] { new(posture.MaterialSlot, material) }));
            camera = engine.CameraView.CreateCamera(new CameraDescriptor(new CameraPose(new Vector3(posture.CameraX, posture.CameraY, posture.CameraZ), posture.CameraPitch, 0), CameraBasisMode.Explicit, new CameraBasis(Vector3.Normalize(new Vector3(0, -0.5f, -1)), Vector3.UnitX, Vector3.UnitY), new CameraProjection(CameraProjectionKind.Perspective, posture.CameraFieldOfView, 0, .1f, posture.CameraFar), new CameraViewport(0, 0, 1, 1)));
            engine.CameraView.SetActiveCamera(camera);
            D20Surface result = new(engine, spatial, material, presentation, camera, posture, occupied, initial); result.ChunkCount = engine.VoxelScenePresentation.RefreshScene(presentation).ChunkCount; spatial = null; material = null; presentation = null; camera = null; return result;
        }
        catch (Exception error) { D20Disposal.DisposeAfterFailure(error, camera, presentation, material, spatial); throw; }
    }
    public void Refresh(AdventureDefinition adventure, EncounterDefinition? encounter)
    {
        ObjectDisposedException.ThrowIf(_disposed, this);
        ArgumentNullException.ThrowIfNull(adventure);
        var next = new HashSet<VoxelAddress>();
        AddRows(next, adventure.Dungeon.Rows, 0, Tuning.DungeonFloorLayer, Tuning.DungeonWallLayer);
        if (encounter is not null) AddRows(next, encounter.Board.Rows, Tuning.TacticalOffsetX, Tuning.TacticalFloorLayer, Tuning.TacticalWallLayer);
        VoxelEdit[] edits = _occupied.Except(next).Select(address => new VoxelEdit(VoxelEditKind.Clear, address, 0)).Concat(next.Except(_occupied).Select(address => new VoxelEdit(VoxelEditKind.Set, address, Tuning.MaterialSlot))).ToArray();
        if (edits.Length != 0)
        {
            VoxelSceneReadout scene = _engine.Voxel.ReadScene(new VoxelSceneReadRequest(_spatial));
            _engine.Voxel.ApplyEdits(new VoxelEditTransaction(_spatial, scene.SourceRevision, edits));
            _occupied.Clear(); _occupied.UnionWith(next);
        }
        VoxelScenePresentationReadout view = _engine.VoxelScenePresentation.RefreshScene(_presentation);
        ChunkCount = view.ChunkCount; Adventure = adventure.Id.Value; Board = encounter?.Id.Value ?? ""; Source = encounter is null ? adventure.Source.SourcePath : $"{adventure.Source.SourcePath};{encounter.Source.SourcePath}";
    }
    public void Dispose()
    {
        if (_disposed) return;
        _disposed = true;
        D20Disposal.DisposeAll(_camera, _presentation, _material, _spatial);
    }

    private static void AddRows(ISet<VoxelAddress> target, IReadOnlyList<string> rows, int offsetX, int floorLayer, int wallLayer)
    {
        for (int z = 0; z < rows.Count; z++)
        {
            for (int x = 0; x < rows[z].Length; x++)
            {
                if (rows[z][x] is not ('.' or '#')) continue;
                target.Add(new VoxelAddress(checked(x + offsetX), floorLayer, z));
                if (rows[z][x] == '#') target.Add(new VoxelAddress(checked(x + offsetX), wallLayer, z));
            }
        }
    }
}
