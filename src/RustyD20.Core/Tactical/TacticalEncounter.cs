using Rusty.Engine.Entities;
using Rusty.Engine.Mechanics;
using RustyD20.Core.Contract;
using RustyD20.Core.Rules;
using RustyD20.Core.Session;

namespace RustyD20.Core.Tactical;

public interface ITacticalSpatialGateway { bool HasLineOfEffect(GridPosition from, GridPosition to); bool HasLegalRoute(GridPosition from, GridPosition to); }
public interface ITacticalSpatialComposition { void ReplaceTacticalBoard(TacticalBoard board); }
public sealed record TacticalParticipant(D20Id Id, EntityId Entity, int Initiative, GridPosition Position);
public sealed record ReactionPrompt(D20Id Defender, D20Id Attacker, D20Id Action, ActionPreview Preview);
public sealed record TacticalEncounterSave(IReadOnlyList<TacticalParticipant> Participants, IReadOnlyList<D20Id> Initiative, int Cursor, D20Id? PendingDefender, D20Id? PendingAttacker, D20Id? PendingAction, ulong OppositionProgress, bool CommittedContinuation = false);
public sealed class TacticalException : InvalidOperationException { public TacticalException(string message) : base(message) { } }

/// <summary>Turn and reaction policy only; faction, life, damage and effects remain canonical D20Session/Engine facts.</summary>
public sealed class TacticalEncounter
{
    private enum ReactionStage { PartyAction, OppositionAction }
    private sealed record CommittedContinuation(bool AdvancePending);

    private readonly D20Session _session; private readonly ITacticalSpatialGateway _spatial; private readonly Dictionary<D20Id, TacticalParticipant> _participants; private readonly List<D20Id> _order; private int _cursor;
    private CommittedContinuation? _continuation;
    private ReactionStage? _pendingReactionStage;
    private ulong _pendingOppositionProgress;
    public TacticalEncounter(D20Session session, ITacticalSpatialGateway spatial, IEnumerable<TacticalParticipant> participants, TacticalBoard? board = null)
    {
        _session = session ?? throw new ArgumentNullException(nameof(session));
        _spatial = spatial ?? throw new ArgumentNullException(nameof(spatial));
        ArgumentNullException.ThrowIfNull(participants);
        TacticalParticipant[] entries = participants.ToArray();
        if (entries.Length is < 1 or > 12 || entries.Select(value => value.Id).Distinct().Count() != entries.Length || entries.Select(value => value.Entity).Distinct().Count() != entries.Length) throw new TacticalException("Encounter participant identity or bound is invalid.");
        _participants = entries.ToDictionary(value => value.Id);
        foreach (TacticalParticipant participant in entries)
        {
            if (!_session.IsParticipant(participant.Entity) || _session.FactionOf(participant.Entity) is not (EncounterFaction.Party or EncounterFaction.Opposition)) throw new TacticalException("Encounter participant is not admitted by the session.");
        }

        if (board is not null && _spatial is ITacticalSpatialComposition composition) composition.ReplaceTacticalBoard(board);
        _order = _participants.Values.OrderByDescending(value => value.Initiative).ThenBy(value => value.Entity.Value).Select(value => value.Id).ToList();
        Skip();
    }
    public D20Id CurrentActor => _order[_cursor]; public ReactionPrompt? PendingReaction { get; private set; } public ulong OppositionProgress { get; private set; }
    /// <summary>True when a committed action still owns automatic progression after a fallible boundary.</summary>
    public bool HasCommittedContinuation => _continuation is not null;

    public void PartyAction(D20Id actor, D20Id target, D20Id action, OperationId operation)
    {
        if (PendingReaction is not null || _continuation is not null || CurrentActor != actor || Faction(actor) != EncounterFaction.Party) throw new TacticalException("Only current party actor may act.");
        ActionPreview preview = Preview(actor, target, action, operation);
        if (CanReact(Participant(target).Entity))
        {
            PendingReaction = new(target, actor, action, preview);
            _pendingReactionStage = ReactionStage.PartyAction;
            return;
        }

        _session.ApplyAction(preview);
        ContinueAfterCommittedAction();
    }

    public void ResolveReaction(D20Id reaction, bool choose)
    {
        ReactionPrompt prompt = PendingReaction ?? throw new TacticalException("No pending reaction.");
        ReactionStage stage = _pendingReactionStage ?? throw new TacticalException("Pending reaction stage is missing.");
        ulong pendingOppositionProgress = _pendingOppositionProgress;
        _session.ResolveReaction(prompt.Preview, choose ? reaction : null);
        if (stage == ReactionStage.OppositionAction) OppositionProgress = pendingOppositionProgress;
        PendingReaction = null;
        _pendingReactionStage = null;
        _pendingOppositionProgress = 0;
        ContinueAfterCommittedAction();
    }

    public void SettleOpposition()
    {
        if (PendingReaction is not null) throw new TacticalException("Pending reaction must be resolved before opposition settlement.");
        _continuation ??= new CommittedContinuation(false);
        SettleOppositionCore();
    }

    /// <summary>Retries only the uncommitted automatic continuation; the preceding action and reaction are never replayed.</summary>
    public void ResumeAutomaticProgression()
    {
        if (PendingReaction is not null) throw new TacticalException("Resolve the pending reaction before automatic progression.");
        if (_continuation is null) throw new TacticalException("No automatic progression is awaiting retry.");
        FinishAdvance();
        SettleOppositionCore();
    }

