using System.Text;
using Rusty.Engine;

namespace RustyD20.Product;

public enum D20Command
{
    None, SelectWarden, SelectEmber, Begin, Forward, Back, Left, Right, Interact,
    PartyNext, ActionNext, TargetNext, CommitAction, React, Decline, Continue, Save, Load, Reset,
}

/// <summary>One strict, current-update input fence. Browser controls and physical mappings only name these intents.</summary>
public static class D20InputClaims
{
    public const string Context = "gameplay.default";
    private static readonly IReadOnlyDictionary<string, D20Command> Commands = new Dictionary<string, D20Command>(StringComparer.Ordinal)
    {
        ["d20.select.warden"] = D20Command.SelectWarden, ["d20.select.ember"] = D20Command.SelectEmber,
        ["d20.begin"] = D20Command.Begin, ["d20.forward"] = D20Command.Forward, ["d20.back"] = D20Command.Back,
        ["d20.left"] = D20Command.Left, ["d20.right"] = D20Command.Right, ["d20.interact"] = D20Command.Interact,
        ["d20.party.next"] = D20Command.PartyNext, ["d20.action.next"] = D20Command.ActionNext,
        ["d20.target.next"] = D20Command.TargetNext, ["d20.action.commit"] = D20Command.CommitAction,
        ["d20.reaction.choose"] = D20Command.React, ["d20.reaction.decline"] = D20Command.Decline,
        ["d20.outcome.continue"] = D20Command.Continue, ["d20.save"] = D20Command.Save,
        ["d20.load"] = D20Command.Load, ["d20.reset"] = D20Command.Reset,
    };

    public static IEnumerable<string> Intents => Commands.Keys;

    public static bool TryClaim(ProductInputEvent input, InputBinding binding, InputContext context, out D20Command command)
    {
        command = D20Command.None;
        if (input.Kind != InputEventKind.DirectDigital || input.Edge != InputEdge.None
            || input.ValueKind != InputValueKind.Digital || input.Phase != InputPhase.DirectUi
            || input.Provenance != InputProvenance.DirectUi || input.Binding != binding
            || !input.Context.Value.Span.SequenceEqual(context.Value.Span) || input.X != 1.0f
            || !input.PayloadContract.IsEmpty || !input.PayloadData.IsEmpty)
        {
            return false;
        }

        return Commands.TryGetValue(Encoding.UTF8.GetString(input.Intent.Span), out command);
    }
}
