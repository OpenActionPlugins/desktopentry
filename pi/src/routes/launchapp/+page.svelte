<script lang="ts">
	import { actionSettings, eventTarget } from "@openaction/svelte-pi";

	interface AppInfo {
		path: string;
		name: string;
		exec: string;
		icon: string | null;
		terminal: boolean;
	}

	let apps: AppInfo[] = $state([]);
	let selectedApp = $derived($actionSettings.app ?? "");

	eventTarget.addEventListener("sendToPropertyInspector", (event: any) => {
		if (event.detail.payload.apps) apps = event.detail.payload.apps;
	});

	function handleSelect(event: Event) {
		const value = (event.target as HTMLSelectElement).value;
		$actionSettings = { ...$actionSettings, app: value };
	}

	function handleArgs(event: Event) {
		const value = (event.target as HTMLInputElement).value;
		$actionSettings = { ...$actionSettings, args: value };
	}
</script>

<div class="space-y-4">
	<div class="select-wrapper">
		<select value={selectedApp} onchange={handleSelect} class="w-full">
			<option value="">Select an app</option>
			<option disabled>──────────</option>
			{#each apps as app}
				<option value={app.path}>{app.name}</option>
			{/each}
		</select>
	</div>

	{#if selectedApp}
		{@const app = apps.find((a) => a.path == selectedApp)}
		{#if app}
			<div
				class="space-y-1.5 rounded-lg border border-neutral-600 bg-neutral-700 p-3 text-sm"
			>
				<div class="flex">
					<span class="w-12 font-medium text-neutral-400">Path</span>
					<span class="break-all text-neutral-100">{app.path}</span>
				</div>
				<div class="flex">
					<span class="w-12 font-medium text-neutral-400">Name</span>
					<span class="text-neutral-100">{app.name}</span>
				</div>
				<div class="flex">
					<span class="w-12 font-medium text-neutral-400">Icon</span>
					<span class="text-neutral-100">{app.icon ?? "N/A"}</span>
				</div>
				<div class="flex">
					<span class="w-12 font-medium text-neutral-400">Exec</span>
					<span class="break-all text-neutral-100">{app.exec}</span>
				</div>
				{#if app.terminal}
					<div class="flex">
						<span class="w-12 font-medium text-neutral-400">Term</span>
						<span class="text-neutral-100">Yes</span>
					</div>
				{/if}
			</div>
		{/if}

		<div>
			<label for="args" class="mb-1 block text-sm font-medium text-neutral-400">
				Custom arguments
			</label>
			<input
				id="args"
				type="text"
				value={$actionSettings.args}
				oninput={handleArgs}
				placeholder="e.g. --fullscreen --debug"
				class="w-full rounded-lg border border-neutral-600 bg-neutral-700 px-3 py-2 text-sm text-neutral-100 placeholder-neutral-500 focus:border-neutral-600 focus:ring-1 focus:ring-neutral-600 focus:outline-none"
			/>
		</div>
	{/if}
</div>
