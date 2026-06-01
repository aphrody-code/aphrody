import {
  version,
  add,
  countChar,
  findBytes,
  hctToArgb,
  argbToHct,
  hctTones,
  deriveScheme,
  compileSass,
  validateSpec,
} from "./src/index.ts";

console.log("=== Bun-RS FFI JS/TS Test ===");
console.log("Version (expected: 1.0.0-canary):", version());
console.log("Add 10 + 20 (expected: 30):", add(10, 20));
console.log("Count 'o' in 'hello world' (expected: 2):", countChar("hello world", "o"));
console.log("Find index of 'fragil' (expected: 9):", findBytes("supercalifragilistic", "fragil"));

const argb = hctToArgb(277, 40, 40);
console.log("HCT(277, 40, 40) to ARGB (expected: ff4d5a9a):", argb.toString(16));
console.log("ARGB(0xFF6750A4) to HCT (expected: h~299, c~48, t~40):", argbToHct(0xff6750a4));
console.log("HCT(277, 40, 40) Tones (expected 13 elements):", hctTones(277, 40, 40));
console.log("Derive Scheme (expected 49 roles):", Object.keys(deriveScheme(277, 40, false)).length);

const sass = `
$primary: #6750A4;
a {
  color: $primary;
  &:hover {
    color: lighten($primary, 10%);
  }
}
`;
console.log("Compile SCSS via Grass Rust FFI:\n", compileSass(sass));

const testCode = `
  const style = "transition: transform 300ms cubic-bezier(0.42, 1.67, 0.21, 0.9);";
  const icon = <md-icon>check</md-icon>;
  const button = <md-icon-button aria-label="A11y Label"></md-icon-button>;

  // Violations
  const bad_color = "color: #ff0077;";
  const bad_role = "--md-sys-color-invalid-role-name";
  const bad_icon = <md-icon>wrong_name_here</md-icon>;
  const bad_curve = "transition: opacity 150ms cubic-bezier(0.1, 0.2, 0.3, 0.4);";
  const bad_btn = <md-icon-button></md-icon-button>;
`;
console.log("Validate Spec results:", JSON.stringify(validateSpec(testCode), null, 2));

console.log("All Bun-RS FFI TS tests passed successfully!");
