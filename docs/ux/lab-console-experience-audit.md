# LAB CONSOLE Experience Audit

Date: 2026-07-14

Baseline reviewed: `0.13.2` plus the post-release planning fixes on `main`

Latest corrective implementation release: `0.16.0`

## Follow-up In 0.16.0

`Matériels réels` now opens a metrologist's dossier instead of a standalone
registration form. The selected inventory/serial number keeps its status,
model reference, characterization history, guided time/frequency entry,
uncertainty and uploaded evidence in one workflow. The 1280 x 720 review also
removed an obscuring sticky action bar and retained a compact three-column
status summary.

## Follow-up In 0.15.1

The signal-correction follow-up removes the remaining abstract entry point.
Operators first choose whether they are converting time samples or compensating
a frequency spectrum. Creation requires a descriptive name while internal
identities are generated; controlled references and history display names and
revision numbers; sensor excitation is explained as power and conditioning.

The method-library empty state, filters, creation, and duplication also use
method vocabulary rather than template or API vocabulary. Playwright now
captures and checks the main method and correction paths at exactly 1440 x 900
and 1280 x 720 with no page-level horizontal overflow.

## Audit Method

The review covered the production LAB CONSOLE served by the local Rust agent,
the React information architecture, desktop and narrow responsive layouts, the
method library and editor, the Equipment catalog, repository administration,
measurement engineering, driver controls, the equipment creation wizard, and
the automated operator workflows.

The review used three questions for every visible control:

1. Is the destination or action implemented and useful now?
2. Does the control belong to the operator's current context?
3. Is the information needed for the current decision, or is it traceability
   evidence that should remain available on demand?

## Findings Addressed In 0.13.3

### Navigation implied capabilities that did not exist

The sidebar presented fourteen destinations although only Methods, Equipment,
and local diagnostics had usable product surfaces. Twelve buttons opened the
same generic unavailable state. This weakened confidence in every navigation
choice.

The shell now makes only implemented workspaces actionable. Metrology,
Planning, Campaigns, and Reports remain visible as future product context but
are not styled or exposed as commands.

### Equipment controls ignored the selected workspace

Catalog filters and creation commands remained visible in administration,
drivers, and measurement engineering. Measurement engineering also repeated
its own navigation in multiple tab rows.

The Equipment workspace now has one primary context selector. Catalog filters,
driver commands, and engineering subspaces appear only where they apply.

### Technical evidence dominated normal tasks

Revision IDs, checksums, internal status codes, and aggregate IDs competed with
model names and lifecycle decisions. They remain necessary for diagnostics and
quality evidence, but not as the first reading level.

Normal headers and lists now use business labels, human status names, and
revision numbers. Traceability context, category trees, raw JSON, and custom IDs
remain available through explicit advanced disclosures.

### Layout reduced the actual working area

Three-column editors compressed the form at ordinary desktop widths. Sparse
grids stretched cards and empty states vertically. The inline model wizard
pushed the current context down the page.

Editors now use two columns at normal desktop widths, move validation below the
form, and reserve three columns for wide screens. Grids align content to the
start. The model wizard is a focused modal task with stable header and actions.

### Responsive and focus behavior were inconsistent

The previous hierarchy was desktop-shaped and relied on wrapping without a
clear mobile order. Keyboard focus did not have one consistent visual contract.

The shell, workspaces, command bars, editor grids, and wizard now reflow at
tablet and mobile widths. Interactive controls use a shared visible focus state,
and icon-only actions have accessible names and tooltips.

## Design Principles Adopted

- One visible command belongs to one current context.
- Implemented workspaces are navigation; future work is roadmap information.
- Business identity comes before technical identity.
- Lifecycle actions follow the current revision state.
- Traceability evidence stays reachable without occupying the primary task.
- Dense laboratory screens preserve working width before adding columns.
- Empty and loading states occupy only the space their message requires.
- Responsive behavior changes information order, not only element width.

## Remaining UX Debt

- Deep method editing still exposes technical enum labels and identifiers
  because the underlying controlled vocabularies do not yet expose localized
  display metadata.
- The method editor needs task-oriented section grouping and richer graphical
  representations for sequences, limits, and instrumentation chains.
- Repository administration needs explicit permission and role boundaries
  before it can be presented safely to every laboratory user.
- Accessibility has automated focus and semantic coverage, but no formal
  assistive-technology or external WCAG audit has been performed.
- Real operator validation in a laboratory remains necessary; automated tests
  demonstrate software behavior, not workflow fitness or EN ISO/IEC 17025
  compliance.

## Recommended Next Design Slice

Keep `0.14.0` focused on the planned physical asset and station vertical, but
design it as one end-to-end technician workflow: identify a serial-numbered
asset, see metrological readiness, bind it to a station connection, and obtain
an explainable measurement-chain verdict. Do not add separate CRUD screens for
each underlying table.
