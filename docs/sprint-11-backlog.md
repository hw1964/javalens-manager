# Sprint 11 Backlog (draft)

> **Status: draft, updated 2026-04-26 after Sprint 10 Phase A landed as fork `v1.3.0`.** This sprint extends the multi-project `WorkspaceManager` shipped in v1.3.0 with detection-matrix completion AND tool-surface consolidation. Targets fork `v1.4.0` (was originally `v1.3.0` in earlier draft — re-versioned to follow Sprint 10's actual release).

## Goal

Three threads, one release:

1. **Detection-matrix completion** — any project type loaded into a workspace gets fully indexed (sources + dependencies) regardless of build system. Workspace bundle pool resolves `Require-Bundle` between sibling PDE bundles loaded together. Gradle resolution moves from heuristic to proper.
2. **Tool-surface consolidation** — collapse ~13 narrow `find_X` tools into 2 parametric tools so the per-service tool count drops from 66 to ~55, freeing budget for IDE-grade additions in Sprint 12+ while staying well under the 100-tool cap.
3. **Cutover** — cut fork release `v1.4.0`, flip manager `single_workspace_mode` default to true, ship manager `v0.11.0`.

End-of-sprint outcome:

- A workspace can hold any mix of regular Maven modules, Maven-Tycho/PDE bundles, pure Eclipse PDE bundles, and Gradle modules, and every project type returns correct source roots + dependencies.
- Cross-bundle navigation, find-references, and refactoring work between PDE bundles in the same workspace.
- javalens-mcp registers ~55 tools per service (down from 66) — leaves ~45 slots in the agent's 100-tool budget for Ring 1+ IDE work without future pressure.
- Fork release `v1.4.0` published; manager-side single-workspace mode becomes the default.

Reference plan: `~/.claude/plans/make-a-plan-happy-fern.md`. Predecessor: `docs/sprint-10-backlog.md` (multi-project workspace, shipped as v1.3.0).

## Problem Statement

Today's detection matrix (post-Sprint 9):

| Layout | Source roots | Dependencies |
|---|---|---|
| Regular Maven (pom.xml) | `<sourceDirectory>` override + heuristic | `mvn dependency:build-classpath` shell-out |
| Maven-Tycho (pom.xml with `<packaging>eclipse-*</packaging>`) | `.classpath` | **broken**: `mvn dependency:build-classpath` returns wrong/empty results — Tycho deps come from `MANIFEST.MF` + target platform, not pom `<dependencies>` |
| Pure Eclipse PDE (`.classpath` + `MANIFEST.MF`, no pom) | `.classpath` | `.classpath kind="lib"` only — `Require-Bundle` references stay unresolved |
| Gradle (build.gradle / .kts) | heuristic `src/main/java` only | **broken**: just walks `build/classes/java/...`, no real extraction |

Three of the four cases have correctness gaps. Sprint 11 closes them.

## Repos touched

- **`javalens-mcp` (fork)**: Tycho packaging detection, `MANIFEST.MF` parsing helpers, workspace bundle pool, Gradle Tooling API integration, **tool-surface consolidation (Phase E, new)**. Cut release `v1.4.0`.
- **`javalens-manager`**: flip `single_workspace_mode` default to true (where Sprint 10 made it opt-in); release a manager version targeting fork `v1.4.0`.

## Phase A — Tycho-aware Maven detection

### A.1 Detect Tycho packaging

File: `org.javalens.core/.../project/ProjectImporter.java`.

- New helper `readPomPackaging(Path pomXml) -> Optional<String>`. Pure DOM, returns the `<packaging>` text (`jar`, `pom`, `eclipse-plugin`, `eclipse-repository`, `eclipse-feature`, `eclipse-test-plugin`, …).
- Extend `BuildSystem` enum or add a sibling field `boolean isTycho` on the result. Tycho packagings: `eclipse-plugin`, `eclipse-test-plugin`, `eclipse-feature`, `eclipse-repository`, `eclipse-update-site`, `eclipse-target-definition`.

### A.2 Skip Maven shell-out for Tycho projects

In `addDependencyEntries`:

- When pom packaging is a Tycho type, **do not** call `getMavenDependencies` — its result is misleading (Tycho injects classpath via target platform; `dependency:build-classpath` misses most of it).
- Rely entirely on `.classpath kind="lib"` (Sprint 9 ✓) plus `Require-Bundle` resolved against the workspace bundle pool (Phase B below).

### A.3 Tests

`ProjectImporterTycho_PackagingTest`:
- `readPomPackaging_returnsEclipsePlugin` — fixture pom with `<packaging>eclipse-plugin</packaging>`.
- `readPomPackaging_returnsJar_byDefault` — pom without `<packaging>`.
- `tycho_skipsMavenShellOut` — fixture Tycho pom + .classpath; `addDependencyEntries` does not invoke `mvn dependency:build-classpath`.

## Phase B — Workspace bundle pool (`Require-Bundle` resolution)

This is the original Sprint 11 core. Helps PDE bundles in any flavor (Tycho or pure) loaded into the same workspace.

### B.1 MANIFEST.MF parsing helpers

In `ProjectImporter.java`:

- `readManifestSymbolicName(Path projectRoot) -> Optional<String>` — read `META-INF/MANIFEST.MF`, parse `Bundle-SymbolicName`, strip `;singleton:=true` and other directives. **Portable.**
- `readManifestRequireBundle(Path projectRoot) -> List<String>` — parse semicolon-delimited `Require-Bundle` header with version/visibility directives, return list of bundle symbolic names. **Portable.**

Use `java.util.jar.Manifest` for proper line-continuation handling.

### B.2 Bundle map in `WorkspaceManager`

Extend Sprint 10's multi-project `WorkspaceManager`:

```java
private final Map<String /*symbolicName*/, String /*projectKey*/> bundleSymbolicNameToProjectKey = new ConcurrentHashMap<>();

public void registerBundle(String symbolicName, String projectKey);
public Optional<IJavaProject> resolveBundle(String symbolicName);
```

`AddProjectTool` (Sprint 10) and `LoadProjectTool` call `readManifestSymbolicName` after registering each project; if present, register the bundle in the map.

### B.3 Wire `Require-Bundle` into dependency resolution

In `ProjectImporter.addDependencyEntries`, after pom + `.classpath` lib resolution:

```java
List<String> requires = readManifestRequireBundle(projectRoot);
for (String requiredBundle : requires) {
    workspaceManager.resolveBundle(requiredBundle).ifPresentOrElse(
        sibling -> entries.add(JavaCore.newProjectEntry(sibling.getPath())),
        () -> log.debug("Require-Bundle {} not found in workspace; skipping", requiredBundle)
    );
}
```

Bundles outside the workspace (e.g., resolved by an external Eclipse target platform) remain unresolved — accepted gap (see Outlook).

### B.4 Tests

`ProjectImporterBundlePoolTest`:
- `readManifestSymbolicName_stripsDirectives` — input `com.foo;singleton:=true` → `com.foo`.
- `readManifestRequireBundle_parsesMultiLineHeader` — Manifest with continuation lines.
- `requireBundle_resolvesWithinWorkspace` — load 2 fixture bundles A → B; A's classpath includes B as a project entry.
- `requireBundle_outsideWorkspace_logsAndContinues` — A requires com.example.unknown; load succeeds with warning.

## Phase C — Gradle Tooling API integration

### C.1 Add Gradle Tooling API dependency

- Add `org.gradle:gradle-tooling-api` (~5 MB) as a runtime dependency in `org.javalens.core/META-INF/MANIFEST.MF` and the fork's target platform.
- Bundle it in the RCP product (`plugins/`).

### C.2 Replace heuristic with Tooling API

In `ProjectImporter`:

- New `getGradleProjectModel(Path projectRoot) -> Optional<GradleProjectInfo>` (DTO with `srcPaths`, `libPaths`, `outputPath`). Uses `GradleConnector.newConnector().forProjectDirectory(...).connect()` and queries the `EclipseProject` model (which Gradle Tooling API exposes — already designed for IDE consumption).
- Replace the existing `getGradleDependencies` heuristic with this. Source roots come from `EclipseProject.getSourceDirectories()`; dependencies from `EclipseProject.getClasspath()` resolving to jar files.

### C.3 Tests

`ProjectImporterGradleToolingTest`:
- Add a minimal Gradle fixture (`org.javalens.core.tests/test-resources/sample-projects/simple-gradle/`).
- `gradle_returnsActualSourceSets` — extracts `src/main/java` and `src/test/java` from the model.
- `gradle_returnsActualDependencies` — finds the resolved jar paths for declared `implementation`/`testImplementation`.
- `gradle_customSrcDir` — fixture with `sourceSets.main.java.srcDirs = ['custom-src']` resolves correctly.

## Phase E — Tool-surface consolidation (NEW, added 2026-04-26)

### Why now

Per-service tool count is 66 today (5 admin + 61 analysis). Single-workspace mode means a healthy 1-workspace setup registers 66 tools against the agent's 100-tool budget. With GitKraken's MCP shim removed (decided 2026-04-26 — `git`/`gh` via Bash already covers everything an autonomous agent needs), 34 slots remain free. Phase E reclaims ~10 more by collapsing redundant `find_X` tools, leaving ~44 free for Ring 1 / Ring 2 IDE additions in Sprint 12+ (Move class, compile_workspace, run_tests, generate_*, encapsulate_field, pull_up/push_down, etc.) without future pressure.

### Consolidation targets

Two parametric tools replace ~13 narrow ones:

**`find_pattern_usages(kind, query, ...)`** — replaces:
- `find_annotation_usages(annotationName)` → `kind=annotation`
- `find_type_instantiations(typeName)` → `kind=instantiation`
- `find_type_arguments(typeName)` → `kind=type_argument`
- `find_casts(typeName)` → `kind=cast`
- `find_instanceof_checks(typeName)` → `kind=instanceof`

**`find_quality_issue(kind, ...)`** — replaces:
- `find_naming_violations()` → `kind=naming`
- `find_possible_bugs()` → `kind=bugs`
- `find_unused_code()` → `kind=unused`
- `find_large_classes()` → `kind=large_classes`
- `find_circular_dependencies()` → `kind=circular_deps`
- `find_reflection_usage()` → `kind=reflection`
- `find_throws_declarations()` → `kind=throws`
- `find_catch_blocks()` → `kind=catches`

**Kept as-is** (different parameter shapes — collapsing them hurts more than helps):
- `find_references(filePath, line, column)` — position-anchored
- `find_implementations(filePath, line, column)` — position-anchored
- `find_method_references(...)` — anchor-based
- `find_field_writes(...)` — anchor-based
- `find_tests(typeName)` — separate semantic

### Net change

13 tools collapse to 2. Tool count drops 66 → 55. Schema size grows (each parametric tool's `kind` enum is ~8 entries with descriptions) but agent context shrinks overall.

### E.1 Define the parametric tools

Files: new `org.javalens.mcp/src/org/javalens/mcp/tools/FindPatternUsagesTool.java`, `FindQualityIssueTool.java`. Each takes `kind` as a required string enum and dispatches to the existing search/analysis methods underneath. **Reuses existing `SearchService` + analysis logic** — only the tool boundary changes.

### E.2 Schema with discoverable `kind` enum

Each tool's input schema lists allowed `kind` values + per-kind descriptions so agents know what's available. Example:

```json
{
  "kind": {
    "type": "string",
    "enum": ["annotation", "instantiation", "type_argument", "cast", "instanceof"],
    "description": "What pattern to find. annotation: usage sites of @X. instantiation: 'new T(...)' calls. type_argument: 'List<T>' style usage. cast: '(T)' casts. instanceof: 'instanceof T' checks."
  }
}
```

Optional kind-specific parameters (e.g. `annotationName` for annotation kind, `query` for type-anchored kinds) are documented in description text.

### E.3 Deprecate-and-delete the 13 narrow tools

Remove the old `Find*Tool.java` files in the same v1.4.0 release. **Breaking change for any external MCP client that wasn't using javalens-manager.** Acceptable — javalens-manager is the only known consumer; the npm/MCP-Registry surface (which the fork doesn't publish to anyway, see v1.2.1's release-workflow strip) means no third-party tooling depends on these names.

### E.4 Update tool registrations

In `JavaLensApplication.registerTools()`: drop the 13 register lines, add the 2 new ones. Tool count goes from 66 to 55.

### E.5 Tests

`FindPatternUsagesToolTest`, `FindQualityIssueToolTest`:
- One test per `kind` value verifying it dispatches to the right underlying search and returns the same shape the old narrow tool did. ~13 small tests.
- One test per tool verifying invalid `kind` returns an `INVALID_PARAMETER` error.

## Phase D — Cutover release

### D.1 Tag fork v1.4.0

Bump pom + MANIFEST.MF qualifiers as needed; `git tag -a v1.4.0 -F docs/release-notes/v1.4.0.md`; push tag → CI publishes the GitHub release. Release notes cover Phases A/B/C (detection matrix) and Phase E (tool consolidation) under one curated note.

### D.2 Flip manager defaults

In `javalens-manager/src-tauri/src/config.rs`:

- Sprint 10 introduced `single_workspace_mode` behind a flag. Sprint 11 flips its default to `true` for fresh installs (one workspace per port group, all related projects share one MCP service).
- Existing user settings preserved.

### D.3 Manager release

Tag the manager (e.g., `v0.11.0`) targeting fork `v1.4.0`. Update README and Help docs to reflect the now-default port-as-workspace + bundle-pool behavior + the consolidated tool surface.

## Tests rollup

- Phase A: `ProjectImporterTychoPackagingTest` (3 tests).
- Phase B: `ProjectImporterBundlePoolTest` (4 tests). Add 2 fixture PDE bundles.
- Phase C: `ProjectImporterGradleToolingTest` (3 tests). Add `simple-gradle` fixture.
- Phase E: `FindPatternUsagesToolTest` + `FindQualityIssueToolTest` (~13 dispatch tests + 2 invalid-kind tests).
- Plus regression suite from Sprint 9 + Sprint 10 must stay green (381 tests today).

## Definition of Done

- [ ] All four detection-matrix cells return correct source roots + deps for their respective fixtures.
- [ ] Cross-bundle find-references works across two PDE bundles loaded into one workspace.
- [ ] Tool count per service drops to ~55 (from 66 in v1.3.0).
- [ ] Fork `v1.4.0` published with detection-matrix completion + workspace bundle pool + Gradle Tooling API + tool-surface consolidation.
- [ ] Manager `single_workspace_mode` default = true; release tagged.
- [ ] No regression on Sprint 9 / Sprint 10 fixtures.

## Out of scope (deferred)

- **External PDE target-platform expansion.** Bundling `org.eclipse.pde.core` + friends (~30 plugins, ~5–10 MB install size) and using `TargetPlatformService` to expand `Require-Bundle` against external `.target` files or an installed Eclipse install. Bigger effort. Workspace bundle pool covers the inter-workspace case which is the dominant case for the user's projects (JATS bundles all live together). Defer until external deps actually become a blocker.
- **Bazel improvements.** Today's Bazel handler walks for BUILD files + jars in `bazel-{bin,out}`. Not a priority for current user projects.
- **Multi-language indexing (Kotlin, Scala).** JDT does some Kotlin via plugins, but full Kotlin/Scala source indexing is out of scope.

## Outlook — strategy, IDE-grade roadmap, deferred items

### Tool budget — staying under 100 (decided 2026-04-26)

Antigravity caps at ~100 tool registrations across all MCP services. The plan to stay under it without giving up capability:

| Server | Tools | Decision |
|---|---|---|
| javalens (1 workspace, post-Sprint 10 Phase B/C) | 66 today, 55 after Sprint 11 Phase E | core |
| GitKraken MCP shim | 25 | **drop** — `git`/`gh` via Bash already covers everything an autonomous agent needs; the desktop GUI stays useful for humans (visual rebase, conflict UI), it's only the MCP shim that's redundant |
| Headroom for Ring 1-4 IDE work (Sprint 12+) | ~45 slots | future |

Math: 55 (javalens) + 0 (GitKraken removed) = 55, leaving 45 slots free for IDE-grade additions. Ring 1 (~6 tools) + Ring 2 (~8 tools) = 14, lands at 69. Still 31 under the cap. Ring 4 (debugging, 15+ tools) is the only thing that could blow the budget — and per the analysis below, debugging is the lowest-leverage addition for autonomous safe-upgrade work, so deferring it is fine.

### IDE-grade roadmap (Sprint 12+, preview only)

Captured 2026-04-26 from the strategic discussion on what extends javalens to "full-IDE-grade autonomous Java development" — driven by the JATS RCP overhaul use case (safe upgrades, bundle reshuffles, JDK migration).

**Ring 1 — highest JATS-overhaul leverage** (~one sprint, 5–6 tools):
- `move_class` / `move_package` — biggest gap for bundle reshuffles. JDT's `JavaRefactoringContribution` + LTK already does the heavy lifting.
- `pull_up` / `push_down` — cross-hierarchy member moves; critical for inheritance refactors.
- `encapsulate_field` — replace direct field access with getter/setter.
- `compile_workspace` — one tool returning every diagnostic across all loaded projects. The missing feedback loop that lets agents verify "did this refactor break anything?" in one call.
- `run_tests(class | method | package)` — JUnit launching + capture. Closes the refactor → test → fix loop.

After Ring 1, JATS upgrades become a "find what to change → refactor → compile → test" loop the agent can run end-to-end.

**Ring 2 — rounds out the IDE-grade autonomous loop** (~one sprint, 6–8 tools):
- `generate_constructor`, `generate_equals_hashcode`, `generate_to_string`, `generate_getters_setters`, `override_methods` — JDT has all the AST APIs.
- `convert_lambda_to_method_reference` (companion to existing `convert_anonymous_to_lambda`).
- `migrate_jdk_api(target=var | switch_expr | sealed_types | records | text_blocks)` — bulk JDK-version migration tool. Critical for JATS JDK 17 → 21 upgrade.
- "Fix all of kind X across project" — bulk quick-fix application.

**Ring 3 — Eclipse plugin packaging** (~one sprint, no new tools):
The same `IJdtService` + `LoadedProject` + tool implementations, repackaged as an Eclipse plugin so the work runs in-IDE (view + command palette + in-IDE AI assist target). The portability constraint we've held since Sprint 9 (ADR 0004 — helpers stay `Path` + DOM, no JDT leakage in helper signatures) makes this cheap. **Not a separate product — same logic, different shell.**

**Ring 4 — debugging surface (deferred)**:
JDT debug uses JPDA/JDI (`org.eclipse.jdt.debug.core`). Full surface: breakpoints (line/method/exception/conditional), attach-to-port, launch, step, evaluate expression, watchpoints, hot-swap, threading. The hard part isn't the JDT API — it's that debug is event-driven and MCP is request/response. Agent sessions need long-polling or a streaming mode. **Deferred** because: (a) agents working from passing tests + good static analysis rarely need to debug, (b) it's mostly a human-in-the-loop feature, (c) it's the largest engineering investment of the four rings, (d) a "debug session" parametric tool with sub-commands would be smarter than 15+ separate top-level tools (similar to Phase E's consolidation philosophy).

### Independence posture — confirmed 2026-04-26

The fork (`hw1964/javalens-mcp`) is the user's own version. Upstream PR back to `pzalutski-pixel/javalens-mcp` is **deferred indefinitely**, not abandoned:

- Sprint 9 (source resolution) and Sprint 10 Phase A (multi-project workspace) work IS general — would benefit any javalens user — and is PR-ready (ADR 0004 portability constraint paid the prep cost up front). Diff stays clean for as long as we want to wait.
- Future work (Rings 1-4, especially Ring 3's Eclipse plugin packaging) is increasingly JATS-driven. PR-able pieces will be a subset; review-cycle tax on the full set isn't justified.
- Pzalutski's roadmap is presumably solved for the single-developer / handful-of-projects case (probably a bash script). The 28-project scale that motivated this fork is a different problem and unlikely to be a priority for him.
- Re-evaluate the PR question only if (a) upstream goes inactive, (b) we want their user base for free testing, or (c) we've stabilized to the point where convergence beats independence. None of those are true today.

The portability constraint stays in force regardless — it's also what makes Ring 3 (Eclipse plugin) cheap.

### Eclipse IDE plugin

(Reframed 2026-04-26 — was previously listed as "separate product, not javalens"; now the plan is to package it as Ring 3 of the javalens roadmap, reusing all of the existing `IJdtService` infrastructure.) See "Ring 3 — Eclipse plugin packaging" above.

### LSP-based standalone server

A future product that keeps the headless model but speaks LSP (Language Server Protocol) instead of MCP. Useful for IDE integrations that prefer LSP over MCP. Reuses the same JDT integration layer and portable helpers from javalens.

This is a separate project. Not on the javalens roadmap.

### Bundling javalens jars inside the manager installer

Currently the manager downloads the runtime separately (per `release_repo`). Bundling them into one AppImage simplifies the install for new users at the cost of binding manager release cadence to fork release cadence. Consider after Sprint 11 ships.

## Team split (preview)

- `java-engineer`: Phases A, B, C — Tycho detection, bundle pool, Gradle Tooling API.
- `tauri-engineer`: Phase D — defaults flip, manager release.
- `release-engineer`: fork `v1.3.0` + manager `v0.11.0` cuts.
- `qa-test-engineer`: full detection-matrix verification across the four layout types.
- `docs-engineer`: README + Help.md updates for v0.11.0.
