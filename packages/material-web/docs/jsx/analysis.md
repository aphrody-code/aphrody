# Deep Analysis: JSX Parsing, Transpiling, and Runtime Execution in Bun & React

This report details the architectural design, source-level implementations, and compile-to-runtime execution pipelines for JSX in two key ecosystems: **Bun** (native runtime written in Zig) and **React** (runtime elements and compiler optimizations written in TypeScript/JavaScript).

---

## 1. Bun JSX Engine Deep Dive

Bun compiles JSX files (`.jsx`, `.tsx`) down to vanilla JavaScript natively during parse-time. This translation is written entirely in Zig and is designed to feed the JavaScriptCore (JSC) virtual machine with highly-optimized JS code.

### 1.1 Native Parsing in Zig

Bun’s lexing and parsing of JSX is handled inside `src/js_parser/lexer.zig` and `src/js_parser/parse/parse_jsx.zig`. 

When the parser encounters a JSX tag start (like `<`), it dispatches `parseJSXElement` defined in [parse_jsx.zig](file:///tmp/bun/src/js_parser/parse/parse_jsx.zig#L11).

#### Tag Differentiation: Lowercase vs. Uppercase & Namespaces
Bun distinguishes between native HTML tags (represented as strings) and custom React components (represented as identifiers) at parse-time. This logic resides in the `JSXTag.parse` method inside [parser.zig](file:///tmp/bun/src/js_parser/parser.zig#L244):

```zig
// The tag is an identifier
var name = p.lexer.identifier;
var tag_range = p.lexer.range();
try p.lexer.expectInsideJSXElementWithName(.t_identifier, "JSX element name");

// Certain identifiers are strings
// <div
// <button
// <Hello-:Button
if (strings.containsComptime(name, "-:") or (p.lexer.token != .t_dot and name[0] >= 'a' and name[0] <= 'z')) {
    return JSXTag{
        .data = Data{ .tag = p.newExpr(E.String{
            .data = name,
        }, loc) },
        .range = tag_range,
        .name = name,
    };
}
```

* **Standard Tags**: If the identifier starts with a lowercase letter (e.g., `div`, `span`) or contains the namespace separator `-:`, it is treated as a string literal expression (`E.String`), resulting in output like `jsx("div", ...)` or `createElement("div", ...)`.
* **Custom Components**: If it starts with an uppercase letter (e.g., `Button`) or contains dots (e.g., `<Button.Red>`), it is parsed as an identifier (`E.Identifier`) or a member expression chain (`E.Dot`), resolving to the local JS scope.

#### XML Entity Reference Decoding
Unlike standard JS strings where backslash escapes are used, JSX follows XML rules for entities (e.g. `&nbsp;`, `&amp;`). Bun handles this at compile-time directly in the lexer. When parsing the child text of JSX elements, it resolves these entity strings and emits raw UTF-8 characters:
* `&nbsp;` is parsed and written as `\xA0` (non-breaking space).
* `&lt;` and `&gt;` become `<` and `>`.
This avoids runtime parsing overhead, baking the resolved unicode characters directly into the compiled output string.

---

### 1.2 Compile-Time Transpilation & Lowering

The transpiler lowering happens during AST visitation in `src/js_parser/visit/visit_expr.zig`. The `e_jsx_element` function ([visit_expr.zig:L192](file:///tmp/bun/src/js_parser/visit/visit_expr.zig#L192)) lowers `.e_jsx_element` AST nodes into regular JavaScript function calls depending on the configured runtime options (`classic` vs `automatic`).

```zig
pub fn e_jsx_element(p: *P, expr: Expr, _: ExprIn) Expr {
    const e_ = expr.data.e_jsx_element;
    switch (comptime jsx_transform_type) {
        .react => {
            // Lowering logic
            ...
```

#### A. The Classic JSX Runtime (`react`)
If `p.options.jsx.runtime == .classic` (equivalent to standard `React.createElement`), Bun lowers the node to a call of the factory function (defaulting to `React.createElement` or custom pragma):
1. **First Argument**: The tag name (either String literal like `"div"` or Identifier variable like `Button`).
2. **Second Argument**: The props object (or `null` if none).
3. **Subsequent Arguments**: Children elements flattened as third, fourth, etc., arguments.

```zig
const target = p.jsxStringsToMemberExpression(expr.loc, p.options.jsx.factory) catch unreachable;
return p.newExpr(E.Call{
    .target = if (runtime == .classic) target else p.jsxImport(.createElement, expr.loc),
    .args = args,
    .can_be_unwrapped_if_unused = if (!p.options.ignore_dce_annotations and !p.options.jsx.side_effects) .if_unused else .never,
    .close_paren_loc = e_.close_tag_loc,
}, expr.loc);
```

#### B. The Automatic JSX Runtime (`react-jsx` / `react-jsxdev`)
If `runtime == .automatic`, Bun removes the `key` prop from the props object, extracts it, and constructs calls to `jsx`, `jsxs`, or `jsxDEV`:
1. **Static vs. Dynamic Children**:
   Bun checks if the children should be static (i.e. children length > 1 or single child is a spread operator). If so, it compiles the child array inside props:
   ```zig
   const is_static_jsx = e_.children.len > 1 or (e_.children.len == 1 and e_.children.ptr[0].data == .e_spread);
   ```
2. **Development Runtime (`react-jsxdev`)**:
   Calls `jsxDEV(type, config, maybeKey, isStaticChildren, source, self)` ([visit_expr.zig:L327](file:///tmp/bun/src/js_parser/visit/visit_expr.zig#L327)):
   * `type`: Tag or Component.
   * `config`: Props object with `children` merged inside.
   * `maybeKey`: Extracted key value (or `undefined`).
   * `isStaticChildren`: A boolean indicating if children are static.
   * `source` / `self`: Source location mapping and execution context (`this`).
3. **Production Runtime (`react-jsx`)**:
   Calls `jsx(type, config, key)` or `jsxs(type, config, key)` depending on `is_static_jsx`. The key is passed as a separate third parameter only if present.

#### C. Automatic Import Insertion
Bun automatically manages runtime imports at the end of parsing. If JSX elements were encountered, `p.jsx_imports` records which functions (`jsx`, `jsxs`, `jsxDEV`, `Fragment`, `createElement`) were generated. In [parse_entry.zig](file:///tmp/bun/src/js_parser/parse/parse_entry.zig#L1384), Bun generates the ESM import statements at the top of the file:

```zig
if (p.options.jsx.parse and p.options.features.auto_import_jsx and p.options.jsx.runtime == .automatic) {
    var buf = [3]string{ "", "", "" };
    const runtime_import_names = p.jsx_imports.runtimeImportNames(&buf);

    if (runtime_import_names.len > 0) {
        p.generateImportStmt(
            p.options.jsx.importSource(), // Returns "react/jsx-runtime" or "react/jsx-dev-runtime"
            runtime_import_names,
            &before,
            &p.jsx_imports,
            null,
            "",
            false,
        ) catch unreachable;
    }
}
```

---

### 1.3 Runtime & JavaScriptCore Integration

When loading JSX files, Bun does not interpret the JSX structure dynamically at runtime in C++. Instead:
1. Bun's native module loader intercepts `.jsx`/`.tsx` loads via `BunLoaderTypeJSX = 1` inside [ModuleLoader.cpp](file:///tmp/bun/src/jsc/bindings/ModuleLoader.cpp#L261).
2. The Zig transpiler lowerer converts all JSX markup to standard JS Jsit-optimized function calls (`jsx`, `jsxs`).
3. This generated JS code is fed to JavaScriptCore (JSC). JSC’s JIT compiler compilation tiers (Baseline, DFG, and FTL) optimize these calls directly. Since the functions are standard JavaScript, JSC can inline them and optimize object shape creation (e.g. allocating the `ReactElement` object structure).

---

## 2. React Runtime Element & Compiler Deep Dive

React 19 introduces a revised runtime representation of JSX elements, and couples it with the **React Compiler** (also known as React Forget), which performs extensive compile-time optimizations on JSX structures.

### 2.1 Runtime Representation: `ReactElement` and Key/Ref Extraction

At runtime, JSX translates to calls to `react/jsx-runtime` (`jsx`, `jsxs`). These functions are defined in [ReactJSXElement.js](file:///tmp/react/packages/react/src/jsx/ReactJSXElement.js).

#### transitional Element Tagging
In React 19, elements are tagged with transitional symbols. Looking at [ReactSymbols.js](file:///tmp/react/packages/shared/ReactSymbols.js#L15):

```javascript
export const REACT_LEGACY_ELEMENT_TYPE: symbol = Symbol.for('react.element');
export const REACT_ELEMENT_TYPE: symbol = Symbol.for(
  'react.transitional.element',
);
```
Transitional elements are labeled with `Symbol.for('react.transitional.element')` to denote support for newer React runtime features (like refs as props).

#### The React 19 Ref Overhaul
Historically, React extracted both `key` and `ref` out of the config object, omitting them from the child's `props`. 
In React 19, **`ref` is now a regular prop**. It is passed down directly within the `props` object.

Looking at `ReactElement` constructor in [ReactJSXElement.js:L170](file:///tmp/react/packages/react/src/jsx/ReactJSXElement.js#L170):

```javascript
function ReactElement(type, key, props, owner, debugStack, debugTask) {
  // Ignore whatever was passed as the ref argument and treat `props.ref` as
  // the source of truth. The only thing we use this for is `element.ref`,
  // which will log a deprecation warning on access. In the next release, we
  // can remove `element.ref` as well as the `ref` argument.
  const refProp = props.ref;
  const ref = refProp !== undefined ? refProp : null;
  
  // In prod:
  element = {
    $$typeof: REACT_ELEMENT_TYPE,
    type,
    key,
    ref,
    props,
  };
}
```

In development, accessing `element.ref` directly logs a deprecation warning:
```javascript
function elementRefGetterWithDeprecationWarning() {
  if (__DEV__) {
    const componentName = getComponentNameFromType(this.type);
    if (!didWarnAboutElementRef[componentName]) {
      didWarnAboutElementRef[componentName] = true;
      console.error(
        'Accessing element.ref was removed in React 19. ref is now a ' +
          'regular prop. It will be removed from the JSX Element ' +
          'type in a future release.',
      );
    }
    const refProp = this.props.ref;
    return refProp !== undefined ? refProp : null;
  }
}
```

#### Key Extraction and Warnings
Unlike `ref`, `key` is still treated as a special element field and stripped from the `props` object. If `key` is present in the configuration object, React clones the props and excludes it:

```javascript
let props;
if (!('key' in config)) {
  // If key was not spread in, reuse the original props object (no allocation!)
  props = config;
} else {
  // Fresh props object to avoid de-optimizing V8 object shapes via `delete`
  props = {};
  for (const propName in config) {
    if (propName !== 'key') {
      props[propName] = config[propName];
    }
  }
}
```
If a `key` prop is spread into JSX (`<div {...props} />` where `props` contains `key`), React logs a dev warning warning that spread keys cannot be resolved statically.

---

### 2.2 React Compiler (React Forget) JSX Optimizations

The React Compiler (`compiler/packages/babel-plugin-react-compiler`) operates on a High-level Intermediate Representation (HIR) to optimize components. It features three crucial JSX-specific optimization strategies.

#### A. Structural Memoization & Constant Folding
The React Compiler automatically memoizes components and JSX elements by grouping them into **Reactive Scopes**.
Instead of rebuilding the React Element tree on every render, the compiler uses a memoization cache array `_c(size)`. If the inputs (dependencies) to the JSX structure haven't changed, it returns the previously cached element object.

* **Constant Folding**: Static JSX trees (e.g. `<div><span>Static</span></div>`) have no reactive dependencies. The compiler caches these elements permanently on index `0` of the cache. They are never recreated.
* **Reactive Memoization**: If an element has dynamic props (e.g. `<Button label={name} />`), the compiler emits dependency checks:

```javascript
const $ = _c(2);
let t0;
if ($[0] !== name) {
  t0 = <Button label={name} />;
  $[0] = name;
  $[1] = t0;
} else {
  t0 = $[1];
}
return t0;
```
Because referential equality is preserved across renders (`oldElement === newElement`), React's fiber reconciler can bypass virtual DOM diffing for this entire subtree, skipping child renders.

#### B. JSX Outlining (`OutlineJsx.ts`)
The `outlineJSX` pass ([OutlineJsx.ts](file:///tmp/react/compiler/packages/babel-plugin-react-compiler/src/Optimization/OutlineJsx.ts)) extracts nested JSX elements from inside callback functions (such as `.map(item => <Card item={item} />)`) into separate top-level component functions.

* **The Problem**: Inline callbacks create a new closure on every render, capturing variables from the parent scope. This breaks memoization and forces re-evaluation.
* **The Solution**: The compiler identifies nested callbacks that only return JSX and outlines them:
  1. Captured variables are mapped and converted into explicit React props.
  2. The inline callback is replaced with a call to the new outlined component:
     `items.map(item => <OutlinedComponent item={item} />)`
  3. The outlined component gains its own independent memoization cache (`_c`).

This is implemented in `OutlineJsx.ts` via the `collectProps` and `emitOutlinedFn` methods:

```typescript
function emitOutlinedFn(
  env: Environment,
  jsx: Array<JsxInstruction>,
  oldProps: Array<OutlinedJsxAttribute>,
  globals: LoadGlobalMap,
): HIRFunction | null {
  const instructions: Array<Instruction> = [];
  const oldToNewProps = createOldToNewPropsMapping(env, oldProps);

  const propsObj: Place = createTemporaryPlace(env, GeneratedSource);
  
  // Destructure captured vars as props
  const destructurePropsInstr = emitDestructureProps(env, propsObj, oldToNewProps);
  instructions.push(destructurePropsInstr);
  ...
```

The resulting compiled JS splits the code into:
```javascript
// Outlined Helper Component with its own cache
function _outlined_Component$1(props) {
  const $ = _c(2);
  const item = props.item;
  let t0;
  if ($[0] !== item.id || $[1] !== item.name) {
    t0 = <Stringify key={item.id} item={item.name} />;
    $[0] = item.id;
    $[1] = item.name;
  } else {
    t0 = $[1];
  }
  return t0;
}
```
This optimization ensures list items are only re-rendered if their specific item data changes, decoupling them from parent component renders.

#### C. SSR Optimizations (`OptimizeForSSR.ts`)
During Server-Side Rendering, React renders elements to HTML strings once; hooks like `useEffect` do not run, and states are never updated. The `OptimizeForSSR` pass ([OptimizeForSSR.ts](file:///tmp/react/compiler/packages/babel-plugin-react-compiler/docs/passes/35-optimizeForSSR.md)) strips runtime overhead under SSR conditions:

1. **Inlines State Hooks**: `useState(initialValue)` is replaced with `[initialValue, () => {}]`, avoiding Hook registrations.
2. **Removes Effects**: `useEffect` and `useLayoutEffect` blocks are completely eliminated from the AST.
3. **Neutered Event Handlers**: Event handler callbacks that trigger state updates are replaced with empty functions `() => {}`, reducing memory allocation.
4. **Strips Refs**: Ref props are omitted from JSX elements entirely. Since refs are not attached to DOM elements in SSR, this removes the need to allocate and pass ref objects.

```typescript
// From docs/passes/35-optimizeForSSR.md Phase 5:
if (isJSX(instr) && hasRefProp(instr)) {
  // Remove ref={...} from JSX props
  removeRefProp(instr.value);
}
```

---

## 3. Synthesis: Bun vs. React JSX Pipelines

| Feature / Phase | Bun Transpilation (`/tmp/bun`) | React Compiler & Runtime (`/tmp/react`) |
| :--- | :--- | :--- |
| **Primary Language** | Zig (`js_parser`, `js_printer`) | TypeScript (`react-compiler`), JavaScript (`jsx`) |
| **Parsing Phase** | Lexes JSX natively. Lowercase tag names are treated as strings (`E.String`); capitalized names are resolved as identifiers (`E.Identifier`). Resolves XML entity codes at parse time. | Standard Babel parsing into AST, then lowered to HIR (High-level Intermediate Representation). |
| **Lowering Target** | Classic (`React.createElement`) or automatic (`jsx`, `jsxs`, `jsxDEV`) runtime code. | Babel AST wrapping JSX elements with memoization cache checks (`_c`), `jsx`, or `jsxs` calls. |
| **Memoization** | None. Pure syntactic translation. | **Reactive Scopes**: Caches element structures globally. Only recreates elements if dependencies change. |
| **Callback Inlining** | None. Emits inline arrow/function expressions. | **JSX Outlining**: Extracts list item callbacks into top-level component functions with distinct caches. |
| **SSR Fast Paths** | None. Emits identical output for Client & Server. | **SSR Optimization Pass**: Strips `useEffect`, inlines state, no-ops event handlers, and drops `ref` props. |
| **Runtime Engine** | None. Transpiles to JS and relies on JavaScriptCore's JIT tier optimizations. | Creates Transitional Element objects tagged with `Symbol.for('react.transitional.element')` at runtime. |
