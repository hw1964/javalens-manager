# Future Sprint Enhancements

## Deploy UX Enhancements

### Reintroduce True Preview Mode

Current state:
- Dashboard deploy actions include `Deploy`, `Dry run`, `Regenerate`, and `Delete`.
- `Preview` button is intentionally removed until a meaningful preview UX exists.

Future enhancement:
- Add back a dedicated `Preview` action and button.
- Render per-client generated MCP/rule output in a compact, readable panel before write.
- Keep preview run-scoped target selection identical to deploy/dry-run/regenerate/delete.

Acceptance goals:
- `Preview` clearly differs from `Dry run` in user-visible output.
- Preview output is compact and does not break dashboard density.
- Per-client preview supports copy/inspect without writing files.

## Platform Release Rollout

### MacOS and Windows Delivery

Future enhancement:
- Add packaging and release workflow for MacOS and Windows builds.
- Document install/update paths, signing requirements, and release notes policy.

Acceptance goals:
- Repeatable CI/CD build pipeline for Linux, MacOS, and Windows artifacts.
- Versioned release bundles and install instructions per platform.

## Services Selection from Probe

### Per-Service Deploy Selection

Current state:
- Settings probe can discover exposed services from JavaLens runtime.
- Deploy currently targets client configs as a full managed set.

Future enhancement:
- Use discovered services as selectable deploy units (single-service granularity).
- Allow selecting or excluding individual services for deploy generation.

Acceptance goals:
- Service discovery output is persisted and used as deploy input options.
- Users can deploy subset of services per client/target run.
- Validation explains missing/incompatible selections clearly.
