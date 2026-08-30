using System.Text.Json;
using System.Text.Json.Serialization;

namespace RustyD20.Core.Contract;

/// <summary>A stable product identity. It deliberately accepts only the portable D20 alphabet.</summary>
[JsonConverter(typeof(D20IdJsonConverter))]
public readonly record struct D20Id
{
    public const int MaximumBytes = 64;
    public string Value { get; }

    private D20Id(string value) => Value = value;

    public static D20Id Parse(string value)
    {
        if (!TryParse(value, out var id, out var error))
        {
            throw new ArgumentException(error, nameof(value));
        }

        return id;
    }

    public static bool TryParse(string? value, out D20Id id, out string error)
    {
        id = default;
        if (string.IsNullOrEmpty(value))
        {
            error = "a D20 id cannot be empty";
            return false;
        }

        if (System.Text.Encoding.UTF8.GetByteCount(value) > MaximumBytes)
        {
            error = $"a D20 id cannot exceed {MaximumBytes} UTF-8 bytes";
            return false;
        }

        if (value.Any(character => !(character is >= 'a' and <= 'z' or >= '0' and <= '9' or '.' or '_' or '-')))
        {
            error = "a D20 id accepts only lowercase ASCII letters, digits, '.', '_' and '-'";
            return false;
        }

        id = new D20Id(value);
        error = string.Empty;
        return true;
    }

    public override string ToString() => Value;
}

/// <summary>NativeAOT-safe scalar persistence for validated product identities.</summary>
public sealed class D20IdJsonConverter : JsonConverter<D20Id>
{
    public override D20Id Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options)
    {
        if (reader.TokenType != JsonTokenType.String)
        {
            throw new JsonException("D20 id must be a JSON string.");
        }

        if (!D20Id.TryParse(reader.GetString(), out D20Id value, out string error))
        {
            throw new JsonException($"D20 id is invalid: {error}");
        }

        return value;
    }

    public override void Write(Utf8JsonWriter writer, D20Id value, JsonSerializerOptions options)
    {
        if (!D20Id.TryParse(value.Value, out _, out string error))
        {
            throw new JsonException($"D20 id is invalid: {error}");
        }

        writer.WriteStringValue(value.Value);
    }
}
