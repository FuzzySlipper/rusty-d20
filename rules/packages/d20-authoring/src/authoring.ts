import { Buffer } from 'node:buffer';

import {
  authorRulePackage,
  type CanonicalRuleArtifact,
  type RulePackageDependencyDraft,
} from '@rusty-engine/gameplay-rules-authoring';
import {
  RuleContractError,
  type JsonValue,
  type RuleDiagnostic,
  type RulePackage,
} from '@rusty-engine/gameplay-rules-contracts';

import {
  D20_CANDIDATE_SCHEMA_VERSION,
  D20_ID_PATTERN,
  D20_LIMITS,
  type AbilityCandidate,
  type ActionCandidate,
  type ArmorCandidate,
  type D20Id,
  type D20RulesCandidate,
  type DamageTypeCandidate,
  type DefenseCandidate,
  type EffectCandidate,
  type ReactionCandidate,
  type ResourceCandidate,
} from './generated.js';

const d20IdRegex = new RegExp(D20_ID_PATTERN);

export interface D20Source {
  readonly id: string;
  readonly path: string;
}

export interface Located<T> {
  readonly value: T;
  readonly line: number;
  readonly column: number;
}

export interface D20Module {
  readonly source: D20Source;
  readonly abilities: readonly Located<AbilityCandidate>[];
  readonly defenses: readonly Located<DefenseCandidate>[];
  readonly damageTypes: readonly Located<DamageTypeCandidate>[];
  readonly resources: readonly Located<ResourceCandidate>[];
  readonly armors: readonly Located<ArmorCandidate>[];
  readonly effects: readonly Located<EffectCandidate>[];
  readonly reactions: readonly Located<ReactionCandidate>[];
  readonly actions: readonly Located<ActionCandidate>[];
}

export interface D20ModuleDraft {
  readonly abilities?: readonly Located<AbilityCandidate>[];
  readonly defenses?: readonly Located<DefenseCandidate>[];
  readonly damageTypes?: readonly Located<DamageTypeCandidate>[];
  readonly resources?: readonly Located<ResourceCandidate>[];
  readonly armors?: readonly Located<ArmorCandidate>[];
  readonly effects?: readonly Located<EffectCandidate>[];
  readonly reactions?: readonly Located<ReactionCandidate>[];
  readonly actions?: readonly Located<ActionCandidate>[];
}

export interface D20ModuleBuilder {
  ability(
    line: number,
    value: AbilityCandidate,
    column?: number,
  ): Located<AbilityCandidate>;
  defense(
    line: number,
    value: DefenseCandidate,
    column?: number,
  ): Located<DefenseCandidate>;
  damageType(
    line: number,
    value: DamageTypeCandidate,
    column?: number,
  ): Located<DamageTypeCandidate>;
  resource(
    line: number,
    value: ResourceCandidate,
    column?: number,
  ): Located<ResourceCandidate>;
  armor(
    line: number,
    value: ArmorCandidate,
    column?: number,
  ): Located<ArmorCandidate>;
  effect(
    line: number,
    value: EffectCandidate,
    column?: number,
  ): Located<EffectCandidate>;
  reaction(
    line: number,
    value: ReactionCandidate,
    column?: number,
  ): Located<ReactionCandidate>;
  action(
    line: number,
    value: ActionCandidate,
    column?: number,
  ): Located<ActionCandidate>;
}

export interface D20PackageDraft {
  readonly domain: string;
  readonly package: string;
  readonly version: number;
  readonly dependencies?: readonly RulePackageDependencyDraft[];
  readonly modules: readonly D20Module[];
}

type CandidateJson = D20RulesCandidate & JsonValue;

export interface D20CanonicalArtifact
  extends Omit<CanonicalRuleArtifact<JsonValue>, 'package'> {
  readonly package: RulePackage<JsonValue> & {
    readonly payload: CandidateJson;
  };
}

export interface D20MappedDiagnostic {
  readonly code: string;
  readonly severity: RuleDiagnostic['severity'];
  readonly logicalPath: string;
  readonly message: string;
  readonly source?: {
    readonly path: string;
    readonly line?: number;
    readonly column?: number;
  };
}

export class D20AuthoringError extends Error {
  public constructor(
    public readonly code: 'invalid-d20-identity' | 'conflicting-source',
    public readonly source: D20Source,
    public readonly line: number,
    public readonly column: number,
    message: string,
  ) {
    super(`${source.path}:${String(line)}:${String(column)}: ${message}`);
    this.name = 'D20AuthoringError';
  }
}

