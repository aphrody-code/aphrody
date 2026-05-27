// SPDX-License-Identifier: Apache-2.0
//! Semantic parser for extracting DesignBrief from raw user input prompts.

import type {DesignBrief, OutputKind} from './types.ts';

/**
 * Parses a raw natural language prompt and extracts structured design directives.
 * Bypasses the turn-1 blocking forms by automating parameter extraction.
 */
export function parseBrief(
  rawBrief: string,
  optionalSeedColor?: string,
): DesignBrief {
  const lowercase = rawBrief.toLowerCase();

  // 1. Detect output kind
  let outputKind: OutputKind = 'prototype';
  if (
    lowercase.includes('deck') ||
    lowercase.includes('slide') ||
    lowercase.includes('ppt') ||
    lowercase.includes('présentation')
  ) {
    outputKind = 'deck';
  } else if (
    lowercase.includes('dashboard') ||
    lowercase.includes('console') ||
    lowercase.includes('admin') ||
    lowercase.includes('tableau de bord')
  ) {
    outputKind = 'dashboard';
  } else if (
    lowercase.includes('mobile') ||
    lowercase.includes('phone') ||
    lowercase.includes('app') ||
    lowercase.includes('screen') ||
    lowercase.includes('écran')
  ) {
    outputKind = 'mobile';
  } else if (
    lowercase.includes('editorial') ||
    lowercase.includes('magazine') ||
    lowercase.includes('article') ||
    lowercase.includes('poster') ||
    lowercase.includes('affiche')
  ) {
    outputKind = 'editorial';
  } else if (
    lowercase.includes('prototype') ||
    lowercase.includes('landing') ||
    lowercase.includes('saas') ||
    lowercase.includes('vitrine')
  ) {
    outputKind = 'prototype';
  }

  // 2. Detect target platforms
  const platforms: string[] = [];
  if (
    lowercase.includes('mobile') ||
    lowercase.includes('phone') ||
    lowercase.includes('ios') ||
    lowercase.includes('android')
  ) {
    platforms.push('ios');
  }
  if (
    lowercase.includes('desktop') ||
    lowercase.includes('web') ||
    lowercase.includes('dashboard') ||
    lowercase.includes('landing') ||
    lowercase.includes('ordinateur')
  ) {
    platforms.push('desktop-web');
  }
  if (outputKind === 'deck' && !platforms.includes('desktop-web')) {
    platforms.push('desktop-web');
  }
  if (platforms.length === 0) {
    platforms.push('responsive-web');
  }

  // 3. Extract audience
  let audience = 'general users';
  if (
    lowercase.includes('investor') ||
    lowercase.includes('investisseur') ||
    lowercase.includes('vc') ||
    lowercase.includes('seed')
  ) {
    audience = 'early-stage investors / venture capitalists';
  } else if (
    lowercase.includes('developer') ||
    lowercase.includes('dev') ||
    lowercase.includes('engineer') ||
    lowercase.includes('développeur') ||
    lowercase.includes('ingénieur')
  ) {
    audience = 'software engineers / developer tools buyers';
  } else if (
    lowercase.includes('enterprise') ||
    lowercase.includes('b2b') ||
    lowercase.includes('exec') ||
    lowercase.includes('entreprise') ||
    lowercase.includes('cadre')
  ) {
    audience = 'enterprise executives and decision makers';
  } else if (
    lowercase.includes('consumer') ||
    lowercase.includes('b2c') ||
    lowercase.includes('grand public')
  ) {
    audience = 'general consumers';
  }

  // 4. Extract visual tones
  const tones: string[] = [];
  if (
    lowercase.includes('editorial') ||
    lowercase.includes('magazine') ||
    lowercase.includes('serif')
  ) {
    tones.push('editorial');
  }
  if (
    lowercase.includes('minimal') ||
    lowercase.includes('clean') ||
    lowercase.includes('quiet') ||
    lowercase.includes('épuré')
  ) {
    tones.push('minimal');
  }
  if (
    lowercase.includes('utility') ||
    lowercase.includes('tech') ||
    lowercase.includes('dense')
  ) {
    tones.push('tech-utility');
  }
  if (
    lowercase.includes('playful') ||
    lowercase.includes('gamified') ||
    lowercase.includes('ludique')
  ) {
    tones.push('playful');
  }
  if (tones.length === 0) {
    tones.push('minimal'); // fallback default
  }

  // 5. Detect seed color (fallback to default Brand Rust #CE422B)
  let seedColor = optionalSeedColor || '#CE422B';
  const hexMatch = rawBrief.match(/#([0-9a-fA-F]{6}|[0-9a-fA-F]{3})/);
  if (hexMatch) {
    seedColor = hexMatch[0];
  } else if (!optionalSeedColor) {
    if (lowercase.includes('rouge') || lowercase.includes('red')) {
      seedColor = '#D32F2F';
    } else if (
      lowercase.includes('bleu') ||
      lowercase.includes('blue') ||
      lowercase.includes('cobalt')
    ) {
      seedColor = '#0F52BA';
    } else if (
      lowercase.includes('vert') ||
      lowercase.includes('green') ||
      lowercase.includes('emerald')
    ) {
      seedColor = '#10B981';
    } else if (lowercase.includes('violet') || lowercase.includes('purple')) {
      seedColor = '#6750A4'; // default M3 seed
    }
  }

  // 6. Extract scale
  let scale = '1 screen';
  const slideMatch = rawBrief.match(
    /(\d+)\s*(slide|page|screen|deck|transparence|écran)/,
  );
  if (slideMatch) {
    scale = `${slideMatch[1]} ${slideMatch[2]}s`;
  } else if (outputKind === 'deck') {
    scale = '8 slides';
  } else if (outputKind === 'mobile') {
    scale = '3 screens';
  } else if (outputKind === 'dashboard') {
    scale = '1 dense tab';
  }

  // 7. Extract constraints
  const constraintList: string[] = [];
  if (lowercase.includes('dense') || lowercase.includes('compact')) {
    constraintList.push('High density layout required');
  }
  if (lowercase.includes('dark') || lowercase.includes('sombre')) {
    constraintList.push('Dark mode first');
  }
  if (lowercase.includes('accessible') || lowercase.includes('wcag')) {
    constraintList.push('Strict WCAG AA contrast (HCT delta >= 50)');
  }
  if (
    lowercase.includes('transparent') ||
    lowercase.includes('ar') ||
    lowercase.includes('vr') ||
    lowercase.includes('xr') ||
    lowercase.includes('glimmer')
  ) {
    constraintList.push(
      'Transparent screens additive contrast (desaturated colors, no absolute black)',
    );
  }
  const constraints =
    constraintList.length > 0
      ? constraintList.join(', ')
      : 'Standard M3 styling rules';

  // 8. Extract layoutType
  let layoutType: 'feed' | 'list-detail' | 'supporting-pane' | 'scaffold' =
    'scaffold';
  if (
    lowercase.includes('list-detail') ||
    lowercase.includes('split') ||
    lowercase.includes('detail')
  ) {
    layoutType = 'list-detail';
  } else if (
    lowercase.includes('supporting') ||
    lowercase.includes('auxiliary') ||
    lowercase.includes('sidebar')
  ) {
    layoutType = 'supporting-pane';
  } else if (
    lowercase.includes('feed') ||
    lowercase.includes('grid') ||
    lowercase.includes('cards')
  ) {
    layoutType = 'feed';
  } else if (lowercase.includes('scaffold') || lowercase.includes('standard')) {
    layoutType = 'scaffold';
  }

  // 9. Extract density
  let density: 'default' | 'high' | 'low' = 'default';
  if (
    lowercase.includes('dense') ||
    lowercase.includes('compact') ||
    lowercase.includes('serré')
  ) {
    density = 'high';
  } else if (
    lowercase.includes('low') ||
    lowercase.includes('spaced') ||
    lowercase.includes('loose') ||
    lowercase.includes('aéré')
  ) {
    density = 'low';
  }

  // 10. Extract feature toggles
  const hasThinkingIndicator =
    lowercase.includes('thinking') ||
    lowercase.includes('indicator') ||
    lowercase.includes('gemini') ||
    lowercase.includes('shimmer');

  const hasStreaming =
    lowercase.includes('stream') ||
    lowercase.includes('streaming') ||
    lowercase.includes('stream') ||
    lowercase.includes('caret') ||
    lowercase.includes('real-time') ||
    lowercase.includes('temps réel');

  // 11. Extract Wiz event delegation
  const hasWizDelegation =
    lowercase.includes('wiz') ||
    lowercase.includes('delegation') ||
    lowercase.includes('router') ||
    lowercase.includes('event delegation') ||
    lowercase.includes('action delegation');

  return {
    outputKind,
    platforms,
    audience,
    tones,
    seedColor,
    scale,
    constraints,
    rawBrief,
    layoutType,
    density,
    hasThinkingIndicator,
    hasStreaming,
    hasWizDelegation,
  };
}
