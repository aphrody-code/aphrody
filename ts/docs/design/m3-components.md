<!-- SPDX-License-Identifier: Apache-2.0 -->

# Material 3 components — catalog & coverage

> **Paraphrase note.** Every M3 spec value below is paraphrased and reduced to
> numeric tokens (dp, corner radius, colour role, elevation level). No sentence
> is copied from Google's documentation; values are facts (sizes, role names)
> not protected expression. Source of truth: **<https://m3.material.io/components>**
> and the per-component `/specs` pages (verified 2026-05-22 for buttons, FAB,
> cards, switch; remaining rows use established M3 baseline tokens).

This catalog enumerates the full M3 component set (the 6 M3 categories: action,
containment, communication, navigation, selection, text input) and grounds each
row in the real aphrody `mui-rs-components` crate
(`crates/mui-rs-components/src/*.rs`). The **Status** column reflects what the
code actually does, not the README.

Status legend:

- ✅ **Widget impl** — a `Widget`/`mui_rs_renderer::pipeline::Widget` `draw()` is implemented with real geometry + colour roles.
- ⚠️ **struct only** — a Rust struct/enum exists but no `Widget::draw` (no rendered geometry).
- ❌ **absent** — no type for this M3 component in the crate.

## Catalog & coverage

| Component | M3 key spec (size dp, radius, role) | mui-rs type (path) | Status |
| --- | --- | --- | --- |
| **Buttons** (common) | Height 40dp; corner Full (pill); padding 24dp (M3) / 16dp (Expressive); label `label-large` 14/500. Roles: Filled = Primary / On-primary; Tonal = Secondary container / On-secondary-container; Outlined = transparent + Outline(-variant) / On-surface-variant; Elevated = Surface container low / Primary (elev 1); Text = Primary | `actions::Button` + `ButtonVariant{Filled,FilledTonal,Outlined,Elevated,Text}` (`actions.rs:9,29`) | ✅ |
| **Button groups** | Row of connected buttons; inner buttons square, end buttons Full; gap/shape morph on press | — | ❌ |
| **Extended FABs** | Height 56dp; corner 16dp; container Primary container / On-primary-container; elevation 3; `label-large` 16/500 + leading icon | `actions::ExtendedFab` (`actions.rs:207`) | ✅ |
| **FAB menu** | FAB that expands a column of labelled mini actions | — | ❌ |
| **Floating action buttons (FABs)** | Standard 56dp square; corner 16dp; container Primary container / On-primary-container (Expressive); elevation 3 (hover 4); icon 24dp. (Small 40dp/12dp, Large 96dp/28dp) | `actions::Fab` (`actions.rs:172`, `SIZE_DP=56`, `RADIUS_DP=16`) | ✅ |
| **Icon buttons** | 40dp container, 48dp target; icon 24dp; variants standard/filled/tonal/outlined; corner Full | — | ❌ |
| **Segmented buttons** | Height 40dp; outer corner Full, inner segments square; selected = Secondary container / On-secondary-container; 1dp Outline divider | `actions::SegmentedButton` (`actions.rs:239`, `HEIGHT_DP=40`, `SEG_W=96`) | ✅ |
| **Split buttons** | Leading action + trailing menu trigger, connected; Full outer corners | — | ❌ |
| **Date pickers** | Modal/docked; container Surface container high; corner 28dp (modal); input variant uses outlined text-field box | `inputs::DatePicker` (`inputs.rs:216`) — outlined-field stand-in only (no full calendar) | ✅ |
| **Time pickers** | Modal/dial; container Surface container high; corner 28dp; clock dial | `inputs::TimePicker` (`inputs.rs:221`) — outlined-field stand-in (no dial) | ✅ |
| **Loading indicator** | Small contained spinner (Expressive); active Primary on container | — | ❌ |
| **Progress indicators** | Linear track 4dp tall, Full corners; track = Surface container highest / Secondary container; active = Primary. Circular 4dp stroke | `feedback::ProgressIndicator` (`feedback.rs:66`, linear, `HEIGHT_DP=4`) — linear only, no circular | ✅ |
| **Navigation bar** | Height 80dp; container Surface container; active indicator pill = Secondary container; active icon On-secondary-container, inactive On-surface-variant; 3–5 items | `navigation::NavigationBar` (`navigation.rs:82`, `HEIGHT_DP=80`) | ✅ |
| **Navigation drawer** | Width 360dp; container Surface container low; item rows 56dp, active row Secondary container, corner Full per active row | `navigation::NavigationDrawer` (`navigation.rs:147`, `WIDTH_DP=360`) — fills background only, no items | ✅ |
| **Navigation rail** | Width 80dp; container Surface; active indicator Secondary container; vertical item stack | `navigation::NavigationRail` (`navigation.rs:115`, `WIDTH_DP=80`) | ✅ |
| **Bottom sheets** | Top corners 28dp; container Surface container low; 32×4dp drag handle = On-surface-variant | `containers::BottomSheet` (`containers.rs:78`, `WIDTH_DP=360`, `HEIGHT_DP=220`, corner 28) | ✅ |
| **Side sheets** | Width ~256–400dp; container Surface container low; leading divider; corner 0 (docked) / 16dp (modal) | `display::SideSheet` (`display.rs:270`, `WIDTH_DP=256`) | ✅ |
| **App bars** | Top app bar height 64dp (small); container Surface; title `title-large` 22/400 On-surface. Bottom app bar 80dp | `navigation::TopAppBar` (`navigation.rs:10`, `HEIGHT_DP=64`) + `containers::TopAppBar` (struct, `containers.rs:83`) + `containers::BottomAppBar` (`containers.rs:88`, `HEIGHT_DP=80`) | ✅ |
| **Badges** | Small dot 6dp; large pill 16dp tall; container Error / On-error; `label-small` 11/500 | `feedback::Badge` (`feedback.rs:14`) | ✅ |
| **Cards** | Corner 12dp; padding 16dp; Elevated = Surface container low (elev 1); Filled = Surface container highest (elev 0); Outlined = Surface + 1dp Outline variant | `containers::Card` + `CardVariant{Elevated,Filled,Outlined}` (`containers.rs:12,19`, `RADIUS_DP=12`) | ✅ |
| **Carousel** | Hero/multi-browse; item corner 16dp (28dp large); items scroll; container Surface variants | `carousel::Carousel` (`carousel.rs:4`, `HEIGHT_DP=120`, item corner 16) | ✅ |
| **Checkbox** | 18dp box, corner 2dp; selected fill Primary + On-primary check; unselected 2dp On-surface-variant outline; 40dp state layer | `inputs::Checkbox` (`inputs.rs:92`, 18dp, corner 2) | ✅ |
| **Chips** | Height 32dp; corner 8dp; unselected = 1dp Outline + On-surface-variant; selected (filter) = Secondary container / On-secondary-container; assist/filter/input/suggestion variants | `display::Chip` (`display.rs:26`, `HEIGHT_DP=32`, corner 8) — single variant, no leading icon/trailing close | ✅ |
| **Dialogs** | Corner 28dp; container Surface container high; headline `headline-small` 24/400 On-surface; min width 280dp | `containers::Dialog` (`containers.rs:93`, `WIDTH_DP=312`, corner 28) | ✅ |
| **Divider** | Thickness 1dp; colour Outline variant; full-bleed or inset 16dp | `divider::Divider` (`divider.rs:4`, `WIDTH_DP=360`, Outline variant `202,196,208`) | ✅ |
| **Lists** | One-line row 56dp (two-line 72dp, three-line 88dp); `body-large` 16/400 On-surface; dividers Outline variant | `display::List` (`display.rs:54`, `ROW_H=56`) | ✅ |
| **Menus** | Min width 112dp; container Surface container; corner 4dp; elevation 2; row height 48dp; `label-large` 14 | `display::Menu` (`display.rs:81`, `WIDTH_DP=200`, `ROW_H=48`, corner 4, elev 2) | ✅ |
| **Radio button** | 20dp ring (2dp stroke); selected dot Primary; unselected On-surface-variant; 40dp state layer | `inputs::Radio` (`inputs.rs:118`, 20dp circle, inner 5dp) | ✅ |
| **Search** | Search bar height 56dp; corner Full; container Surface container high; leading search icon; `body-large` 16 | `inputs::SearchBar` (`inputs.rs:226`, `HEIGHT_DP=56`, `WIDTH_DP=360`, Full) | ✅ |
| **Sliders** | Track 4dp (active/inactive); active Primary, inactive Secondary container; handle; value labels | `inputs::Slider` (`inputs.rs:187`, 4dp track, 10dp thumb) | ✅ |
| **Snackbar** | Single line 48dp tall; corner 4dp; container Inverse surface / Inverse on-surface; elevation 3; width ≤ ~344dp | `feedback::Snackbar` (`feedback.rs:41`, `HEIGHT_DP=48`, `WIDTH_DP=344`, corner 4, elev 3) | ✅ |
| **Switch** | Track 32×52dp, corner Full, 2dp outline; handle 16dp (off) / 24dp (on) / 28dp (pressed); track off = Surface container highest + Outline, on = Primary; handle off = Outline, on = On-primary | `inputs::Switch` (`inputs.rs:139`, track 52×32, radius 16, thumb 16/24) | ✅ |
| **Tabs** | Height 48dp (primary); active indicator 3dp Primary; label `title-small` 14; inactive On-surface-variant; bottom hairline Outline variant | `display::Tab` (`display.rs:109`, `HEIGHT_DP=48`, indicator 3dp) | ✅ |
| **Text fields** | Filled & Outlined; height 56dp; Outlined corner 4dp + 1dp Outline; Filled = Surface container highest + bottom line; label `body-small` 12, value `body-large` 16; error = Error role | `inputs::TextField` + `TextFieldVariant{Standard,Filled,Outlined}` (`inputs.rs:11,20`, `HEIGHT_DP=56`, corner 4) | ✅ |
| **Toolbars** | Docked/floating; corner Full (floating); container Surface container; elevation 2; action icons 24dp | `display::Toolbar` (`display.rs:242`, `HEIGHT_DP=64`, Full, elev 2) | ✅ |
| **Tooltips** | Plain tooltip min height 24dp; corner 4dp; container Inverse surface / Inverse on-surface; `body-small` 12. (Rich tooltip = Surface container, corner 12dp) | `containers::Tooltip` (`containers.rs:107`, `HEIGHT_DP=24`, corner 4) | ✅ |

