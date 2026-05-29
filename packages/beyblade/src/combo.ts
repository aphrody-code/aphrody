// SPDX-License-Identifier: Apache-2.0
import { findBlade, findRatchet, findBit } from "./parts";
import type { Blade, Ratchet, Bit, ComboStats, PartType } from "./types";

export class BeybladeCombo {
  public blade: Blade;
  public ratchet: Ratchet;
  public bit: Bit;

  constructor(bladeId: string, ratchetId: string, bitId: string) {
    const b = findBlade(bladeId);
    if (!b) throw new Error(`Blade not found with ID: ${bladeId}`);
    const r = findRatchet(ratchetId);
    if (!r) throw new Error(`Ratchet not found with ID: ${ratchetId}`);
    const bt = findBit(bitId);
    if (!bt) throw new Error(`Bit not found with ID: ${bitId}`);

    this.blade = b;
    this.ratchet = r;
    this.bit = bt;
  }

  public get name(): string {
    return `${this.blade.name} ${this.ratchet.name}${this.bit.name}`;
  }

  public get stats(): ComboStats {
    const totalWeight = parseFloat((this.blade.weight + this.ratchet.weight + this.bit.weight).toFixed(2));
    
    // In Beyblade X, the Bit's type dominates movement style, so it defines the overall combo type.
    const type: PartType = this.bit.type;

    // Aggregate stats from components
    const attack = Math.round((this.blade.attack * 0.6 + this.bit.speed * 0.4) * 10) / 10;
    const defense = Math.round((this.blade.defense * 0.5 + this.bit.defense * 0.5) * 10) / 10;
    const stamina = Math.round((this.blade.stamina * 0.5 + this.bit.stamina * 0.5) * 10) / 10;
    const burstResistance = Math.round((this.ratchet.burstResistance * 0.4 + this.bit.burstResistance * 0.6) * 10) / 10;

    return {
      weight: totalWeight,
      height: this.ratchet.height,
      type,
      spinDirection: this.blade.spinDirection,
      attack,
      defense,
      stamina,
      burstResistance,
    };
  }
}
