export function toSnakeCase(input: unknown): unknown {
	if (typeof input !== 'object' || input === null) {
		return input;
	}

	if (Array.isArray(input)) {
		return input.map(toSnakeCase);
	}

	const result: unknown = {};
	for (const key in input) {
		if (Object.hasOwn(input, key)) {
			const snakeCaseKey = key
				.replace(/([A-Z])/g, '_$1')
				.toLowerCase()
				.replace(/^_/, '');
			result[snakeCaseKey] = toSnakeCase(input[key]);
		}
	}

	return result;
}

export const encodeTo = (value: string, from: BufferEncoding, to: BufferEncoding): string => {
	return Buffer.from(from === 'hex' && value.startsWith('0x') ? value.slice(2) : value, from).toString(to);
}

export const sleep = (ms: number) => {
	return new Promise(resolve => {
		setTimeout(resolve, ms)
	})
};