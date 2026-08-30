using Rusty.Engine.Entities;
using Rusty.Engine.Mechanics;
using RustyD20.Core.Campaign;
using RustyD20.Core.Contract;
using RustyD20.Core.Rules;
using RustyD20.Core.Session;

namespace RustyD20.Core.Tactical;

public interface ITacticalSpatialGateway { bool HasLineOfEffect(GridPosition from, GridPosition to); bool HasLegalRoute(GridPosition from, GridPosition to); }
public interface ITacticalSpatialComposition { void ReplaceTacticalBoard(TacticalBoard board); }
public sealed record TacticalParticipant(D20Id Id, EntityId Entity, int Initiative, GridPosition Position);
public sealed record TacticalTuning(int MovementBudget = D20Limits.ForcedMovement, int MaximumOppositionSettlements = D20Limits.EncounterParticipants)
{
    public void Validate()
    {
        if (MovementBudget is < 1 or > D20Limits.ForcedMovement || MaximumOppositionSettlements is < 1 or > D20Limits.EncounterParticipants) throw new TacticalException("Tactical tuning is outside the admitted product bounds.");
    }
}
public sealed record TacticalMovementReadout(int Budget, int Remaining, IReadOnlyList<TacticalParticipant> Participants);
public sealed record ReactionPrompt(D20Id Defender, D20Id Attacker, D20Id Action, ActionPreview Preview);
public sealed record TacticalEncounterSave(IReadOnlyList<TacticalParticipant> Participants, IReadOnlyList<D20Id> Initiative, int Cursor, D20Id? PendingDefender, D20Id? PendingAttacker, D20Id? PendingAction, ulong OppositionProgress, [property: System.Text.Json.Serialization.JsonRequired] int MovementBudget, [property: System.Text.Json.Serialization.JsonRequired] int RemainingMovement, bool CommittedContinuation = false);
public sealed class TacticalException : InvalidOperationException { public TacticalException(string message) : base(message) { } }

/// <summary>Turn, movement, reaction, and bounded opposition policy over canonical session and Engine spatial facts.</summary>
public sealed class TacticalEncounter
{
    private enum ReactionStage { PartyAction, OppositionAction }
    private sealed record CommittedContinuation(bool AdvancePending);

    private readonly D20Session _session;
    private readonly ITacticalSpatialGateway _spatial;
    private readonly Dictionary<D20Id, TacticalParticipant> _participants;
    private readonly List<D20Id> _order;
    private readonly TacticalTuning _tuning;
    private int _cursor;
    private int _remainingMovement;
    private CommittedContinuation? _continuation;
    private ReactionStage? _pendingReactionStage;
    private ulong _pendingOppositionProgress;

    public TacticalEncounter(D20Session session, ITacticalSpatialGateway spatial, IEnumerable<TacticalParticipant> participants, TacticalBoard? board = null, TacticalTuning? tuning = null)
    {
        _session = session ?? throw new ArgumentNullException(nameof(session));
        _spatial = spatial ?? throw new ArgumentNullException(nameof(spatial));
        _tuning = tuning ?? new TacticalTuning();
        _tuning.Validate();
        ArgumentNullException.ThrowIfNull(participants);
        TacticalParticipant[] entries = participants.ToArray();
        if (entries.Length is < 1 or > D20Limits.EncounterParticipants || entries.Select(value => value.Id).Distinct().Count() != entries.Length || entries.Select(value => value.Entity).Distinct().Count() != entries.Length) throw new TacticalException("Encounter participant identity or bound is invalid.");
        _participants = entries.ToDictionary(value => value.Id);
        foreach (TacticalParticipant participant in entries)
        {
            if (!_session.IsParticipant(participant.Entity) || _session.FactionOf(participant.Entity) is not (EncounterFaction.Party or EncounterFaction.Opposition)) throw new TacticalException("Encounter participant is not admitted by the session.");
        }

        if (board is not null && _spatial is ITacticalSpatialComposition composition) composition.ReplaceTacticalBoard(board);
        _order = _participants.Values.OrderByDescending(value => value.Initiative).ThenBy(value => value.Entity.Value).Select(value => value.Id).ToList();
        _remainingMovement = _tuning.MovementBudget;
        Skip();
    }

