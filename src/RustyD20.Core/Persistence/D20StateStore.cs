using System.Buffers;
using System.Text.Json;
using Rusty.Engine;
using Rusty.Engine.Persistence;
using RustyD20.Core.Campaign;
using RustyD20.Core.Contract;
using RustyD20.Core.Rules;
using RustyD20.Core.Session;
using RustyD20.Core.Tactical;

namespace RustyD20.Core.Persistence;

internal static class D20PersistenceDisposal
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

    public static Exception DisposeAfterFailure(Exception primary, params IDisposable?[] owners)
    {
        var failures = new List<Exception> { primary };
        foreach (IDisposable? owner in owners)
        {
            if (owner is null) continue;
            try { owner.Dispose(); }
            catch (Exception error) { failures.Add(error); }
        }

        return failures.Count == 1 ? primary : new AggregateException(failures);
    }
}

/// <summary>Current C#-only durable envelope. It deliberately has no migration edges or legacy decoder.</summary>
public sealed record D20DurableState(int Schema, string ContentFingerprint, string Campaign, D20SessionSave Session, TacticalEncounterSave? Tactical, bool PendingReaction, ulong ProductRevision, IReadOnlyList<string> Log);
public sealed record D20RestoreCandidate(D20CampaignRuntime Campaign, D20Session Session, TacticalEncounter? Tactical, IReadOnlyList<string> Log, ulong Revision) : IDisposable
{
    public void Dispose()
    {
        D20PersistenceDisposal.DisposeAll(Campaign, Session);
    }
}

public sealed class D20DurableStateCodec : IProductStateCodec<D20DurableState>
{
    public const uint CurrentSchema = 2;
    public uint SchemaVersion => CurrentSchema;
    public void Encode(in D20DurableState state, IBufferWriter<byte> destination)
    {
        if (state.Schema != CurrentSchema || state.PendingReaction || HasPendingReaction(state.Tactical)) throw new CampaignException("Only current non-transient D20 state can be persisted.");
        destination.Write(JsonSerializer.SerializeToUtf8Bytes(state, D20PersistenceJsonContext.Default.D20DurableState));
    }
    public D20DurableState Decode(ReadOnlySpan<byte> payload)
    {
        try
        {
            D20DurableState? value = JsonSerializer.Deserialize(payload, D20PersistenceJsonContext.Default.D20DurableState);
            if (value is null || value.Schema != CurrentSchema || value.PendingReaction || HasPendingReaction(value.Tactical) || value.Session is null || value.Log is null || string.IsNullOrWhiteSpace(value.ContentFingerprint) || string.IsNullOrWhiteSpace(value.Campaign)) throw new CampaignException("Legacy, unknown, or transient D20 save is rejected.");
            return value;
        }
        catch (JsonException error) { throw new CampaignException($"D20 save is not a strict current document: {error.Message}"); }
    }

    private static bool HasPendingReaction(TacticalEncounterSave? tactical) => tactical is not null && (tactical.PendingDefender is not null || tactical.PendingAttacker is not null || tactical.PendingAction is not null || tactical.CommittedContinuation);
}

[System.Text.Json.Serialization.JsonSourceGenerationOptions(UnmappedMemberHandling = System.Text.Json.Serialization.JsonUnmappedMemberHandling.Disallow)]
[System.Text.Json.Serialization.JsonSerializable(typeof(D20DurableState))]
internal partial class D20PersistenceJsonContext : System.Text.Json.Serialization.JsonSerializerContext { }

