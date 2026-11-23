// Minimal ANSI SGR → HTML for terminal run output. Other escape codes are dropped.
const BASE = ['#1a1e27', '#f0524f', '#2fbf71', '#e5a50a', '#5a92ff', '#a585ff', '#4cc3d9', '#d7dce6'];
const BRIGHT = ['#697285', '#ff7a77', '#5bd893', '#f5c451', '#8fb4ff', '#c2adff', '#7fd9e9', '#ffffff'];

function esc(s: string) {
	return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}

function c256(n: number): string {
	if (n < 8) return BASE[n];
	if (n < 16) return BRIGHT[n - 8];
	if (n >= 232) {
		const v = 8 + (n - 232) * 10;
		return `rgb(${v},${v},${v})`;
	}
	n -= 16;
	const r = Math.floor(n / 36), g = Math.floor((n % 36) / 6), b = n % 6;
	const f = (x: number) => (x ? 55 + x * 40 : 0);
	return `rgb(${f(r)},${f(g)},${f(b)})`;
}

export function ansiToHtml(input: string): string {
	let fg: string | null = null;
	let bold = false;
	let out = '';
	let open = false;
	const text = input.replace(/\r\n/g, '\n').replace(/\r(?!\n)/g, '\n');
	const re = /\x1b\[([0-9;]*)([A-Za-z])|\x1b\][^\x07]*\x07|\x1b[()][A-Z0-9]|\x1b[=>]/g;
	let last = 0;
	let m: RegExpExecArray | null;
	const flush = (s: string) => {
		if (!s) return;
		out += esc(s);
	};
	const restyle = () => {
		if (open) out += '</span>';
		open = Boolean(fg || bold);
		if (open) out += `<span style="${fg ? `color:${fg};` : ''}${bold ? 'font-weight:600;' : ''}">`;
	};
	while ((m = re.exec(text))) {
		flush(text.slice(last, m.index));
		last = re.lastIndex;
		if (m[2] !== 'm') continue;
		const codes = (m[1] || '0').split(';').map(Number);
		for (let i = 0; i < codes.length; i++) {
			const c = codes[i];
			if (c === 0) (fg = null), (bold = false);
			else if (c === 1) bold = true;
			else if (c === 22) bold = false;
			else if (c >= 30 && c <= 37) fg = BASE[c - 30];
			else if (c >= 90 && c <= 97) fg = BRIGHT[c - 90];
			else if (c === 39) fg = null;
			else if (c === 38 && codes[i + 1] === 5) (fg = c256(codes[i + 2])), (i += 2);
			else if (c === 38 && codes[i + 1] === 2) (fg = `rgb(${codes[i + 2]},${codes[i + 3]},${codes[i + 4]})`), (i += 4);
		}
		restyle();
	}
	flush(text.slice(last));
	if (open) out += '</span>';
	return out;
}
