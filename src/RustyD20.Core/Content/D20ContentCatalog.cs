using RustyD20.Core.Contract;
using RustyD20.Core.Rules;

namespace RustyD20.Core.Content;

/// <summary>Inspectable, clean-room C# content. The old Rust/TypeScript trees are provenance only.</summary>
public static class D20ContentCatalog
{
    public static IReadOnlyList<D20ContentModule> Modules { get; } =
    [
        Abilities(),
        Fundamentals(),
        SteelGuard(),
        EmberWard(),
        WardensGate(),
        EmbersWake(),
    ];

    public static CompiledD20Content Compile() => new D20SemanticCompiler().Compile(Modules);

    private static D20ContentModule Abilities()
    {
        var source = S("starter-abilities", "rules/packages/starter-ruleset/src/content/abilities.ts");
        return new D20ContentModule(I("starter-abilities"), D20Schemas.Content, [], source,
            Abilities:
            [
                new(I("might"), 1, 20, source), new(I("finesse"), 1, 20, source),
                new(I("acuity"), 1, 20, source), new(I("intellect"), 1, 20, source),
                new(I("conviction"), 1, 20, source), new(I("spirit"), 1, 20, source),
            ]);
    }

    private static D20ContentModule Fundamentals()
    {
        var source = S("starter-fundamentals", "rules/packages/starter-ruleset/src/content/fundamentals.ts");
        return new D20ContentModule(I("starter-fundamentals"), D20Schemas.Content, [I("starter-abilities")], source,
            Defenses:
            [
                new(I("armor"), 10, [I("finesse")], source),
                new(I("grit"), 10, [I("might")], source),
                new(I("wits"), 10, [I("acuity"), I("intellect")], source),
                new(I("nerve"), 10, [I("conviction"), I("spirit")], source),
            ],
            Budgets:
            [
                new(I("standard-action"), ActivationTiming.Action, 1, source),
                new(I("bonus-action"), ActivationTiming.Action, 1, source),
                new(I("reaction"), ActivationTiming.Reaction, 1, source),
                new(I("movement"), ActivationTiming.Movement, 1, source),
            ],
            DamageTypes: [new(I("physical"), source), new(I("energy"), source), new(I("psychic"), source)],
            Resources: [new(I("guard"), 2, source), new(I("focus"), 3, source), new(I("resolve-points"), 2, source)]);
    }

    private static D20ContentModule SteelGuard()
    {
        var source = S("steel-guard-content", "rules/packages/starter-ruleset/src/content/steel_guard.ts");
        var physical = new DamageDefinition(I("physical"), 1, 8, 2);
        return new D20ContentModule(I("steel-guard-content"), D20Schemas.Content, [I("starter-abilities"), I("starter-fundamentals")], source,
            Armors:
            [
                new(I("chain-armor"), I("armor"), 3, I("body"), source),
                new(I("buckler"), I("armor"), 1, I("offhand"), source),
            ],
            Implements:
            [
                new(I("training-blade"), I("mainhand"), [I("blade")], I("might"), I("armor"), physical, 1, source),
                new(I("field-bow"), I("mainhand"), [I("bow")], I("finesse"), I("armor"), new DamageDefinition(I("physical"), 1, 6, 1), 8, source),
            ],
            Effects:
            [
                new(I("parry-stance"), I("armor"), 2, 1, [], source),
                new(I("bleeding"), null, 0, 2, [new(ConditionKind.AttackPenalty, Amount: 1)], source),
                new(I("held"), null, 0, 1, [new(ConditionKind.ForbidMovement)], source),
                new(I("unsettled"), I("nerve"), -1, 1, [], source),
            ],
            Reactions: [new(I("parry"), I("armor"), 2, I("guard"), 1, [new(I("reaction"), 1)], I("parry-stance"), source)],
            Actions:
            [
                new(I("longsword-strike"), [I("weapon")], [new(I("standard-action"), 1)], new(TargetKind.Participant, TargetTeam.Hostile, 1, true), new(null, null, null, I("training-blade")), null, 0, source),
                new(I("precise-shot"), [I("weapon"), I("ranged")], [new(I("standard-action"), 1)], new(TargetKind.Participant, TargetTeam.Hostile, 1, true), new(null, null, null, I("field-bow")), I("bleeding"), 0, source),
                new(I("pin-in-place"), [I("weapon"), I("control")], [new(I("standard-action"), 1)], new(TargetKind.Participant, TargetTeam.Hostile, 1, true), new(null, null, null, I("field-bow")), I("held"), 0, source),
                new(I("disrupt"), [I("martial")], [new(I("bonus-action"), 1)], new(TargetKind.Participant, TargetTeam.Hostile, 1, true), new(I("might"), I("grit"), new DamageDefinition(I("physical"), 1, 4, 0), null), I("unsettled"), 0, source),
            ]);
    }

