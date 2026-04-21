# Maintainer Runbook

This document outlines the steps to cut a new release of `javalens-manager`.

## 1. Prepare the Release

1. Ensure all changes for the release are merged into the `main` branch.
2. Update the version number in `src-tauri/tauri.conf.json`:
   ```json
   "version": "x.y.z"
   ```
3. Update the `managerBuildVersion` in `src/App.svelte` if necessary.
4. Commit the version bump:
   ```bash
   git add src-tauri/tauri.conf.json src/App.svelte
   git commit -m "chore: bump version to vX.Y.Z"
   git push origin main
   ```

## 2. Cut the Release

The release process is automated via GitHub Actions. To trigger a release, simply create and push a new Git tag:

```bash
git tag vX.Y.Z
git push origin vX.Y.Z
```

## 3. Verify the Release

1. Go to the [GitHub Actions tab](https://github.com/hw1964/javalens-manager/actions) and monitor the `Release` workflow.
2. Once the workflow completes, go to the [GitHub Releases page](https://github.com/hw1964/javalens-manager/releases).
3. Verify that the new release is published as a draft (or published, depending on workflow config) and contains the `.deb` and `.AppImage` assets.
4. If it's a draft, edit the release notes to include the changelog, and publish it.

## 4. Rollback (If necessary)

If a critical issue is found in the released version:
1. Delete the tag locally and remotely:
   ```bash
   git tag -d vX.Y.Z
   git push origin :refs/tags/vX.Y.Z
   ```
2. Delete the release from the GitHub Releases page.
3. Fix the issue, bump the patch version, and repeat the release process.
