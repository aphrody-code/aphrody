// SPDX-License-Identifier: Apache-2.0
import { type BeybladeCombo } from "./combo";
import { simulateBattle, type BattleResult } from "./battle";

export type BattleType = "1on1" | "3on3" | "counter";
export type MatchFormat = "4-point" | "5-point" | "7-point" | "best-of-3";

export interface SetResult {
  winner: "A" | "B" | "Draw";
  scoreA: number;
  scoreB: number;
  rounds: BattleResult[];
}

export interface WboMatchResult {
  winner: "A" | "B" | "Draw";
  setScores: { A: number; B: number };
  sets: SetResult[];
}

/**
 * Simulates a single set under WBO tournament rules.
 */
export function simulateSet(
  combosA: BeybladeCombo[],
  combosB: BeybladeCombo[],
  battleType: BattleType = "1on1",
  targetPoints: number = 4,
  randomFactor: boolean = true
): SetResult {
  let scoreA = 0;
  let scoreB = 0;
  const rounds: BattleResult[] = [];
  let roundIndex = 0;

  // Deck states for 3on3 and Counter
  let deckA = [...combosA];
  let deckB = [...combosB];

  // Active indices for Counter battle
  let activeIdxA = 0;
  let activeIdxB = 0;

  while (scoreA < targetPoints && scoreB < targetPoints) {
    let comboA: BeybladeCombo;
    let comboB: BeybladeCombo;

    if (battleType === "3on3") {
      // 3on3 Battle Official:
      // Battles order: 1st Bey vs 1st Bey, 2nd Bey vs 2nd Bey, 3rd Bey vs 3rd Bey.
      // After the 3rd Bey, both bladers may change the order of their deck in secret.
      const matchIndex = roundIndex % 3;
      if (matchIndex === 0 && roundIndex > 0) {
        // Re-order decks in secret (we simulate this by shuffling or keeping the best order)
        if (randomFactor) {
          deckA = shuffleDeck(deckA);
          deckB = shuffleDeck(deckB);
        }
      }
      comboA = deckA[matchIndex % deckA.length];
      comboB = deckB[matchIndex % deckB.length];
    } else if (battleType === "counter") {
      // Counter Battle (WBO):
      // Before first battle, select first Bey in secret.
      // Loser declares rematch or counter:
      // - Rematch: same Beys.
      // - Counter: winner selects first, then loser selects.
      if (roundIndex === 0) {
        // First selection (secret): pick first combo or random
        activeIdxA = randomFactor ? Math.floor(Math.random() * deckA.length) : 0;
        activeIdxB = randomFactor ? Math.floor(Math.random() * deckB.length) : 0;
      } else {
        const lastResult = rounds[rounds.length - 1];
        // Loser chooses: 30% rematch, 70% counter (rematches are rarer in competitive play)
        const chooseCounter = randomFactor ? Math.random() < 0.7 : true;

        if (chooseCounter && lastResult.winner !== "Draw") {
          const winnerIsA = lastResult.winner === "A";
          if (winnerIsA) {
            // Winner A selects first
            activeIdxA = selectBestCounter(deckA, deckB[activeIdxB]);
            // Loser B counters
            activeIdxB = selectBestCounter(deckB, deckA[activeIdxA]);
          } else {
            // Winner B selects first
            activeIdxB = selectBestCounter(deckB, deckA[activeIdxA]);
            // Loser A counters
            activeIdxA = selectBestCounter(deckA, deckB[activeIdxB]);
          }
        }
      }
      comboA = deckA[activeIdxA];
      comboB = deckB[activeIdxB];
    } else {
      // 1on1 Battle: Use the single Bey for the entire match
      comboA = deckA[0];
      comboB = deckB[0];
    }

    const roundResult = simulateBattle(comboA, comboB, { randomFactor });
    rounds.push(roundResult);

    if (roundResult.winner === "A") {
      scoreA += roundResult.points;
    } else if (roundResult.winner === "B") {
      scoreB += roundResult.points;
    }

    roundIndex++;
    if (roundIndex > 50) {
      break; // Safety limit
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

/**
 * Simulates a full WBO match supporting multi-set (Best of 3) or single set formats.
 */
export function simulateWboMatch(
  combosA: BeybladeCombo[],
  combosB: BeybladeCombo[],
  battleType: BattleType = "1on1",
  format: MatchFormat = "4-point",
  randomFactor: boolean = true
): WboMatchResult {
  const targetPoints = format === "5-point" ? 5 : (format === "7-point" ? 7 : 4);
  const sets: SetResult[] = [];
  const setScores = { A: 0, B: 0 };

  if (format === "best-of-3") {
    // Best of 3 sets: win 2 sets of 4 points each to win the match
    while (setScores.A < 2 && setScores.B < 2) {
      const setRes = simulateSet(combosA, combosB, battleType, 4, randomFactor);
      sets.push(setRes);
      
      if (setRes.winner === "A") {
        setScores.A++;
      } else if (setRes.winner === "B") {
        setScores.B++;
      } else {
        // Draw in set: replay the set
      }

      if (sets.length > 10) break; // Safety limit
    }
  } else {
    // Single set format
    const setRes = simulateSet(combosA, combosB, battleType, targetPoints, randomFactor);
    sets.push(setRes);
    if (setRes.winner === "A") {
      setScores.A = 1;
    } else if (setRes.winner === "B") {
      setScores.B = 1;
    }
  }

  let winner: "A" | "B" | "Draw" = "Draw";
  if (setScores.A > setScores.B) {
    winner = "A";
  } else if (setScores.B > setScores.A) {
    winner = "B";
  }

  return {
    winner,
    setScores,
    sets,
  };
}

// Helper to shuffle a deck (Fisher-Yates)
function shuffleDeck(deck: BeybladeCombo[]): BeybladeCombo[] {
  const result = [...deck];
  for (let i = result.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [result[i], result[j]] = [result[j], result[i]];
  }
  return result;
}

// Simple heuristic to pick the best combo against an opponent combo
function selectBestCounter(myDeck: BeybladeCombo[], opponentCombo: BeybladeCombo): number {
  if (myDeck.length <= 1) return 0;
  
  const opType = opponentCombo.stats.type;
  
  // Traditional Rock-Paper-Scissors: Attack beats Stamina, Stamina beats Defense, Defense beats Attack.
  // Balance is neutral.
  let bestIndex = 0;
  let highestScore = -1;

  myDeck.forEach((combo, idx) => {
    let score = 0;
    const type = combo.stats.type;

    if (opType === "Attack") {
      if (type === "Defense") score += 3;
      if (type === "Stamina") score -= 2;
    } else if (opType === "Defense") {
      if (type === "Stamina") score += 3;
      if (type === "Attack") score -= 2;
    } else if (opType === "Stamina") {
      if (type === "Attack") score += 3;
      if (type === "Defense") score -= 2;
    }
    
    // Add minor score for stats alignment
    score += combo.stats.weight * 0.05;

    if (score > highestScore) {
      highestScore = score;
      bestIndex = idx;
    }
  });

  return bestIndex;
}
