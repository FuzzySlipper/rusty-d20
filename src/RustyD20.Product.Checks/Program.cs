using System.Text;
using Rusty.Engine;
using RustyD20.Product;

CheckInputBindingFence();
CheckDisposalFence();
Console.WriteLine("RustyD20.Product focused checks passed.");

static void CheckInputBindingFence()
{
    InputBinding current = new(7, 9, 11); InputContext context = new(Encoding.UTF8.GetBytes(D20InputClaims.Context));
    ProductInputEvent Event(InputBinding binding, InputEdge edge = InputEdge.None, InputPhase phase = InputPhase.DirectUi) => new(InputEventKind.DirectDigital, edge, InputDevice.Product, InputChannel.Intent, InputAxis.None, KeyboardControl.None, PointerButton.None, ControllerButton.None, ControllerAxis.None, InputClearReason.None, InputValueKind.Digital, phase, InputProvenance.DirectUi, binding, new InputSequence(1), context, 1, 0, ReadOnlyMemory<byte>.Empty, ReadOnlyMemory<byte>.Empty, "d20.begin"u8.ToArray(), ReadOnlyMemory<byte>.Empty, ReadOnlyMemory<byte>.Empty);
    Require(D20InputClaims.TryClaim(Event(current), current, context, out D20Command claimed) && claimed == D20Command.Begin, "current direct input must be admitted");
    Require(!D20InputClaims.TryClaim(Event(new InputBinding(7, 8, 11)), current, context, out _), "stale generation must be ignored");
    Require(!D20InputClaims.TryClaim(Event(new InputBinding(8, 9, 11)), current, context, out _), "foreign instance must be ignored");
    Require(!D20InputClaims.TryClaim(Event(current, InputEdge.Released), current, context, out _), "released input must be ignored");
    Require(!D20InputClaims.TryClaim(Event(current, InputEdge.None, InputPhase.Held), current, context, out _), "inactive physical input must be ignored");
}

static void CheckDisposalFence()
{
    var first = new RecordingOwner("first", throws: true);
    var second = new RecordingOwner("second", throws: false);
    try
    {
        D20Disposal.DisposeAll(first, second);
        throw new InvalidOperationException("A throwing owner must be reported after cleanup.");
    }
    catch (AggregateException error)
    {
        Require(error.InnerExceptions.Count == 1, "cleanup must report the recorded owner failure once");
    }

    Require(first.Disposed && second.Disposed, "cleanup must attempt every owned resource after one owner fails");
}

static void Require(bool condition, string message) { if (!condition) throw new InvalidOperationException(message); }

sealed class RecordingOwner(string name, bool throws) : IDisposable
{
    public bool Disposed { get; private set; }
    public void Dispose()
    {
        Disposed = true;
        if (throws) throw new InvalidOperationException(name);
    }
}