### Extra components in mui-rs (not standalone M3 component pages)

| mui-rs type (path) | Notes | Status |
| --- | --- | --- |
| `inputs::Select` (`inputs.rs:87`) | Exposed-dropdown / menu pattern (an M3 menu+text-field composite, not a top-level component page); outlined field + chevron | ✅ |
| `display::DataTable` (`display.rs:148`) | "Data tables" is an M2 component; M3 has no dedicated page yet. Header + rows + dividers | ✅ |
| `display::ImageList` (`display.rs:191`) | M2 "Image lists" / grid; rounded 12dp tiles | ✅ |
| `display::Banner` (`display.rs:218`) | M2 "Banners"; full-width strip + bottom divider | ✅ |

## Discrepancies vs M3 spec

Concrete mismatches found between the verified M3 specs and the mui-rs impl
values. **No source files were edited** — values below are the corrected targets
for the main agent.

1. **Outlined / Elevated / Text button label colour role**
   (`crates/mui-rs-components/src/actions.rs:121-124`).
   The impl paints these labels with `primary` (`103,80,164`). Per the updated
   buttons spec the *default* outlined button label/icon is **On-surface-variant**
   (≈ `73,69,79`) with the outline drawn in **Outline variant**; only Elevated
   and Text legitimately use Primary for the label. Outlined should not reuse the
   Primary role for its label. (Elevated/Text rows are fine.)

