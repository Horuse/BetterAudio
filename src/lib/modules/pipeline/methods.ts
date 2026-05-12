import { LazyStore } from '@tauri-apps/plugin-store';
import type { NodeKind, Pipeline, PipelineNode } from './types';

const STORE_FILE = 'pipelines.json';
const KEY_PREFIX = 'pipeline:';
const store = new LazyStore(STORE_FILE);

// Legacy kinds → new kinds. Persistent because older pipelines were saved with
// {kind:'input'|'output'}; the runtime contract is now {kind:'microphone'|'speaker'}.
const LEGACY_KIND_MAP: Record<string, NodeKind> = {
	input: 'microphone',
	output: 'speaker'
};

function migrateNode(n: PipelineNode): PipelineNode {
	const remapped = LEGACY_KIND_MAP[n.kind as string];
	if (!remapped) return n;
	return { ...n, kind: remapped, data: { ...n.data } } as PipelineNode;
}

function migratePipeline(p: Pipeline): Pipeline {
	return { ...p, nodes: p.nodes.map(migrateNode) };
}

export const methods = {
	emptyPipeline(id: string, name: string): Pipeline {
		const now = Date.now();
		return { id, name, nodes: [], edges: [], createdAt: now, updatedAt: now };
	},

	async list(): Promise<Pipeline[]> {
		const entries = await store.entries<Pipeline>();
		return entries
			.filter(([k]) => k.startsWith(KEY_PREFIX))
			.map(([, v]) => migratePipeline(v))
			.sort((a, b) => b.updatedAt - a.updatedAt);
	},

	async get(id: string): Promise<Pipeline | null> {
		const v = await store.get<Pipeline>(KEY_PREFIX + id);
		return v ? migratePipeline(v) : null;
	},

	async save(p: Pipeline): Promise<void> {
		await store.set(KEY_PREFIX + p.id, p);
		await store.save();
	},

	async remove(id: string): Promise<void> {
		await store.delete(KEY_PREFIX + id);
		await store.save();
	}
};