    private static D20ContentModule EmberWard()
    {
        var source = S("ember-ward-content", "rules/packages/starter-ruleset/src/content/ember_ward.ts");
        return new D20ContentModule(I("ember-ward-content"), D20Schemas.Content, [I("starter-abilities"), I("starter-fundamentals")], source,
            Armors: [new(I("runed-robe"), I("nerve"), 2, I("body"), source), new(I("mindward-charm"), I("wits"), 2, I("charm"), source)],
            Implements: [new(I("ember-ward"), I("focus-slot"), [I("ember")], I("spirit"), I("nerve"), new DamageDefinition(I("energy"), 1, 8, 2), 6, source)],
            Effects: [new(I("scorched"), null, 0, 2, [new(ConditionKind.AttackPenalty, Amount: 1)], source)],
            Reactions: [new(I("ward-flare"), I("nerve"), 2, I("focus"), 1, [new(I("reaction"), 1)], I("scorched"), source)],
            Actions:
            [
                new(I("fire-bolt"), [I("ember")], [new(I("standard-action"), 1)], new(TargetKind.Participant, TargetTeam.Hostile, 1, true), new(null, null, null, I("ember-ward")), I("scorched"), 0, source),
                new(I("mind-spike"), [I("psychic")], [new(I("standard-action"), 1)], new(TargetKind.Participant, TargetTeam.Hostile, 1, true), new(I("intellect"), I("wits"), new DamageDefinition(I("psychic"), 1, 6, 1), null), null, 0, source),
            ]);
    }

