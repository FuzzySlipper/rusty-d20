using System.Collections.Immutable;
using System.Numerics;
using System.Text.Json;
using Rusty.Engine;
using Rusty.Engine.Entities;
using RustyD20.Core.Contract;
using RustyD20.Core.Rules;
using RustyD20.Core.Session;
using RustyD20.Core.Tactical;

namespace RustyD20.Core.Campaign;

public enum CampaignPhase { Camp, Exploration, Encounter, Outcome, AdventureComplete }
public enum EncounterResult { Victory, Defeat }
public enum ExplorationCommand { TurnLeft, TurnRight, StepForward, StepBackward, Interact }
public sealed record CampaignTuning(int ViewDepth = 3, int MaximumOppositionSettlements = 12, int MaximumReceipts = 128)
{
    public void Validate() { if (ViewDepth != 3 || MaximumOppositionSettlements is < 1 or > 12 || MaximumReceipts is < 1 or > 128) throw new CampaignException("Campaign tuning is outside the admitted product bounds."); }
}
public sealed record VisibleDepth(int Depth, bool FrontBlocked, bool LeftBlocked, bool RightBlocked);
public sealed record ExplorationReadout(GridPosition Position, DungeonFacing Facing, IReadOnlyList<VisibleDepth> View, IReadOnlyList<D20Id> Landmarks, CampaignTuning Tuning, IReadOnlyList<D20Id>? InspectedLandmarks = null);
public sealed record CampaignReceipt(string Kind, string Detail, ulong Revision);
public sealed record CampaignSnapshot(CampaignPhase Phase, D20Id Adventure, ExplorationReadout? Exploration, D20Id? ActiveEncounter, IReadOnlyList<D20Id> CompletedEncounters, EncounterResult? Outcome, IReadOnlyList<CampaignReceipt> Receipts, ulong Revision);
public sealed class CampaignException : InvalidOperationException { public CampaignException(string message) : base(message) { } }

/// <summary>Product policy asks this retained Engine-backed gateway for all navigation and occlusion facts; it never searches a grid locally.</summary>
public interface ICampaignSpatialGateway
{
    bool CanMove(GridPosition from, GridPosition to, IReadOnlySet<D20Id> openedDoors);
    bool IsOccluded(GridPosition from, GridPosition to, IReadOnlySet<D20Id> openedDoors);
}

/// <summary>Optional retained projection boundary used when a campaign changes an authored door.</summary>
public interface ICampaignSpatialDoorProjection
{
    void ReplaceOpenedDoors(IReadOnlySet<D20Id> openedDoors);
}

/// <summary>Creates the fresh retained spatial candidate used by campaign restore.</summary>
public delegate ICampaignSpatialGateway CampaignSpatialFactory(DungeonDefinition dungeon);

/// <summary>Thin adapter over the current public Engine Spatial navigation and collision APIs. Geometry is admitted once by product composition.</summary>
public sealed class EngineCampaignSpatialGateway : ICampaignSpatialGateway, ITacticalSpatialGateway, ICampaignSpatialDoorProjection, ITacticalSpatialComposition, IDisposable
{
    private readonly ISpatialService _spatial;
    private SpatialSession _session;
    private readonly DungeonDefinition _dungeon;
    private readonly SpatialSessionConfig _sessionConfig;
    private bool _ownsSession;
    private bool _disposed;

    public EngineCampaignSpatialGateway(
        ISpatialService spatial,
        DungeonDefinition dungeon,
        SpatialSession? session = null,
        SpatialSessionConfig? sessionConfig = null)
        : this(spatial, dungeon, session, sessionConfig, admitProjection: true)
    {
    }

    private EngineCampaignSpatialGateway(
        ISpatialService spatial,
        DungeonDefinition dungeon,
        SpatialSession? session,
        SpatialSessionConfig? sessionConfig,
        bool admitProjection)
    {
        _spatial = spatial ?? throw new ArgumentNullException(nameof(spatial));
        _dungeon = dungeon ?? throw new ArgumentNullException(nameof(dungeon));
        _sessionConfig = sessionConfig ?? new SpatialSessionConfig(1.0, 16, VoxelSurfaceMode.GreedyCubes);
        _ownsSession = session is null;
        try
        {
            _session = session ?? _spatial.CreateSession(_sessionConfig);
            if (admitProjection)
            {
                ReplaceProjection(_dungeon.Rows, StableGridId(_dungeon), _dungeon.Doors.Select(door => door.Position).ToHashSet(), useFreshSession: false);
            }
        }
        catch
        {
            if (_ownsSession)
            {
                _session?.Dispose();
            }

            throw;
        }
    }

