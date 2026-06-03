// Ambient shims for @aphrody/material-web build artifacts that are NOT present in this
// source checkout of the fork.
//
// material-web's build pipeline generates two kinds of style modules that the
// component sources import but that only exist after `npm run build`
// (build:sass -> build:css-to-ts):
//   - `*-styles.cssresult.js`  (upstream + aphrody-labs SASS components)
//   - `*.css`                  (labs/gb Google-Sans components import raw CSS as a CSSResult)
//
// We only need the *type* of these modules so the wrappers typecheck; the
// actual CSS is irrelevant to the React wrapper layer (it ships with the
// element at runtime). Both export `styles: CSSResult`.
// NOTE: typed as `any`, not `CSSResult`. material-web bundles its own nested
// `lit@2` whose `CSSResult` is a nominally different class from the wrappers'
// `lit@3` `CSSResult`; importing either side here would make these style
// modules non-assignable to the elements' `static styles: CSSResultOrNative[]`.
// The wrapper layer never touches these values, so `any` is correct and safe.
declare module "*.cssresult.js" {
  export const styles: any;
}

declare module "*.css" {
  const styles: any;
  export default styles;
  export { styles };
}