    private static D20ContentModule WardensGate()
    {
        var source = S("wardens-gate", "rules/packages/starter-ruleset/src/content/adventures/wardens_gate.ts");
        var party = new[] { I("mara-venn"), I("ilyra-fen"), I("corin-ash"), I("veyra-quill") };
        var characters = new[]
        {
            Character("mara-venn", "Mara Venn", "Ward Anchor", ["longsword-strike", "precise-shot", "pin-in-place", "disrupt"], ["parry"], source),
            Character("ilyra-fen", "Ilyra Fen", "Pathfinder", ["precise-shot", "pin-in-place", "disrupt"], ["parry"], source),
            Character("corin-ash", "Corin Ash", "Signal Guide", ["longsword-strike", "precise-shot", "disrupt"], ["parry"], source),
            Character("veyra-quill", "Veyra Quill", "Field Shaper", ["longsword-strike", "pin-in-place", "disrupt"], ["parry"], source),
            Character("iron-warden", "Iron Warden", "Gate Sentinel", ["longsword-strike", "disrupt"], ["parry"], source),
            Character("gate-skirmisher", "Gate Skirmisher", "Warden Pathfinder", ["precise-shot", "pin-in-place"], ["parry"], source),
            Character("gate-sentry", "Gate Sentry", "Line Sentry", ["longsword-strike", "precise-shot"], ["parry"], source),
            Character("seal-adept", "Seal Adept", "Field Adept", ["precise-shot", "disrupt"], ["parry"], source),
        };
        var wardenItems = Items(source,
            ("warden-chain", "Warden chain armor", EquipmentKind.Armor, "chain-armor", "iron-warden", true),
            ("mara-chain", "Mara's chain armor", EquipmentKind.Armor, "chain-armor", "mara-venn", true),
            ("mara-buckler", "Mara's buckler", EquipmentKind.Armor, "buckler", "mara-venn", true),
            ("spare-buckler", "Spare buckler", EquipmentKind.Armor, "buckler", "camp-stash", false),
            ("spare-chain", "Spare chain armor", EquipmentKind.Armor, "chain-armor", "camp-stash", false),
            ("spare-blade", "Spare training blade", EquipmentKind.Implement, "training-blade", "camp-stash", false),
            ("spare-bow", "Spare field bow", EquipmentKind.Implement, "field-bow", "camp-stash", false),
            ("warden-blade", "Warden's training blade", EquipmentKind.Implement, "training-blade", "iron-warden", true),
            ("mara-blade", "Mara's training blade", EquipmentKind.Implement, "training-blade", "mara-venn", true),
            ("warden-bow", "Warden's field bow", EquipmentKind.Implement, "field-bow", "iron-warden", true),
            ("mara-bow", "Mara's field bow", EquipmentKind.Implement, "field-bow", "mara-venn", true),
            ("ilyra-chain", "Ilyra's chain armor", EquipmentKind.Armor, "chain-armor", "ilyra-fen", true),
            ("ilyra-blade", "Ilyra's training blade", EquipmentKind.Implement, "training-blade", "ilyra-fen", true),
            ("ilyra-bow", "Ilyra's field bow", EquipmentKind.Implement, "field-bow", "ilyra-fen", true),
            ("corin-chain", "Corin's chain armor", EquipmentKind.Armor, "chain-armor", "corin-ash", true),
            ("corin-blade", "Corin's training blade", EquipmentKind.Implement, "training-blade", "corin-ash", true),
            ("corin-bow", "Corin's field bow", EquipmentKind.Implement, "field-bow", "corin-ash", true),
            ("veyra-chain", "Veyra's chain armor", EquipmentKind.Armor, "chain-armor", "veyra-quill", true),
            ("veyra-blade", "Veyra's training blade", EquipmentKind.Implement, "training-blade", "veyra-quill", true),
            ("veyra-bow", "Veyra's field bow", EquipmentKind.Implement, "field-bow", "veyra-quill", true),
            ("skirmisher-chain", "Skirmisher chain armor", EquipmentKind.Armor, "chain-armor", "gate-skirmisher", true),
            ("skirmisher-blade", "Skirmisher training blade", EquipmentKind.Implement, "training-blade", "gate-skirmisher", true),
            ("skirmisher-bow", "Skirmisher field bow", EquipmentKind.Implement, "field-bow", "gate-skirmisher", true),
            ("sentry-chain", "Sentry chain armor", EquipmentKind.Armor, "chain-armor", "gate-sentry", true),
            ("sentry-blade", "Sentry training blade", EquipmentKind.Implement, "training-blade", "gate-sentry", true),
            ("sentry-bow", "Sentry field bow", EquipmentKind.Implement, "field-bow", "gate-sentry", true),
            ("adept-chain", "Adept chain armor", EquipmentKind.Armor, "chain-armor", "seal-adept", true),
            ("adept-blade", "Adept training blade", EquipmentKind.Implement, "training-blade", "seal-adept", true),
            ("adept-bow", "Adept field bow", EquipmentKind.Implement, "field-bow", "seal-adept", true),
            ("gate-sigil-buckler", "Gate sigil buckler", EquipmentKind.Armor, "buckler", "gate-cache", false));
        var encounters = new[]
        {
            Encounter("iron-warden", "The Iron Warden", ["mara-venn", "ilyra-fen", "corin-ash", "veyra-quill"], ["iron-warden", "gate-skirmisher"], 12, 8, source, "warden-chain"),
            Encounter("seal-guard", "The Seal Guard", ["mara-venn", "ilyra-fen", "corin-ash", "veyra-quill"], ["gate-sentry", "seal-adept"], 11, 7, source, null),
            Encounter("wardens-reckoning", "The Warden's Reckoning", ["mara-venn", "ilyra-fen", "corin-ash", "veyra-quill"], ["iron-warden", "seal-adept"], 12, 8, source, null),
        };
        var dungeon = new DungeonDefinition("Warden's Gate Pass", I("mountain-fortress"), 11, 7,
            ["###########", "#.........#", "#.#####.#.#", "#.....#.#.#", "#####.#.#.#", "#.........#", "###########"],
            new(1, 1), I("gate-camp"), DungeonFacing.East,
            [new(I("iron-warden"), new(9, 1)), new(I("seal-guard"), new(9, 5)), new(I("wardens-reckoning"), new(1, 5))],
            [new(I("gate-murder-holes"), new(5, 1), "Silent murder holes", "Arrow slits watch the pass."), new(I("warden-seal"), new(5, 5), "The broken seal", "A split iron seal marks the redoubt.")],
            [new(I("inner-sigil-gate"), new(9, 4), DungeonFacing.South, "The inner sigil gate", "The recovered sigil opens the iron leaves.", I("sigil-cache"))],
            [new(I("sigil-cache"), new(9, 2), I("gate-sigil-buckler"), "The Warden sigil cache", "A marked buckler waits beneath a loose stone.")],
            [new(I("gate-camp"), new(1, 1), "Pass camp", "The company can return safely."), new(I("warden-refuge"), new(9, 3), "Warden refuge", "A sheltered alcove offers a route back.")]);
        return new D20ContentModule(I("wardens-gate-adventure"), D20Schemas.Content, [I("starter-abilities"), I("starter-fundamentals"), I("steel-guard-content")], source,
            Features: [new(I("warden-training"), "Warden training", "A clean-room authored gate expedition trait.", source)],
            Characters: characters, Storage: [new(I("camp-stash"), "Camp stash", 24, source), new(I("gate-cache"), "Sealed gate cache", 1, source)], Items: wardenItems, Encounters: encounters,
            Adventures: [new(I("wardens-gate"), "The Warden's Gate", true, true, party, characters.Select(value => value.Id).ToArray(), I("camp-stash"), [I("camp-stash"), I("gate-cache")], wardenItems.Select(value => value.Id).ToArray(), encounters.Select(value => value.Id).ToArray(), dungeon, new("Warden's Gate", "The mountain pass is secure", "Mara's company opens the pass.", "The expedition ends at the redoubt", "The company survives but cannot break the final redoubt.", ["Ordered outcomes, treasure, door, checkpoints, and party facts are saved."]), source)]);
    }