2. **Outlined button outline colour** (`actions.rs:159`).
   Stroke uses `Outline` (`121,116,126`). The current M3 outlined button uses
   **Outline variant** (`202,196,208`) for the resting container outline. Update
   the non-disabled stroke to `Color::from_rgb8(202, 196, 208)`.

3. **FAB icon / content colour role** (`actions.rs:197`).
   The FAB container is `232,222,248` (treated as primary-container) but the icon
   is drawn with `29,25,43` which is the **On-secondary-container** tone. The
   default FAB pairing is **Primary container / On-primary-container**, so the
   icon role is mismatched — use the On-primary-container tone, not
   On-secondary-container.

4. **FAB default container role naming** (`actions.rs:189`, `:227`).
   Both `Fab` and `ExtendedFab` fill with `232,222,248`. In the Expressive update
   the default FAB/extended-FAB container role was renamed to **Primary
   container** (previously labelled "primary"). The RGB `232,222,248` is the M3
   baseline *secondary*-container tone, not primary-container
   (baseline primary-container ≈ `234,221,255`). If targeting baseline M3, the
   primary-container value should be used for the default FAB.

5. **Filled card container colour** (`containers.rs:129`).
   Comment says "Surface container highest" but the value `232,222,248` is the
   secondary/primary-container tone. Surface container highest in the M3 baseline
   light scheme is ≈ `236,230,240` (`E6E0E9`). Filled card should use
   `Color::from_rgb8(236, 230, 240)`.

