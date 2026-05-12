<script lang="ts">
	import { listen, type UnlistenFn } from '@tauri-apps/api/event';
	import { onDestroy, onMount } from 'svelte';
	import { type Node, type NodeProps } from '@xyflow/svelte';
	import type { LevelMeterNodeData } from '$lib/modules/pipeline/types';
	import Wrapper from '../node.svelte';

	type LevelMeterNodeType = Node<LevelMeterNodeData, 'levelMeter'>;
	let { id }: NodeProps<LevelMeterNodeType> = $props();

	let peakL = $state(0);
	let peakR = $state(0);
	let rmsL = $state(0);
	let rmsR = $state(0);

	// dB floor for the bar. Anything quieter than this maps to 0%.
	const DB_FLOOR = -60;

	interface MeterTick {
		nodeId: string;
		peakL: number;
		peakR: number;
		rmsL: number;
		rmsR: number;
	}

	let unlisten: UnlistenFn | undefined;
	onMount(async () => {
		unlisten = await listen<MeterTick>('audio://meter', (event) => {
			const p = event.payload;
			if (p.nodeId !== id) return;
			peakL = p.peakL;
			peakR = p.peakR;
			rmsL = p.rmsL;
			rmsR = p.rmsR;
		});
	});

	onDestroy(() => unlisten?.());

	function ampToPct(amp: number): number {
		if (amp <= 1e-6) return 0;
		const db = 20 * Math.log10(amp);
		return Math.max(0, Math.min(100, ((db - DB_FLOOR) / -DB_FLOOR) * 100));
	}

	function ampToDb(amp: number): string {
		if (amp <= 1e-6) return '−∞';
		return (20 * Math.log10(amp)).toFixed(1);
	}
</script>

<Wrapper label="Level Meter" accent="effect" hasInput hasOutput>
	<div class="flex w-50 flex-col gap-1.5">
		<div class="flex h-32 gap-1.5">
			{#each [{ p: peakL, r: rmsL, n: 'L' }, { p: peakR, r: rmsR, n: 'R' }] as ch (ch.n)}
				<div class="flex flex-1 flex-col items-center gap-1">
					<div class="relative w-full flex-1 overflow-hidden rounded bg-neutral-100">
						<div
							class="absolute right-0 bottom-0 left-0 transition-[height] duration-75"
							style="height: {ampToPct(ch.p)}%; background: linear-gradient(to top, #22c55e 0%, #22c55e 60%, #eab308 75%, #ef4444 92%);"
						></div>
						<div
							class="absolute right-0 left-0 h-px bg-white/90 mix-blend-overlay"
							style="bottom: {ampToPct(ch.r)}%;"
						></div>
					</div>
					<span class="text-[10px] text-neutral-1000">{ch.n}</span>
				</div>
			{/each}
		</div>
		<div class="flex justify-between font-mono text-[10px] text-neutral-900">
			<span>L: {ampToDb(peakL)} dB</span>
			<span>R: {ampToDb(peakR)} dB</span>
		</div>
		<span class="text-[9px] text-neutral-900">peak bar · white line = RMS</span>
	</div>
</Wrapper>
