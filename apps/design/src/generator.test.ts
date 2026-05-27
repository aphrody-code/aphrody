// SPDX-License-Identifier: Apache-2.0
import {expect, test, describe} from 'bun:test';
import {DesignCompiler} from './generator.ts';

describe('DesignCompiler React code generation', () => {
  const compiler = new DesignCompiler();

  test('should compile list-detail layout with appropriate elements', () => {
    const result = compiler.compile(
      'Crée un split detail screen en mode compact aéré',
    );
    const code =
      result.files.find(f => f.path === 'src/index.tsx')?.content || '';

    expect(code).toContain('<ListDetail');
    expect(code).toContain('List View');
    expect(code).toContain('Detail Content');
    expect(code).not.toContain('<SupportingPane');
  });

  test('should compile supporting-pane layout with appropriate elements', () => {
    const result = compiler.compile('Crée un sidebar layout');
    const code =
      result.files.find(f => f.path === 'src/index.tsx')?.content || '';

    expect(code).toContain('<SupportingPane');
    expect(code).toContain('Main Application Workspace');
    expect(code).toContain('Supporting Details');
    expect(code).not.toContain('<ListDetail');
  });

  test('should compile feed layout with cards', () => {
    const result = compiler.compile('Crée un grid cards feed');
    const code =
      result.files.find(f => f.path === 'src/index.tsx')?.content || '';

    expect(code).toContain('Feed Item 1');
    expect(code).toContain('Feed Item 2');
    expect(code).toContain('Feed Item 3');
  });

  test('should inject density overrides', () => {
    const resultHigh = compiler.compile('Crée un layout dense');
    const codeHigh =
      resultHigh.files.find(f => f.path === 'src/index.tsx')?.content || '';
    expect(codeHigh).toContain("'--md-sys-density': -2");
    expect(/\bp-6\b/.test(codeHigh)).toBe(false);

    const resultLow = compiler.compile('Crée un layout aéré');
    const codeLow =
      resultLow.files.find(f => f.path === 'src/index.tsx')?.content || '';
    expect(codeLow).toContain("'--md-sys-density': 1");
    expect(/\bp-6\b/.test(codeLow)).toBe(false);
  });

  test('should inject thinking indicator', () => {
    const result = compiler.compile('Crée un layout avec gemini shimmer');
    const code =
      result.files.find(f => f.path === 'src/index.tsx')?.content || '';

    expect(code).toContain('ThinkingIndicator');
    expect(code).toContain('Reflecting...');
  });

  test('should inject streaming hook and component', () => {
    const result = compiler.compile('Crée un layout avec streaming caret');
    const code =
      result.files.find(f => f.path === 'src/index.tsx')?.content || '';

    expect(code).toContain('StreamingText');
    expect(code).toContain('useStreamingText');
    expect(code).toContain('Simulate stream for StreamingText');
  });

  test('should inject Wiz action registry and router', () => {
    const result = compiler.compile('Crée un layout avec wiz actions');
    const code =
      result.files.find(f => f.path === 'src/index.tsx')?.content || '';

    expect(code).toContain('wizActionRegistry');
    expect(code).toContain('Wiz Action Registry dynamically mapped');
    expect(code).toContain('Wiz delegated view-spec');
  });

  test('should export W3C tokens.json configuration alongside theme.css', () => {
    const result = compiler.compile('Crée un layout standard');
    const tokensFile = result.files.find(f => f.path === 'src/tokens.json');
    expect(tokensFile).toBeDefined();

    const tokens = JSON.parse(tokensFile!.content);
    expect(tokens.color).toBeDefined();
    expect(tokens.color.primary).toBeDefined();
    expect(tokens.color.primary['40']).toBeDefined();
    expect(tokens.color.primary['40'].$type).toBe('color');
    expect(tokens.color.primary['40'].$value).toMatch(/^#[0-9a-fA-F]{6}$/);
  });

  test('should load custom design system overrides and fall back to its accent color', () => {
    // Custom design system "material" has --accent: #1a73e8; in packages/open-design/design-systems/material/tokens.css
    const result = compiler.compile('Crée un layout', undefined, 'material');
    const themeFile = result.files.find(f => f.path === 'src/theme.css');
    expect(themeFile).toBeDefined();
    expect(themeFile!.content).toContain(
      '/* --- Design System Override: material --- */',
    );
    expect(themeFile!.content).toContain('--accent: #1a73e8;');

    // Seed color should fall back to the design system's accent color (#1a73e8)
    expect(result.brief.seedColor).toBe('#1a73e8');
  });
});
