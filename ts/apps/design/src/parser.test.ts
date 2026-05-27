// SPDX-License-Identifier: Apache-2.0
import {expect, test, describe} from 'bun:test';
import {parseBrief} from './parser.ts';

describe('Design brief semantic parser', () => {
  test('should parse deck request with audience and color', () => {
    const brief = parseBrief(
      'Crée une présentation de pitch deck pour investisseurs en bleu',
      '#1E3A8A',
    );
    expect(brief.outputKind).toBe('deck');
    expect(brief.platforms).toContain('desktop-web');
    expect(brief.audience).toContain('investors');
    expect(brief.seedColor).toBe('#1E3A8A');
    expect(brief.scale).toBe('8 slides');
  });

  test('should parse dashboard request with density and dark mode', () => {
    const brief = parseBrief(
      'Fais un console dashboard dense sombre avec un bouton rouge',
    );
    expect(brief.outputKind).toBe('dashboard');
    expect(brief.seedColor).toBe('#D32F2F');
    expect(brief.constraints).toContain('High density layout');
    expect(brief.constraints).toContain('Dark mode first');
    expect(brief.density).toBe('high');
  });

  test('should parse AR/VR/XR transparent screen request', () => {
    const brief = parseBrief(
      'Fais un app mobile pour transparent screen en réalité augmentée',
    );
    expect(brief.outputKind).toBe('mobile');
    expect(brief.platforms).toContain('ios');
    expect(brief.constraints).toContain(
      'Transparent screens additive contrast',
    );
  });

  test('should extract all layout types correctly', () => {
    expect(parseBrief('Fais un feed de news').layoutType).toBe('feed');
    expect(parseBrief('Fais une grid de cartes').layoutType).toBe('feed');
    expect(parseBrief('Split detail view screen').layoutType).toBe(
      'list-detail',
    );
    expect(parseBrief('List-detail layout').layoutType).toBe('list-detail');
    expect(parseBrief('Supporting pane with sidebar').layoutType).toBe(
      'supporting-pane',
    );
    expect(parseBrief('Auxiliary column').layoutType).toBe('supporting-pane');
    expect(parseBrief('Standard scaffold').layoutType).toBe('scaffold');
    expect(parseBrief('Random prompt').layoutType).toBe('scaffold');
  });

  test('should extract density values correctly', () => {
    expect(parseBrief('Dense layout').density).toBe('high');
    expect(parseBrief('Interface compact').density).toBe('high');
    expect(parseBrief('Low spacing').density).toBe('low');
    expect(parseBrief('Loose arrangement').density).toBe('low');
    expect(parseBrief('Normal layout').density).toBe('default');
  });

  test('should extract Gemini features correctly', () => {
    const briefWithThinking = parseBrief('Show a thinking indicator animation');
    expect(briefWithThinking.hasThinkingIndicator).toBe(true);
    expect(briefWithThinking.hasStreaming).toBe(false);

    const briefWithStreaming = parseBrief(
      'Real-time streaming text response with caret',
    );
    expect(briefWithStreaming.hasThinkingIndicator).toBe(false);
    expect(briefWithStreaming.hasStreaming).toBe(true);

    const briefWithBoth = parseBrief(
      'Gemini shimmer thinking and stream output',
    );
    expect(briefWithBoth.hasThinkingIndicator).toBe(true);
    expect(briefWithBoth.hasStreaming).toBe(true);

    const briefWithNone = parseBrief('Static simple page');
    expect(briefWithNone.hasThinkingIndicator).toBe(false);
    expect(briefWithNone.hasStreaming).toBe(false);
  });

  test('should extract Wiz event delegation flags correctly', () => {
    expect(parseBrief('Wiz routing active').hasWizDelegation).toBe(true);
    expect(parseBrief('Action delegation framework').hasWizDelegation).toBe(
      true,
    );
    expect(parseBrief('Standard event delegation').hasWizDelegation).toBe(true);
    expect(parseBrief('Simple React handlers').hasWizDelegation).toBe(false);
  });
});
