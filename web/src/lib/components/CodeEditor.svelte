<script lang="ts">
	import { onDestroy, onMount } from 'svelte';
	import { EditorState, Compartment } from '@codemirror/state';
	import { EditorView, keymap, lineNumbers, highlightActiveLine, highlightActiveLineGutter, drawSelection } from '@codemirror/view';
	import { defaultKeymap, history, historyKeymap, indentWithTab } from '@codemirror/commands';
	import { bracketMatching, foldGutter, indentOnInput, StreamLanguage, syntaxHighlighting, HighlightStyle } from '@codemirror/language';
	import { searchKeymap, highlightSelectionMatches } from '@codemirror/search';
	import { closeBrackets, closeBracketsKeymap } from '@codemirror/autocomplete';
	import { tags as t } from '@lezer/highlight';
	import { hcl } from '#lib/hcl';
	import { rego } from '#lib/rego';

	let {
		value,
		path,
		onChange,
		onSave
	}: { value: string; path: string; onChange?: (v: string) => void; onSave?: () => void } = $props();

	let host: HTMLDivElement;
	let view: EditorView | undefined;
	const lang = new Compartment();

	function langFor(p: string) {
		if (/\.(tf|tfvars|hcl|tftest\.hcl)$/.test(p)) return StreamLanguage.define(hcl);
		if (p.endsWith('.rego')) return StreamLanguage.define(rego);
		return [];
	}

	const highlight = HighlightStyle.define([
		{ tag: t.comment, color: 'var(--fg-subtle)', fontStyle: 'italic' },
		{ tag: t.keyword, color: 'var(--violet)' },
		{ tag: [t.definitionKeyword, t.typeName], color: 'var(--primary-text)', fontWeight: '600' },
		{ tag: t.string, color: 'light-dark(#0a5bb5, #9ecbff)' },
		{ tag: [t.number, t.bool, t.null], color: 'light-dark(#b35900, #f5b370)' },
		{ tag: t.propertyName, color: 'var(--fg)' },
		{ tag: t.variableName, color: 'light-dark(#7a3fd1, #c2adff)' },
		{ tag: t.operator, color: 'var(--fg-muted)' },
		{ tag: t.special(t.string), color: 'light-dark(#138a4b, #5bd893)' },
		{ tag: t.punctuation, color: 'var(--fg-subtle)' }
	]);

	const theme = EditorView.theme({
		'&': { height: '100%', fontSize: '12.5px', backgroundColor: 'var(--surface)', color: 'var(--fg)' },
		'.cm-scroller': { fontFamily: "'JetBrains Mono Variable', ui-monospace, monospace", lineHeight: '1.6', fontVariantLigatures: 'none' },
		'.cm-content': { caretColor: 'var(--primary)', padding: '8px 0' },
		'.cm-gutters': { backgroundColor: 'var(--surface)', color: 'var(--fg-subtle)', border: 'none', paddingRight: '4px' },
		'.cm-activeLine': { backgroundColor: 'color-mix(in srgb, var(--surface-2), transparent 30%)' },
		'.cm-activeLineGutter': { backgroundColor: 'transparent', color: 'var(--fg-muted)' },
		'&.cm-focused .cm-selectionBackground, .cm-selectionBackground': { backgroundColor: 'var(--primary-soft) !important' },
		'.cm-selectionMatch': { backgroundColor: 'color-mix(in srgb, var(--primary-soft), transparent 40%)' },
		'&.cm-focused': { outline: 'none' },
		'.cm-cursor': { borderLeftColor: 'var(--primary)', borderLeftWidth: '2px' },
		'.cm-panels': { backgroundColor: 'var(--surface-2)', color: 'var(--fg)' },
		'.cm-foldGutter span': { color: 'var(--fg-subtle)' }
	});

	onMount(() => {
		view = new EditorView({
			parent: host,
			state: EditorState.create({
				doc: value,
				extensions: [
					lineNumbers(),
					foldGutter(),
					highlightActiveLine(),
					highlightActiveLineGutter(),
					drawSelection(),
					history(),
					indentOnInput(),
					bracketMatching(),
					closeBrackets(),
					highlightSelectionMatches(),
					EditorState.tabSize.of(2),
					keymap.of([
						{ key: 'Mod-s', preventDefault: true, run: () => (onSave?.(), true) },
						...closeBracketsKeymap,
						...defaultKeymap,
						...historyKeymap,
						...searchKeymap,
						indentWithTab
					]),
					lang.of(langFor(path)),
					syntaxHighlighting(highlight),
					theme,
					EditorView.updateListener.of((u) => {
						if (u.docChanged) onChange?.(u.state.doc.toString());
					})
				]
			})
		});
	});

	// Replace content when another file opens.
	// svelte-ignore state_referenced_locally
	let lastPath = path;
	$effect(() => {
		const p = path;
		const v = value;
		if (view && p !== lastPath) {
			lastPath = p;
			view.dispatch({
				changes: { from: 0, to: view.state.doc.length, insert: v },
				effects: lang.reconfigure(langFor(p))
			});
		}
	});

	onDestroy(() => view?.destroy());
</script>

<div bind:this={host} class="h-full min-h-0 overflow-hidden"></div>