export function defineD20Module(
  source: D20Source,
  compose: (builder: D20ModuleBuilder) => D20ModuleDraft,
): D20Module {
  const stableSource = Object.freeze({ ...source });
  const builder = moduleBuilder(stableSource);
  const draft = compose(builder);
  return deepFreeze({
    source: stableSource,
    abilities: [...(draft.abilities ?? [])],
    defenses: [...(draft.defenses ?? [])],
    damageTypes: [...(draft.damageTypes ?? [])],
    resources: [...(draft.resources ?? [])],
    armors: [...(draft.armors ?? [])],
    effects: [...(draft.effects ?? [])],
    reactions: [...(draft.reactions ?? [])],
    actions: [...(draft.actions ?? [])],
  });
}

export function authorD20Package(
  draft: D20PackageDraft,
): D20CanonicalArtifact {
  const sources = collectSources(draft.modules);
  const abilities = collect<AbilityCandidate>(draft.modules, 'abilities');
  const defenses = collect<DefenseCandidate>(draft.modules, 'defenses');
  const damageTypes = collect<DamageTypeCandidate>(
    draft.modules,
    'damageTypes',
  );
  const resources = collect<ResourceCandidate>(draft.modules, 'resources');
  const armors = collect<ArmorCandidate>(draft.modules, 'armors');
  const effects = collect<EffectCandidate>(draft.modules, 'effects');
  const reactions = collect<ReactionCandidate>(draft.modules, 'reactions');
  const actions = collect<ActionCandidate>(draft.modules, 'actions');
  const payload = deepFreeze({
    schemaVersion: D20_CANDIDATE_SCHEMA_VERSION,
    abilities: values(abilities),
    defenses: values(defenses),
    damageTypes: values(damageTypes),
    resources: values(resources),
    armors: values(armors),
    effects: values(effects),
    reactions: values(reactions),
    actions: values(actions),
  }) as CandidateJson;
  const provenance = [
    ...provenanceFor('ability', abilities),
    ...provenanceFor('defense', defenses),
    ...provenanceFor('damage-type', damageTypes),
    ...provenanceFor('resource', resources),
    ...provenanceFor('armor', armors),
    ...provenanceFor('effect', effects),
    ...provenanceFor('reaction', reactions),
    ...provenanceFor('action', actions),
  ];

  return authorRulePackage<JsonValue>({
    domain: draft.domain,
    package: draft.package,
    version: draft.version,
    dependencies: draft.dependencies ?? [],
    sources,
    provenance,
    payload,
  }) as D20CanonicalArtifact;
}

export function exactDependencyOn(
  artifact: D20CanonicalArtifact,
): RulePackageDependencyDraft {
  return Object.freeze({
    domain: artifact.package.domain,
    package: artifact.package.package,
    version: artifact.package.version,
    fingerprint: artifact.fingerprint,
  });
}

export function mapD20Diagnostic(
  artifact: D20CanonicalArtifact,
  diagnostic: RuleDiagnostic,
): D20MappedDiagnostic {
  const correlation = diagnostic.correlation;
  const source =
    correlation === undefined
      ? undefined
      : artifact.package.sources.find(
          (candidate) => candidate.id === correlation.source,
        );
  return Object.freeze({
    code: diagnostic.code,
    severity: diagnostic.severity,
    logicalPath: diagnostic.logicalPath,
    message: diagnostic.message,
    ...(source === undefined || correlation === undefined
      ? {}
      : {
          source: Object.freeze({
            path: source.path,
            ...(correlation.line === undefined
              ? {}
              : { line: correlation.line }),
            ...(correlation.column === undefined
              ? {}
              : { column: correlation.column }),
          }),
        }),
  });
}

