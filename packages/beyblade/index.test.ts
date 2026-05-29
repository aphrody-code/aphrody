// SPDX-License-Identifier: Apache-2.0
import { expect, test, describe } from "bun:test";
import {
  findBlade,
  findRatchet,
  findBit,
  BeybladeCombo,
  simulateBattle,
  StrategyAdvisor,
} from "./src/index";

describe("Beyblade X Module Tests", () => {
  test("Parts Catalog resolves official parts", () => {
    const blade = findBlade("dran_sword");
    expect(blade).toBeDefined();
    expect(blade?.name).toBe("Dran Sword");
    expect(blade?.type).toBe("Attack");
    expect(blade?.weight).toBe(35.0);

    const ratchet = findRatchet("3_60");
    expect(ratchet).toBeDefined();
    expect(ratchet?.points).toBe(3);
    expect(ratchet?.height).toBe(60);

    const bit = findBit("flat");
    expect(bit).toBeDefined();
    expect(bit?.type).toBe("Attack");
    expect(bit?.speed).toBe(9);
  });

  test("Combo Calculator calculates aggregates and weights", () => {
    const combo = new BeybladeCombo("phoenix_wing", "9_60", "ball");
    expect(combo.name).toBe("Phoenix Wing 9-60Ball");

    const stats = combo.stats;
    expect(stats.weight).toBe(47.3); // 38.5 + 6.8 + 2.0 = 47.3
    expect(stats.height).toBe(60);
    expect(stats.type).toBe("Stamina"); // Inherited from Ball bit
    expect(stats.spinDirection).toBe("Right");
    
    // Check aggregate calculations
    expect(stats.attack).toBeGreaterThanOrEqual(1);
    expect(stats.defense).toBeGreaterThanOrEqual(1);
    expect(stats.stamina).toBeGreaterThanOrEqual(1);
  });

  test("Battle Simulator resolves finite outcomes", () => {
    const comboA = new BeybladeCombo("phoenix_wing", "9_60", "flat"); // High attack
    const comboB = new BeybladeCombo("wizard_arrow", "5_80", "ball"); // High stamina

    const result = simulateBattle(comboA, comboB, { randomFactor: false });
    expect(result.winner).toBeDefined();
    expect(result.finishType).toBeDefined();
    expect(result.points).toBeGreaterThanOrEqual(1);
    expect(result.roundsLog.length).toBeGreaterThan(1);
  });

  test("Strategy Advisor returns correct archetype and matchup statistics", () => {
    const combo = new BeybladeCombo("phoenix_wing", "9_60", "flat");
    const analysis = StrategyAdvisor.analyze(combo);
    expect(analysis.archetype).toBe("Hyper Aggressive Striker");
    expect(analysis.strengths.length).toBeGreaterThan(0);

    const matchups = StrategyAdvisor.testMatchups(combo, 10);
    expect(matchups.length).toBe(4);
    expect(matchups[0].winRate).toBeGreaterThanOrEqual(0);
    expect(matchups[0].winRate).toBeLessThanOrEqual(100);
  });
});
