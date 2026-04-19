<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { createEventDispatcher } from "svelte";
  import type { AddProjectInput } from "../api/tauri";

  export let disabled = false;

  const dispatch = createEventDispatcher<{
    submit: AddProjectInput;
  }>();

  let name = "";
  let projectPath = "";
  let lastSuggestedName = "";

  $: canSubmit = name.trim().length > 0 && projectPath.trim().length > 0;

  function inferNameFromPath(path: string): string {
    const trimmed = path.trim().replace(/[\\/]+$/, "");
    if (!trimmed) {
      return "";
    }

    const parts = trimmed.split(/[\\/]/);
    return parts[parts.length - 1] ?? "";
  }

  function maybeAdoptSuggestedName(projectFolderName: string) {
    if (!projectFolderName) {
      return;
    }

    if (!name.trim() || name.trim() === lastSuggestedName) {
      name = projectFolderName;
      lastSuggestedName = projectFolderName;
    }
  }

  async function chooseProjectFolder() {
    const selected = await open({
      directory: true,
      multiple: false,
      title: "Select Java project folder"
    });

    if (typeof selected === "string") {
      projectPath = selected;
      maybeAdoptSuggestedName(inferNameFromPath(selected));
    }
  }

  function handleSubmit() {
    dispatch("submit", {
      name,
      projectPath,
    });

    name = "";
    projectPath = "";
  }
</script>

<form class="panel stack" on:submit|preventDefault={handleSubmit}>
  <div class="section-intro">
    <h2>Register Project</h2>
    <p class="muted">
      Pick a Java project folder to manage its JavaLens runtime.
    </p>
  </div>

  <label class="field">
    <span>Name</span>
    <input
      bind:value={name}
      disabled={disabled}
      placeholder="Defaults to the selected folder name"
      required
    />
  </label>

  <label class="field">
    <span>Project path</span>
    <div class="field-row">
      <input
        bind:value={projectPath}
        disabled={disabled}
        placeholder="/path/to/java/project"
        required
      />
      <button disabled={disabled} on:click={chooseProjectFolder} type="button">Browse</button>
    </div>
  </label>

  <button class="primary" disabled={disabled || !canSubmit} type="submit">Save project</button>
</form>
