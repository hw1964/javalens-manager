<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import type { AddProjectInput } from "../api/tauri";

  export let disabled = false;

  const dispatch = createEventDispatcher<{
    submit: AddProjectInput;
  }>();

  let name = "";
  let projectPath = "";
  let javalensJarPath = "";
  let workspaceDir = "";

  function handleSubmit() {
    dispatch("submit", {
      name,
      projectPath,
      javalensJarPath,
      workspaceDir: workspaceDir.trim() || undefined
    });

    name = "";
    projectPath = "";
    javalensJarPath = "";
    workspaceDir = "";
  }
</script>

<form class="panel stack" on:submit|preventDefault={handleSubmit}>
  <div>
    <h2>Register Project</h2>
    <p class="muted">
      Add one Java project plus the JavaLens JAR you want this manager to launch.
    </p>
  </div>

  <label class="field">
    <span>Name</span>
    <input bind:value={name} disabled={disabled} placeholder="Example Service" required />
  </label>

  <label class="field">
    <span>Project path</span>
    <input
      bind:value={projectPath}
      disabled={disabled}
      placeholder="/path/to/java/project"
      required
    />
  </label>

  <label class="field">
    <span>JavaLens JAR path</span>
    <input
      bind:value={javalensJarPath}
      disabled={disabled}
      placeholder="/path/to/javalens.jar"
      required
    />
  </label>

  <label class="field">
    <span>Workspace dir override</span>
    <input
      bind:value={workspaceDir}
      disabled={disabled}
      placeholder="Optional. Defaults to manager-owned cache path."
    />
  </label>

  <button class="primary" disabled={disabled} type="submit">Save project</button>
</form>
