// SPDX-License-Identifier: Apache-2.0
import type { BeybladeCombo } from "./combo";

export type FinishType = "Spin Finish" | "Over Finish" | "Extreme Finish" | "Burst Finish" | "Draw";

export interface BattleRoundLog {
  tick: number;
  event: string;
  spinA: number;
  spinB: number;
}

export interface BattleResult {
  winner: "A" | "B" | "Draw";
  finishType: FinishType;
  points: number;
  roundsLog: BattleRoundLog[];
}

export interface BattleOptions {
  maxTicks?: number;
  randomFactor?: boolean;
}

/**
 * Simulates a single battle round between two Beyblade Combos based on their stats
 * and physical interactions (collisions, stamina, burst, X-Dash).
 */
export function simulateBattle(
  comboA: BeybladeCombo,
  comboB: BeybladeCombo,
  options: BattleOptions = {}
): BattleResult {
  const maxTicks = options.maxTicks || 120;
  const useRandom = options.randomFactor !== false;

  const statsA = comboA.stats;
  const statsB = comboB.stats;

  // Initialize state
  let spinA = statsA.stamina * 100 + statsA.weight * 30;
  let spinB = statsB.stamina * 100 + statsB.weight * 30;



  let burstThresholdA = statsA.burstResistance * 20;
  let burstThresholdB = statsB.burstResistance * 20;

  let burstProgressA = 0;
  let burstProgressB = 0;

  const roundsLog: BattleRoundLog[] = [];
  
  // Stamina drain rates
  const drainA = 1.0 + (statsA.attack * 0.15) - (statsA.stamina * 0.05);
  const drainB = 1.0 + (statsB.attack * 0.15) - (statsB.stamina * 0.05);

  roundsLog.push({
    tick: 0,
    event: `Battle starts: A [${comboA.name}] vs B [${comboB.name}]`,
    spinA,
    spinB,
  });

  // Main tick loop
  for (let tick = 1; tick <= maxTicks; tick++) {
    // 1. Drain spin
    spinA = Math.max(0, parseFloat((spinA - drainA).toFixed(2)));
    spinB = Math.max(0, parseFloat((spinB - drainB).toFixed(2)));

    // Check spin finish
    if (spinA <= 0 && spinB <= 0) {
      return { winner: "Draw", finishType: "Draw", points: 0, roundsLog };
    }
    if (spinA <= 0) {
      roundsLog.push({ tick, event: `${comboA.name} stopped spinning.`, spinA, spinB });
      return { winner: "B", finishType: "Spin Finish", points: 1, roundsLog };
    }
    if (spinB <= 0) {
      roundsLog.push({ tick, event: `${comboB.name} stopped spinning.`, spinA, spinB });
      return { winner: "A", finishType: "Spin Finish", points: 1, roundsLog };
    }

    // 2. Collision detection
    // Attack-heavy combos cause more frequent collisions
    const baseCollisionChance = 0.25;
    const modifier = (statsA.attack + statsB.attack) * 0.03;
    const collisionRoll = useRandom ? Math.random() : 0.4;

    if (collisionRoll < (baseCollisionChance + modifier)) {
      // Collision occurred!
      // Base impact is based on attack stats and weight differences
      let impactA = statsA.attack * 2.5 + (statsA.weight - statsB.weight) * 1.5;
      let impactB = statsB.attack * 2.5 + (statsB.weight - statsA.weight) * 1.5;
      
      impactA = Math.max(2, impactA);
      let impactBFinal = Math.max(2, impactB);

      // Check for X-Dash (Extreme Dash)
      // Attack bits have higher chance of riding the rail
      const xDashChanceA = statsA.attack * 0.05;
      const xDashChanceB = statsB.attack * 0.05;
      const xDashRollA = useRandom ? Math.random() : 0.8;
      const xDashRollB = useRandom ? Math.random() : 0.8;

      let isXDashA = xDashRollA < xDashChanceA;
      let isXDashB = xDashRollB < xDashChanceB;

      if (isXDashA) {
        impactA *= 2.0;
        roundsLog.push({ tick, event: `⚡️ ${comboA.name} triggers an EXTREME DASH!`, spinA, spinB });
      }
      if (isXDashB) {
        impactBFinal *= 2.0;
        roundsLog.push({ tick, event: `⚡️ ${comboB.name} triggers an EXTREME DASH!`, spinA, spinB });
      }

      // Apply damage to spin/stamina
      spinA = Math.max(0, parseFloat((spinA - impactBFinal * 1.8).toFixed(2)));
      spinB = Math.max(0, parseFloat((spinB - impactA * 1.8).toFixed(2)));

      // Add to burst progress
      // Recoil increases burst risk. Height difference also adds minor penalty.
      const heightDiff = Math.abs(statsA.height - statsB.height);
      const heightPenalty = heightDiff > 0 ? 1.2 : 1.0;

      const burstDmgA = (impactBFinal * 1.2 * heightPenalty) / (statsA.burstResistance * 0.5);
      const burstDmgB = (impactA * 1.2 * heightPenalty) / (statsB.burstResistance * 0.5);

      burstProgressA += burstDmgA;
      burstProgressB += burstDmgB;

      roundsLog.push({
        tick,
        event: `Collision! Impact A: ${impactA.toFixed(1)}, Impact B: ${impactBFinal.toFixed(1)} | Burst progress: A=${burstProgressA.toFixed(1)}/${burstThresholdA}, B=${burstProgressB.toFixed(1)}/${burstThresholdB}`,
        spinA,
        spinB,
      });

      // 3. Check Burst Finish
      if (burstProgressA >= burstThresholdA && burstProgressB >= burstThresholdB) {
        // Double burst! Winner is the one with more remaining spin
        roundsLog.push({ tick, event: `💥 DOUBLE BURST FINISH!`, spinA, spinB });
        return spinA > spinB
          ? { winner: "A", finishType: "Burst Finish", points: 2, roundsLog }
          : { winner: "B", finishType: "Burst Finish", points: 2, roundsLog };
      }
      if (burstProgressA >= burstThresholdA) {
        roundsLog.push({ tick, event: `💥 ${comboA.name} BURSTS!`, spinA, spinB });
        return { winner: "B", finishType: "Burst Finish", points: 2, roundsLog };
      }
      if (burstProgressB >= burstThresholdB) {
        roundsLog.push({ tick, event: `💥 ${comboB.name} BURSTS!`, spinA, spinB });
        return { winner: "A", finishType: "Burst Finish", points: 2, roundsLog };
      }

      // 4. Check Knock-out (Over Finish / Extreme Finish)
      const koRollA = useRandom ? Math.random() : 0.95;
      const koRollB = useRandom ? Math.random() : 0.95;

      const baseKoChance = 0.02;
      const koChanceA = baseKoChance + (impactA * 0.008);
      const koChanceB = baseKoChance + (impactBFinal * 0.008);

      if (isXDashA && koRollA < (koChanceA * 1.5)) {
        roundsLog.push({ tick, event: `🚩 ${comboB.name} is knocked out through the EXTREME ZONE!`, spinA, spinB });
        return { winner: "A", finishType: "Extreme Finish", points: 3, roundsLog };
      }
      if (isXDashB && koRollB < (koChanceB * 1.5)) {
        roundsLog.push({ tick, event: `🚩 ${comboA.name} is knocked out through the EXTREME ZONE!`, spinA, spinB });
        return { winner: "B", finishType: "Extreme Finish", points: 3, roundsLog };
      }

      if (koRollA < koChanceA) {
        roundsLog.push({ tick, event: `🚪 ${comboB.name} is knocked into the pocket for an OVER FINISH!`, spinA, spinB });
        return { winner: "A", finishType: "Over Finish", points: 2, roundsLog };
      }
      if (koRollB < koChanceB) {
        roundsLog.push({ tick, event: `🚪 ${comboA.name} is knocked into the pocket for an OVER FINISH!`, spinA, spinB });
        return { winner: "B", finishType: "Over Finish", points: 2, roundsLog };
      }
    }
  }

  // End of match by ticks (Spin Finish)
  roundsLog.push({ tick: maxTicks, event: `Time out limit reached. Checking remaining spin...`, spinA, spinB });
  if (Math.abs(spinA - spinB) < 10) {
    return { winner: "Draw", finishType: "Draw", points: 0, roundsLog };
  }
  return spinA > spinB
    ? { winner: "A", finishType: "Spin Finish", points: 1, roundsLog }
    : { winner: "B", finishType: "Spin Finish", points: 1, roundsLog };
}