    public D20Id CurrentActor => _order[_cursor];
    public ReactionPrompt? PendingReaction { get; private set; }
    public ulong OppositionProgress { get; private set; }
    public TacticalMovementReadout Movement => new(_tuning.MovementBudget, _remainingMovement, _participants.Values.OrderBy(value => value.Entity.Value).ToArray());
    public IReadOnlyList<TacticalParticipant> Participants => Movement.Participants;
    public bool HasCommittedContinuation => _continuation is not null;

    public bool TryGetTerminalResult(out EncounterResult result)
    {
        bool partyLiving = _participants.Values.Any(value => Faction(value.Id) == EncounterFaction.Party && _session.IsLiving(value.Entity));
        bool oppositionLiving = _participants.Values.Any(value => Faction(value.Id) == EncounterFaction.Opposition && _session.IsLiving(value.Entity));
        if (partyLiving == oppositionLiving) { result = default; return false; }
        result = oppositionLiving ? EncounterResult.Defeat : EncounterResult.Victory;
        return true;
    }

    public void PartyMove(D20Id actor, GridPosition destination)
    {
        if (PendingReaction is not null || _continuation is not null || CurrentActor != actor || Faction(actor) != EncounterFaction.Party) throw new TacticalException("Only the current party actor may move.");
        if (!TryRelocate(actor, destination, _remainingMovement, spendMovement: true)) throw new TacticalException("Engine retained spatial projection or tactical movement budget rejects that move.");
    }

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