    /// <summary>Compatibility constructor for callers that already admitted the geometry.</summary>
    public EngineCampaignSpatialGateway(ISpatialService spatial, SpatialSession session)
        : this(spatial, new DungeonDefinition("borrowed", D20Id.Parse("borrowed-wall"), 1, 1, ["."], new(0, 0), D20Id.Parse("borrowed-checkpoint"), DungeonFacing.North, [], [], [], [], [new(D20Id.Parse("borrowed-checkpoint"), new(0, 0), "borrowed", "borrowed")]), session, null, admitProjection: false)
    {
    }

    public SpatialSession Session => _session;
    public bool OwnsSession => _ownsSession;

    public bool CanMove(GridPosition from, GridPosition to, IReadOnlySet<D20Id> openedDoors)
    {
        ThrowIfDisposed();
        // Door traversal remains D20 policy, while the retained Engine navigation projection owns reachability and route admission.
        NavigationPathReadout route = _spatial.RequestNavigationPath(new NavigationPathRequest(_session, new PlanarNavCell(from.X, 0, from.Y), new PlanarNavCell(to.X, 0, to.Y), 256));
        return route.Outcome == NavigationPathOutcome.Reached;
    }
    public bool IsOccluded(GridPosition from, GridPosition to, IReadOnlySet<D20Id> openedDoors)
    {
        ThrowIfDisposed();
        SpatialHit hit = _spatial.CastSegment(new SpatialSegmentCastRequest(_session, new Vector3(from.X + .5f, .5f, from.Y + .5f), new Vector3(to.X + .5f, .5f, to.Y + .5f), new SpatialQueryFilter(uint.MaxValue, uint.MaxValue), ReadOnlyMemory<SpatialEntityCollider>.Empty, ReadOnlyMemory<ulong>.Empty, ReadOnlyMemory<SpatialEntityCollider>.Empty));
        return hit.Present;
    }

    public bool HasLineOfEffect(GridPosition from, GridPosition to) => !IsOccluded(from, to, new HashSet<D20Id>());

    public bool HasLegalRoute(GridPosition from, GridPosition to) => CanMove(from, to, new HashSet<D20Id>());

    /// <summary>
    /// Replaces both retained projections from the authored dungeon. Navigation is admitted from
    /// floor rows and collision is rebuilt from wall/closed-door cubes, so opening a door changes
    /// an Engine projection revision and hash rather than becoming an ignored product boolean.
    /// </summary>
    public void ReplaceOpenedDoors(IReadOnlySet<D20Id> openedDoors)
    {
        ThrowIfDisposed();
        ArgumentNullException.ThrowIfNull(openedDoors);
        if (openedDoors.Any(door => !_dungeon.Doors.Any(authored => authored.Id == door))) throw new ArgumentException("Opened-door projection contains an unknown authored door.", nameof(openedDoors));
        ReplaceProjection(_dungeon.Rows, StableGridId(_dungeon), _dungeon.Doors.Where(door => !openedDoors.Contains(door.Id)).Select(door => door.Position).ToHashSet());
    }

    public void ReplaceTacticalBoard(TacticalBoard board)
    {
        ThrowIfDisposed();
        ArgumentNullException.ThrowIfNull(board);
        ReplaceProjection(board.Rows, StableGridId(board), new HashSet<GridPosition>());
    }

    public NavigationReplaceReceipt LastNavigation { get; private set; }
    public CollisionReplaceReceipt LastCollision { get; private set; }

    public void Dispose()
    {
        if (_disposed)
        {
            return;
        }

        _disposed = true;
        if (_ownsSession)
        {
            _session.Dispose();
        }
    }

    private void ReplaceProjection(IReadOnlyList<string> rows, ulong gridId, IReadOnlySet<GridPosition> closedDoorCells, bool useFreshSession = true)
    {
        // ReplaceContentArtifact is reserved for an Engine ContentReference;
        // these C# authored rows have no such published artifact, so both raw
        // projections are staged on a fresh retained session before swapping.
        ArgumentNullException.ThrowIfNull(rows);
        ArgumentNullException.ThrowIfNull(closedDoorCells);
        PlanarNavCell[] walkable = rows
            .SelectMany((row, y) => row.Select((cell, x) => (cell, x, y)))
            .Where(value => value.cell == '.' && !closedDoorCells.Contains(new GridPosition(value.x, value.y)))
            .Select(value => new PlanarNavCell(value.x, 0, value.y))
            .ToArray();
        (StaticMeshAsset[] assets, Vector3[] vertices, Triangle[] triangles, StaticMeshInstance[] instances) = BuildCollision(rows, closedDoorCells);
        ValidateProjection(rows, closedDoorCells, walkable, assets, vertices, triangles, instances);
        SpatialSession candidate = useFreshSession ? _spatial.CreateSession(_sessionConfig) : _session;
        bool ownsCandidate = useFreshSession;
        try
        {
            NavigationReplaceReceipt navigation = _spatial.ReplaceNavigation(new NavigationReplaceRequest(candidate, new PlanarNavConfig(gridId, 1.0, 16, 1), walkable));
            CollisionReplaceReceipt collision = _spatial.ReplaceCollision(new CollisionReplaceRequest(candidate, assets, vertices, triangles, instances));
            if (useFreshSession)
            {
                SpatialSession previous = _session;
                bool previousOwned = _ownsSession;
                _session = candidate;
                _ownsSession = true;
                candidate = null!;
                ownsCandidate = false;
                if (previousOwned) previous.Dispose();
            }
            LastNavigation = navigation;
            LastCollision = collision;
        }
        catch
        {
            if (ownsCandidate) candidate.Dispose();
            throw;
        }
    }

