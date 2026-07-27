<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { onMount } from "svelte";

  interface Item { label: string; path: string; kind: string; exists: boolean }
  interface ScanResult { app: Item[]; projects: Item[]; system_drive: string }
  interface UninstallResult { removed: number; errors: string[] }

  type Status = "idle" | "scanning" | "ready" | "confirming" | "running" | "done";

  // shared recipes — the variants differ only in colour, so compose off one base
  const BTN =
    "cursor-pointer rounded-ctl border text-[13px] font-medium transition active:translate-y-px";
  const GHOST =
    `${BTN} self-start border-line bg-transparent px-4 py-2.5 text-ink hover:border-line-hi hover:bg-surface-2`;
  const DANGER =
    `${BTN} border-danger/70 bg-danger px-4 py-2.5 font-semibold text-white inset-shadow-lip hover:bg-danger-lift ` +
    "disabled:cursor-not-allowed disabled:border-line disabled:bg-surface-2 disabled:text-dim " +
    "disabled:inset-shadow-none disabled:active:translate-y-0";

  const PANEL = "rounded-panel border border-line bg-surface";
  const EYEBROW = "font-mono text-eyebrow uppercase text-dim";
  // kept out of the markup so the class stays literal for Tailwind's scanner
  const SWEEP =
    "after:absolute after:inset-y-0 after:w-px after:animate-sweep after:bg-amber " +
    "after:shadow-sweep after:content-[''] motion-reduce:after:animate-none";

  let status = $state<Status>("idle");
  let admin = $state(true);
  let scan = $state<ScanResult | null>(null);
  let removeProjects = $state(false);
  let log = $state<string[]>([]);
  let result = $state<UninstallResult | null>(null);
  let errorMsg = $state<string | null>(null);

  let unlisten: UnlistenFn | undefined;
  let logEl = $state<HTMLElement | null>(null);

  const found = $derived(
    scan ? [...scan.app, ...scan.projects].filter((i) => i.exists).length : 0
  );

  onMount(() => {
    init();
    return () => unlisten?.();
  });

  // follow the log as it streams
  $effect(() => {
    log.length;
    if (logEl) logEl.scrollTop = logEl.scrollHeight;
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

<main class="min-h-screen px-6 pt-10 pb-8 max-sm:px-3.5 max-sm:pt-6">
  <div class="mx-auto flex max-w-[760px] flex-col gap-4.5">
    <header class="flex items-center gap-3.5 border-b border-line-soft pb-4 max-sm:flex-wrap">
      <div
        class="grid size-9.5 flex-none place-items-center rounded-ctl border border-line bg-linear-to-b from-surface-2 to-surface text-muted"
        aria-hidden="true"
      >
        <svg viewBox="0 0 24 24" fill="none" stroke-width="1.4" class="size-5.5">
          <circle cx="12" cy="12" r="8.5" class="stroke-current opacity-45" />
          <circle cx="12" cy="12" r="3.2" class="stroke-current" />
          <path d="M12 1.5v21" class="stroke-amber" stroke-linecap="round" />
        </svg>
      </div>

      <div class="min-w-0 flex-1">
        <h1 class="font-display text-title">DaVinci Resolve Uninstaller</h1>
        <p class="mt-0.5 text-row text-muted">Removes every trace of Resolve from this computer.</p>
      </div>

      <div class="flex flex-none gap-1.5 max-sm:w-full">
        {#if scan}
          <span class="rounded-full border border-line px-2.5 py-1 font-mono text-tag whitespace-nowrap text-dim">
            {scan.system_drive} drive
          </span>
        {/if}
        <span
          class="rounded-full border px-2.5 py-1 font-mono text-tag whitespace-nowrap {admin
            ? 'border-line text-dim'
            : 'border-amber/40 text-amber'}"
        >
          {admin ? "admin" : "no admin"}
        </span>
      </div>
    </header>

    {#if errorMsg}
      <div class="{PANEL} flex flex-col items-start gap-2 border-danger/40 bg-danger-wash p-4">
        <p class="font-mono text-path break-words text-danger-soft">{errorMsg}</p>
        <button class="{GHOST} px-3 py-1.5 text-xs" onclick={runScan}>Try again</button>
      </div>
    {/if}

    {#if !admin}
      <div class="{PANEL} flex flex-col gap-1.5 border-amber/40 bg-amber-wash p-4 text-row leading-relaxed">
        <b class="font-semibold text-amber">Administrator rights are required.</b>
        <span class="text-muted">
          Close this window and reopen it with “Run as administrator” so it can delete system files
          and registry keys.
        </span>
      </div>
    {/if}

    {#if status === "idle"}
      <button class={GHOST} onclick={runScan}>Scan this computer</button>
    {/if}

    {#if status === "scanning" || (scan && (status === "ready" || status === "confirming"))}
      <!-- signature: an amber playhead sweeps the panel while scanning -->
      <section
        class="{PANEL} relative overflow-hidden {status === 'scanning' ? SWEEP : ''}"
      >
        <div class="flex items-baseline justify-between border-b border-line-soft px-4 py-3.5">
          <span class={EYEBROW}>What was found</span>
          <span class="font-mono text-[11px] text-muted">
            {#if status === "scanning"}scanning{:else}{found} item{found === 1 ? "" : "s"}{/if}
          </span>
        </div>

        {#if scan}
          {#each [{ title: "Application", items: scan.app, hot: false }, { title: "Projects & databases", items: scan.projects, hot: true }] as group (group.title)}
            <div>
              <span class="{EYEBROW} block px-4 pt-3.5 pb-2">{group.title}</span>
              <ul class="pb-1.5">
                {#each group.items as it, i (i)}
                  <li
                    class="flex items-center gap-3 px-4 py-1.5 text-row hover:bg-surface-2"
                    class:opacity-45={!it.exists}
                  >
                    <span
                      class="size-1.5 flex-none rounded-xs {it.exists
                        ? group.hot
                          ? 'bg-danger shadow-glow-danger'
                          : 'bg-amber shadow-glow-amber'
                        : 'bg-line'}"
                    ></span>
                    <span class="w-40 flex-none max-sm:w-28">{it.label}</span>
                    <span class="min-w-0 flex-1 truncate font-mono text-path text-dim max-sm:hidden" title={it.path}>
                      {it.path}
                    </span>
                    <span class="flex-none font-mono text-tag uppercase {it.exists ? 'text-muted' : 'text-dim'}">
                      {it.exists ? "found" : "clean"}
                    </span>
                  </li>
                {/each}
              </ul>
            </div>
          {/each}
        {/if}
      </section>
    {/if}

    {#if scan && (status === "ready" || status === "confirming")}
      <label
        class="{PANEL} group flex cursor-pointer items-start gap-3.5 p-4 transition
               hover:border-line-hi has-checked:border-danger/45 has-checked:bg-danger-wash"
      >
        <input
          type="checkbox"
          bind:checked={removeProjects}
          class="relative mt-px h-5 w-[34px] flex-none cursor-pointer appearance-none rounded-full border border-line bg-surface-2 transition
                 checked:border-danger checked:bg-danger
                 after:absolute after:top-0.5 after:left-0.5 after:size-3.5 after:rounded-full after:bg-dim after:transition after:content-['']
                 checked:after:translate-x-3.5 checked:after:bg-white"
        />
        <span class="block text-row leading-normal">
          <b class="mb-0.5 block font-semibold transition group-has-checked:text-danger-soft">
            Also delete my projects
          </b>
          <span class="text-muted">
            Deletes every project, timeline and database, including PostgreSQL. This cannot be
            undone. Leave it off to keep your work.
          </span>
        </span>
      </label>

      {#if status === "confirming"}
        <div class="{PANEL} border-danger/40 bg-danger-wash p-4">
          <p class="mb-3.5 text-[13.5px]">
            {#if removeProjects}
              Resolve <b class="text-danger-soft">and all your projects</b> will be deleted. This is permanent.
            {:else}
              Resolve will be removed. Your projects stay where they are.
            {/if}
          </p>
          <div class="flex gap-2">
            <button class={DANGER} onclick={runUninstall}>Yes, uninstall</button>
            <button class={GHOST} onclick={() => (status = "ready")}>Cancel</button>
          </div>
        </div>
      {:else}
        <button class="{DANGER} w-full py-3.5 text-sm" disabled={!admin} onclick={() => (status = "confirming")}>
          Uninstall DaVinci Resolve
        </button>
      {/if}
    {/if}

    {#if status === "running" || status === "done"}
      <section
        bind:this={logEl}
        class="h-70 overflow-auto rounded-panel border border-line bg-console p-4 font-mono text-path leading-[1.75] wrap-anywhere"
      >
        {#each log as line, i (i)}
          <div class={line.startsWith(" ") ? "pl-1 text-dim" : "text-ink"}>{line}</div>
        {/each}
        {#if status === "running"}
          <div class="animate-blink text-amber motion-reduce:animate-none">▌</div>
        {/if}
      </section>
    {/if}

    {#if status === "done" && result}
      <div class="{PANEL} flex flex-col items-start gap-2 border-good/35 bg-good-wash p-4 text-row leading-relaxed">
        <b class="font-semibold text-good">Uninstall complete</b>
        <span class="text-muted">
          {result.removed} item{result.removed === 1 ? "" : "s"} removed. Restart Windows to clear
          anything that was locked while running.
        </span>
        {#if result.errors.length}
          <details class="w-full text-xs">
            <summary class="cursor-pointer text-amber">
              {result.errors.length} warning{result.errors.length === 1 ? "" : "s"}
            </summary>
            <ul class="mt-2 list-disc pl-4.5 text-dim">
              {#each result.errors as e, i (i)}
                <li class="font-mono text-[11px] wrap-anywhere">{e}</li>
              {/each}
            </ul>
          </details>
        {/if}
        <button class="{GHOST} px-3 py-1.5 text-xs" onclick={runScan}>Scan again</button>
      </div>
    {/if}

    <footer class="mt-1.5 flex justify-between border-t border-line-soft pt-3.5 text-[10.5px] text-faint">
      <span>Only the system drive is cleaned.</span>
      <span class="font-mono">v0.2.0</span>
    </footer>
  </div>
</main>