    public TacticalEncounterSave CaptureSave() => new(_participants.Values.OrderBy(value => value.Entity.Value).ToArray(), _order.ToArray(), _cursor, PendingReaction?.Defender, PendingReaction?.Attacker, PendingReaction?.Action, OppositionProgress, _continuation is not null);
    public static TacticalEncounter Restore(D20Session session, ITacticalSpatialGateway spatial, TacticalEncounterSave save, TacticalBoard? board = null)
    {
        ArgumentNullException.ThrowIfNull(session);
        ArgumentNullException.ThrowIfNull(spatial);
        ArgumentNullException.ThrowIfNull(save);
        if (save.Participants is null || save.Initiative is null || save.Participants.Count is < 1 or > 12 || save.PendingDefender is not null || save.PendingAttacker is not null || save.PendingAction is not null || save.CommittedContinuation || save.OppositionProgress > 12) throw new TacticalException("Invalid or transient tactical save.");
        if (save.Participants.Select(value => value.Id).Distinct().Count() != save.Participants.Count || save.Participants.Select(value => value.Entity).Distinct().Count() != save.Participants.Count) throw new TacticalException("Saved tactical participant identities are duplicated.");
        if (board is not null)
        {
            if (board.Placements.Count != save.Participants.Count || board.Placements.Select(value => value.Character).Distinct().Count() != board.Placements.Count) throw new TacticalException("Saved tactical board closure is invalid.");
            foreach (TacticalParticipant participant in save.Participants)
            {
                TacticalPlacement placement = board.Placements.SingleOrDefault(value => value.Character == participant.Id) ?? throw new TacticalException("Saved tactical participant is missing from the authored board.");
                if (placement.Position != participant.Position) throw new TacticalException("Saved tactical participant placement disagrees with the authored board.");
            }
        }
        D20Id[] expectedOrder = save.Participants.OrderByDescending(value => value.Initiative).ThenBy(value => value.Entity.Value).Select(value => value.Id).ToArray();
        if (save.Initiative.Count != expectedOrder.Length || save.Initiative.Where((value, index) => value != expectedOrder[index]).Any() || save.Cursor < 0 || save.Cursor >= expectedOrder.Length) throw new TacticalException("Invalid initiative save.");
        TacticalParticipant current = save.Participants.Single(value => value.Id == save.Initiative[save.Cursor]);
        if (!session.IsLiving(current.Entity)) throw new TacticalException("Saved tactical cursor points at a dead participant.");
        var candidate = new TacticalEncounter(session, spatial, save.Participants);
        candidate._cursor = save.Cursor; candidate.OppositionProgress = save.OppositionProgress;
        if (board is not null && spatial is ITacticalSpatialComposition composition) composition.ReplaceTacticalBoard(board);
        return candidate;
    }

    private void ContinueAfterCommittedAction()
    {
        _continuation = new CommittedContinuation(true);
        FinishAdvance();
        SettleOppositionCore();
    }

    private void FinishAdvance()
    {
        if (_continuation is not { AdvancePending: true }) return;
        Advance();
        _continuation = new CommittedContinuation(false);
    }

    private void SettleOppositionCore()
    {
        if (PendingReaction is not null) throw new TacticalException("Pending reaction custody must be resolved before settlement.");
        for (int settled = 0; settled < 12 && Faction(CurrentActor) == EncounterFaction.Opposition; settled++)
        {
            D20Id target = _participants.Values.Where(value => _session.IsLiving(value.Entity) && _session.FactionOf(value.Entity) == EncounterFaction.Party).OrderBy(value => value.Entity.Value).Select(value => value.Id).FirstOrDefault();
            if (target == default)
            {
                _continuation = null;
                return;
            }

            ulong nextProgress = checked(OppositionProgress + 1);
            D20Id action = D20Id.Parse("disrupt");
            ActionPreview preview = Preview(CurrentActor, target, action, OperationId.Parse($"opposition-{nextProgress}"));
            if (CanReact(Participant(target).Entity))
            {
                PendingReaction = new(target, CurrentActor, action, preview);
                _pendingReactionStage = ReactionStage.OppositionAction;
                _pendingOppositionProgress = nextProgress;
                return;
            }

            _session.ApplyAction(preview);
            OppositionProgress = nextProgress;
            _continuation = new CommittedContinuation(true);
            FinishAdvance();
        }

        if (Faction(CurrentActor) != EncounterFaction.Opposition) _continuation = null;
    }
    private ActionPreview Preview(D20Id actor, D20Id target, D20Id action, OperationId operation) { TacticalParticipant source = Participant(actor), destination = Participant(target); if (!_spatial.HasLegalRoute(source.Position, destination.Position) || !_spatial.HasLineOfEffect(source.Position, destination.Position)) throw new TacticalException("Engine retained spatial projection rejects target route or line of effect."); return _session.PreviewAction(source.Entity, destination.Entity, action, operation); }
    private bool CanReact(EntityId entity) => _session.Entities.Get(entity, D20ComponentTypes.Resources).Values.Any(value => value.Value > 0) && _session.Entities.Get(entity, D20ComponentTypes.Budgets).Values.Any(value => value.Id == D20Id.Parse("reaction") && value.Value > 0);
    private void Advance() { _cursor = (_cursor + 1) % _order.Count; Skip(); }
    private void Skip() { for (int step = 0; step < _order.Count && !_session.IsLiving(Participant(_order[_cursor]).Entity); step++) _cursor = (_cursor + 1) % _order.Count; }
    private TacticalParticipant Participant(D20Id id) => _participants.TryGetValue(id, out TacticalParticipant? value) ? value : throw new TacticalException("Unknown participant.");
    private EncounterFaction Faction(D20Id id) => _session.FactionOf(Participant(id).Entity);
}
