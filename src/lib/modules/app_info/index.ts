import { arch, hostname, platform, version } from '@tauri-apps/plugin-os';
import { getVersion, getTauriVersion } from '@tauri-apps/api/app';

export interface AppInfo {
	appVersion: string;
	tauriVersion: string;
	platform: string;
	osVersion: string;
	arch: string;
	hostname: string | null;
}

let cached: AppInfo | null = null;

export async function loadAppInfo(): Promise<AppInfo> {
	if (cached) return cached;
	const [appVersion, tauriVersion, plat, ver, ar, host] = await Promise.all([
		getVersion(),
		getTauriVersion(),
		platform(),
		version(),
		arch(),
		hostname()
	]);
	cached = {
		appVersion,
		tauriVersion,
		platform: plat,
		osVersion: ver,
		arch: ar,
		hostname: host
	};
	return cached;
}

export function getCachedAppInfo(): AppInfo | null {
	return cached;
}

export function formatAppInfo(info: AppInfo): string {
	const osLabel = info.platform === 'macos' ? 'macOS' : info.platform;
	return `${osLabel} ${info.osVersion} (${info.arch})`;
}