    private static D20ContentModule EmbersWake()
    {
        var source = S("embers-wake", "rules/packages/starter-ruleset/src/content/adventures/embers_wake.ts");
        var sera = Character("sera-vale", "Sera Vale", "Ember Adept", ["fire-bolt", "mind-spike"], ["ward-flare"], source);
        var seer = Character("ash-seer", "Ash Seer", "Reliquary Keeper", ["mind-spike", "fire-bolt"], ["ward-flare"], source);
        var items = Items(source,
            ("seer-charm", "Ash Seer's mindward charm", EquipmentKind.Armor, "mindward-charm", "ash-seer", true),
            ("sera-robe", "Sera's runed robe", EquipmentKind.Armor, "runed-robe", "sera-vale", true),
            ("sera-charm", "Sera's mindward charm", EquipmentKind.Armor, "mindward-charm", "sera-vale", true),
            ("spare-robe", "Spare runed robe", EquipmentKind.Armor, "runed-robe", "ember-camp-stash", false));
        var encounter = Encounter("ash-seer", "The Ash Seer", ["sera-vale"], ["ash-seer"], 10, 7, source, "seer-charm");
        var dungeon = new DungeonDefinition("Ember Reliquary", I("ember-vault"), 9, 7,
            ["#########", "#.......#", "#.#####.#", "#.#...#.#", "#.#.#.#.#", "#...#...#", "#########"],
            new(1, 1), I("ember-camp"), DungeonFacing.East, [new(I("ash-seer"), new(7, 5))],
            [new(I("ember-inscription"), new(7, 1), "Ash-written warning", "Only a focused mind may pass.")], [], [],
            [new(I("ember-camp"), new(1, 1), "Reliquary threshold", "Sera can leave safely.")]);
        return new D20ContentModule(I("embers-wake-adventure"), D20Schemas.Content, [I("starter-abilities"), I("starter-fundamentals"), I("ember-ward-content")], source,
            Features: [new(I("ember-attunement"), "Ember attunement", "A clean-room authored reliquary trait.", source)],
            Characters: [sera, seer], Storage: [new(I("ember-camp-stash"), "Ember camp stash", 8, source)], Items: items, Encounters: [encounter],
            Adventures: [new(I("embers-wake"), "Ember's Wake", false, true, [I("sera-vale")], [I("sera-vale"), I("ash-seer")], I("ember-camp-stash"), [I("ember-camp-stash")], items.Select(value => value.Id).ToArray(), [I("ash-seer")], dungeon, new("Ember Reliquary", "Ember's Wake complete", "Sera carries the recovered charm beyond the reliquary.", "Ember's Wake ended", "Sera survives but its ward remains unbroken.", ["Terminal outcome and resources are saved."]), source)]);
    }

