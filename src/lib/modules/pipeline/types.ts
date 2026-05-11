export type NodeKind = 'input' | 'output';

export interface InputNodeData extends Record<string, unknown> {
	deviceId: string | null;
}

export interface OutputNodeData extends Record<string, unknown> {
	deviceId: string | null;
}

export type NodeDataMap = {
	input: InputNodeData;
	output: OutputNodeData;
};

export type AnyNodeData = NodeDataMap[NodeKind];

export interface PipelineNode<K extends NodeKind = NodeKind> {
	id: string;
	kind: K;
	data: NodeDataMap[K];
	position: { x: number; y: number };
}

export interface PipelineEdge {
	id: string;
	source: string;
	target: string;
}

export interface Pipeline {
	id: string;
	name: string;
	nodes: PipelineNode[];
	edges: PipelineEdge[];
	createdAt: number;
	updatedAt: number;
}
