// A small Rego tokenizer for CodeMirror's StreamLanguage.
import type { StreamParser } from '@codemirror/language';

const KEYWORDS = new Set(['package', 'import', 'as', 'default', 'else', 'not', 'some', 'every', 'with', 'if', 'contains', 'in']);
const RULES = new Set(['deny', 'warn', 'violation', 'allow']);

export const rego: StreamParser<{ raw: boolean }> = {
	name: 'rego',
	startState: () => ({ raw: false }),
	token(stream, state) {
		if (state.raw) {
			if (stream.skipTo('`')) {
				stream.next();
				state.raw = false;
			} else stream.skipToEnd();
			return 'string';
		}
		if (stream.eatSpace()) return null;
		if (stream.match('#')) {
			stream.skipToEnd();
			return 'comment';
		}
		if (stream.match('`')) {
			state.raw = true;
			return 'string';
		}
		if (stream.match(/^"(?:[^"\\]|\\.)*"/)) return 'string';
		if (stream.match(/^-?\d+(\.\d+)?/)) return 'number';
		if (stream.match(/^(true|false|null)\b/)) return 'bool';
		const w = stream.match(/^[A-Za-z_][\w]*/) as RegExpMatchArray | null;
		if (w) {
			if (KEYWORDS.has(w[0])) return 'keyword';
			if (RULES.has(w[0])) return 'definitionKeyword';
			if (w[0] === 'input' || w[0] === 'data') return 'variableName.special';
			if (stream.peek() === '(') return 'variableName.function';
			return 'variableName';
		}
		if (stream.match(/^(:=|==|!=|>=|<=|[=<>+\-*/|&])/)) return 'operator';
		stream.next();
		return 'punctuation';
	},
	languageData: { commentTokens: { line: '#' } }
};
