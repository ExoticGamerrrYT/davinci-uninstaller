<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { onMount } from "svelte";

  interface Item { label: string; path: string; kind: string; exists: boolean }
  interface ScanResult { app: Item[]; projects: Item[]; system_drive: string }
  interface UninstallResult { removed: number; errors: string[] }

  type Status = "idle" | "scanning" | "ready" | "confirming" | "running" | "done";

  let status = $state<Status>("idle");
  let admin = $state(true);
  let scan = $state<ScanResult | null>(null);
  let removeProjects = $state(false);
  let log = $state<string[]>([]);
  let result = $state<UninstallResult | null>(null);
  let errorMsg = $state<string | null>(null);

  let unlisten: UnlistenFn | undefined;

  onMount(() => {
    init();
    return () => unlisten?.();
  });

  async function init() {
    try { admin = await invoke<boolean>("is_admin"); } catch (e) { errorMsg = `is_admin: ${e}`; }
    try { unlisten = await listen<string>("log", (e) => { log = [...log, e.payload]; }); } catch (e) { errorMsg = `listen: ${e}`; }
    await runScan();
  }

  async function runScan() {
    errorMsg = null;
    status = "scanning";
    try {
      scan = await invoke<ScanResult>("scan");
      status = "ready";
    } catch (e) {
      errorMsg = `Scan failed: ${e}`;
      status = "idle";
    }
  }

  async function runUninstall() {
    status = "running";
    log = [];
    try {
      result = await invoke<UninstallResult>("uninstall", { removeProjects });
      status = "done";
    } catch (e) {
      errorMsg = `Uninstall failed: ${e}`;
      status = "ready";
    }
  }
</script>

