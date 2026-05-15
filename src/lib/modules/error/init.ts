import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { errorStore } from './stores.svelte';

const PANIC_EVENT = 'error://panic';

interface PanicPayload {
	message: string;
	backtrace: string;
	thread: string;
	version: string;
}

let installed = false;
let unlistenPanic: UnlistenFn | undefined;

export async function installErrorHandlers(): Promise<void> {
	if (installed) return;
	installed = true;

	unlistenPanic = await listen<PanicPayload>(PANIC_EVENT, (e) => {
		errorStore.report({
			source: 'rustPanic',
			message: e.payload.message,
			stack: e.payload.backtrace,
			thread: e.payload.thread,
			at: Date.now()
		});
	});

	window.addEventListener('error', (e) => {
		errorStore.report({
			source: 'jsError',
			message: e.message || String(e.error),
			stack: e.error instanceof Error ? e.error.stack : undefined,
			at: Date.now()
		});
	});

	window.addEventListener('unhandledrejection', (e) => {
		const reason = e.reason;
		const message =
			reason instanceof Error
				? reason.message
				: typeof reason === 'string'
					? reason
					: JSON.stringify(reason);
		errorStore.report({
			source: 'unhandledRejection',
			message,
			stack: reason instanceof Error ? reason.stack : undefined,
			at: Date.now()
		});
	});
}

export function uninstallErrorHandlers(): void {
	unlistenPanic?.();
	unlistenPanic = undefined;
	installed = false;
}