    private static CharacterDefinition Character(string id, string name, string title, string[] actions, string[] reactions, SourceProvenance source) =>
        new(I(id), name, title, 1, 0, 24, new Dictionary<D20Id, int> { [I("might")] = 12, [I("finesse")] = 12, [I("acuity")] = 12, [I("intellect")] = 12, [I("conviction")] = 12, [I("spirit")] = 12 }, actions.Select(I).ToArray(), reactions.Select(I).ToArray(), [], source);

    private static ItemDefinition[] Items(SourceProvenance source, params (string Id, string Name, EquipmentKind Kind, string Equipment, string Owner, bool Equipped)[] entries) =>
        entries.Select(entry => new ItemDefinition(I(entry.Id), entry.Name, entry.Kind, I(entry.Equipment), I(entry.Owner), entry.Equipped, source)).ToArray();

    private static EncounterDefinition Encounter(string id, string title, string[] party, string[] opposition, int width, int height, SourceProvenance source, string? reward)
    {
        var roster = party.Select(value => new EncounterParticipant(I(value), EncounterFaction.Party)).Concat(opposition.Select(value => new EncounterParticipant(I(value), EncounterFaction.Opposition))).ToArray();
        var rows = Enumerable.Range(0, height).Select(y => y == 0 || y == height - 1 ? new string('#', width) : "#" + new string('.', width - 2) + "#").ToArray();
        var placements = roster.Select((entry, index) => new TacticalPlacement(entry.Character, new GridPosition(index < party.Length ? 1 + index : width - 2 - (index - party.Length), 1 + (index % (height - 2))))).ToArray();
        return new EncounterDefinition(I(id), title, roster, new TacticalBoard(width, height, rows, placements), new EncounterOutcome($"{title} defeated", "The authored encounter reward transfers exactly once.", reward is null ? null : I(reward), null), new EncounterOutcome($"{title} defeated the party", "No reward is granted; recovery remains bounded.", null, 12), source);
    }

    private static D20Id I(string value) => D20Id.Parse(value);
    private static SourceProvenance S(string subject, string donorPath) => new($"src/RustyD20.Core/Content/{subject}.cs", subject, "clean-room C# adaptation", donorPath);
}
