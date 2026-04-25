# Sprint 11 Backlog (draft)

> **Status: draft.** Sprint 10 has not landed yet — this sprint depends on the multi-project `WorkspaceManager` and `add_project` / `remove_project` MCP tools that Sprint 10 introduces. Detail will be hardened once Sprint 10 is in flight.

## Goal

Complete the detection matrix so any project type loaded into a workspace gets fully indexed (sources + dependencies), regardless of which build system declares it. Make the workspace bundle pool resolve `Require-Bundle` cross-references between sibling PDE bundles loaded together. Bring Gradle resolution up from heuristic to proper. Cut fork release `v1.3.0` and flip manager defaults.

End-of-sprint outcome:

- A workspace can hold any mix of regular Maven modules, Maven-Tycho/PDE bundles, pure Eclipse PDE bundles, and Gradle modules, and every project type returns correct source roots + dependencies.
- Cross-bundle navigation, find-references, and refactoring work between PDE bundles in the same workspace.
- Fork release `v1.3.0` published; manager-side single-workspace mode becomes the default.

Reference plan: `~/.claude/plans/make-a-plan-happy-fern.md`. Predecessor: `docs/sprint-10-backlog.md` (port-as-workspace).

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

- **`javalens-mcp` (fork)**: Tycho packaging detection, `MANIFEST.MF` parsing helpers, workspace bundle pool, Gradle Tooling API integration. Cut release `v1.3.0`.
- **`javalens-manager`**: flip `single_workspace_mode` default to true (where Sprint 10 made it opt-in); release a manager version targeting fork `v1.3.0`.

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

## Phase D — Cutover release

### D.1 Tag fork v1.3.0

`mvn -B clean package` → CI publishes the GitHub release.

### D.2 Flip manager defaults

In `javalens-manager/src-tauri/src/config.rs`:

- Sprint 10 introduced `single_workspace_mode` behind a flag. Sprint 11 flips its default to `true` for fresh installs (one workspace per port group, all related projects share one MCP service).
- Existing user settings preserved.

### D.3 Manager release

Tag the manager (e.g., `v0.11.0`) targeting fork `v1.3.0`. Update README and Help docs to reflect the now-default port-as-workspace + bundle-pool behavior.

## Tests rollup

- Phase A: `ProjectImporterTychoPackagingTest` (3 tests).
- Phase B: `ProjectImporterBundlePoolTest` (4 tests). Add 2 fixture PDE bundles.
- Phase C: `ProjectImporterGradleToolingTest` (3 tests). Add `simple-gradle` fixture.
- Plus regression suite from Sprint 9 + Sprint 10 must stay green.

## Definition of Done

- [ ] All four detection-matrix cells return correct source roots + deps for their respective fixtures.
- [ ] Cross-bundle find-references works across two PDE bundles loaded into one workspace.
- [ ] Fork `v1.3.0` published with detection-matrix completion + workspace bundle pool + Gradle Tooling API.
- [ ] Manager `single_workspace_mode` default = true; release tagged.
- [ ] No regression on Sprint 9 / Sprint 10 fixtures.

## Out of scope (deferred)

- **External PDE target-platform expansion.** Bundling `org.eclipse.pde.core` + friends (~30 plugins, ~5–10 MB install size) and using `TargetPlatformService` to expand `Require-Bundle` against external `.target` files or an installed Eclipse install. Bigger effort. Workspace bundle pool covers the inter-workspace case which is the dominant case for the user's projects (JATS bundles all live together). Defer until external deps actually become a blocker.
- **Bazel improvements.** Today's Bazel handler walks for BUILD files + jars in `bazel-{bin,out}`. Not a priority for current user projects.
- **Multi-language indexing (Kotlin, Scala).** JDT does some Kotlin via plugins, but full Kotlin/Scala source indexing is out of scope.

## Outlook — separate products, not javalens

These are deliberately separate from the javalens roadmap. Captured here only so they don't get confused with javalens itself.

### Eclipse IDE plugin

A future product running **inside** an Eclipse IDE installation (not as a headless RCP). Hooks deeper Eclipse / Equinox / m2e / PDE / refactoring-participant services that javalens (headless RCP + MCP) cannot reach. Different runtime model entirely. May reuse small dependency-light helper methods from javalens (`readPomSourceDirs`, `readEclipseClasspath`, `readManifestSymbolicName`, `readManifestRequireBundle`, the bundle-pool builder — all designed under ADR 0004 to be liftable verbatim).

This is a separate project. Not on the javalens roadmap.

### LSP-based standalone server

A future product that keeps the headless model but speaks LSP (Language Server Protocol) instead of MCP. Useful for IDE integrations that prefer LSP over MCP. Reuses the same JDT integration layer and portable helpers from javalens.

This is a separate project. Not on the javalens roadmap.

### Upstream PR back to `pzalutski-pixel/javalens-mcp`

Optional. Decision postponed until the fork stabilizes after Sprint 11. Bring Sprint 9 + Sprint 10 + Sprint 11 changes back upstream as separate PRs if maintainer is responsive. Stay on the fork permanently if not. Either path is acceptable.

### Bundling javalens jars inside the manager installer

Currently the manager downloads the runtime separately (per `release_repo`). Bundling them into one AppImage simplifies the install for new users at the cost of binding manager release cadence to fork release cadence. Consider after Sprint 11 ships.

## Team split (preview)

- `java-engineer`: Phases A, B, C — Tycho detection, bundle pool, Gradle Tooling API.
- `tauri-engineer`: Phase D — defaults flip, manager release.
- `release-engineer`: fork `v1.3.0` + manager `v0.11.0` cuts.
- `qa-test-engineer`: full detection-matrix verification across the four layout types.
- `docs-engineer`: README + Help.md updates for v0.11.0.
