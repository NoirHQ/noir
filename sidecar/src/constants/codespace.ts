export const convertToCodespace = (codespace: number): string => {
	switch (codespace) {
		case 0: return 'sdk';
		case 1: return 'wasm';
		default: return 'unknown';
	}
}