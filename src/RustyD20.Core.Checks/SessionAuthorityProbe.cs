using System.Reflection;
using Rusty.Engine;
using Rusty.Engine.Entities;
using Rusty.Engine.Mechanics;
using RustyD20.Core.Content;
using RustyD20.Core.Contract;
using RustyD20.Core.Rules;
using RustyD20.Core.Session;

namespace RustyD20.Core.Checks;

/// <summary>Focused externally-shaped probes for session-issued action previews and authored reaction effects.</summary>
internal static class SessionAuthorityProbe
{
    public static void AssertPreviewOutcomeAuthority()
    {
        using (D20Session missed = NewSession(new StaticActionRoll(1, [1]), out EntityId actor, out EntityId target))
        {
            ActionPreview preview = missed.PreviewAction(actor, target, Id("disrupt"), OperationId.Parse("preview-authority-miss"));
            Tamper(preview, nameof(ActionPreview.AbilityModifier), 1000);
            Tamper(preview, nameof(ActionPreview.Defense), 0);
            ActionReceipt receipt = missed.ApplyAction(preview);
            Assert(!receipt.Hit && receipt.Total == 2 && receipt.Defense == 11 && receipt.Damage == 0, "a tampered preview must not dictate ability, defense, or hit outcome");
        }

        using (D20Session hit = NewSession(new StaticActionRoll(20, [1]), out EntityId actor, out EntityId target))
        {
            ActionPreview preview = hit.PreviewAction(actor, target, Id("disrupt"), OperationId.Parse("preview-authority-damage"));
            Tamper(preview, nameof(ActionPreview.Damage), new DamageDefinition(Id("physical"), 1, 4, 999));
            ActionReceipt receipt = hit.ApplyAction(preview);
            Assert(receipt.Hit && receipt.Damage == 1, "a tampered preview must not dictate authored damage");
        }

        Assert(typeof(ActionPreview).GetConstructors(BindingFlags.Public | BindingFlags.Instance).Length == 0, "callers cannot construct action previews");
        Assert(typeof(ActionPreview).GetProperty(nameof(ActionPreview.Defense))?.GetSetMethod(nonPublic: true)?.IsPublic is false, "callers cannot use a public with-setter to alter action preview outcomes");
    }

    public static void AssertReactionDefenseBoundary()
    {
        using D20Session session = NewSession(new StaticActionRoll(10, [4]), out EntityId actor, out EntityId target);
        session.RegisterLoadoutOwner(actor);
        ImplementDefinition blade = D20ContentCatalog.Compile().Catalog.Implements[Id("training-blade")];
        session.EquipImplement(actor, blade);
        ActionPreview preview = session.PreviewAction(actor, target, Id("longsword-strike"), OperationId.Parse("reaction-defense-boundary"));
        Assert(preview.Defense == 11 && 10 + preview.AbilityModifier == 11, "the deterministic tape must begin at the authored hit boundary");

        ReactionResolutionReceipt result = session.ResolveReaction(preview, Id("parry"));
        Assert(result.Reaction is not null && result.Reaction.Effect == Id("parry-stance"), "the authored reaction effect must be committed with its resource spend");
        Assert(!result.Action.Hit && result.Action.Defense == 13 && result.Action.Damage == 0, "the matching authored reaction defense bonus must turn the boundary hit into a miss");
    }

    private static D20Session NewSession(StaticActionRoll roll, out EntityId actor, out EntityId target)
    {
        CompiledD20Content content = D20ContentCatalog.Compile();
        var session = new D20Session(content, RollSourceState.Static([roll]));
        actor = session.AddParticipant(content.Catalog.Characters[Id("mara-venn")], EncounterFaction.Party);
        target = session.AddParticipant(content.Catalog.Characters[Id("gate-skirmisher")], EncounterFaction.Opposition);
        session.SetActivationBudget(actor, Id("bonus-action"), 2);
        return session;
    }

    private static void Tamper(ActionPreview preview, string property, object value)
    {
        MethodInfo setter = typeof(ActionPreview).GetProperty(property, BindingFlags.Public | BindingFlags.Instance)?.GetSetMethod(nonPublic: true)
            ?? throw new InvalidOperationException($"ActionPreview.{property} has no setter for the authority probe.");
        setter.Invoke(preview, [value]);
    }

    private static D20Id Id(string value) => D20Id.Parse(value);
    private static void Assert(bool condition, string message) { if (!condition) throw new InvalidOperationException(message); }
}
