using Rusty.Engine;
using Rusty.Engine.Mechanics;
using RustyD20.Core.Campaign;
using RustyD20.Core.Contract;
using RustyD20.Core.Rules;
using RustyD20.Core.Session;
using RustyD20.Core.Tactical;

namespace RustyD20.Core.Checks;

/// <summary>Focused review probes; Program owns scenario composition and invocation order.</summary>
internal static class TacticalReviewProbes
{
    public static void AssertOutOfRangeRejected(TacticalEncounter encounter, D20Id actor, D20Id target, D20Id action, OperationId operation)
    {
        try
        {
            encounter.PartyAction(actor, target, action, operation);
        }
        catch (TacticalException error) when (error.Message.Contains("range", StringComparison.OrdinalIgnoreCase))
        {
            return;
        }

        throw new InvalidOperationException("Expected an Engine-admitted but authored-out-of-range action to be rejected.");
    }

    public static void AssertMovementSurvivesRestore(TacticalEncounter encounter, D20Session session, ITacticalSpatialGateway spatial, TacticalBoard board, D20Id actor, GridPosition destination)
    {
        encounter.PartyMove(actor, destination);
        TacticalEncounterSave save = encounter.CaptureSave();
        TacticalEncounter restored = TacticalEncounter.Restore(session, spatial, save, board);
        TacticalParticipant participant = restored.Participants.Single(value => value.Id == actor);
        if (participant.Position != destination || restored.Movement.Remaining != save.RemainingMovement)
            throw new InvalidOperationException("Tactical position or movement budget did not survive strict restore.");
    }

    public static void AssertMovementConditionBoundary(TacticalEncounter encounter, D20Session session, D20Id actor, GridPosition destination)
    {
        try
        {
            encounter.PartyMove(actor, destination);
            throw new InvalidOperationException("Expected an active authored movement prohibition to reject voluntary relocation.");
        }
        catch (TacticalException error) when (error.Message.Contains("rejects", StringComparison.OrdinalIgnoreCase))
        {
        }

        session.AdvanceTurn();
        encounter.PartyMove(actor, destination);
        if (encounter.Participants.Single(value => value.Id == actor).Position != destination)
            throw new InvalidOperationException("Voluntary movement did not resume after the authored prohibition expired.");
    }

    public static void AssertUntouchedOutcomeRejected(D20CampaignRuntime campaign, TacticalEncounter untouched)
    {
        try
        {
            campaign.ResolveEncounter(untouched);
        }
        catch (CampaignException error) when (error.Message.Contains("unresolved", StringComparison.OrdinalIgnoreCase))
        {
            return;
        }

        throw new InvalidOperationException("Expected untouched tactical facts to reject campaign outcome admission.");
    }

    public static void AssertNoHardcodedOppositionFallback(TacticalEncounter encounter, D20Session session)
    {
        int before = session.Receipts.Count;
        encounter.SettleOpposition();
        if (session.Receipts.Skip(before).Any(receipt => receipt.Action == D20Id.Parse("disrupt")))
            throw new InvalidOperationException("Opposition selected the removed hardcoded disrupt fallback.");
    }
}
