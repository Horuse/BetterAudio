<script lang="ts">
	import { updaterStore } from '../stores.svelte';
	import { installUpdate, skipVersion } from '../methods';

	let s = $derived(updaterStore.state);

	function progressPct(): number {
		if (s.phase !== 'downloading' || !s.total || s.total === 0) return 0;
		return Math.min(100, Math.round((s.downloaded / s.total) * 100));
	}

	function dismiss() {
		updaterStore.state = { phase: 'idle' };
	}

	function onSkip() {
		if (s.phase !== 'available') return;
		skipVersion(s.update.version);
	}
</script>

{#if s.phase === 'available' || s.phase === 'downloading' || s.phase === 'installing' || s.phase === 'error'}
	<div
		class="fixed inset-0 z-100 flex items-center justify-center bg-theme/1 backdrop-blur-sm p-6"
		role="dialog"
		aria-modal="true"
		aria-labelledby="updater-title"
	>
		<div class="flex max-h-[80vh] w-full max-w-2xl flex-col overflow-hidden rounded-2xl border border-neutral-400 bg-neutral-100 shadow-xl">
			<header class="flex items-center justify-between border-b border-neutral-300 px-5 py-3">
				<h2 id="updater-title" class="text-md font-semibold text-emerald-700">
					{#if s.phase === 'available'}
						Update available
					{:else if s.phase === 'downloading'}
						Downloading update
					{:else if s.phase === 'installing'}
						Installing update
					{:else}
						Update failed
					{/if}
				</h2>
				{#if s.phase === 'available' || s.phase === 'downloading'}
					<span class="rounded bg-neutral-200 px-2 py-0.5 font-mono text-[10px] text-neutral-1000">
						v{s.update.version}
					</span>
				{/if}
			</header>

			<div class="flex-1 overflow-y-auto px-5 py-4">
				{#if s.phase === 'available'}
					<p class="mb-2 text-xs text-neutral-1000">
						A new version is ready to install. Your work will be saved before restarting.
					</p>
					{#if s.update.body}
						<pre class="max-h-60 overflow-auto rounded bg-neutral-200 p-3 font-mono text-[11px] leading-tight whitespace-pre-wrap break-words text-neutral-1100">{s.update.body}</pre>
					{/if}
				{:else if s.phase === 'downloading'}
					<div class="flex items-center gap-3 text-xs text-neutral-1000">
						<div class="h-2 flex-1 overflow-hidden rounded bg-neutral-300">
							<div class="h-full bg-emerald-500 transition-all" style="width: {progressPct()}%;"></div>
						</div>
						<span class="font-mono tabular-nums">{progressPct()}%</span>
					</div>
				{:else if s.phase === 'installing'}
					<p class="text-xs text-neutral-1000">Finalizing. The app will restart in a moment.</p>
				{:else if s.phase === 'error'}
					<pre class="max-h-40 overflow-auto rounded bg-neutral-200 p-2 font-mono text-[11px] leading-tight whitespace-pre-wrap break-words text-red-700">{s.message}</pre>
				{/if}
			</div>

			<footer class="flex items-center justify-end gap-2 border-t border-neutral-300 bg-neutral-200 px-5 py-3">
				{#if s.phase === 'available'}
					<button type="button" class="button-main primary rounded-lg" onclick={onSkip}>
						Skip this version
					</button>
					<button type="button" class="button-main primary rounded-lg" onclick={dismiss}>
						Later
					</button>
					<button
						type="button"
						class="button-main green rounded-lg"
						onclick={() => installUpdate()}
					>
						Install &amp; restart
					</button>
				{:else if s.phase === 'error'}
					<button type="button" class="button-main primary rounded-lg" onclick={dismiss}>
						Dismiss
					</button>
				{/if}
			</footer>
		</div>
	</div>
{/if}
