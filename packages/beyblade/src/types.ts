// SPDX-License-Identifier: Apache-2.0
import { z } from "zod";

export const SpinDirectionSchema = z.enum(["Right", "Left", "Dual"]);
export type SpinDirection = z.infer<typeof SpinDirectionSchema>;

export const PartTypeSchema = z.enum(["Attack", "Defense", "Stamina", "Balance"]);
export type PartType = z.infer<typeof PartTypeSchema>;

export const BladeSchema = z.object({
  id: z.string(),
  name: z.string(),
  type: PartTypeSchema,
  weight: z.number(), // in grams
  spinDirection: SpinDirectionSchema,
  description: z.string().optional(),
  attack: z.number().min(1).max(10),
  defense: z.number().min(1).max(10),
  stamina: z.number().min(1).max(10),
});
export type Blade = z.infer<typeof BladeSchema>;

export const RatchetSchema = z.object({
  id: z.string(),
  name: z.string(), // e.g. "3-60"
  weight: z.number(),
  points: z.number(), // number of contact points/protrusions, e.g. 3, 5
  height: z.number(), // in tenths of mm (e.g., 60, 80)
  description: z.string().optional(),
  burstResistance: z.number().min(1).max(10),
});
export type Ratchet = z.infer<typeof RatchetSchema>;

export const BitSchema = z.object({
  id: z.string(),
  name: z.string(), // e.g. "Flat", "Ball"
  type: PartTypeSchema,
  weight: z.number(),
  speed: z.number().min(1).max(10),
  stamina: z.number().min(1).max(10),
  defense: z.number().min(1).max(10),
  burstResistance: z.number().min(1).max(10),
  description: z.string().optional(),
});
export type Bit = z.infer<typeof BitSchema>;

export const ComboSchema = z.object({
  name: z.string().optional(),
  bladeId: z.string(),
  ratchetId: z.string(),
  bitId: z.string(),
});
export type Combo = z.infer<typeof ComboSchema>;

export interface ComboStats {
  weight: number;
  height: number;
  type: PartType;
  spinDirection: SpinDirection;
  attack: number;
  defense: number;
  stamina: number;
  burstResistance: number;
}