        CommitAction(preview);
        ContinueAfterCommittedAction();
    }

    public void ResolveReaction(D20Id reaction, bool choose)
    {
        ReactionPrompt prompt = PendingReaction ?? throw new TacticalException("No pending reaction.");
        ReactionStage stage = _pendingReactionStage ?? throw new TacticalException("Pending reaction stage is missing.");
        ulong pendingOppositionProgress = _pendingOppositionProgress;
        ReactionResolutionReceipt resolution = _session.ResolveReaction(prompt.Preview, choose ? reaction : null);
        ApplyForcedMovement(resolution.Action);
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

    public void ResumeAutomaticProgression()
    {
        if (PendingReaction is not null) throw new TacticalException("Resolve the pending reaction before automatic progression.");
        if (_continuation is null) throw new TacticalException("No automatic progression is awaiting retry.");
        FinishAdvance();
        SettleOppositionCore();
    }

    public TacticalEncounterSave CaptureSave() => new(_participants.Values.OrderBy(value => value.Entity.Value).ToArray(), _order.ToArray(), _cursor, PendingReaction?.Defender, PendingReaction?.Attacker, PendingReaction?.Action, OppositionProgress, _tuning.MovementBudget, _remainingMovement, _continuation is not null);

    public static TacticalEncounter Restore(D20Session session, ITacticalSpatialGateway spatial, TacticalEncounterSave save, TacticalBoard? board = null)
    {
        ArgumentNullException.ThrowIfNull(session);
        ArgumentNullException.ThrowIfNull(spatial);
        ArgumentNullException.ThrowIfNull(save);
        TacticalTuning tuning = new(save.MovementBudget);
        tuning.Validate();
        if (save.Participants is null || save.Initiative is null || save.Participants.Count is < 1 or > D20Limits.EncounterParticipants || save.PendingDefender is not null || save.PendingAttacker is not null || save.PendingAction is not null || save.CommittedContinuation || save.OppositionProgress > D20Limits.EncounterParticipants || save.RemainingMovement < 0 || save.RemainingMovement > save.MovementBudget) throw new TacticalException("Invalid or transient tactical save.");
        if (save.Participants.Select(value => value.Id).Distinct().Count() != save.Participants.Count || save.Participants.Select(value => value.Entity).Distinct().Count() != save.Participants.Count) throw new TacticalException("Saved tactical participant identities are duplicated.");
        if (board is not null)
        {
            if (board.Placements.Count != save.Participants.Count || board.Placements.Select(value => value.Character).Distinct().Count() != board.Placements.Count) throw new TacticalException("Saved tactical board closure is invalid.");
            if (spatial is ITacticalSpatialComposition composition) composition.ReplaceTacticalBoard(board);
            foreach (TacticalParticipant participant in save.Participants)
            {
                TacticalPlacement placement = board.Placements.SingleOrDefault(value => value.Character == participant.Id) ?? throw new TacticalException("Saved tactical participant is missing from the authored board.");
                if (!spatial.HasLegalRoute(placement.Position, participant.Position)) throw new TacticalException("Saved tactical participant position is not admitted by Engine navigation.");
            }
        }
        D20Id[] expectedOrder = save.Participants.OrderByDescending(value => value.Initiative).ThenBy(value => value.Entity.Value).Select(value => value.Id).ToArray();
        if (save.Initiative.Count != expectedOrder.Length || save.Initiative.Where((value, index) => value != expectedOrder[index]).Any() || save.Cursor < 0 || save.Cursor >= expectedOrder.Length) throw new TacticalException("Invalid initiative save.");
        TacticalParticipant current = save.Participants.Single(value => value.Id == save.Initiative[save.Cursor]);
        if (!session.IsLiving(current.Entity)) throw new TacticalException("Saved tactical cursor points at a dead participant.");
        var candidate = new TacticalEncounter(session, spatial, save.Participants, null, tuning);
        candidate._cursor = save.Cursor;
        candidate.OppositionProgress = save.OppositionProgress;
        candidate._remainingMovement = save.RemainingMovement;
        return candidate;
    }

    private void ContinueAfterCommittedAction() { _continuation = new CommittedContinuation(true); FinishAdvance(); SettleOppositionCore(); }
    private void FinishAdvance() { if (_continuation is not { AdvancePending: true }) return; Advance(); _continuation = new CommittedContinuation(false); }

    private void SettleOppositionCore()
    {
        if (PendingReaction is not null) throw new TacticalException("Pending reaction custody must be resolved before settlement.");
        for (int settled = 0; settled < _tuning.MaximumOppositionSettlements && Faction(CurrentActor) == EncounterFaction.Opposition; settled++)
        {
            ulong nextProgress = checked(OppositionProgress + 1);
            if (!TrySelectOppositionPreview(CurrentActor, nextProgress, out ActionPreview? preview))
            {
                AdvanceOpposition(CurrentActor);
                Advance();
                continue;
            }

            ActionPreview selected = preview ?? throw new TacticalException("Opposition selection did not retain its legal action preview.");
            if (CanReact(ParticipantByEntity(selected.Target).Entity))
            {
                PendingReaction = new(ParticipantByEntity(selected.Target).Id, CurrentActor, selected.Action, selected);
                _pendingReactionStage = ReactionStage.OppositionAction;
                _pendingOppositionProgress = nextProgress;
                return;
            }

            CommitAction(selected);
            OppositionProgress = nextProgress;
            _continuation = new CommittedContinuation(true);
            FinishAdvance();
        }

        if (Faction(CurrentActor) != EncounterFaction.Opposition) _continuation = null;
    }

    private bool TrySelectOppositionPreview(D20Id actor, ulong progress, out ActionPreview? preview)
    {
        TacticalParticipant source = Participant(actor);
        foreach (ActionDefinition action in _session.AdmittedActions(source.Entity).OrderBy(value => value.Id.Value, StringComparer.Ordinal))
        foreach (TacticalParticipant target in _participants.Values.Where(value => Faction(value.Id) == EncounterFaction.Party && _session.IsLiving(value.Entity)).OrderBy(value => value.Entity.Value))
        {
            if (!_spatial.HasLegalRoute(source.Position, target.Position) || !_spatial.HasLineOfEffect(source.Position, target.Position)) continue;
            try { preview = Preview(actor, target.Id, action.Id, OperationId.Parse($"opposition-{progress}")); return true; }
            catch (D20SessionException) { }
            catch (TacticalException error) when (error.Message.StartsWith("Authored action range", StringComparison.Ordinal)) { }
        }

        preview = null;
        return false;
    }

    private void CommitAction(ActionPreview preview) { ActionReceipt receipt = _session.ApplyAction(preview); ApplyForcedMovement(receipt); }
    private void ApplyForcedMovement(ActionReceipt receipt)
    {
        if (!receipt.Hit || receipt.ForcedMovementIntent <= 0) return;
        TacticalParticipant attacker = ParticipantByEntity(receipt.Actor);
        TacticalParticipant target = ParticipantByEntity(receipt.Target);
        int stepX = Math.Sign(target.Position.X - attacker.Position.X);
        int stepY = Math.Sign(target.Position.Y - attacker.Position.Y);
        if (stepX == 0 && stepY == 0) return;
        for (int step = 0; step < receipt.ForcedMovementIntent; step++)
        {
            GridPosition destination = new(target.Position.X + stepX, target.Position.Y + stepY);
            if (!TryRelocate(target.Id, destination, 1, spendMovement: false)) return;
            target = Participant(target.Id);
        }
    }

    private void AdvanceOpposition(D20Id actor)
    {
        TacticalParticipant source = Participant(actor);
        TacticalParticipant? target = _participants.Values.Where(value => Faction(value.Id) == EncounterFaction.Party && _session.IsLiving(value.Entity)).OrderBy(value => Chebyshev(source.Position, value.Position)).ThenBy(value => value.Entity.Value).FirstOrDefault();
        if (target is null || _remainingMovement == 0) return;
        int x = Math.Sign(target.Position.X - source.Position.X);
        int y = Math.Sign(target.Position.Y - source.Position.Y);
        foreach (GridPosition candidate in new[] { new GridPosition(source.Position.X + x, source.Position.Y + y), new GridPosition(source.Position.X + x, source.Position.Y), new GridPosition(source.Position.X, source.Position.Y + y) }.Distinct())
        {
            if (candidate != source.Position && TryRelocate(actor, candidate, _remainingMovement, spendMovement: true)) return;
        }
    }

    private ActionPreview Preview(D20Id actor, D20Id target, D20Id action, OperationId operation)
    {
        TacticalParticipant source = Participant(actor);
        TacticalParticipant destination = Participant(target);
        if (!_spatial.HasLegalRoute(source.Position, destination.Position) || !_spatial.HasLineOfEffect(source.Position, destination.Position)) throw new TacticalException("Engine retained spatial projection rejects target route or line of effect.");
        ActionPreview preview = _session.PreviewAction(source.Entity, destination.Entity, action, operation);
        if (Chebyshev(source.Position, destination.Position) > preview.Range) throw new TacticalException("Authored action range rejects that Engine-admitted target.");
        return preview;
    }

    private bool TryRelocate(D20Id actor, GridPosition destination, int maximumDistance, bool spendMovement)
    {
        TacticalParticipant source = Participant(actor);
        int distance = Chebyshev(source.Position, destination);
        if (distance < 1 || distance > D20Limits.ForcedMovement || distance > maximumDistance || (spendMovement && distance > _remainingMovement) || !_spatial.HasLegalRoute(source.Position, destination)) return false;
        _participants[actor] = source with { Position = destination };
        if (spendMovement) _remainingMovement -= distance;
        return true;
    }

    private bool CanReact(EntityId entity) => _session.Entities.Get(entity, D20ComponentTypes.Resources).Values.Any(value => value.Value > 0) && _session.Entities.Get(entity, D20ComponentTypes.Budgets).Values.Any(value => value.Id == D20Id.Parse("reaction") && value.Value > 0);
    private void Advance() { _cursor = (_cursor + 1) % _order.Count; _remainingMovement = _tuning.MovementBudget; Skip(); }
    private void Skip() { for (int step = 0; step < _order.Count && !_session.IsLiving(Participant(_order[_cursor]).Entity); step++) _cursor = (_cursor + 1) % _order.Count; }
    private TacticalParticipant Participant(D20Id id) => _participants.TryGetValue(id, out TacticalParticipant? value) ? value : throw new TacticalException("Unknown participant.");
    private TacticalParticipant ParticipantByEntity(EntityId entity) => _participants.Values.SingleOrDefault(value => value.Entity == entity) ?? throw new TacticalException("Action refers to a non-encounter participant.");
    private EncounterFaction Faction(D20Id id) => _session.FactionOf(Participant(id).Entity);
    private static int Chebyshev(GridPosition left, GridPosition right) => Math.Max(Math.Abs(left.X - right.X), Math.Abs(left.Y - right.Y));
}
