// @aphrody-code/ui: Native Bun JSX to HTML String Renderer

export namespace Html {
  export function createElement(tag: string | Function, attrs: Record<string, any> | null, ...children: any[]): string {
    if (typeof tag === 'function') {
      return tag({ ...attrs, children });
    }

    const attributes = attrs
      ? Object.entries(attrs)
          .map(([key, value]) => {
            if (key === 'className') key = 'class';
            if (typeof value === 'boolean') {
              return value ? key : '';
            }
            if (value === null || value === undefined) return '';
            return `${key}="${String(value).replace(/"/g, '&quot;')}"`;
          })
          .filter(Boolean)
          .join(' ')
      : '';

    const attrString = attributes.length > 0 ? ` ${attributes}` : '';
    const content = flattenChildren(children).join('');

    // Self-closing tags (void elements)
    const voidElements = new Set(['area', 'base', 'br', 'col', 'embed', 'hr', 'img', 'input', 'link', 'meta', 'param', 'source', 'track', 'wbr']);
    if (voidElements.has(tag.toLowerCase())) {
      return `<${tag}${attrString}>`;
    }

    return `<${tag}${attrString}>${content}</${tag}>`;
  }

  export function Fragment({ children }: { children?: any[] }) {
    return flattenChildren(children || []).join('');
  }

  function flattenChildren(children: any[]): any[] {
    return children.reduce((acc, child) => {
      if (Array.isArray(child)) {
        acc.push(...flattenChildren(child));
      } else if (child !== null && child !== undefined && typeof child !== 'boolean') {
        acc.push(child);
      }
      return acc;
    }, []);
  }
}

// Ensure TypeScript recognizes Web Components and standard HTML tags in JSX
declare global {
  namespace JSX {
    interface IntrinsicElements {
      [elemName: string]: any;
    }
  }
}