export interface MatchResult {
  winner: "A" | "B" | "Draw";
  scoreA: number;
  scoreB: number;
  rounds: BattleResult[];
}

/**
 * Simulates a full WBO match (1on1 or 3on3) until one player reaches the target point limit.
 */
export function simulateMatch(
  combosA: BeybladeCombo[],
  combosB: BeybladeCombo[],
  format: "1on1" | "3on3" = "1on1",
  targetPoints: number = 4,
  options: BattleOptions = {}
): MatchResult {
  if (combosA.length === 0 || combosB.length === 0) {
    throw new Error("Each player must have at least one combo.");
  }

  let scoreA = 0;
  let scoreB = 0;
  const rounds: BattleResult[] = [];

  let roundIndex = 0;

  while (scoreA < targetPoints && scoreB < targetPoints) {
    let comboA: BeybladeCombo;
    let comboB: BeybladeCombo;

    if (format === "3on3") {
      const idxA = roundIndex % Math.min(3, combosA.length);
      const idxB = roundIndex % Math.min(3, combosB.length);
      comboA = combosA[idxA];
      comboB = combosB[idxB];
    } else {
      comboA = combosA[0];
      comboB = combosB[0];
    }

    const roundResult = simulateBattle(comboA, comboB, options);
    rounds.push(roundResult);

    if (roundResult.winner === "A") {
      scoreA += roundResult.points;
    } else if (roundResult.winner === "B") {
      scoreB += roundResult.points;
    }

    roundIndex++;

    if (roundIndex > 50) {
      break;
    }
  }

  let winner: "A" | "B" | "Draw" = "Draw";
  if (scoreA >= targetPoints && scoreB >= targetPoints) {
    winner = scoreA > scoreB ? "A" : (scoreB > scoreA ? "B" : "Draw");
  } else if (scoreA >= targetPoints) {
    winner = "A";
  } else if (scoreB >= targetPoints) {
    winner = "B";
  }

  return {
    winner,
    scoreA,
    scoreB,
    rounds,
  };
}
