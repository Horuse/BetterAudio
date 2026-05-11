import { methods } from './methods';
import { pipelineStore } from './stores.svelte';
import type * as types from './types';

const Pipeline = {
	methods,
	stores: { pipeline: pipelineStore }
};

export type { types };
export default Pipeline;
