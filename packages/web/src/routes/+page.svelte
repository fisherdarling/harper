<script>
	import Graph from '$lib/Graph.svelte';
	import Editor from '$lib/Editor.svelte';
	import Section from '$lib/Section.svelte';
	import GutterCenter from '$lib/GutterCenter.svelte';
	import Logo from '$lib/Logo.svelte';
	import GitHubLogo from '$lib/GitHubLogo.svelte';
	import { DarkMode } from 'flowbite-svelte';

	let width = window.innerWidth;

	window.addEventListener('resize', () => {
		width = window.innerWidth;
	});

	$: mobile = width < 640;
</script>

<div class="h-16" />
<div class="absolute top-8 right-8"><DarkMode /></div>

<GutterCenter>
	<div class="w-full flex flex-col items-center">
		<Logo width="200px" />
	</div>
	<h1 class="text-5xl font-bold text-center dark:text-white">Hi. I’m Harper.</h1>
	<h2 class="text-3xl text-center dark:text-white">The Grammar Checker for Developers</h2>

	<div class="flex flex-row justify-evenly mt-5">
		<a
			href="https://github.com/elijah-potter/harper"
			class="flex flex-row items-center [&>*]:m-2 dark:text-white"
			><GitHubLogo width="40px" height="40px" />GitHub</a
		>
		<a href="https://elijahpotter.dev" class="flex flex-row items-center [&>*]:m-2 dark:text-white"
			><img
				width="40"
				height="40"
				class="hover:scale-105 transition-all"
				src="/icons/profile.svg"
				alt="Author"
			/>Author</a
		>
	</div>

	<Section noChild>
		<span slot="title"> What is it? </span>
		<span slot="subtitle">
			Harper is an English grammar checker designed to be <em>just right</em>. You can think of it
			as an open-source alternative to Grammarly. I created it after years of dealing with the
			shortcomings of the competition.
		</span>
	</Section>

	{#if width > 1080}
		<div class="w-full h-[600px] overflow-hidden">
			<Editor />
		</div>
	{/if}

	<Section>
		<span slot="title">Private</span>
		<span slot="subtitle"
			>Harper is completely private, in every sense of the word. <br /><br />
			Since Harper runs on-device, your data doesn't go anywhere you don't want it to. <br />
			<br />That means you have 100% certainty we don't violate your copyright by training large
			language models.
		</span>

		<img src="/images/camera.webp" class="rounded" alt="Graffiti of a camera." />
	</Section>

	<Section swapped={!mobile}>
		<span slot="title">Native Everywhere</span>
		<span slot="subtitle"
			>Harper is both available as a <a
				class="underline"
				href="https://github.com/elijah-potter/harper/tree/master/harper-ls">language server</a
			>, and through WebAssembly, so you can get fantastic grammar checking anywhere you work.
			<br /><br /> That said, we take extra care to make sure the Neovim,
			<a class="underline" href="https://github.com/Stef16Robbe/harper_zed">Zed</a>
			and
			<a class="underline" href="/obsidian">Obsidian</a> integration is amazing.
		</span>

		<img
			src="/images/example_nvim.webp"
			class="dark:invert rounded"
			alt="A screenshot of Neovim with Harper suggestions."
		/></Section
	>

	<Section>
		<span slot="title">Wicked Fast</span>
		<span slot="subtitle"
			>Since Harper runs on <strong>your</strong> devices, its able to serve up suggestions in under
			10 milliseconds.
			<br />
			<br />
			No network request, no massive language models, no fuss.</span
		>
		<Graph /></Section
	>

	<Section noChild swapped={!mobile}>
		<span slot="title">Open Source</span>
		<span slot="subtitle"
			>Harper is completely open source under the Apache-2.0 license. <br /><br /> Come pay us a
			visit on <a class="underline" href="https://github.com/elijah-potter/harper">GitHub.</a>
		</span>
	</Section>
</GutterCenter>
