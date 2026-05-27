// SPDX-License-Identifier: Apache-2.0
import {expect, test, describe} from 'bun:test';
import {generateTheme} from './hct.ts';

describe('HCT palette and CSS variables compiler', () => {
  test('should generate correct tone steps for seed color', () => {
    const theme = generateTheme('#CE422B');
    expect(theme.seed).toBe('#CE422B');

    // Check that tone steps are present
    const toneSteps = [
      0, 6, 10, 12, 20, 22, 30, 40, 50, 60, 70, 80, 90, 94, 95, 98, 99, 100,
    ];
    for (const step of toneSteps) {
      expect(theme.palettes.primary.tones[step]).toBeDefined();
      expect(theme.palettes.secondary.tones[step]).toBeDefined();
      expect(theme.palettes.tertiary.tones[step]).toBeDefined();
      expect(theme.palettes.neutral.tones[step]).toBeDefined();
      expect(theme.palettes.neutralVariant.tones[step]).toBeDefined();
      expect(theme.palettes.error.tones[step]).toBeDefined();
    }

    expect(theme.palettes.primary.tones[100]).toBe('#FFFFFF');
    expect(theme.palettes.primary.tones[0]).toBe('#000000');

    // Check that CSS Custom properties are exported
    expect(theme.cssCustomProperties).toContain('--md-ref-palette-primary-40');
    expect(theme.cssCustomProperties).toContain('--md-sys-color-primary');
    expect(theme.cssCustomProperties).toContain(
      '@media (prefers-color-scheme: dark)',
    );
  });

  test('should verify light mode system tokens generation', () => {
    const theme = generateTheme('#CE422B');
    const css = theme.cssCustomProperties;

    expect(css).toContain('/* System Tokens - Light Mode */');
    expect(css).toContain(
      '--md-sys-color-primary: var(--md-ref-palette-primary-40);',
    );
    expect(css).toContain(
      '--md-sys-color-on-primary: var(--md-ref-palette-primary-100);',
    );
    expect(css).toContain(
      '--md-sys-color-primary-container: var(--md-ref-palette-primary-90);',
    );
    expect(css).toContain(
      '--md-sys-color-on-primary-container: var(--md-ref-palette-primary-10);',
    );
    expect(css).toContain(
      '--md-sys-color-surface: var(--md-ref-palette-neutral-98);',
    );
    expect(css).toContain(
      '--md-sys-color-on-surface: var(--md-ref-palette-neutral-10);',
    );
    expect(css).toContain(
      '--md-sys-color-outline: var(--md-ref-palette-neutral-variant-50);',
    );
  });

  test('should verify dark mode system tokens overrides', () => {
    const theme = generateTheme('#CE422B');
    const css = theme.cssCustomProperties;

    // Dark mode block check
    expect(css).toContain('@media (prefers-color-scheme: dark)');
    // Split the CSS to check declarations within the dark media query block
    const parts = css.split('@media (prefers-color-scheme: dark) {');
    expect(parts.length).toBe(2);
    const darkBlock = parts[1];

    expect(darkBlock).toContain(
      '--md-sys-color-primary: var(--md-ref-palette-primary-80);',
    );
    expect(darkBlock).toContain(
      '--md-sys-color-on-primary: var(--md-ref-palette-primary-20);',
    );
    expect(darkBlock).toContain(
      '--md-sys-color-primary-container: var(--md-ref-palette-primary-30);',
    );
    expect(darkBlock).toContain(
      '--md-sys-color-on-primary-container: var(--md-ref-palette-primary-90);',
    );
    expect(darkBlock).toContain(
      '--md-sys-color-surface: var(--md-ref-palette-neutral-6);',
    );
    expect(darkBlock).toContain(
      '--md-sys-color-on-surface: var(--md-ref-palette-neutral-90);',
    );
    expect(darkBlock).toContain(
      '--md-sys-color-outline: var(--md-ref-palette-neutral-variant-60);',
    );
  });

  test('should generate distinct themes for different seeds', () => {
    const redTheme = generateTheme('#CE422B');
    const blueTheme = generateTheme('#0F52BA');

    expect(redTheme.palettes.primary.tones[40]).not.toBe(
      blueTheme.palettes.primary.tones[40],
    );
    expect(redTheme.palettes.secondary.tones[40]).not.toBe(
      blueTheme.palettes.secondary.tones[40],
    );
  });
});
