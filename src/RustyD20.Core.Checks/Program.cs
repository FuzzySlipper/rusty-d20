using RustyD20.Core.Content;
using RustyD20.Core.Contract;
using RustyD20.Core.Rules;

var checks = new (string Name, Action Run)[]
{
    ("strict id and schema", StrictIdentityAndSchema),
    ("duplicate, quota, and reference admission", DuplicateQuotaAndReference),
    ("topology, placement, dependency, and reachability admission", TopologyPlacementAndDependency),
    ("stable composition fingerprint", StableFingerprint),
    ("named adventure compilation", NamedAdventures),
};

foreach (var check in checks)
{
    check.Run();
    Console.WriteLine($"passed: {check.Name}");
}

return;

static void StrictIdentityAndSchema()
{
    Assert(!D20Id.TryParse("Uppercase", out _, out _), "uppercase identifiers must reject");
    Assert(!D20Id.TryParse("", out _, out _), "empty identifiers must reject");
    Assert(D20Id.TryParse("warden.sigil_1", out var id, out _), "stable lowercase id must admit");
    Assert(id.ToString() == "warden.sigil_1", "id must retain exact identity");
    Expect("D20_UNSUPPORTED_CONTENT_SCHEMA", () => new D20SemanticCompiler().Compile([new D20ContentModule(D20Id.Parse("wrong-schema"), "rusty-d20.candidate/6", [], Source("wrong-schema"))]));
}

static void DuplicateQuotaAndReference()
{
    var source = Source("bad-content");
    var duplicate = new D20ContentModule(D20Id.Parse("duplicate-content"), D20Schemas.Content, [], source,
        Abilities: [new(D20Id.Parse("might"), 1, 20, source), new(D20Id.Parse("might"), 1, 20, source)]);
    Expect("D20_DUPLICATE_ID", () => new D20SemanticCompiler().Compile([duplicate]));

    var tooMany = new D20ContentModule(D20Id.Parse("quota-content"), D20Schemas.Content, [], source,
        Abilities: Enumerable.Range(0, D20Limits.DefinitionsPerKind + 1).Select(index => new AbilityDefinition(D20Id.Parse($"ability-{index}"), 1, 20, source)).ToArray());
    Expect("D20_DEFINITION_QUOTA", () => new D20SemanticCompiler().Compile([tooMany]));

    var dangling = new D20ContentModule(D20Id.Parse("reference-content"), D20Schemas.Content, [], source,
        Armors: [new(D20Id.Parse("ghost-armor"), D20Id.Parse("unknown-defense"), 1, D20Id.Parse("body"), source)]);
    Expect("D20_UNKNOWN_DEFENSE", () => new D20SemanticCompiler().Compile([dangling]));
}

static void TopologyPlacementAndDependency()
{
    var source = Source("topology-content");
    var module = new D20ContentModule(D20Id.Parse("topology-content"), D20Schemas.Content, [D20Id.Parse("missing-dependency")], source,
        Adventures:
        [
            new(D20Id.Parse("broken-adventure"), "Broken", true, true, [], [], D20Id.Parse("missing-storage"), [], [], [], new DungeonDefinition("Broken", D20Id.Parse("stone"), 3, 3, ["###", "#.#", "##x"], new(1, 1), D20Id.Parse("missing-checkpoint"), DungeonFacing.East, [], [], [], [], []), new AdventureOutcome("test", "won", "won", "lost", "lost", []), source),
        ]);
    ExpectAny(["D20_UNKNOWN_MODULE_DEPENDENCY", "D20_INVALID_DUNGEON_TOPOLOGY", "D20_UNKNOWN_STORAGE"], () => new D20SemanticCompiler().Compile([module]));

    var cycleA = new D20ContentModule(D20Id.Parse("cycle-a"), D20Schemas.Content, [D20Id.Parse("cycle-b")], Source("cycle-a"));
    var cycleB = new D20ContentModule(D20Id.Parse("cycle-b"), D20Schemas.Content, [D20Id.Parse("cycle-a")], Source("cycle-b"));
    Expect("D20_MODULE_DEPENDENCY_CYCLE", () => new D20SemanticCompiler().Compile([cycleA, cycleB]));

    var catalog = D20ContentCatalog.Modules.ToArray();
    var wardenIndex = Array.FindIndex(catalog, module => module.Id == D20Id.Parse("wardens-gate-adventure"));
    var warden = catalog[wardenIndex];
    var adventure = warden.AdventuresOrEmpty.Single();
    var isolatedRows = adventure.Dungeon.Rows.Select((row, index) => index is 1 or 5 ? "#.#######.#" : row).ToArray();
    catalog[wardenIndex] = warden with { Adventures = [adventure with { Dungeon = adventure.Dungeon with { Rows = isolatedRows } }] };
    Expect("D20_UNREACHABLE_ENCOUNTER_PLACEMENT", () => new D20SemanticCompiler().Compile(catalog));
}

