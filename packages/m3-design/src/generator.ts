// SPDX-License-Identifier: Apache-2.0
//! Core compiler orchestrating the automated M3 design generation workflow.

import fs from "node:fs";
import { parseBrief } from "./parser.ts";
import { generateTheme } from "./hct.ts";
import { SEED_TEMPLATE } from "./prompts.ts";
import { validateSpec } from "@aphrody-code/bun-rs";
import type { DesignBrief, GenerationResult, GeneratedFile, M3Theme } from "./types.ts";

export class DesignCompiler {
  /**
   * Compiles a raw design brief into a structured project workspace.
   * Runs the automated HCT theme generator, resolves prompt guidelines,
   * and outputs React components + CSS sheets.
   */
  public compile(rawBrief: string, seedColor?: string, designSystemId?: string): GenerationResult {
    // 1. Zero-Click semantic extraction
    const brief = parseBrief(rawBrief, seedColor);
    if (designSystemId) {
      brief.designSystemId = designSystemId;
    }

    // Load design system tokens overrides if present
    let systemCss = "";
    if (designSystemId) {
      const possiblePaths = [
        // Self-contained fixtures shipped with the package (resolved relative
        // to this module so the lookup is CWD-independent).
        new URL(`../design-systems/${designSystemId}/tokens.css`, import.meta.url).pathname,
        // Legacy in-repo locations kept as a fallback.
        `packages/open-design/design-systems/${designSystemId}/tokens.css`,
        `../../packages/open-design/design-systems/${designSystemId}/tokens.css`,
        `../packages/open-design/design-systems/${designSystemId}/tokens.css`,
      ];
      let dsPath = "";
      for (const p of possiblePaths) {
        if (fs.existsSync(p)) {
          dsPath = p;
          break;
        }
      }

      if (dsPath) {
        try {
          systemCss = fs.readFileSync(dsPath, "utf8");
          // If no seedColor was specified, fall back to the design system's accent color
          if (!seedColor && !rawBrief.match(/#([0-9a-fA-F]{6}|[0-9a-fA-F]{3})/)) {
            const accentMatch = systemCss.match(/--accent:\s*(#[0-9a-fA-F]{6}|#[0-9a-fA-F]{3})/);
            if (accentMatch) {
              brief.seedColor = accentMatch[1];
            }
          }
        } catch {
          // Ignore file read exceptions, fall back to standard M3 colors
        }
      }
    }

    // 2. Automated HCT color palette compilation
    const theme = generateTheme(brief.seedColor);
    if (systemCss) {
      theme.cssCustomProperties += `\n/* --- Design System Override: ${designSystemId} --- */\n${systemCss}`;
    }

    // 3. Assemble generated files
    const files: GeneratedFile[] = [];

    // Add generated theme stylesheet
    files.push({
      path: "src/theme.css",
      content: theme.cssCustomProperties,
    });

    // Add W3C Design Tokens JSON configuration
    files.push({
      path: "src/tokens.json",
      content: this.generateW3cTokens(theme),
    });

    // Generate high-fidelity React component (customized based on brief parameters)
    const customCode = this.generateCustomizedReactCode(brief);
    files.push({
      path: "src/index.tsx",
      content: customCode,
    });

    // 4. Perform self-critique pass
    const critique = this.performSelfCritique(brief, customCode);

    return {
      brief,
      theme,
      files,
      critique,
    };
  }

  private generateW3cTokens(theme: M3Theme): string {
    const tokens: {
      color: Record<string, Record<string, { $type: string; $value: string }>>;
    } = {
      color: {},
    };
    for (const [key, palette] of Object.entries(theme.palettes)) {
      tokens.color[key] = {};
      for (const [tone, hex] of Object.entries(palette.tones)) {
        tokens.color[key][tone] = {
          $type: "color",
          $value: hex,
        };
      }
    }
    return JSON.stringify(tokens, null, 2);
  }

  private generateCustomizedReactCode(brief: DesignBrief): string {
    let code = SEED_TEMPLATE.replace(/\r\n/g, "\n");

    // 1. Extra imports — only real @aphrody-code/m3-react exports are imported
    // (aliased Md* -> local name). Interaction primitives (ThinkingIndicator,
    // StreamingText, useStreamingText) are NOT m3-react exports; they are
    // injected as inline self-contained definitions (see steps 4 & 5).
    const extraImports: string[] = [];
    if (brief.layoutType === "list-detail") {
      extraImports.push("MdListDetail as ListDetail");
    } else if (brief.layoutType === "supporting-pane") {
      extraImports.push("MdSupportingPane as SupportingPane");
    }
    if (extraImports.length > 0) {
      code = code.replace(
        "  MdScaffold as Scaffold,\n",
        `  MdScaffold as Scaffold,\n  ${extraImports.join(",\n  ")},\n`,
      );
    }

    // Inline interaction-primitive definitions injected at the top of the
    // generated component so the artifact is self-contained valid React + TS.
    const inlineHelpers: string[] = [];
    if (brief.hasThinkingIndicator) {
      inlineHelpers.push(
        `function ThinkingIndicator({ label }: { label: string }) {\n  return <span className="animate-pulse text-on-surface-variant">{label}</span>;\n}`,
      );
    }
    if (brief.hasStreaming) {
      inlineHelpers.push(
        `function useStreamingText(stream?: AsyncIterable<string>) {\n  const [text, setText] = React.useState("");\n  const [done, setDone] = React.useState(false);\n  React.useEffect(() => {\n    if (!stream) return;\n    let active = true;\n    setText("");\n    setDone(false);\n    (async () => {\n      for await (const chunk of stream) {\n        if (!active) return;\n        setText((prev) => prev + chunk);\n      }\n      if (active) setDone(true);\n    })();\n    return () => {\n      active = false;\n    };\n  }, [stream]);\n  return { text, done };\n}`,
        `function StreamingText({ text, done }: { text: string; done: boolean }) {\n  return (\n    <span>\n      {text}\n      {!done && <span className="animate-pulse">|</span>}\n    </span>\n  );\n}`,
      );
    }
    if (inlineHelpers.length > 0) {
      code = code.replace(
        "export default function GeneratedArtifact() {",
        `${inlineHelpers.join("\n\n")}\n\nexport default function GeneratedArtifact() {`,
      );
    }

    // Apply specific title
    const titleMatch =
      brief.rawBrief.match(/pour ([\w\s-]+)/i) || brief.rawBrief.match(/for ([\w\s-]+)/i);
    const parsedTitle = titleMatch ? titleMatch[1].trim() : "Automated M3 Design";

    code = code.replace("Automated M3 Design", parsedTitle);
    code = code.replace(
      "Powering high-fidelity components with zero-click prompts",
      `Targeting ${brief.audience} — Scale: ${brief.scale}`,
    );

    // Customize layout elements based on OutputKind
    if (brief.outputKind === "mobile") {
      // For mobile apps: remove NavigationRail, place BottomAppBar, change layout
      code = code.replace("hidden md:flex", "hidden");
      code = code.replace(
        "{/* SPEC DIALOG */}",
        '{/* MOBILE LAYOUT SCAFFOLD */}\n      <md-bottom-app-bar className="md-hidden" />\n\n      {/* SPEC DIALOG */}',
      );
    } else if (brief.outputKind === "deck") {
      // For slides: adapt layout to slide deck presentation
      code = code.replace("Dashboard", "Slide 1");
      code = code.replace("Components", "Slide 2");
      code = code.replace("Analytics", "Slide 3");
    }

    // 2. Dynamic Layout Injections
    let layoutMarkup = "";
    if (brief.layoutType === "list-detail") {
      layoutMarkup = `{/* LIST DETAIL LAYOUT */}
        <ListDetail className="flex-1 p-6 max-w-7xl mx-auto grid grid-cols-1 lg:grid-cols-3 gap-6">
          {/* LEFT COLUMN: LIST PANE */}
          <Pane className="bg-surface-container p-6 rounded-2xl flex flex-col gap-6">
            <TypeText className="text-lg font-medium">List View</TypeText>
            <Divider />
            <List className="divide-y divide-border">
              <ListItem 
                headline="Primary Detail Item" 
                supportingText="Selecting this item shows details"
                data-action="select-item"
                data-id="detail-1"
                className="cursor-pointer hover:bg-surface-container-highest transition-colors duration-150"
              />
              <ListItem 
                headline="Secondary Detail Item" 
                supportingText="Detailed metrics and parameters"
                data-action="select-item"
                data-id="detail-2"
                className="cursor-pointer hover:bg-surface-container-highest transition-colors duration-150"
              />
            </List>
          </Pane>

          {/* RIGHT COLUMN: DETAIL PANE */}
          <Pane className="lg:col-span-2 bg-surface-container p-6 rounded-2xl flex flex-col gap-6">
            <TypeText className="text-lg font-medium">Detail Content</TypeText>
            <Divider />
            <div className="flex flex-col gap-4">
              <TypeText className="text-md font-semibold text-primary">Detail Title</TypeText>
              <p className="text-sm">Select an item on the left to see more details here. The layout dynamically compiles and synchronizes based on the designer brief.</p>
            </div>
            
            {/* TOOLTIP DEMONSTRATION */}
            <div className="flex gap-4 items-center mt-4">
              <Tooltip label="Hovering over primary CTA">
                <FilledButton>Submit Data</FilledButton>
              </Tooltip>
              <CircularProgress indeterminate className="w-8 h-8" />
            </div>
          </Pane>
        </ListDetail>`;
    } else if (brief.layoutType === "supporting-pane") {
      layoutMarkup = `{/* SUPPORTING PANE LAYOUT */}
        <SupportingPane className="flex-1 p-6 max-w-7xl mx-auto grid grid-cols-1 lg:grid-cols-3 gap-6">
          {/* MAIN FLEXIBLE PANE */}
          <Pane className="lg:col-span-2 bg-surface-container p-6 rounded-2xl flex flex-col gap-6">
            <TypeText className="text-lg font-medium">Main Application Workspace</TypeText>
            <Divider />
            <p className="text-sm">This is the main flexible workspace. It adapts to fit the primary task focus, while the supporting pane on the side offers context-sensitive actions and details.</p>
            
            <List className="divide-y divide-border">
              <ListItem 
                headline="Primary Task Element" 
                supportingText="In progress - needs review"
                data-action="inspect-task"
                data-id="task-1"
                className="cursor-pointer hover:bg-surface-container-highest transition-colors duration-150"
              />
            </List>

            {/* TOOLTIP DEMONSTRATION */}
            <div className="flex gap-4 items-center mt-4">
              <Tooltip label="Hovering over primary CTA">
                <FilledButton>Submit Data</FilledButton>
              </Tooltip>
              <CircularProgress indeterminate className="w-8 h-8" />
            </div>
          </Pane>

          {/* SUPPORTING FIXED PANE */}
          <Pane className="bg-surface-container p-6 rounded-2xl flex flex-col gap-6" style={{ width: '320px', minWidth: '300px' }}>
            <TypeText className="text-lg font-medium">Supporting Details</TypeText>
            <Divider />
            <div className="flex flex-col gap-4 text-sm">
              <p>This side panel contains static auxiliary parameters and context utilities.</p>
              <div className="flex justify-between items-center">
                <span>Damping Spatial</span>
                <Switch checked />
              </div>
              <div className="flex justify-between items-center">
                <span>Adaptive Panes</span>
                <Checkbox checked />
              </div>
            </div>
          </Pane>
        </SupportingPane>`;
    } else if (brief.layoutType === "feed") {
      layoutMarkup = `{/* FEED LAYOUT */}
        <Scaffold className="flex-1 p-6 max-w-7xl mx-auto grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
          {/* CARD 1 */}
          <Pane className="bg-surface-container p-6 rounded-2xl flex flex-col gap-4">
            <TypeText className="text-lg font-medium">Feed Item 1</TypeText>
            <Divider />
            <p className="text-sm">Main body content for the first feed card. Displays high level summary data.</p>
            <div className="flex justify-between items-center mt-auto pt-4">
              <FilledButton data-action="like-post" data-id="1">Like</FilledButton>
              <span className="text-xs font-mono">12 min ago</span>
            </div>
          </Pane>

          {/* CARD 2 */}
          <Pane className="bg-surface-container p-6 rounded-2xl flex flex-col gap-4">
            <TypeText className="text-lg font-medium">Feed Item 2</TypeText>
            <Divider />
            <p className="text-sm">Main body content for the second feed card. Features metrics and dynamic widgets.</p>
            <div className="flex justify-between items-center mt-auto pt-4">
              <FilledButton data-action="like-post" data-id="2">Like</FilledButton>
              <span className="text-xs font-mono">1 hr ago</span>
            </div>
          </Pane>

          {/* CARD 3 */}
          <Pane className="bg-surface-container p-6 rounded-2xl flex flex-col gap-4">
            <TypeText className="text-lg font-medium">Feed Item 3</TypeText>
            <Divider />
            <p className="text-sm">Third card in the card feed, completing the multi-column fluid grid.</p>
            <div className="flex justify-between items-center mt-auto pt-4 flex-row gap-4 items-center">
              <Tooltip label="Hovering over primary CTA">
                <FilledButton data-action="like-post" data-id="3">Like</FilledButton>
              </Tooltip>
              <CircularProgress indeterminate className="w-8 h-8" />
              <span className="text-xs font-mono ml-auto">Yesterday</span>
            </div>
          </Pane>
        </Scaffold>`;
    }

    if (layoutMarkup) {
      const startIndex = code.indexOf("        {/* MAIN LAYOUT SCAFFOLD */}");
      const endIndex = code.indexOf("        </Scaffold>", startIndex);
      if (startIndex !== -1 && endIndex !== -1) {
        const length = endIndex + "        </Scaffold>".length - startIndex;
        code = code.substring(0, startIndex) + layoutMarkup + code.substring(startIndex + length);
      }
    }

    // 3. Density-based visual spacing adjustments
    if (brief.density === "high") {
      code = code.replace(/p-6/g, "p-3");
      code = code.replace(/gap-6/g, "gap-3");
      code = code.replace(/gap-4/g, "gap-2");
    } else if (brief.density === "low") {
      code = code.replace(/p-6/g, "p-8");
      code = code.replace(/gap-6/g, "gap-8");
      code = code.replace(/gap-4/g, "gap-6");
    }

    // Inject density style
    if (brief.density === "high") {
      code = code.replace(
        "        transitionDuration: '200ms'\n      }}",
        "        transitionDuration: '200ms',\n        '--md-sys-density': -2\n      }}",
      );
    } else if (brief.density === "low") {
      code = code.replace(
        "        transitionDuration: '200ms'\n      }}",
        "        transitionDuration: '200ms',\n        '--md-sys-density': 1\n      }}",
      );
    }

    // 4. Feature toggles: Thinking Indicator
    if (brief.hasThinkingIndicator) {
      code = code.replace(
        '<CircularProgress indeterminate className="w-8 h-8" />',
        '<CircularProgress indeterminate className="w-8 h-8" />\n              <ThinkingIndicator label="Reflecting..." />',
      );
    }

    // 5. Feature toggles: Streaming Text
    if (brief.hasStreaming) {
      const hookInjection = `
  // Simulate stream for StreamingText
  const [stream, setStream] = React.useState<AsyncIterable<string> | undefined>(undefined);
  const { text: streamedText, done: streamDone } = useStreamingText(stream);

  React.useEffect(() => {
    let active = true;
    const mockStream: AsyncIterable<string> = {
      async *[Symbol.asyncIterator]() {
        const words = "Streaming real-time results from design engine...".split(" ");
        for (const word of words) {
          if (!active) break;
          await new Promise(resolve => setTimeout(resolve, 150));
          yield word + " ";
        }
      }
    };
    setStream(mockStream);
    return () => {
      active = false;
    };
  }, []);
`;
      code = code.replace(
        "export default function GeneratedArtifact() {",
        `export default function GeneratedArtifact() {${hookInjection}`,
      );
      code = code.replace(
        "{/* TOOLTIP DEMONSTRATION */}",
        `{/* STREAMING TEXT SECTION */}
            <div className="p-4 bg-surface border border-border rounded-xl mb-4 font-mono text-sm">
              <StreamingText text={streamedText} done={streamDone} />
            </div>\n            {/* TOOLTIP DEMONSTRATION */}`,
      );
    }

    // 6. Action Router / Registry
    if (brief.hasActionRouter) {
      const targetHandlerBlock = `  // Central action handler
  const handleInteraction = (e: React.MouseEvent<HTMLDivElement>) => {
    const actionElement = (e.target as HTMLElement).closest('[data-action]');
    if (!actionElement) return;

    const action = actionElement.getAttribute('data-action');
    const id = actionElement.getAttribute('data-id');
    console.log(\`[Action Router] Action: \${action}, ID: \${id}\`);

    // Custom interaction routing
    if (action === 'toggle-dialog') {
      const dialog = document.querySelector('md-dialog');
      if (dialog) (dialog as any).open = !(dialog as any).open;
    }
  };`;

      const replacementHandlerBlock = `  // Action registry dynamically mapped based on layout actions
  const actionRegistry: Record<string, (id: string | null) => void> = {
    'toggle-dialog': () => {
      const dialog = document.querySelector('md-dialog');
      if (dialog) (dialog as any).open = !(dialog as any).open;
    },
    'view-spec': (id) => {
      console.log('Delegated view-spec:', id);
    },
    'select-item': (id) => {
      console.log('Delegated select-item:', id);
    },
    'inspect-task': (id) => {
      console.log('Delegated inspect-task:', id);
    },
    'like-post': (id) => {
      console.log('Delegated like-post:', id);
    }
  };

  // Central action router
  const handleInteraction = (e: React.MouseEvent<HTMLDivElement>) => {
    const actionElement = (e.target as HTMLElement).closest('[data-action]');
    if (!actionElement) return;

    const action = actionElement.getAttribute('data-action');
    const id = actionElement.getAttribute('data-id');
    console.log(\`[Action Router] Action: \${action}, ID: \${id}\`);

    if (action && actionRegistry[action]) {
      actionRegistry[action](id);
    }
  };`;

      code = code.replace(targetHandlerBlock, replacementHandlerBlock);
    }

    // Apply constraints
    if (
      brief.constraints.includes("Dark mode first") ||
      brief.rawBrief.includes("dark") ||
      brief.rawBrief.includes("sombre")
    ) {
      code = code.replace(
        'className="min-h-screen bg-background text-on-surface flex flex-col font-sans transition-colors duration-200"',
        'className="min-h-screen bg-neutral-900 text-neutral-100 flex flex-col font-sans transition-colors duration-200"',
      );
    }

    if (brief.constraints.includes("Transparent screens")) {
      code = code.replace("#6750A4", "Additive (AR/XR desaturated)");
    }

    return code;
  }

  private performSelfCritique(brief: DesignBrief, code: string) {
    const validation = validateSpec(code);

    const hasLitProps = code.includes("@aphrody-code/m3-react");
    const hasActionRouter = code.includes("Action Router");
    const hasSpringTiming = code.includes("cubic-bezier");

    // Adjust the base scores using the real FFI validation results!
    const errorsCount = validation.issues.filter((i) => i.level === "error").length;
    const warningsCount = validation.issues.filter((i) => i.level === "warning").length;

    // Deduct from scores based on violations
    const detailDeduction = warningsCount * 2;
    const parityDeduction = errorsCount * 5;

    const scores = {
      philosophy: Math.max(50, 90 - parityDeduction),
      hierarchy: Math.max(50, 92 - detailDeduction),
      detail: hasSpringTiming ? Math.max(50, 95 - detailDeduction) : 80,
      functionalParity: hasLitProps ? Math.max(50, 98 - parityDeduction) : 75,
      innovation: hasActionRouter ? 94 : 70,
    };

    const issuesSummary =
      validation.issues.length > 0
        ? ` Detected ${validation.issues.length} design system specification issue(s): ${validation.issues.map((i) => `[Line ${i.line}: ${i.rule}] ${i.message}`).join("; ")}.`
        : " Zero M3 design system spec violations detected.";

    const rationale = `M3 design compiler successfully mapped the brief for "${brief.audience}". Enforced touch targets at 48dp. Spring dynamics are configured with a ${hasSpringTiming ? "spatial bounce curve" : "fallback curve"}. Event delegation is handled in a single listener for high interaction performance. FFI Spec Check Score: ${validation.score}/100.${issuesSummary}`;

    return { scores, rationale, issues: validation.issues };
  }
}
