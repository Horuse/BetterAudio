<script lang="ts">
	import {
		Background,
		Controls,
		SvelteFlow,
		useSvelteFlow,
		type Edge as XyEdge,
		type Node as XyNode
	} from '@xyflow/svelte';
	import { createId } from '@paralleldrive/cuid2';
	import { listen, type UnlistenFn } from '@tauri-apps/api/event';
	import { onDestroy, onMount, untrack } from 'svelte';
	import type { NodeKind, Pipeline } from '$lib/modules/pipeline/types';
	import { pipelineStore } from '$lib/modules/pipeline/stores.svelte';
	import { methods as pipelineMethods } from '$lib/modules/pipeline/methods';
	import { audioStore } from '$lib/modules/audio/stores.svelte';
	import { methods as audioMethods } from '$lib/modules/audio/methods';
	import {
		DND_MIME,
		defaultDataFor,
		fromXyEdges,
		fromXyNodes,
		nodeTypes,
		registry,
		toXyEdges,
		toXyNodes
	} from '../utils';
	import Sidebar from './sidebar.svelte';

	let { pipeline }: { pipeline: Pipeline } = $props();

	const flow = useSvelteFlow();

	let nodes = $state.raw<XyNode[]>(untrack(() => toXyNodes(pipeline.nodes)));
	let edges = $state.raw<XyEdge[]>(untrack(() => toXyEdges(pipeline.edges)));

	type ContextMenu =
		| { kind: 'node'; nodeId: string; x: number; y: number }
		| { kind: 'edge'; edgeId: string; x: number; y: number };

	let contextMenu = $state<ContextMenu | null>(null);

	function onDragOver(event: DragEvent) {
		event.preventDefault();
		if (event.dataTransfer) event.dataTransfer.dropEffect = 'move';
	}

	function onDrop(event: DragEvent) {
		event.preventDefault();
		const kind = event.dataTransfer?.getData(DND_MIME) as NodeKind | undefined;
		if (!kind || !(kind in registry)) return;
		const position = flow.screenToFlowPosition({ x: event.clientX, y: event.clientY });
		addNode(kind, position);
	}

	function addNode(kind: NodeKind, position?: { x: number; y: number }) {
		const fallback = { x: 100 + nodes.length * 40, y: 100 + nodes.length * 40 };
		nodes = [
			...nodes,
			{
				id: createId(),
				type: kind,
				position: position ?? fallback,
				data: defaultDataFor(kind)
			}
		];
	}

	function deleteNode(nodeId: string) {
		nodes = nodes.filter((n) => n.id !== nodeId);
		edges = edges.filter((e) => e.source !== nodeId && e.target !== nodeId);
	}

	function deleteEdge(edgeId: string) {
		edges = edges.filter((e) => e.id !== edgeId);
	}

	function onNodeContextMenu({ node, event }: { node: XyNode; event: MouseEvent }) {
		event.preventDefault();
		contextMenu = { kind: 'node', nodeId: node.id, x: event.clientX, y: event.clientY };
	}

	function onEdgeContextMenu({ edge, event }: { edge: XyEdge; event: MouseEvent }) {
		event.preventDefault();
		contextMenu = { kind: 'edge', edgeId: edge.id, x: event.clientX, y: event.clientY };
	}

	function closeContextMenu() {
		contextMenu = null;
	}

	function onMenuDelete() {
		if (!contextMenu) return;
		if (contextMenu.kind === 'node') deleteNode(contextMenu.nodeId);
		else deleteEdge(contextMenu.edgeId);
		contextMenu = null;
	}

	function getSnapshot(): Pipeline {
		return {
			id: pipeline.id,
			name: pipeline.name,
			createdAt: pipeline.createdAt,
			nodes: fromXyNodes(nodes),
			edges: fromXyEdges(edges),
			updatedAt: Date.now()
		};
	}

	function revertToSnapshot(p: Pipeline) {
		nodes = toXyNodes(p.nodes);
		edges = toXyEdges(p.edges);
	}

	// Capture on the debounced save tick when enough time has passed --
	// piggy-backs on real edits, no blind interval.
	const SNAPSHOT_MIN_SPACING_MS = 30_000;
	let lastSnapshotSig = '';
	let lastSnapshotAt = 0;
	function snapshotSignature(p: Pipeline): string {
		return JSON.stringify({ nodes: p.nodes, edges: p.edges });
	}

	// Undo/redo history. Cursor points at the current state inside `history`;
	// undo decrements, redo increments. New edits truncate forward history.
	const MAX_HISTORY = 50;
	let history = $state.raw<Pipeline[]>([untrack(() => getSnapshot())]);
	let cursor = $state(0);
	let canUndo = $derived(cursor > 0);
	let canRedo = $derived(cursor < history.length - 1);

	function captureIfChanged(snap: Pipeline) {
		const sig = snapshotSignature(snap);
		const currentSig = snapshotSignature(history[cursor]);
		if (sig === currentSig) return;
		const next = history.slice(0, cursor + 1);
		next.push(snap);
		const trimmed = next.length > MAX_HISTORY ? next.slice(next.length - MAX_HISTORY) : next;
		history = trimmed;
		cursor = trimmed.length - 1;
	}

	function commit(snap: Pipeline) {
		pipelineStore.save(snap);
		const sig = snapshotSignature(snap);
		const now = Date.now();
		if (sig !== lastSnapshotSig && now - lastSnapshotAt >= SNAPSHOT_MIN_SPACING_MS) {
			pipelineMethods.addSnapshot(snap).then(() => {
				lastSnapshotSig = sig;
				lastSnapshotAt = now;
			});
		}
		captureIfChanged(snap);
	}

	function flushPendingCommit() {
		if (saveTimer === undefined) return;
		clearTimeout(saveTimer);
		saveTimer = undefined;
		untrack(() => commit(getSnapshot()));
	}

	function undo() {
		flushPendingCommit();
		if (cursor === 0) return;
		cursor -= 1;
		revertToSnapshot(history[cursor]);
	}

	function redo() {
		flushPendingCommit();
		if (cursor >= history.length - 1) return;
		cursor += 1;
		revertToSnapshot(history[cursor]);
	}

	pipelineStore.editorActions = {
		addNode,
		getSnapshot,
		revertToSnapshot,
		undo,
		redo,
		canUndo: () => canUndo,
		canRedo: () => canRedo
	};

	let saveTimer: ReturnType<typeof setTimeout> | undefined;
	$effect(() => {
		nodes;
		edges;
		clearTimeout(saveTimer);
		saveTimer = setTimeout(() => {
			saveTimer = undefined;
			untrack(() => commit(getSnapshot()));
		}, 500);
		return () => clearTimeout(saveTimer);
	});

	onMount(() => {
		lastSnapshotSig = snapshotSignature(getSnapshot());
		// First edit always snapshots -- pretend the previous capture was
		// just past the spacing window.
		lastSnapshotAt = Date.now() - SNAPSHOT_MIN_SPACING_MS - 1;
	});

	// Auto-restart on routing changes only — effect params flow through
	// update_effect live, no restart needed.
	function routingSignature(): string {
		return JSON.stringify({
			nodes: nodes.map((n) => ({
				id: n.id,
				type: n.type,
				deviceId: (n.data as Record<string, unknown>).deviceId ?? null,
				bundleId: (n.data as Record<string, unknown>).bundleId ?? null,
				filePath: (n.data as Record<string, unknown>).filePath ?? null,
				excludeCurrentApp: (n.data as Record<string, unknown>).excludeCurrentApp ?? null
			})),
			edges: edges.map((e) => ({
				id: e.id,
				source: e.source,
				target: e.target,
				targetHandle: e.targetHandle ?? null
			}))
		});
	}

	let lastRoutingSig = untrack(routingSignature);
	let restartTimer: ReturnType<typeof setTimeout> | undefined;
	$effect(() => {
		const sig = routingSignature();
		if (sig === lastRoutingSig) return;
		lastRoutingSig = sig;
		if (!audioStore.isRunning) return;
		clearTimeout(restartTimer);
		restartTimer = setTimeout(() => {
			untrack(async () => {
				try {
					await audioStore.restartPipeline({
						nodes: fromXyNodes(nodes),
						edges: fromXyEdges(edges)
					});
				} catch (e) {
					audioStore.lastError = e instanceof Error ? e.message : String(e);
				}
			});
		}, 400);
		return () => clearTimeout(restartTimer);
	});

	// The Tauri WebView (and historic browser behavior) treats Backspace outside
	// of editable fields as "navigate back". XYFlow also defaults `deleteKey` to
	// Backspace, so we explicitly accept Delete too and swallow the default
	// navigation in case the press lands outside the flow.
	function onWindowKeyDown(e: KeyboardEvent) {
		if (e.key !== 'Backspace' && e.key !== 'Delete') return;
		const t = e.target as HTMLElement | null;
		const tag = t?.tagName?.toLowerCase();
		if (tag === 'input' || tag === 'textarea' || tag === 'select' || t?.isContentEditable) {
			return;
		}
		e.preventDefault();
	}

	// Auto-stop the pipeline when every AudioFile source has reached EOF and
	// no live capture (mic / system / app) is running. Mixed graphs keep
	// running so live recording survives the file finishing.
	const LIVE_INPUT_TYPES = ['microphone', 'systemAudio', 'appAudio'];
	let audioFileDone = $state<Record<string, boolean>>({});

	$effect(() => {
		if (!audioStore.isRunning) {
			audioFileDone = {};
		}
	});

	interface AudioFileProgress {
		nodeId: string;
		stopped: boolean;
	}

	let unlistenAudioFile: UnlistenFn | undefined;
	onMount(() => {
		window.addEventListener('keydown', onWindowKeyDown, { capture: true });
		listen<AudioFileProgress>('audio://audio_file_progress', (e) => {
			const { nodeId, stopped } = e.payload;
			if (!audioStore.isRunning) return;
			audioFileDone[nodeId] = stopped;
			if (!stopped) return;
			const hasLive = nodes.some((n) => LIVE_INPUT_TYPES.includes(n.type ?? ''));
			if (hasLive) return;
			const audioFiles = nodes.filter((n) => n.type === 'audioFile');
			if (audioFiles.length === 0) return;
			if (audioFiles.every((n) => audioFileDone[n.id])) {
				audioMethods.stopPipeline().catch(() => {});
			}
		}).then((fn) => {
			unlistenAudioFile = fn;
		});
		return () => window.removeEventListener('keydown', onWindowKeyDown, { capture: true });
	});

	onDestroy(() => {
		clearTimeout(saveTimer);
		clearTimeout(restartTimer);
		unlistenAudioFile?.();
		if (pipelineStore.editorActions?.getSnapshot === getSnapshot) {
			pipelineStore.editorActions = null;
		}
	});