    private static void ValidateProjection(IReadOnlyList<string> rows, IReadOnlySet<GridPosition> closedDoorCells, IReadOnlyList<PlanarNavCell> walkable, IReadOnlyList<StaticMeshAsset> assets, IReadOnlyList<Vector3> vertices, IReadOnlyList<Triangle> triangles, IReadOnlyList<StaticMeshInstance> instances)
    {
        if (rows.Count == 0 || rows.Any(row => row is null || row.Length == 0 || row.Any(cell => cell is not '#' and not '.'))) throw new ArgumentException("Spatial projection rows must be non-empty ASCII floor/wall rows.", nameof(rows));
        if (closedDoorCells.Any(cell => cell.X < 0 || cell.Y < 0 || cell.Y >= rows.Count || cell.X >= rows[cell.Y].Length || rows[cell.Y][cell.X] != '.')) throw new ArgumentException("Closed-door projection cells must be authored floor cells.", nameof(closedDoorCells));
        if (assets.Count == 0 && (vertices.Count != 0 || triangles.Count != 0 || instances.Count != 0) || assets.Count != 0 && (vertices.Count == 0 || triangles.Count == 0 || instances.Count == 0)) throw new ArgumentException("Spatial collision projection arrays are incomplete.", nameof(assets));
    }

    private static (StaticMeshAsset[] Assets, Vector3[] Vertices, Triangle[] Triangles, StaticMeshInstance[] Instances) BuildCollision(IReadOnlyList<string> rows, IReadOnlySet<GridPosition> closedDoorCells)
    {
        var vertices = new List<Vector3>();
        var triangles = new List<Triangle>();
        foreach ((string row, int y) in rows.Select((value, index) => (value, index)))
        {
            for (int x = 0; x < row.Length; x++)
            {
                if (row[x] == '#' || closedDoorCells.Contains(new GridPosition(x, y)))
                {
                    AddCube(vertices, triangles, x, y);
                }
            }
        }

        if (vertices.Count == 0)
        {
            return ([], [], [], []);
        }

        return (
            [new StaticMeshAsset(1, 0, checked((uint)vertices.Count), 0, checked((uint)triangles.Count))],
            vertices.ToArray(),
            triangles.ToArray(),
            [new StaticMeshInstance(1, 1, new Transform(Vector3.Zero, Quaternion.Identity, Vector3.One))]);
    }

    private static void AddCube(List<Vector3> vertices, List<Triangle> triangles, int x, int y)
    {
        uint start = checked((uint)vertices.Count);
        float left = x;
        float right = x + 1;
        float front = y;
        float back = y + 1;
        vertices.AddRange([
            new(left, 0, front), new(right, 0, front), new(right, 1, front), new(left, 1, front),
            new(left, 0, back), new(right, 0, back), new(right, 1, back), new(left, 1, back)]);
        triangles.AddRange([
            new(start, start + 1, start + 2), new(start, start + 2, start + 3),
            new(start + 4, start + 6, start + 5), new(start + 4, start + 7, start + 6),
            new(start, start + 4, start + 5), new(start, start + 5, start + 1),
            new(start + 3, start + 2, start + 6), new(start + 3, start + 6, start + 7),
            new(start, start + 3, start + 7), new(start, start + 7, start + 4),
            new(start + 1, start + 5, start + 6), new(start + 1, start + 6, start + 2)]);
    }

    private static ulong StableGridId(DungeonDefinition dungeon)
    {
        unchecked
        {
            ulong value = 1469598103934665603UL;
            foreach (char character in dungeon.Title)
            {
                value ^= character;
                value *= 1099511628211UL;
            }

            return value == 0 ? 1 : value;
        }
    }

    private static ulong StableGridId(TacticalBoard board)
    {
        unchecked
        {
            ulong value = 1469598103934665603UL;
            foreach (char character in string.Join("/", board.Rows))
            {
                value ^= character;
                value *= 1099511628211UL;
            }

            return value == 0 ? 1 : value;
        }
    }

