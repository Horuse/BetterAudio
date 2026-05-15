<script lang="ts">
	import { openUrl } from '@tauri-apps/plugin-opener';
	import { errorStore } from '../stores.svelte';
	import { formatAppInfo, getCachedAppInfo } from '$lib/modules/app_info';
	import { Copy } from '$lib/components/icons';

	const REPO = 'Horuse/Splitwave';

	let current = $derived(errorStore.current);

	function buildDetails(): string {
		if (!current) return '';
		const info = getCachedAppInfo();
		const lines = [
			'## Diagnostic info',
			`- **App:** Splitwave ${info?.appVersion ?? '?'} (tauri ${info?.tauriVersion ?? '?'})`,
			`- **OS:** ${info ? formatAppInfo(info) : navigator.userAgent}`,
			`- **Source:** ${current.source}${current.thread ? ` (thread: ${current.thread})` : ''}`,
			`- **Time:** ${new Date(current.at).toISOString()}`,
			'',
			'**Message:**',
			'```',
			current.message,
			'```'
		];
		if (current.stack) {
			lines.push('', '**Stack:**', '```', current.stack, '```');
		}
		lines.push(
			'',
			'## Steps to reproduce',
			'<!-- What were you doing when this happened? Be specific. -->',
			'1. ',
			'2. ',
			'3. ',
			'',
			'## Expected behavior',
			'<!-- What did you expect to happen? -->',
			'',
			'## Additional context',
			'<!-- Screenshots, logs, related issues, anything else useful. -->',
			''
		);
		return lines.join('\n');
	}

	async function copyDetails() {
		try {
			await navigator.clipboard.writeText(buildDetails());
		} catch {
			// no-op
		}
	}

	async function reportOnGitHub() {
		if (!current) return;
		const title = `[crash] ${current.message.split('\n')[0].slice(0, 80)}`;
		const body = buildDetails();
		const url = `https://github.com/${REPO}/issues/new?template=crash.yml&title=${encodeURIComponent(title)}&body=${encodeURIComponent(body)}`;
		try {
			await openUrl(url);
		} catch {
			// no-op
		}
	}

	function dismiss() {
		errorStore.dismiss();
	}

	function sourceLabel(s: typeof current extends null ? never : NonNullable<typeof current>['source']): string {
		switch (s) {
			case 'rustPanic':
				return 'Rust panic';
			case 'jsError':
				return 'JS error';
			case 'unhandledRejection':
				return 'Unhandled promise rejection';
		}
	}
</script>

{#if current}
	<div
		class="fixed inset-0 z-100 flex items-center justify-center bg-theme/1 backdrop-blur-sm  p-6"
		role="dialog"
		aria-modal="true"
		aria-labelledby="error-title"
	>
		<div class="flex max-h-[80vh] w-full max-w-2xl flex-col overflow-hidden rounded-2xl border border-neutral-400 bg-neutral-100 shadow-xl">
			<header class="flex items-center justify-between border-b border-neutral-300 px-5 py-3">
				<h2 id="error-title" class="text-md font-semibold text-red-500">
					Something went wrong
				</h2>
				<span class="rounded bg-neutral-200 px-2 py-0.5 font-mono text-[10px] text-neutral-1000">
					{sourceLabel(current.source)}
				</span>
			</header>

			<div class="flex-1 overflow-y-auto px-5 py-4">
				<p class="mb-2 text-xs text-neutral-1000">
					Please report this so we can fix it.
				</p>
				<pre class="max-h-40 overflow-auto rounded bg-neutral-200 p-2 font-mono text-[11px] leading-tight whitespace-pre-wrap break-words text-neutral-1100">{current.message}</pre>
				{#if current.stack}
					<details class="mt-3">
						<summary class="cursor-pointer text-[11px] text-neutral-900">Stack trace</summary>
						<pre class="mt-2 max-h-60 overflow-auto rounded bg-neutral-200 p-2 font-mono text-[10px] leading-tight whitespace-pre-wrap break-words text-neutral-1000">{current.stack}</pre>
					</details>
				{/if}
			</div>

			<footer class="flex items-center justify-end gap-2 border-t border-neutral-300 bg-neutral-200 px-5 py-3">
				<button
					type="button"
					class="button-main primary rounded-lg"
					onclick={dismiss}
				>
					Dismiss
				</button>
				<button
					type="button"
					class="button-main primary gap-3 rounded-lg"
					onclick={copyDetails}
				>
					<Copy class="size-4" />
					Copy details
				</button>
				<button
					type="button"
					class="button-main red rounded-lg"
					onclick={reportOnGitHub}
				>
					Report on GitHub
				</button>
			</footer>
		</div>
	</div>
{/if}
