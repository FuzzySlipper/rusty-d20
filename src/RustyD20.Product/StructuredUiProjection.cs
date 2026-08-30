using System.Text;
using Rusty.Engine;

namespace RustyD20.Product;

internal readonly record struct UiField(string Key, double? Number, string? Text)
{
    public static UiField NumberValue(string key, double value) => new(key, value, null);
    public static UiField TextValue(string key, string value) => new(key, null, value);
}

internal static class StructuredUiProjection
{
    public static UiValue Object(IReadOnlyList<UiField> fields)
    {
        if (fields.Count > 160) throw new InvalidOperationException("D20 UI projection exceeded its bounded field budget.");
        UiField[] ordered = fields.OrderBy(field => field.Key, StringComparer.Ordinal).ToArray();
        if (ordered.Any(field => string.IsNullOrWhiteSpace(field.Key) || (field.Number is null && field.Text is null))) throw new InvalidOperationException("D20 UI projection contains an invalid field.");
        List<byte> bytes = []; var keys = new Dictionary<string, (uint Offset, uint Length)>(); var values = new Dictionary<string, (uint Offset, uint Length)>();
        foreach (UiField field in ordered) { keys.Add(field.Key, Add(bytes, field.Key)); if (field.Text is { } text) values.Add(field.Key, Add(bytes, text.Length <= 256 ? text : text[..256])); }
        StructuredValueNode[] nodes = new StructuredValueNode[ordered.Length + 1]; nodes[0] = new(StructuredValueKind.Object, 0, 0, 0, 0, 0, 0, 0, checked((uint)ordered.Length));
        for (int index = 0; index < ordered.Length; index++) { UiField field = ordered[index]; (uint offset, uint length) = keys[field.Key]; nodes[index + 1] = field.Number is double number ? new(StructuredValueKind.Number, 0, number, offset, length, 0, 0, 0, 0) : new(StructuredValueKind.String, 0, 0, offset, length, values[field.Key].Offset, values[field.Key].Length, 0, 0); }
        return new UiValue(nodes, Enumerable.Range(1, ordered.Length).Select(static value => checked((uint)value)).ToArray(), 0, bytes.ToArray());
    }
    private static (uint Offset, uint Length) Add(List<byte> bytes, string text) { byte[] encoded = Encoding.UTF8.GetBytes(text); if (encoded.Length > 256) throw new InvalidOperationException("D20 UI field exceeds its byte budget."); uint offset = checked((uint)bytes.Count); bytes.AddRange(encoded); return (offset, checked((uint)encoded.Length)); }
}