    private void ThrowIfDisposed()
    {
        if (_disposed)
        {
            throw new ObjectDisposedException(nameof(EngineCampaignSpatialGateway));
        }
    }
}

/// <summary>One concrete C# campaign: it owns phase, ordered trigger policy, and observations; Engine owns the queried spatial mechanism.</summary>
public sealed class D20CampaignRuntime : IDisposable
{
    private readonly CompiledD20Content _content;
    private readonly AdventureDefinition _adventure;
    private readonly ICampaignSpatialGateway _spatial;
    private readonly CampaignTuning _tuning;
    private readonly HashSet<D20Id> _openedDoors = [];
    private readonly HashSet<D20Id> _collectedTreasures = [];
    private readonly HashSet<D20Id> _inspectedLandmarks = [];
    private readonly List<D20Id> _completed = [];
    private readonly List<CampaignReceipt> _receipts = [];
    private readonly D20Session? _session;
    private readonly bool _ownsSpatialGateway;
    private GridPosition _position;
    private DungeonFacing _facing;
    private D20Id _checkpoint;
    private D20Id? _active;
    private EncounterResult? _outcome;
    private bool _disposed;

    public D20CampaignRuntime(CompiledD20Content content, D20Id adventure, ICampaignSpatialGateway spatial, CampaignTuning? tuning = null, D20Session? session = null, bool ownsSpatialGateway = false, bool projectInitialSpatial = true)
    {
        _content = content ?? throw new ArgumentNullException(nameof(content));
        if (!_content.Adventures.TryGetValue(adventure, out _adventure!)) throw new CampaignException("The exact compiled adventure is not admitted.");
        _spatial = spatial ?? throw new ArgumentNullException(nameof(spatial)); _session = session; _ownsSpatialGateway = ownsSpatialGateway; _tuning = tuning ?? new CampaignTuning(); _tuning.Validate();
        _position = _adventure.Dungeon.Start; _facing = _adventure.Dungeon.StartFacing; _checkpoint = _adventure.Dungeon.StartCheckpoint;
        if (projectInitialSpatial) RestoreCampaignSpatialProjection();
    }
    public CampaignPhase Phase { get; private set; } = CampaignPhase.Camp;
    public ulong Revision { get; private set; }
    public string ContentFingerprint => _content.ContentFingerprint;
    public ICampaignSpatialGateway Spatial => _spatial;
    public IReadOnlyList<D20Id> CompletedEncounters => _completed;
    public void BeginExploration() { Require(CampaignPhase.Camp); RestoreCampaignSpatialProjection(); Phase = CampaignPhase.Exploration; Note("expedition", "The collapsed party enters exploration."); }
    public void Explore(ExplorationCommand command)
    {
        Require(CampaignPhase.Exploration);
        if (command == ExplorationCommand.TurnLeft) _facing = Turn(_facing, false);
        else if (command == ExplorationCommand.TurnRight) _facing = Turn(_facing, true);
        else if (command is ExplorationCommand.StepForward or ExplorationCommand.StepBackward)
        {
            GridPosition destination = Offset(_position, _facing, command == ExplorationCommand.StepForward);
            if (!_spatial.CanMove(_position, destination, _openedDoors)) throw new CampaignException("Engine navigation rejects that exploration step.");
            _position = destination; AdmitExactTrigger();
        }
        else Interact();
        if (Phase == CampaignPhase.Exploration) Note("exploration", $"Party at {_position.X},{_position.Y}, facing {_facing}.");
    }
    public void ResolveEncounter(EncounterResult result)
    {
        if (!Enum.IsDefined(result)) throw new CampaignException("Unknown encounter result.");
        Require(CampaignPhase.Encounter); D20Id encounter = _active ?? throw new CampaignException("Encounter phase has no active encounter.");
        if (Revision == ulong.MaxValue) throw new CampaignException("Campaign revision is exhausted before encounter resolution.");
        EncounterDefinition authored = _content.Modules.SelectMany(module => module.EncountersOrEmpty).Single(value => value.Id == encounter);
        if (result == EncounterResult.Victory)
        {
            if (authored.Victory.RewardItem is D20Id reward)
            {
                if (_session is null) throw new CampaignException("Encounter reward transfer requires an admitted D20 session.");
                RustyD20.Core.Rules.ItemDefinition rewardDefinition = _content.Modules
                    .SelectMany(module => module.ItemsOrEmpty)
                    .SingleOrDefault(value => value.Id == reward)
                    ?? throw new CampaignException($"Encounter reward item {reward} is not authored.");
                _session.RequireAdventureItemOwner(reward, rewardDefinition.Owner);
                _session.TransferAdventureItem(reward, _adventure.CampStorage);
            }
        }
        else
        {
            if (_session is null) throw new CampaignException("Defeat recovery requires an admitted D20 session.");
            if (authored.Defeat.RecoveryVitality is not int recovery) throw new CampaignException("Authored defeat has no recovery vitality.");
            EntityId[] party = _adventure.Party.Select(_session.OwnerEntity).ToArray();
            _session.ApplyDefeatRecovery(party, recovery);
        }

        _active = null; _outcome = result; Phase = CampaignPhase.Outcome;
        if (result == EncounterResult.Victory) { _completed.Add(encounter); Note("victory", $"{encounter} completed exactly once."); }
        else { DungeonCheckpoint checkpoint = _adventure.Dungeon.Checkpoints.Single(value => value.Id == _checkpoint); _position = checkpoint.Position; _facing = _adventure.Dungeon.StartFacing; Note("defeat", "Party recovered at its active checkpoint."); }
    }
    public void ContinueOutcome()
    {
        Require(CampaignPhase.Outcome);
        if (_outcome == EncounterResult.Defeat) { RestoreCampaignSpatialProjection(); Phase = CampaignPhase.Camp; Note("recovery", "Defeat returns the party to camp."); return; }
        if (_completed.Count == _adventure.Encounters.Count) { RestoreCampaignSpatialProjection(); Phase = CampaignPhase.AdventureComplete; Note("complete", _adventure.Completion.VictoryTitle); }
        else { RestoreCampaignSpatialProjection(); Phase = CampaignPhase.Exploration; Note("continue", "The next authored trigger is now eligible."); }
    }
    public CampaignSnapshot Snapshot()
    {
        ExplorationReadout? exploration = Phase is CampaignPhase.Exploration or CampaignPhase.Camp ? new(_position, _facing, View(), _adventure.Dungeon.Landmarks.Where(value => value.Position == _position).Select(value => value.Id).ToArray(), _tuning, _inspectedLandmarks.OrderBy(value => value.Value, StringComparer.Ordinal).ToArray()) : null;
        return new(Phase, _adventure.Id, exploration, _active, _completed.ToArray(), _outcome, _receipts.ToArray(), Revision);
    }
    public string EncodeSave()
    {
        var document = new CampaignSaveDocument(1, _content.ContentFingerprint, _adventure.Id.Value, Phase, _active?.Value, _outcome, _position, _facing, _checkpoint.Value, _completed.Select(value => value.Value).ToArray(), _openedDoors.Select(value => value.Value).OrderBy(value => value, StringComparer.Ordinal).ToArray(), _collectedTreasures.Select(value => value.Value).OrderBy(value => value, StringComparer.Ordinal).ToArray(), _inspectedLandmarks.Select(value => value.Value).OrderBy(value => value, StringComparer.Ordinal).ToArray(), _receipts.ToArray(), Revision);
        return JsonSerializer.Serialize(document);
    }
    public static D20CampaignRuntime Restore(string encoded, CompiledD20Content content, CampaignSpatialFactory spatialFactory, CampaignTuning? tuning = null, D20Session? session = null)
    {
        ArgumentNullException.ThrowIfNull(content);
        ArgumentNullException.ThrowIfNull(spatialFactory);
        ArgumentNullException.ThrowIfNull(encoded);
        CampaignTuning policy = tuning ?? new CampaignTuning();
        policy.Validate();
        ValidatedCampaignSave validated;
        try
        {
            validated = ValidateSave(encoded, content, policy, session);
        }
        catch (ArgumentException)
        {
            throw new CampaignException("Saved campaign identity is malformed.");
        }
        ICampaignSpatialGateway? gateway = null;
        D20CampaignRuntime? candidate = null;
        try
        {
            gateway = spatialFactory(validated.Adventure.Dungeon) ?? throw new CampaignException("Campaign spatial factory returned no candidate gateway.");
            candidate = new D20CampaignRuntime(content, validated.Adventure.Id, gateway, policy, session, ownsSpatialGateway: true, projectInitialSpatial: false);
            candidate._position = validated.Position;
            candidate._facing = validated.Facing;
            candidate._checkpoint = validated.Checkpoint;
            candidate._outcome = validated.Outcome;
            candidate._completed.AddRange(validated.Completed);
            candidate._openedDoors.UnionWith(validated.OpenedDoors);
            candidate._collectedTreasures.UnionWith(validated.CollectedTreasures);
            candidate._inspectedLandmarks.UnionWith(validated.InspectedLandmarks);
            candidate._receipts.AddRange(validated.Receipts);
            candidate.Revision = validated.Revision;
            candidate._active = validated.ActiveEncounter;
            candidate.RestoreCampaignSpatialProjection();
            if (!candidate._spatial.CanMove(candidate._position, candidate._position, candidate._openedDoors)) throw new CampaignException("Saved exploration topology facts are impossible in the retained Engine candidate.");
            candidate.Phase = validated.Phase;
            return candidate;
        }
        catch (ArgumentException)
        {
            candidate?.Dispose();
            if (candidate is null && gateway is IDisposable disposable) disposable.Dispose();
            throw new CampaignException("Saved campaign identity is malformed.");
        }
        catch
        {
            candidate?.Dispose();
            if (candidate is null && gateway is IDisposable disposable) disposable.Dispose();
            throw;
        }
    }

