// A small HCL / Terraform tokenizer for CodeMirror's StreamLanguage.
import type { StreamParser } from '@codemirror/language';

const BLOCKS = new Set([
	'resource', 'data', 'variable', 'output', 'locals', 'module', 'provider', 'terraform', 'backend', 'required_providers',
	'moved', 'import', 'removed', 'check', 'dynamic', 'lifecycle', 'provisioner', 'connection', 'content', 'run', 'mock_provider'
]);
const KEYWORDS = new Set(['for', 'in', 'if', 'else', 'endif', 'endfor', 'for_each', 'count', 'depends_on', 'each', 'self']);
const ROOTS = new Set(['var', 'local', 'module', 'data', 'path', 'terraform', 'count', 'each', 'self']);

interface State {
	inString: boolean;
	interpDepth: number;
	heredoc: string | null;
	blockComment: boolean;
}

export const hcl: StreamParser<State> = {
	name: 'hcl',
	startState: () => ({ inString: false, interpDepth: 0, heredoc: null, blockComment: false }),
	token(stream, state) {
		if (state.blockComment) {
			if (stream.skipTo('*/')) {
				stream.match('*/');
				state.blockComment = false;
			} else stream.skipToEnd();
			return 'comment';
		}
		if (state.heredoc) {
			if (stream.sol() && stream.match(new RegExp(`^\\s*${state.heredoc}\\s*$`))) {
				state.heredoc = null;
				return 'string';
			}
			stream.skipToEnd();
			return 'string';
		}
		if (state.inString && state.interpDepth === 0) {
			if (stream.match('${') || stream.match('%{')) {
				state.interpDepth = 1;
				return 'string.special';
			}
			while (!stream.eol()) {
				const ch = stream.next();
				if (ch === '\\') stream.next();
				else if (ch === '"') {
					state.inString = false;
					return 'string';
				} else if ((ch === '$' || ch === '%') && stream.peek() === '{') {
					stream.backUp(1);
					return 'string';
				}
			}
			return 'string';
		}
		if (stream.eatSpace()) return null;
		if (stream.match('#') || stream.match('//')) {
			stream.skipToEnd();
			return 'comment';
		}
		if (stream.match('/*')) {
			state.blockComment = true;
			return 'comment';
		}
		const heredoc = stream.match(/^<<-?([A-Za-z_]+)/) as RegExpMatchArray | null;
		if (heredoc) {
			state.heredoc = heredoc[1];
			return 'string';
		}
		if (state.interpDepth > 0) {
			if (stream.peek() === '{') state.interpDepth++;
			if (stream.peek() === '}') {
				state.interpDepth--;
				stream.next();
				return state.interpDepth === 0 ? 'string.special' : 'punctuation';
			}
		}
		if (stream.peek() === '"') {
			stream.next();
			state.inString = true;
			return 'string';
		}
		if (stream.match(/^-?\d+(\.\d+)?([eE][+-]?\d+)?/)) return 'number';
		if (stream.match(/^(true|false|null)\b/)) return 'bool';
		const word = stream.match(/^[A-Za-z_][\w-]*/) as RegExpMatchArray | null;
		if (word) {
			const w = word[0];
			if (stream.sol() || /^\s*$/.test(stream.string.slice(0, stream.start))) {
				if (BLOCKS.has(w)) return 'definitionKeyword';
			}
			if (KEYWORDS.has(w)) return 'keyword';
			if (ROOTS.has(w) && stream.peek() === '.') return 'variableName';
			if (/^\s*=(?!=)/.test(stream.string.slice(stream.pos))) return 'propertyName';
			if (stream.peek() === '(') return 'variableName.function';
			return 'variableName';
		}
		if (stream.match(/^(==|!=|>=|<=|&&|\|\||=>|[=!<>?:+\-*/%])/)) return 'operator';
		stream.next();
		return 'punctuation';
	},
	languageData: { commentTokens: { line: '#', block: { open: '/*', close: '*/' } } }
};