<main class="min-h-screen bg-neutral-900 text-neutral-100 font-sans p-6">
  <div class="max-w-3xl mx-auto">
    <header class="mb-6">
      <h1 class="text-2xl font-semibold">DaVinci Resolve Uninstaller</h1>
      <p class="text-neutral-400 text-sm mt-1">Removes every trace of DaVinci Resolve from this computer.</p>
    </header>

    {#if errorMsg}
      <div class="mb-6 rounded-lg border border-red-700/60 bg-red-950/40 text-red-200 p-4 text-sm">
        <p class="font-mono break-all">{errorMsg}</p>
        <button onclick={runScan} class="mt-3 px-3 py-1.5 rounded-md bg-red-700 hover:bg-red-600 text-white text-xs">
          Retry
        </button>
      </div>
    {/if}

    {#if !admin}
      <div class="mb-6 rounded-lg border border-amber-600/50 bg-amber-950/40 text-amber-200 p-4 text-sm">
        ⚠️ This app needs administrator rights. Close it and reopen it with
        <b>“Run as administrator”</b> so it can delete system files and registry keys.
      </div>
    {/if}

    {#if status === "idle"}
      <button onclick={runScan}
        class="px-4 py-2 rounded-md bg-neutral-700 hover:bg-neutral-600 text-sm font-medium">
        Scan this computer
      </button>
    {/if}

    {#if status === "scanning"}
      <p class="text-neutral-400 animate-pulse">Scanning…</p>
    {/if}

    {#if scan && (status === "ready" || status === "confirming")}
      <p class="text-xs text-neutral-500 mb-4">
        Only the system drive ({scan.system_drive}) is cleaned — where Windows keeps Resolve's
        configuration, registry entries and databases.
      </p>

      <!-- App trace -->
      <section class="mb-6">
        <h2 class="text-sm font-semibold text-neutral-300 mb-2">DaVinci Resolve trace</h2>
        <ul class="rounded-lg border border-neutral-700 divide-y divide-neutral-800">
          {#each scan.app as it}
            <li class="flex items-center gap-3 px-3 py-2 text-sm {it.exists ? '' : 'opacity-40'}">
              <span class="w-2 h-2 rounded-full {it.exists ? 'bg-emerald-500' : 'bg-neutral-600'}"></span>
              <span class="w-40 shrink-0 text-neutral-300">{it.label}</span>
              <span class="flex-1 truncate text-neutral-500" title={it.path}>{it.path}</span>
              <span class="text-xs text-neutral-600">{it.exists ? "found" : "—"}</span>
            </li>
          {/each}
        </ul>
      </section>

      <!-- Projects -->
      <section class="mb-4">
        <h2 class="text-sm font-semibold text-neutral-300 mb-2">Projects & databases</h2>
        <ul class="rounded-lg border border-neutral-700 divide-y divide-neutral-800">
          {#each scan.projects as it}
            <li class="flex items-center gap-3 px-3 py-2 text-sm {it.exists ? '' : 'opacity-40'}">
              <span class="w-2 h-2 rounded-full {it.exists ? 'bg-red-500' : 'bg-neutral-600'}"></span>
              <span class="w-40 shrink-0 text-neutral-300">{it.label}</span>
              <span class="flex-1 truncate text-neutral-500" title={it.path}>{it.path}</span>
              <span class="text-xs text-neutral-600">{it.exists ? "found" : "—"}</span>
            </li>
          {/each}
        </ul>
      </section>

      <!-- Projects toggle -->
      <label class="flex items-start gap-3 rounded-lg border border-red-800/60 bg-red-950/30 p-4 mb-6 cursor-pointer">
        <input type="checkbox" bind:checked={removeProjects} class="mt-1 accent-red-500 w-4 h-4" />
        <span class="text-sm">
          <b class="text-red-300">Also delete projects</b>
          <span class="block text-red-300/80 mt-0.5">
            This will delete all your DaVinci projects, timelines and databases (including PostgreSQL).
            <b>This cannot be undone.</b> Leave it unchecked to keep your projects.
          </span>
        </span>
      </label>

      {#if status === "confirming"}
        <div class="rounded-lg border border-neutral-600 bg-neutral-800 p-4 mb-4">
          <p class="text-sm mb-3">
            {#if removeProjects}
              DaVinci Resolve <b class="text-red-400">and all your projects</b> will be deleted. Are you sure?
            {:else}
              DaVinci Resolve will be removed (projects are kept). Continue?
            {/if}
          </p>
          <div class="flex gap-2">
            <button onclick={runUninstall}
              class="px-4 py-2 rounded-md bg-red-600 hover:bg-red-500 text-white text-sm font-medium">
              Yes, remove it
            </button>
            <button onclick={() => (status = "ready")}
              class="px-4 py-2 rounded-md bg-neutral-700 hover:bg-neutral-600 text-sm">
              Cancel
            </button>
          </div>
        </div>
      {:else}
        <button onclick={() => (status = "confirming")} disabled={!admin}
          class="w-full px-4 py-3 rounded-md bg-red-600 hover:bg-red-500 disabled:bg-neutral-700 disabled:text-neutral-500 text-white font-medium">
          Uninstall DaVinci Resolve
        </button>
      {/if}
    {/if}

    {#if status === "running" || status === "done"}
      <section class="rounded-lg border border-neutral-700 bg-neutral-950 p-4 font-mono text-xs text-neutral-300 h-72 overflow-auto mb-4">
        {#each log as line, i (i)}<div>{line}</div>{/each}
        {#if status === "running"}<div class="text-neutral-500 animate-pulse">…</div>{/if}
      </section>
    {/if}

    {#if status === "done" && result}
      <div class="rounded-lg border border-emerald-700/50 bg-emerald-950/30 p-4">
        <p class="font-medium text-emerald-300">Uninstall complete</p>
        <p class="text-sm text-neutral-300 mt-1">Items removed: <b>{result.removed}</b></p>
        {#if result.errors.length}
          <details class="mt-2 text-sm">
            <summary class="cursor-pointer text-amber-400">{result.errors.length} warning(s)</summary>
            <ul class="mt-1 text-neutral-400 list-disc pl-5">
              {#each result.errors as e, i (i)}<li>{e}</li>{/each}
            </ul>
          </details>
        {/if}
        <p class="text-sm text-neutral-400 mt-3">
          🔁 A <b>Windows restart</b> is recommended to finish cleaning up any files that were locked.
        </p>
        <button onclick={runScan}
          class="mt-4 px-4 py-2 rounded-md bg-neutral-700 hover:bg-neutral-600 text-sm font-medium">
          Scan again
        </button>
      </div>
    {/if}

    <p class="text-[10px] text-neutral-700 mt-8">build 6</p>
  </div>
</main>
