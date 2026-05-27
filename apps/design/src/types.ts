// SPDX-License-Identifier: Apache-2.0
//! Core type definitions for design compiler engine.

export type OutputKind =
  | 'deck'
  | 'prototype'
  | 'dashboard'
  | 'mobile'
  | 'editorial'
  | 'other';

export interface DesignBrief {
  /** The type of artifact we are generating */
  outputKind: OutputKind;
  /** Target device classes, e.g. ["responsive-web", "ios", "desktop"] */
  platforms: string[];
  /** Target demographic or user base */
  audience: string;
  /** Design tone keys, e.g. ["editorial", "minimal", "utility"] */
  tones: string[];
  /** Numeric color seed (HEX string, e.g. "#CE422B") */
  seedColor: string;
  /** Approximate scale of output, e.g. "5 slides" or "3 screens" */
  scale: string;
  /** Hard structural constraints */
  constraints: string;
  /** Raw natural language user brief */
  rawBrief: string;
  /** Layout organization style */
  layoutType: 'feed' | 'list-detail' | 'supporting-pane' | 'scaffold';
  /** Visual spacing and density levels */
  density: 'default' | 'high' | 'low';
  /** Presence of interactive thinking animation indicator */
  hasThinkingIndicator: boolean;
  /** Presence of real-time careted streaming responses */
  hasStreaming: boolean;
  /** Presence of Wizard dynamic action delegation and dynamic handler maps */
  hasWizDelegation: boolean;
  /** Optional custom design system ID override */
  designSystemId?: string;
}

export interface TonalPalette {
  /** Palette name: primary, secondary, tertiary, error, neutral, neutral-variant */
  name: string;
  /** 13 tone steps (0, 6, 10, 12, 20, 22, 30, 40, 50, 60, 70, 80, 90, 94, 95, 98, 99, 100) */
  tones: {[tone: number]: string};
}

export interface M3Theme {
  seed: string;
  palettes: {
    primary: TonalPalette;
    secondary: TonalPalette;
    tertiary: TonalPalette;
    neutral: TonalPalette;
    neutralVariant: TonalPalette;
    error: TonalPalette;
  };
  cssCustomProperties: string;
}

export interface GeneratedFile {
  path: string;
  content: string;
}

export interface GenerationResult {
  brief: DesignBrief;
  theme: M3Theme;
  files: GeneratedFile[];
  critique: {
    scores: {
      philosophy: number;
      hierarchy: number;
      detail: number;
      functionalParity: number;
      innovation: number;
    };
    rationale: string;
  };
}
