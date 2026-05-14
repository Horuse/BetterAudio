<script lang="ts">
	import { listen, type UnlistenFn } from '@tauri-apps/api/event';
	import { onDestroy, onMount } from 'svelte';

	let { nodeId }: { nodeId: string } = $props();

	let targetL = 0;
	let targetR = 0;

	let displayL = $state(-Infinity);
	let displayR = $state(-Infinity);

	const DB_FLOOR = -60;
	const PEAK_FALL_DB_PER_SEC = 30;

	interface MeterTick {
		nodeId: string;
		peakL: number;
		peakR: number;
		rmsL: number;
		rmsR: number;
	}

	function ampToDb(amp: number): number {
		return amp <= 1e-6 ? -Infinity : 20 * Math.log10(amp);
	}

	function dbToPct(db: number): number {
		if (!isFinite(db)) return 0;
		return Math.max(0, Math.min(100, ((db - DB_FLOOR) / -DB_FLOOR) * 100));
	}

	let rafId: number | undefined;
	let unlisten: UnlistenFn | undefined;
	let lastFrame = 0;

	function tick(now: number) {
		const dt = lastFrame ? Math.min((now - lastFrame) / 1000, 0.1) : 0;
		lastFrame = now;
		const tL = ampToDb(targetL);
		const tR = ampToDb(targetR);
		displayL = tL > displayL ? tL : Math.max(tL, displayL - PEAK_FALL_DB_PER_SEC * dt);
		displayR = tR > displayR ? tR : Math.max(tR, displayR - PEAK_FALL_DB_PER_SEC * dt);
		rafId = requestAnimationFrame(tick);
	}

	onMount(async () => {
		unlisten = await listen<MeterTick>('audio://meter', (event) => {
			const p = event.payload;
			if (p.nodeId !== nodeId) return;
			targetL = p.peakL;
			targetR = p.peakR;
		});
		rafId = requestAnimationFrame(tick);
	});

	onDestroy(() => {
		unlisten?.();
		if (rafId) cancelAnimationFrame(rafId);
	});
</script>

<div class="flex w-full flex-col gap-[2px]" aria-label="Live input level">
	{#each [displayL, displayR] as db, i (i)}
		<div class="relative h-1.5 overflow-hidden rounded-sm bg-neutral-200">
			<div
				class="absolute inset-y-0 left-0 bg-gradient-to-r from-green-500 via-yellow-500 to-red-500"
				style="width: {dbToPct(db)}%;"
			></div>
		</div>
	{/each}
</div>