</script>

<svelte:window onclick={closeContextMenu} />

<div class="flex h-full w-full">
	<div
		class="flex-1"
		role="region"
		aria-label="Flow editor"
		ondragover={onDragOver}
		ondrop={onDrop}
	>
		<SvelteFlow
			class="!bg-background"
			bind:nodes
			bind:edges
			{nodeTypes}
			defaultEdgeOptions={{ animated: true }}
			deleteKey={['Delete', 'Backspace']}
			onnodecontextmenu={onNodeContextMenu}
			onedgecontextmenu={onEdgeContextMenu}
			onpaneclick={closeContextMenu}
			fitView
		>
			<Background patternClass="fill-neutral-200"/>
			<Controls />
		</SvelteFlow>
	</div>
	<Sidebar />
</div>

{#if contextMenu}
	<div
		class="fixed z-50 min-w-40 overflow-hidden rounded-lg border border-neutral-400 bg-neutral-100 shadow-lg"
		style="top: {contextMenu.y}px; left: {contextMenu.x}px"
		role="menu"
		onclick={(e) => e.stopPropagation()}
		oncontextmenu={(e) => e.preventDefault()}
		onkeydown={(e) => e.key === 'Escape' && closeContextMenu()}
		tabindex="-1"
	>
		<div class="px-3 py-1.5 text-[10px] tracking-wider text-neutral-900 uppercase">
			{contextMenu.kind === 'node' ? 'Node' : 'Edge'}
		</div>
		<button
			class="flex w-full items-center gap-2 px-3 py-1.5 text-left text-sm text-red-700 hover:bg-red-500/15 dark:text-red-300"
			onclick={onMenuDelete}
			role="menuitem"
		>
			Delete
			<span class="ml-auto text-[10px] text-neutral-900">⌫</span>
		</button>
	</div>
{/if}