    private static ValidatedCampaignSave ValidateSave(string encoded, CompiledD20Content content, CampaignTuning tuning, D20Session? session)
    {
        CampaignSaveDocument? document;
        try
        {
            document = JsonSerializer.Deserialize<CampaignSaveDocument>(encoded, new JsonSerializerOptions { UnmappedMemberHandling = System.Text.Json.Serialization.JsonUnmappedMemberHandling.Disallow });
        }
        catch (JsonException error)
        {
            throw new CampaignException($"Invalid current C# save: {error.Message}");
        }

        if (document is null || document.Schema != 1 || !Enum.IsDefined(document.Phase) || !Enum.IsDefined(document.Facing) || document.Completed is null || document.OpenedDoors is null || document.CollectedTreasures is null || document.InspectedLandmarks is null || document.Receipts is null || document.Outcome is EncounterResult outcome && !Enum.IsDefined(outcome)) throw new CampaignException("Legacy, unknown, or malformed campaign save schema is rejected.");
        if (document.Fingerprint != content.ContentFingerprint) throw new CampaignException("Saved content fingerprint does not match the compiled adventure.");
        D20Id adventureId = D20Id.Parse(document.Adventure);
        if (!content.Adventures.TryGetValue(adventureId, out AdventureDefinition? adventure)) throw new CampaignException("Saved campaign adventure is not in the compiled content.");
        string[] completedValues = CanonicalStrings(document.Completed, "completed encounter");
        if (completedValues.Length > adventure.Encounters.Count || completedValues.Where((value, index) => value != adventure.Encounters[index].Value).Any()) throw new CampaignException("Saved encounter history is not the exact authored prefix.");
        string[] openedDoorValues = CanonicalStrings(document.OpenedDoors, "opened door");
        string[] collectedTreasureValues = CanonicalStrings(document.CollectedTreasures, "collected treasure");
        string[] inspectedLandmarkValues = CanonicalStrings(document.InspectedLandmarks, "inspected landmark");
        D20Id[] completed = completedValues.Select(D20Id.Parse).ToArray();
        HashSet<D20Id> openedDoors = openedDoorValues.Select(D20Id.Parse).ToHashSet();
        HashSet<D20Id> collectedTreasures = collectedTreasureValues.Select(D20Id.Parse).ToHashSet();
        HashSet<D20Id> inspectedLandmarks = inspectedLandmarkValues.Select(D20Id.Parse).ToHashSet();
        foreach (D20Id door in openedDoors)
        {
            DungeonDoor authored = adventure.Dungeon.Doors.SingleOrDefault(value => value.Id == door) ?? throw new CampaignException("Saved opened-door fact is unknown.");
            if (authored.RequiresTreasure is D20Id required && !collectedTreasures.Contains(required)) throw new CampaignException("Saved opened-door prerequisite is not collected.");
        }
        foreach (D20Id treasure in collectedTreasures)
        {
            DungeonTreasure authored = adventure.Dungeon.Treasures.SingleOrDefault(value => value.Id == treasure) ?? throw new CampaignException("Saved treasure fact is unknown.");
            if (session is null) throw new CampaignException("Saved treasure facts require the admitted restored D20 session.");
            ItemDefinition item = content.Modules.SelectMany(module => module.ItemsOrEmpty).SingleOrDefault(value => value.Id == authored.Item) ?? throw new CampaignException($"Saved treasure item {authored.Item} is not authored.");
            RequireCampOwnership(session, authored.Item, item.Owner, adventure.CampStorage);
        }
        foreach (D20Id landmark in inspectedLandmarks) if (!adventure.Dungeon.Landmarks.Any(value => value.Id == landmark)) throw new CampaignException("Saved inspected-landmark fact is unknown.");
        D20Id checkpoint = D20Id.Parse(document.Checkpoint);
        if (!adventure.Dungeon.Checkpoints.Any(value => value.Id == checkpoint)) throw new CampaignException("Saved checkpoint fact is unknown.");
        if (!Floor(adventure.Dungeon.Rows, document.Position)) throw new CampaignException("Saved exploration position is not an authored floor cell.");
        foreach (D20Id completedId in completed)
        {
            EncounterDefinition encounter = content.Modules.SelectMany(module => module.EncountersOrEmpty).Single(value => value.Id == completedId);
            if (encounter.Victory.RewardItem is D20Id reward)
            {
                if (session is null) throw new CampaignException("Completed reward history requires the admitted restored D20 session.");
                ItemDefinition item = content.Modules.SelectMany(module => module.ItemsOrEmpty).SingleOrDefault(value => value.Id == reward) ?? throw new CampaignException($"Saved encounter reward item {reward} is not authored.");
                RequireCampOwnership(session, reward, item.Owner, adventure.CampStorage);
            }
        }
        D20Id? active = document.ActiveEncounter is null ? null : D20Id.Parse(document.ActiveEncounter);
        if (document.Phase == CampaignPhase.Encounter)
        {
            D20Id expected = completed.Length < adventure.Encounters.Count ? adventure.Encounters[completed.Length] : throw new CampaignException("Saved encounter exceeds authored history.");
            if (active != expected || !adventure.Dungeon.Encounters.Any(value => value.Encounter == expected && value.Position == document.Position)) throw new CampaignException("Saved active encounter is not the exact admitted trigger.");
        }
        else if (active is not null) throw new CampaignException("Saved non-encounter phase has an active encounter.");
        if (document.Phase == CampaignPhase.Outcome && document.Outcome is null) throw new CampaignException("Saved outcome phase has no outcome.");
        if (document.Phase == CampaignPhase.AdventureComplete && document.Outcome != EncounterResult.Victory) throw new CampaignException("Saved adventure-complete phase has no victory outcome.");
        if (document.Phase == CampaignPhase.AdventureComplete && completed.Length != adventure.Encounters.Count) throw new CampaignException("Saved impossible campaign phase is rejected.");
        if (document.Receipts.Length > tuning.MaximumReceipts || document.Receipts.Any(receipt => receipt is null || receipt.Revision > document.Revision || string.IsNullOrWhiteSpace(receipt.Kind) || string.IsNullOrWhiteSpace(receipt.Detail)) || document.Receipts.Zip(document.Receipts.Skip(1)).Any(pair => pair.First.Revision >= pair.Second.Revision)) throw new CampaignException("Saved campaign receipts are inconsistent.");
        return new ValidatedCampaignSave(adventure, document.Phase, active, document.Outcome, document.Position, document.Facing, checkpoint, completed, openedDoors, collectedTreasures, inspectedLandmarks, document.Receipts, document.Revision);
    }

