// SPDX-License-Identifier: Apache-2.0
import { BeybladeCombo } from "./combo";
import { simulateBattle } from "./battle";

export interface ArchetypeAnalysis {
  archetype: string;
  strengths: string[];
  weaknesses: string[];
  suggestions: string[];
}

export interface MatchupWinRate {
  opponentName: string;
  winRate: number; // percentage (0 - 100)
  primaryLossReason?: string;
}

export interface OptimizationSuggestion {
  partType: "blade" | "ratchet" | "bit";
  originalPart: string;
  recommendedPart: string;
  reason: string;
}

export const StrategyAdvisor = {
  /**
   * Analyzes a Beyblade combo to identify its archetype, strengths, and weaknesses.
   */
  analyze(combo: BeybladeCombo): ArchetypeAnalysis {
    const stats = combo.stats;
    const strengths: string[] = [];
    const weaknesses: string[] = [];
    const suggestions: string[] = [];
    let archetype = "Balanced Competitor";

    // 1. Determine Archetype
    if (stats.type === "Attack") {
      if (stats.attack >= 9) {
        archetype = "Hyper Aggressive Striker";
      } else {
        archetype = "Mobile Attacker";
      }
    } else if (stats.type === "Defense") {
      if (stats.defense >= 8.5) {
        archetype = "Impenetrable Wall";
      } else {
        archetype = "Stationary Defender";
      }
    } else if (stats.type === "Stamina") {
      if (stats.stamina >= 8.5) {
        archetype = "Spin-Maximizing Zombie";
      } else {
        archetype = "Outspin Specialist";
      }
    } else if (stats.type === "Balance") {
      archetype = "Versatile All-Rounder";
    }

    // 2. Strengths & Weaknesses
    if (stats.weight >= 46) {
      strengths.push("High mass increases smash recoil and knock-out resistance.");
    } else if (stats.weight < 40) {
      weaknesses.push("Low total mass makes the combo vulnerable to heavy recoils from Phoenix Wing/Cobalt Drake.");
    }

    if (stats.attack >= 8) {
      strengths.push("Devastating strike power, high probability of Over/Extreme Finishes.");
      weaknesses.push("Extremely fast spin/stamina depletion, prone to losing Spin Finishes if the opponent survives.");
    }

    if (stats.defense >= 8) {
      strengths.push("Excellent impact deflection, resists knock-outs.");
    }

    if (stats.stamina >= 8) {
      strengths.push("Incredible spin longevity. Almost guaranteed to win on Spin Finishes.");
      weaknesses.push("Susceptible to high recoil bursts due to lower baseline burst resistance.");
    }

    if (stats.burstResistance >= 8.0) {
      strengths.push("Superb burst resistance, highly secure ratchet-bit locking mechanism.");
    } else if (stats.burstResistance < 6.0) {
      weaknesses.push("Low burst resistance; high risk of bursting when facing left-spin or high-recoil attack blades.");
    }

    if (stats.height === 80) {
      strengths.push("High stature allows striking opponents downward, compromising their stability.");
      weaknesses.push("Exposed ratchet makes it significantly easier for low-height (60) attackers to trigger a Burst Finish.");
    } else {
      strengths.push("Low height (60) keeps the center of gravity stable and shields the ratchet.");
    }

    // 3. Suggestions
    if (stats.type === "Attack" && stats.burstResistance < 7) {
      suggestions.push("Consider upgrading your Bit to Gear Flat or your Ratchet to 5-60/9-60 to secure your locks during high-impact collisions.");
    }
    if (stats.type === "Stamina" && stats.weight < 42) {
      suggestions.push("Switch to a heavier blade like Phoenix Wing or Cobalt Drake to gain recoil-deflection without sacrificing your Ball bit.");
    }
    if (stats.type === "Defense" && stats.height === 80) {
      suggestions.push("Lower your height to 60 (using 3-60 or 9-60) to keep your center of gravity low and prevent under-cuts.");
    }

    return {
      archetype,
      strengths,
      weaknesses,
      suggestions,
    };
  },

  /**
   * Simulates matchup win rates against standard baseline competitive combos.
   */
  testMatchups(combo: BeybladeCombo, iterations = 100): MatchupWinRate[] {
    const baselines = [
      { name: "Phoenix Wing 9-60 Flat (Attack)", blade: "phoenix_wing", ratchet: "9_60", bit: "flat" },
      { name: "Hell Scythe 5-60 Taper (Balance)", blade: "hell_scythe", ratchet: "5_60", bit: "taper" },
      { name: "Wizard Arrow 9-60 Ball (Stamina)", blade: "wizard_arrow", ratchet: "9_60", bit: "ball" },
      { name: "Black Shell 5-80 Hexa (Defense)", blade: "black_shell", ratchet: "5_80", bit: "hexa" },
    ];

    const results: MatchupWinRate[] = [];

    for (const b of baselines) {
      const oppCombo = new BeybladeCombo(b.blade, b.ratchet, b.bit);
      let wins = 0;
      let lossTypesCount: Record<string, number> = {};

      for (let i = 0; i < iterations; i++) {
        const battle = simulateBattle(combo, oppCombo, { randomFactor: true });
        if (battle.winner === "A") {
          wins++;
        } else if (battle.winner === "B") {
          lossTypesCount[battle.finishType] = (lossTypesCount[battle.finishType] || 0) + 1;
        }
      }

      // Find primary loss reason
      let primaryLossReason = "Outplayed";
      let maxLosses = 0;
      for (const [type, count] of Object.entries(lossTypesCount)) {
        if (count > maxLosses) {
          maxLosses = count;
          primaryLossReason = type;
        }
      }

      results.push({
        opponentName: b.name,
        winRate: Math.round((wins / iterations) * 100),
        primaryLossReason: maxLosses > 0 ? primaryLossReason : undefined,
      });
    }

    return results;
  }
};