function moduleBuilder(source: D20Source): D20ModuleBuilder {
  const builder: D20ModuleBuilder = {
    ability: (line, value, column = 1) =>
      locate(source, line, column, value, [['id', value.id]]),
    defense: (line, value, column = 1) =>
      locate(source, line, column, value, [
        ['id', value.id],
        ['ability', value.ability],
      ]),
    damageType: (line, value, column = 1) =>
      locate(source, line, column, value, [['id', value.id]]),
    resource: (line, value, column = 1) =>
      locate(source, line, column, value, [['id', value.id]]),
    armor: (line, value, column = 1) =>
      locate(source, line, column, value, [
        ['id', value.id],
        ['defense', value.defense],
        ['slot', value.slot],
      ]),
    effect: (line, value, column = 1) =>
      locate(
        source,
        line,
        column,
        value,
        value.defense === null
          ? [['id', value.id]]
          : [
              ['id', value.id],
              ['defense', value.defense],
            ],
      ),
    reaction: (line, value, column = 1) =>
      locate(source, line, column, value, [
        ['id', value.id],
        ['defense', value.defense],
        ['resource', value.resource],
        ['effect', value.effect],
      ]),
    action: (line, value, column = 1) =>
      locate(
        source,
        line,
        column,
        {
          ...value,
          damage: { ...value.damage },
        },
        [
          ['id', value.id],
          ['ability', value.ability],
          ['defense', value.defense],
          ['damage.kind', value.damage.kind],
          ...(value.effect === null
            ? []
            : ([['effect', value.effect]] as const)),
        ],
      ),
  };
  return Object.freeze(builder);
}

function locate<T>(
  source: D20Source,
  line: number,
  column: number,
  value: T,
  identities: readonly (readonly [string, D20Id])[],
): Located<T> {
  requireLocation(line, column);
  for (const [field, identity] of identities) {
    requireD20Id(source, line, column, field, identity);
  }
  return deepFreeze({ value: { ...value }, line, column });
}

function requireLocation(line: number, column: number): void {
  if (
    !Number.isSafeInteger(line) ||
    !Number.isSafeInteger(column) ||
    line <= 0 ||
    column <= 0
  ) {
    throw new RuleContractError(
      'invalid-source-location',
      '$/provenance',
      'source line and column must be positive safe integers',
    );
  }
}

function requireD20Id(
  source: D20Source,
  line: number,
  column: number,
  field: string,
  value: D20Id,
): void {
  if (
    value.length === 0 ||
    Buffer.byteLength(value, 'utf8') > D20_LIMITS.maxIdBytes ||
    !d20IdRegex.test(value)
  ) {
    throw new D20AuthoringError(
      'invalid-d20-identity',
      source,
      line,
      column,
      `${field} is not a valid d20 identity: ${JSON.stringify(value)}`,
    );
  }
}

function collectSources(modules: readonly D20Module[]): readonly D20Source[] {
  const sources = new Map<string, D20Source>();
  for (const module of modules) {
    const previous = sources.get(module.source.id);
    if (previous !== undefined && previous.path !== module.source.path) {
      throw new D20AuthoringError(
        'conflicting-source',
        module.source,
        1,
        1,
        `source ${module.source.id} was already registered as ${previous.path}`,
      );
    }
    sources.set(module.source.id, module.source);
  }
  return [...sources.values()].sort((left, right) =>
    compareUtf8(left.id, right.id),
  );
}

function collect<T extends { readonly id: D20Id }>(
  modules: readonly D20Module[],
  key: DefinitionKey,
): readonly Collected<T>[] {
  return modules
    .flatMap((module) =>
      (module[key] as readonly Located<T>[]).map((entry) => ({
        ...entry,
        source: module.source,
      })),
    )
    .sort((left, right) => compareUtf8(left.value.id, right.value.id));
}

type DefinitionKey =
  | 'abilities'
  | 'defenses'
  | 'damageTypes'
  | 'resources'
  | 'armors'
  | 'effects'
  | 'reactions'
  | 'actions';

type Collected<T> = Located<T> & { readonly source: D20Source };

function values<T extends { readonly id: D20Id }>(
  definitions: readonly Collected<T>[],
): readonly T[] {
  return definitions.map((definition) => definition.value);
}

function provenanceFor<T extends { readonly id: D20Id }>(
  kind: string,
  definitions: readonly Collected<T>[],
): readonly {
  readonly subject: string;
  readonly source: string;
  readonly line: number;
  readonly column: number;
}[] {
  return definitions.map((definition) => ({
    subject: `${kind}:${definition.value.id}`,
    source: definition.source.id,
    line: definition.line,
    column: definition.column,
  }));
}

function compareUtf8(left: string, right: string): number {
  return Buffer.compare(Buffer.from(left, 'utf8'), Buffer.from(right, 'utf8'));
}

function deepFreeze<T>(value: T): T {
  if (Array.isArray(value)) {
    for (const entry of value) deepFreeze(entry);
  } else if (value !== null && typeof value === 'object') {
    for (const entry of Object.values(value)) deepFreeze(entry);
  }
  return Object.freeze(value);
}