    private static string[] CanonicalStrings(IReadOnlyList<string> values, string label)
    {
        if (values.Any(value => string.IsNullOrWhiteSpace(value)) || values.Distinct(StringComparer.Ordinal).Count() != values.Count || !values.SequenceEqual(values.OrderBy(value => value, StringComparer.Ordinal))) throw new CampaignException($"Saved {label} facts contain duplicates or noncanonical order.");
        return values.ToArray();
    }

    private static void RequireCampOwnership(D20Session session, D20Id item, D20Id authoredOwner, D20Id campStorage)
    {
        try
        {
            session.RequireAdventureItemOwner(item, campStorage);
        }
        catch (D20SessionException error)
        {
            throw new CampaignException($"Saved item {item} is not in camp storage; authored source owner was {authoredOwner}: {error.Message}");
        }
    }
    private void AdmitExactTrigger()
    {
        if (_completed.Count >= _adventure.Encounters.Count) return;
        D20Id next = _adventure.Encounters[_completed.Count];
        DungeonEncounter? trigger = _adventure.Dungeon.Encounters.SingleOrDefault(value => value.Encounter == next && value.Position == _position);
        if (trigger is null) return; _active = next; Phase = CampaignPhase.Encounter; Note("encounter", $"Admitted next incomplete trigger {next}.");
    }
    private void Interact()
    {
        GridPosition destination = Offset(_position, _facing, true);
        DungeonDoor? door = _adventure.Dungeon.Doors.SingleOrDefault(value => DoorConnects(value, _position, destination));
        if (door is not null)
        {
            if (door.RequiresTreasure is D20Id required && !_collectedTreasures.Contains(required)) throw new CampaignException("The authored door is still locked.");
            if (_openedDoors.Contains(door.Id)) throw new CampaignException("The authored door is already open.");
            var nextOpenedDoors = new HashSet<D20Id>(_openedDoors) { door.Id };
            if (_spatial is ICampaignSpatialDoorProjection projection) projection.ReplaceOpenedDoors(nextOpenedDoors);
            _openedDoors.Add(door.Id);
            Note("door", door.Title); return;
        }
        DungeonTreasure? treasure = _adventure.Dungeon.Treasures.SingleOrDefault(value => value.Position == _position);
        if (treasure is not null) { if (_collectedTreasures.Contains(treasure.Id)) throw new CampaignException("Treasure was already transferred."); if (_session is null) throw new CampaignException("Treasure transfer requires an admitted D20 session."); RustyD20.Core.Rules.ItemDefinition treasureDefinition = _content.Modules.SelectMany(module => module.ItemsOrEmpty).SingleOrDefault(value => value.Id == treasure.Item) ?? throw new CampaignException($"Treasure item {treasure.Item} is not authored."); _session.RequireAdventureItemOwner(treasure.Item, treasureDefinition.Owner); _session.TransferAdventureItem(treasure.Item, _adventure.CampStorage); _collectedTreasures.Add(treasure.Id); Note("treasure", treasure.Title); return; }
        DungeonCheckpoint? checkpoint = _adventure.Dungeon.Checkpoints.SingleOrDefault(value => value.Position == _position);
        if (checkpoint is not null) { _checkpoint = checkpoint.Id; Phase = CampaignPhase.Camp; Note("checkpoint", checkpoint.Title); return; }
        DungeonLandmark? landmark = _adventure.Dungeon.Landmarks.SingleOrDefault(value => value.Position == _position);
        if (landmark is null) throw new CampaignException("There is no authored interaction at this location."); _inspectedLandmarks.Add(landmark.Id); Note("landmark", landmark.Title);
    }
    private IReadOnlyList<VisibleDepth> View()
    {
        var result = new List<VisibleDepth>(_tuning.ViewDepth); bool opaque = false; GridPosition cell = _position;
        for (int depth = 0; depth < _tuning.ViewDepth; depth++) { if (opaque) { result.Add(new(depth, true, true, true)); continue; } GridPosition front = Offset(cell, _facing, true); GridPosition left = Offset(cell, Turn(_facing, false), true); GridPosition right = Offset(cell, Turn(_facing, true), true); bool blocked = _spatial.IsOccluded(cell, front, _openedDoors); result.Add(new(depth, blocked, _spatial.IsOccluded(cell, left, _openedDoors), _spatial.IsOccluded(cell, right, _openedDoors))); opaque = blocked; cell = front; } return result;
    }
    private void Note(string kind, string detail) { Revision = checked(Revision + 1); _receipts.Add(new(kind, detail, Revision)); if (_receipts.Count > _tuning.MaximumReceipts) _receipts.RemoveAt(0); }
    private void Require(CampaignPhase phase) { if (Phase != phase) throw new CampaignException($"Command requires {phase}, current phase is {Phase}."); }
    private static DungeonFacing Turn(DungeonFacing facing, bool right) => (facing, right) switch { (DungeonFacing.North, true) or (DungeonFacing.South, false) => DungeonFacing.East, (DungeonFacing.East, true) or (DungeonFacing.West, false) => DungeonFacing.South, (DungeonFacing.South, true) or (DungeonFacing.North, false) => DungeonFacing.West, _ => DungeonFacing.North };
    private static GridPosition Offset(GridPosition point, DungeonFacing facing, bool forward) { int distance = forward ? 1 : -1; return facing switch { DungeonFacing.North => new(point.X, point.Y - distance), DungeonFacing.East => new(point.X + distance, point.Y), DungeonFacing.South => new(point.X, point.Y + distance), _ => new(point.X - distance, point.Y) }; }
    private static bool DoorConnects(DungeonDoor door, GridPosition from, GridPosition to) => (door.Position == from && Offset(door.Position, door.Facing, true) == to) || (door.Position == to && Offset(door.Position, door.Facing, true) == from);
    private void RestoreCampaignSpatialProjection() { if (_spatial is ICampaignSpatialDoorProjection projection) projection.ReplaceOpenedDoors(_openedDoors); }
    public void Dispose()
    {
        if (_disposed) return;
        _disposed = true;
        if (_ownsSpatialGateway && _spatial is IDisposable disposable) disposable.Dispose();
    }

    private static bool Floor(IReadOnlyList<string> rows, GridPosition point) => point.Y >= 0 && point.Y < rows.Count && point.X >= 0 && point.X < rows[point.Y].Length && rows[point.Y][point.X] == '.';
    private sealed record CampaignSaveDocument(int Schema, string Fingerprint, string Adventure, CampaignPhase Phase, string? ActiveEncounter, EncounterResult? Outcome, GridPosition Position, DungeonFacing Facing, string Checkpoint, string[] Completed, string[] OpenedDoors, string[] CollectedTreasures, string[] InspectedLandmarks, CampaignReceipt[] Receipts, ulong Revision);
    private sealed record ValidatedCampaignSave(AdventureDefinition Adventure, CampaignPhase Phase, D20Id? ActiveEncounter, EncounterResult? Outcome, GridPosition Position, DungeonFacing Facing, D20Id Checkpoint, IReadOnlyList<D20Id> Completed, IReadOnlySet<D20Id> OpenedDoors, IReadOnlySet<D20Id> CollectedTreasures, IReadOnlySet<D20Id> InspectedLandmarks, IReadOnlyList<CampaignReceipt> Receipts, ulong Revision);
}
