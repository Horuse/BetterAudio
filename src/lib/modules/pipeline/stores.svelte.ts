import { methods } from './methods';
import type { NodeKind, Pipeline } from './types';

export type EditorActions = {
	addNode: (kind: NodeKind) => void;
	getSnapshot: () => Pipeline | null;
};

class PipelineStore {
	pipelines = $state<Pipeline[]>([]);
	editorActions: EditorActions | null = null;

	async refresh(): Promise<void> {
		this.pipelines = await methods.list();
	}

	async save(p: Pipeline): Promise<void> {
		await methods.save(p);
		await this.refresh();
	}

	async remove(id: string): Promise<void> {
		await methods.remove(id);
		await this.refresh();
	}
}

export const pipelineStore = new PipelineStore();
