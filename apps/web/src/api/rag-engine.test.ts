// SPDX-License-Identifier: Apache-2.0
import { describe, expect, test, beforeAll, afterAll } from "bun:test";
import { redis } from "bun";
import {
  searchShenron,
  searchRpbey,
  getSemanticCache,
  setSemanticCache,
  getSystemInstruction,
  performServerlessScrape,
} from "./rag-engine.ts";

describe("RAG Engine Unit Tests", () => {
  test("getSystemInstruction formats system prompts with persona rules", () => {
    const context = "Some secret data about Univers 7.";
    const shenronPrompt = getSystemInstruction("shenron", context);
    expect(shenronPrompt).toContain("Shenron");
    expect(shenronPrompt).toContain(context);

    const rpbeyPrompt = getSystemInstruction("rpbey", context);
    expect(rpbeyPrompt).toContain("Ryuga");
    expect(rpbeyPrompt).toContain(context);
  });

  test("Rpbey RAG search integration with fallback", async () => {
    // Tests public endpoint search
    const results = await searchRpbey("wizard rod", 3);
    expect(Array.isArray(results)).toBe(true);
    if (results.length > 0) {
      expect(results[0].title).toBeDefined();
      expect(results[0].url).toContain("rpbey.fr");
    }
  });

  test("Shenron RAG search integration on local db", async () => {
    // Querying local wiki sqlite database
    const results = await searchShenron("goku", 2);
    expect(Array.isArray(results)).toBe(true);
    // If database exists and contains chunks, we assert properties
    if (results.length > 0) {
      expect(results[0].title).toBeDefined();
      expect(results[0].snippet).toBeDefined();
    }
  });

  test("Serverless Scraping via bxc can handle invalid domains gracefully", async () => {
    const content = await performServerlessScrape("https://invalid.domain.that.does.not.exist.xyz");
    expect(content).toContain("[Failed to scrape");
  });
});

describe("Redis Semantic Cache Integration Tests", () => {
  const testQuery = "What is the ultimate combination?";
  const testAnswer = "The ultimate combination uses the Wizard Rod 5-60DB setup.";
  const testModel = "rpbey-test";

  beforeAll(async () => {
    // Clean up test keys
    const lruKey = `dbz:scache:${testModel}:lru`;
    await redis.del(lruKey);
  });

  afterAll(async () => {
    const lruKey = `dbz:scache:${testModel}:lru`;
    await redis.del(lruKey);
  });

  test("Writes and reads back from Redis semantic cache", async () => {
    // 1. Initial lookup should be a cache miss (null)
    const miss = await getSemanticCache(testQuery, testModel);
    expect(miss).toBeNull();

    // 2. Set the cache with our query and answer
    await setSemanticCache(testQuery, testAnswer, testModel);

    // 3. Subsequent lookup should be a cache hit (similarity >= 0.90)
    const hit = await getSemanticCache(testQuery, testModel);
    expect(hit).toBe(testAnswer);
  });

  test("Allows slight variations in queries for semantic matching", async () => {
    // "What is the ultimate combination?" is cached.
    // Let's query "Tell me about the ultimate combination" which should have high similarity.
    const variationQuery = "Tell me about the ultimate combination";
    const hit = await getSemanticCache(variationQuery, testModel);
    expect(hit).toBe(testAnswer);
  });
});