6. **Outlined card outline colour** (`containers.rs:140`).
   Uses `Outline` (`121,116,126`). The cards spec specifies the outlined-card
   border is **Outline variant** (`202,196,208`). Update the resting outline to
   `Color::from_rgb8(202, 196, 208)`.

7. **Checkbox container size** (`inputs.rs:99`).
   Box is `18.0`dp. The M3 checkbox container is **18dp** for the box but the
   broader spec target/state-layer is 40dp and the *icon container* is commonly
   modeled at 18dp — this is borderline acceptable. Note: there is **no 40dp
   state layer / 48dp touch target** modeled, so hit-area is undersized vs spec.

8. **Radio ring size** (`inputs.rs:125`).
   Outer ring radius 10dp (= 20dp diameter) is correct, but like the checkbox
   there is **no 40dp state layer / 48dp target** — accessibility target missing.

9. **Chip corner radius** (`display.rs:42`).
   Chip uses corner `8.0`dp. M3 chips use **8dp** (small) — this matches. However
   the chip models only one generic variant; M3 defines assist/filter/input/
   suggestion with leading icon + trailing close affordances that are absent
   (functional gap, not a wrong-value bug).

10. **Tooltip is plain-only** (`containers.rs:107`).
    Height 24dp + corner 4dp + inverse-surface is correct for the **plain**
    tooltip, but the **rich tooltip** variant (Surface container, corner 12dp,
    title + supporting text + actions) is not modeled.

11. **Progress indicator track role** (`feedback.rs:80`).
    Track uses `230,224,233` (Secondary container). The current M3 linear track
    role is **Surface container highest** (the track also shows a stop indicator
    dot in the latest spec). Acceptable but role drift; latest spec leans on
    surface-container-highest for the track.

12. **Divider default thickness** — correct (1dp, Outline variant). No issue;
    listed for completeness as the one fully-conformant primitive.

### Functional gaps (no struct at all — ❌)

These M3 component pages have **no** corresponding type in `mui-rs-components`:

- **Button groups** — connected button row with shape morph.
- **FAB menu** — FAB-anchored expanding action column.
- **Icon buttons** — 40dp standard/filled/tonal/outlined icon-only button (note: the in-bar window-control glyphs in `navigation.rs` are bespoke, not an M3 icon-button widget).
- **Split buttons** — action + connected menu trigger.
- **Loading indicator** — the new Expressive contained spinner (distinct from progress indicators).

(Circular progress is also missing — `ProgressIndicator` is linear-only.)
