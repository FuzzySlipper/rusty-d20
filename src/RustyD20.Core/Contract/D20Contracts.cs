namespace RustyD20.Core.Contract;

public static class D20Limits
{
    public const int DefinitionsPerKind = 64;
    public const int AdventuresPerPackage = 16;
    public const int AdventureEntries = 64;
    public const int AuthoredTextBytes = 512;
    public const int DungeonWidth = 24;
    public const int DungeonHeight = 24;
    public const int TacticalBoardWidth = 16;
    public const int TacticalBoardHeight = 16;
    public const int DamageDice = 32;
    public const int DamageDieSides = 1_000;
    public const int EffectDurationTurns = 10_000;
    public const int Experience = 1_000_000_000;
    public const int ActionTags = 16;
    public const int ImplementTags = 16;
    public const int ActivationCosts = 4;
    public const int ConditionClauses = 8;
    public const int TacticalRange = 32;
    public const int ForcedMovement = 6;
    public const int ActionTargets = 12;
    public const int PartyMembers = 4;
    public const int EncounterParticipants = 12;
    public const int StaticActionRolls = 4_096;
}

/// <summary>Closed C# schema names. Legacy Rust candidate/session schemas are intentionally never admitted.</summary>
public static class D20Schemas
{
    public const string Content = "rusty-d20.content.csharp/1";
    public const string Session = "rusty-d20.session.csharp/1";
    public const string Save = "rusty-d20.save.csharp/1";

    public static bool IsCurrentContent(string value) => value == Content;
    public static bool IsCurrentSession(string value) => value == Session;
    public static bool IsCurrentSave(string value) => value == Save;
}

public sealed record SourceProvenance(
    string SourcePath,
    string Subject,
    string Adaptation,
    string? DonorPath = null);

public sealed record D20Diagnostic(
    string Code,
    string Message,
    SourceProvenance Source,
    string CorrelationId,
    string? Detail = null);

public sealed class D20CompilationException : Exception
{
    public D20CompilationException(IReadOnlyList<D20Diagnostic> diagnostics)
        : base($"D20 content admission failed with {diagnostics.Count} diagnostic(s).") => Diagnostics = diagnostics;

    public IReadOnlyList<D20Diagnostic> Diagnostics { get; }
}
