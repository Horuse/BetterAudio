<script lang="ts">
	import { listen, type UnlistenFn } from '@tauri-apps/api/event';
	import { onDestroy, onMount } from 'svelte';

	let { nodeId }: { nodeId: string } = $props();

	let targetL = 0;
	let targetR = 0;

	let displayL = $state(-Infinity);
	let displayR = $state(-Infinity);
	let holdL = $state(-Infinity);
	let holdR = $state(-Infinity);
	let holdTimeL = 0;
	let holdTimeR = 0;

	const DB_FLOOR = -60;
	const PEAK_FALL_DB_PER_SEC = 30;
	const HOLD_SEC = 1.5;
	const HOLD_FALL_DB_PER_SEC = 20;

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

	function updateChannel(
		target: number,
		display: number,
		hold: number,
		holdTime: number,
		dt: number
	): [number, number, number] {
		const t = ampToDb(target);
		const newDisplay = t > display ? t : Math.max(t, display - PEAK_FALL_DB_PER_SEC * dt);
		let newHold = hold;
		let newHoldTime = holdTime + dt;
		if (t >= hold) {
			newHold = t;
			newHoldTime = 0;
		} else if (newHoldTime > HOLD_SEC) {
			newHold = Math.max(t, hold - HOLD_FALL_DB_PER_SEC * dt);
		}
		return [newDisplay, newHold, newHoldTime];
	}

	function tick(now: number) {
		const dt = lastFrame ? Math.min((now - lastFrame) / 1000, 0.1) : 0;
		lastFrame = now;
		[displayL, holdL, holdTimeL] = updateChannel(targetL, displayL, holdL, holdTimeL, dt);
		[displayR, holdR, holdTimeR] = updateChannel(targetR, displayR, holdR, holdTimeR, dt);
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
	{#each [[displayL, holdL], [displayR, holdR]] as [db, hold], i (i)}
		<div class="relative h-1.5 overflow-hidden rounded-sm bg-neutral-300">
			<div
				class="absolute inset-0"
				style="
					background: linear-gradient(to right, #22c55e 0%, #22c55e 70%, #eab308 70%, #eab308 90%, #f97316 90%, #f97316 95%, #ef4444 95%, #ef4444 100%);
					clip-path: inset(0 {100 - dbToPct(db)}% 0 0);
				"
			></div>
			{#if isFinite(hold) && dbToPct(hold) > 0}
				<div class="absolute inset-y-0 w-px bg-white" style="left: {dbToPct(hold)}%;"></div>
			{/if}
		</div>
	{/each}
</div>
