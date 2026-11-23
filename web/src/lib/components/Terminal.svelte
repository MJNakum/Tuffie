<script lang="ts">
	// Integrated terminal. The line editor runs in the browser; the server
	// only receives complete command lines and runs terraform/tofu in a PTY.
	import { onDestroy, onMount } from 'svelte';
	import { Terminal } from '@xterm/xterm';
	import { FitAddon } from '@xterm/addon-fit';
	import '@xterm/xterm/css/xterm.css';
	import { wsUrl } from '#lib/api';

	let {
		workspaceId,
		workspaceName,
		tool = 'terraform',
		onCommandStarted,
		onCommandFinished
	}: {
		workspaceId: string;
		workspaceName: string;
		tool?: string;
		onCommandStarted?: (runId: string) => void;
		onCommandFinished?: (code: number | null) => void;
	} = $props();

	let host: HTMLDivElement;
	let term: Terminal;
	let fit: FitAddon;
	let sock: WebSocket | null = null;
	let ro: ResizeObserver;
	let disposed = false;

	let line = '';
	let cursor = 0;
	let running = false;
	let history: string[] = [];
	let hIndex = -1;
	const histKey = () => `tuffie-history-${workspaceId}`;

	const SUBCOMMANDS = [
		'init', 'validate', 'plan', 'apply', 'destroy', 'console', 'fmt', 'force-unlock', 'get', 'graph', 'import',
		'login', 'logout', 'metadata', 'modules', 'output', 'providers', 'refresh', 'show', 'state', 'taint', 'test',
		'untaint', 'version', 'workspace'
	];
	const STATE_SUB = ['list', 'mv', 'pull', 'push', 'replace-provider', 'rm', 'show'];

	const C = { dim: '\x1b[38;2;105;114;133m', blue: '\x1b[38;2;90;146;255m', red: '\x1b[38;2;240;82;79m', reset: '\x1b[0m', bold: '\x1b[1m' };
	const promptText = () => `${C.blue}${workspaceName}${C.reset} ${C.dim}❯${C.reset} `;

	function cssVar(name: string) {
		return getComputedStyle(document.documentElement).getPropertyValue(name).trim();
	}

	function redraw() {
		term.write('\r\x1b[2K' + promptText() + line);
		const back = line.length - cursor;
		if (back > 0) term.write(`\x1b[${back}D`);
	}

	function prompt() {
		line = '';
		cursor = 0;
		hIndex = -1;
		term.write('\r\n' + promptText());
	}

	function send(msg: object) {
		if (sock?.readyState === WebSocket.OPEN) sock.send(JSON.stringify(msg));
	}

	function sendSize() {
		send({ type: 'resize', cols: term.cols, rows: term.rows });
	}

	function help() {
		term.write(
			[
				'',
				`${C.bold}Tuffie terminal${C.reset}`,
				`Runs ${C.blue}terraform${C.reset} and ${C.blue}tofu${C.reset} commands in this workspace's folder. ${C.blue}tf${C.reset} is an alias for the workspace tool (${tool}).`,
				'Workspace variables and environment variables are set for you.',
				'Commands are saved in the run history. Commands that change state take the workspace lock.',
				'',
				`${C.dim}Built-in: help, clear, history.  Keys: ↑/↓ history, Tab complete, Ctrl+C cancel, Ctrl+L clear.${C.reset}`,
				`${C.dim}There is no shell: pipes, redirects and other programs are not available.${C.reset}`
			].join('\r\n')
		);
	}

	function exec(cmd: string) {
		const trimmed = cmd.trim();
		if (!trimmed) return prompt();
		if (history[history.length - 1] !== trimmed) {
			history.push(trimmed);
			history = history.slice(-200);
			try {
				localStorage.setItem(histKey(), JSON.stringify(history));
			} catch {
				/* storage unavailable */
			}
		}
		if (trimmed === 'clear') {
			term.clear();
			line = '';
			cursor = 0;
			term.write('\r\x1b[2K' + promptText());
			return;
		}
		if (trimmed === 'help') {
			help();
			return prompt();
		}
		if (trimmed === 'history') {
			term.write('\r\n' + history.map((h, i) => `${C.dim}${String(i + 1).padStart(4)}${C.reset}  ${h}`).join('\r\n'));
			return prompt();
		}
		if (sock?.readyState !== WebSocket.OPEN) {
			term.write(`\r\n${C.red}Not connected. Reconnecting…${C.reset}`);
			return prompt();
		}
		running = true;
		term.write('\r\n');
		send({ type: 'exec', command: trimmed });
	}

	function complete() {
		const words = line.slice(0, cursor).split(/\s+/);
		let options: string[] = [];
		if (words.length === 1) options = ['terraform', 'tofu', 'tf', 'help', 'clear', 'history'];
		else if (words.length === 2 && ['terraform', 'tofu', 'tf'].includes(words[0])) options = SUBCOMMANDS;
		else if (words.length === 3 && words[1] === 'state') options = STATE_SUB;
		const cur = words[words.length - 1];
		const matches = options.filter((o) => o.startsWith(cur));
		if (matches.length === 1) {
			const add = matches[0].slice(cur.length) + ' ';
			line = line.slice(0, cursor) + add + line.slice(cursor);
			cursor += add.length;
			redraw();
		} else if (matches.length > 1) {
			term.write('\r\n' + C.dim + matches.join('  ') + C.reset);
			term.write('\r\n');
			redraw();
		}
	}

	function onData(data: string) {
		if (running) {
			if (data === '\x03') send({ type: 'interrupt' });
			else send({ type: 'input', data });
			return;
		}
		// Paste of several characters (or lines).
		if (data.length > 1 && !data.startsWith('\x1b')) {
			const parts = data.replace(/\r\n/g, '\r').split('\r');
			for (let i = 0; i < parts.length; i++) {
				line = line.slice(0, cursor) + parts[i] + line.slice(cursor);
				cursor += parts[i].length;
				if (i < parts.length - 1) {
					redraw();
					exec(line);
					return;
				}
			}
			redraw();
			return;
		}
		switch (data) {
			case '\r':
				exec(line);
				return;
			case '\x7f': // backspace
				if (cursor > 0) {
					line = line.slice(0, cursor - 1) + line.slice(cursor);
					cursor--;
					redraw();
				}
				return;
			case '\x1b[3~': // delete
				if (cursor < line.length) {
					line = line.slice(0, cursor) + line.slice(cursor + 1);
					redraw();
				}
				return;
			case '\x03': // Ctrl+C
				term.write('^C');
				prompt();
				return;
			case '\x0c': // Ctrl+L
				term.clear();
				redraw();
				return;
			case '\x15': // Ctrl+U
				line = line.slice(cursor);
				cursor = 0;
				redraw();
				return;
			case '\x17': {
				// Ctrl+W
				const left = line.slice(0, cursor).replace(/\S+\s*$/, '');
				line = left + line.slice(cursor);
				cursor = left.length;
				redraw();
				return;
			}
			case '\x01':
			case '\x1b[H':
				cursor = 0;
				redraw();
				return;
			case '\x05':
			case '\x1b[F':
				cursor = line.length;
				redraw();
				return;
			case '\x1b[D':
				if (cursor > 0) {
					cursor--;
					term.write(data);
				}
				return;
			case '\x1b[C':
				if (cursor < line.length) {
					cursor++;
					term.write(data);
				}
				return;
			case '\x1b[A':
				if (history.length) {
					hIndex = hIndex < 0 ? history.length - 1 : Math.max(0, hIndex - 1);
					line = history[hIndex];
					cursor = line.length;
					redraw();
				}
				return;
			case '\x1b[B':
				if (hIndex >= 0) {
					hIndex++;
					line = hIndex >= history.length ? '' : history[hIndex];
					if (hIndex >= history.length) hIndex = -1;
					cursor = line.length;
					redraw();
				}
				return;
			case '\t':
				complete();
				return;
		}
		if (data >= ' ' && !data.startsWith('\x1b')) {
			line = line.slice(0, cursor) + data + line.slice(cursor);
			cursor += data.length;
			redraw();
		}
	}

	function connect() {
		if (disposed) return;
		sock = new WebSocket(wsUrl(`/workspaces/${workspaceId}/terminal`));
		sock.binaryType = 'arraybuffer';
		sock.onopen = () => sendSize();
		sock.onmessage = (m) => {
			if (typeof m.data !== 'string') {
				term.write(new Uint8Array(m.data));
				return;
			}
			const msg = JSON.parse(m.data);
			if (msg.type === 'started') onCommandStarted?.(msg.run_id);
			else if (msg.type === 'error') term.write(`${C.red}${msg.message}${C.reset}`);
			else if (msg.type === 'exit') {
				running = false;
				if (msg.code !== null && msg.code !== 0) term.write(`\r\n${C.dim}exit code ${msg.code}${C.reset}`);
				onCommandFinished?.(msg.code);
				prompt();
			}
		};
		sock.onclose = () => {
			if (disposed) return;
			if (running) {
				running = false;
				term.write(`\r\n${C.red}Connection lost.${C.reset}`);
				prompt();
			}
			setTimeout(connect, 1500);
		};
	}

	onMount(() => {
		try {
			history = JSON.parse(localStorage.getItem(histKey()) ?? '[]');
		} catch {
			history = [];
		}
		term = new Terminal({
			fontFamily: "'JetBrains Mono Variable', ui-monospace, Menlo, monospace",
			fontSize: 12.5,
			lineHeight: 1.35,
			cursorBlink: true,
			cursorStyle: 'bar',
			scrollback: 5000,
			allowProposedApi: false,
			theme: {
				background: cssVar('--term-bg') || '#07080b',
				foreground: '#d7dce6',
				cursor: '#5a92ff',
				cursorAccent: '#07080b',
				selectionBackground: 'rgba(61,126,255,0.35)',
				black: '#1a1e27',
				brightBlack: '#697285',
				red: '#f0524f',
				brightRed: '#ff7a77',
				green: '#2fbf71',
				brightGreen: '#5bd893',
				yellow: '#e5a50a',
				brightYellow: '#f5c451',
				blue: '#5a92ff',
				brightBlue: '#8fb4ff',
				magenta: '#a585ff',
				brightMagenta: '#c2adff',
				cyan: '#4cc3d9',
				brightCyan: '#7fd9e9',
				white: '#d7dce6',
				brightWhite: '#ffffff'
			}
		});
		fit = new FitAddon();
		term.loadAddon(fit);
		term.open(host);
		fit.fit();
		term.write(`${C.dim}Tuffie terminal · ${tool} · type ${C.reset}help${C.dim} for help${C.reset}`);
		prompt();
		term.onData(onData);
		term.onResize(() => sendSize());
		ro = new ResizeObserver(() => {
			try {
				fit.fit();
			} catch {
				/* hidden */
			}
		});
		ro.observe(host);
		connect();
	});

	onDestroy(() => {
		disposed = true;
		ro?.disconnect();
		sock?.close();
		term?.dispose();
	});

	export function focus() {
		term?.focus();
	}

	/** Type a command into the terminal and run it. */
	export function run(cmd: string) {
		if (running) return;
		line = cmd;
		cursor = cmd.length;
		redraw();
		exec(cmd);
		term.focus();
	}
</script>

<div bind:this={host} class="h-full w-full bg-[var(--term-bg)] pl-3 pt-2"></div>