/// <summary>Engine blob/revision composition only; product supplies the scope/key and owns all save meaning.</summary>
public sealed class D20EngineStateStore : IDisposable
{
    private readonly ProductStateStore<D20DurableState> _store;
    public D20EngineStateStore(IEngineContext engine, string productInstanceScope) => _store = new ProductStateStore<D20DurableState>(engine, productInstanceScope, new D20DurableStateCodec(), []);
    public PersistenceSaveReceipt Save(string key, string contentFingerprint, D20CampaignRuntime campaign, D20Session session, TacticalEncounter? tactical, IReadOnlyList<string> log, ulong revision, bool pendingReaction, PersistenceRevisionGuard guard = default, ulong expectedRevision = 0)
    {
        ArgumentNullException.ThrowIfNull(campaign); ArgumentNullException.ThrowIfNull(session);
        if (string.IsNullOrWhiteSpace(contentFingerprint) || !string.Equals(contentFingerprint, campaign.ContentFingerprint, StringComparison.Ordinal)) throw new CampaignException("Persistence content fingerprint does not match the compiled campaign content.");
        if (tactical?.PendingReaction is not null || tactical?.HasCommittedContinuation == true) pendingReaction = true;
        D20DurableState state = new(checked((int)D20DurableStateCodec.CurrentSchema), contentFingerprint, campaign.EncodeSave(), session.CaptureSave(), tactical?.CaptureSave(), pendingReaction, revision, log.ToArray());
        if (pendingReaction) throw new CampaignException("Pending reaction custody rejects before Engine persistence writes bytes.");
        return _store.Save(key, state, guard, expectedRevision);
    }
    public ProductStateLoad<D20RestoreCandidate> Load(string key, Rules.CompiledD20Content content, CampaignSpatialFactory spatialFactory, SessionTuning? tuning = null, ScopedSeededRollAdapter? seededRolls = null)
    {
        ArgumentNullException.ThrowIfNull(content);
        ArgumentNullException.ThrowIfNull(spatialFactory);
        ProductStateLoad<D20DurableState> loaded = _store.Load(key);
        if (!loaded.Present || loaded.State is null) return new ProductStateLoad<D20RestoreCandidate>(false, loaded.Revision, null);
        D20DurableState state = loaded.State;
        if (state.ContentFingerprint != content.ContentFingerprint) throw new CampaignException("Saved D20 content fingerprint does not match the compiled content.");
        if (state.Session.Adventure is not D20Id sessionAdventure || !content.Adventures.ContainsKey(sessionAdventure)) throw new CampaignException("Saved session has no admitted adventure identity.");

        D20Session? session = null;
        D20CampaignRuntime? campaign = null;
        try
        {
            // Both restore calls construct candidates before the caller receives a replacement aggregate.
            session = D20Session.Restore(content, state.Session, tuning, seededRolls);
            campaign = D20CampaignRuntime.Restore(state.Campaign, content, spatialFactory, session: session);
            CampaignSnapshot snapshot = campaign.Snapshot();
            if (snapshot.Adventure != sessionAdventure) throw new CampaignException("Saved campaign/session adventure identities disagree.");
            AdventureDefinition adventure = content.Adventures[snapshot.Adventure];
            foreach (D20Id completed in snapshot.CompletedEncounters)
            {
                EncounterDefinition encounter = content.Catalog.Encounters.TryGetValue(completed, out EncounterDefinition? admittedCompleted) ? admittedCompleted : throw new CampaignException("Saved completed encounter is not in the compiled definition catalog.");
                if (encounter.Victory.RewardItem is D20Id reward) session.RequireAdventureItemOwner(reward, adventure.CampStorage);
            }

            TacticalEncounter? tactical = null;
            if (snapshot.Phase == CampaignPhase.Encounter)
            {
                if (state.Tactical is null || snapshot.ActiveEncounter is not D20Id active) throw new CampaignException("Saved encounter phase has no tactical aggregate.");
                EncounterDefinition encounter = content.Catalog.Encounters.TryGetValue(active, out EncounterDefinition? admittedActive) ? admittedActive : throw new CampaignException("Saved active encounter is not in the compiled definition catalog.");
                ValidateTacticalClosure(session, encounter, state.Tactical);
                if (campaign.Spatial is not ITacticalSpatialGateway tacticalSpatial) throw new CampaignException("The fresh campaign spatial candidate does not expose tactical Engine queries.");
                tactical = TacticalEncounter.Restore(session, tacticalSpatial, state.Tactical, encounter.Board);
            }
            else if (state.Tactical is not null) throw new CampaignException("Saved non-encounter phase has a tactical aggregate.");

            return new ProductStateLoad<D20RestoreCandidate>(true, loaded.Revision, new D20RestoreCandidate(campaign, session, tactical, state.Log.ToArray(), state.ProductRevision));
        }
        catch (Exception error)
        {
            throw D20PersistenceDisposal.DisposeAfterFailure(error, campaign, session);
        }
    }
    public void Dispose() => _store.Dispose();

    private static void ValidateTacticalClosure(D20Session session, EncounterDefinition encounter, TacticalEncounterSave save)
    {
        if (save.Participants.Count != encounter.Roster.Count || save.Participants.Select(value => value.Id).Distinct().Count() != save.Participants.Count) throw new CampaignException("Saved tactical roster is not the authored encounter closure.");
        foreach (EncounterParticipant authored in encounter.Roster)
        {
            TacticalParticipant participant = save.Participants.SingleOrDefault(value => value.Id == authored.Character) ?? throw new CampaignException("Saved tactical participant is missing from the authored roster.");
            if (participant.Entity != session.OwnerEntity(authored.Character) || session.FactionOf(participant.Entity) != authored.Faction) throw new CampaignException("Saved tactical participant identity or faction disagrees with authored content.");
        }
    }
}