static void StableFingerprint()
{
    var modules = D20ContentCatalog.Modules;
    var first = D20SemanticCompiler.Fingerprint(modules);
    var second = D20SemanticCompiler.Fingerprint(modules.Reverse());
    Assert(first == second, "helper/module ordering must not alter a content fingerprint");

    var steel = modules.Single(module => module.Id == D20Id.Parse("steel-guard-content"));
    var mutatedEffect = steel.EffectsOrEmpty.Select(effect => effect.Id == D20Id.Parse("bleeding") ? effect with { DurationTurns = effect.DurationTurns + 1 } : effect).ToArray();
    var mutatedModules = modules.Select(module => module.Id == steel.Id ? module with { Effects = mutatedEffect } : module).ToArray();
    Assert(first != D20SemanticCompiler.Fingerprint(mutatedModules), "a gameplay-semantic effect mutation must change the fingerprint");

    var reorderedEffects = modules.Select(module => module.Id == steel.Id ? module with { Effects = steel.EffectsOrEmpty.Reverse().ToArray() } : module).ToArray();
    Assert(first == D20SemanticCompiler.Fingerprint(reorderedEffects), "unordered definition helper ordering must be canonical");
}

static void NamedAdventures()
{
    var compiled = D20ContentCatalog.Compile();
    Assert(compiled.Adventures.ContainsKey(D20Id.Parse("wardens-gate")), "Warden's Gate must compile");
    Assert(compiled.Adventures.ContainsKey(D20Id.Parse("embers-wake")), "Ember's Wake must compile");
    Assert(compiled.Adventures[D20Id.Parse("wardens-gate")].Party.Count == 4, "Warden's Gate preserves its four-person party");
    Assert(compiled.Adventures[D20Id.Parse("embers-wake")].Party.Count == 1, "Ember's Wake preserves Sera's solo party");
    Assert(compiled.ContentFingerprint.Length == 64, "receipt exposes a SHA-256 content fingerprint");
}

static SourceProvenance Source(string subject) => new($"checks/{subject}.cs", subject, "focused check");

static void Expect(string code, Action action)
{
    try
    {
        action();
        throw new InvalidOperationException($"expected {code}");
    }
    catch (D20CompilationException exception)
    {
        Assert(exception.Diagnostics.Any(diagnostic => diagnostic.Code == code), $"expected diagnostic {code}");
    }
}

static void ExpectAny(IEnumerable<string> codes, Action action)
{
    try
    {
        action();
        throw new InvalidOperationException("expected admission failure");
    }
    catch (D20CompilationException exception)
    {
        Assert(exception.Diagnostics.Any(diagnostic => codes.Contains(diagnostic.Code, StringComparer.Ordinal)), "expected one relevant topology/dependency diagnostic");
    }
}

static void Assert(bool condition, string message)
{
    if (!condition) throw new InvalidOperationException(message);
}
