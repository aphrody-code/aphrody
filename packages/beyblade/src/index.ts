// SPDX-License-Identifier: Apache-2.0
export {
  SpinDirectionSchema,
  PartTypeSchema,
  BladeSchema,
  RatchetSchema,
  BitSchema,
  ComboSchema,
  type SpinDirection,
  type PartType,
  type Blade,
  type Ratchet,
  type Bit,
  type Combo,
  type ComboStats,
} from "./types";

export {
  BLADES,
  RATCHETS,
  BITS,
  PARTS_CATALOG,
  findBlade,
  findRatchet,
  findBit,
} from "./parts";

export { BeybladeCombo } from "./combo";

export {
  simulateBattle,
  type FinishType,
  type BattleRoundLog,
  type BattleResult,
  type BattleOptions,
} from "./battle";

export {
  StrategyAdvisor,
  type ArchetypeAnalysis,
  type MatchupWinRate,
  type OptimizationSuggestion,
} from "./advisor";
