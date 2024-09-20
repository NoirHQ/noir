export function toSnakeCase(input: unknown): unknown {
  if (typeof input !== "object" || input === null) {
    return input;
  }

  if (Array.isArray(input)) {
    return input.map(toSnakeCase);
  }

  const result: unknown = {};
  for (const key in input) {
    if (input.hasOwnProperty(key)) {
      const snakeCaseKey = key
        .replace(/([A-Z])/g, "_$1")
        .toLowerCase()
        .replace(/^_/, "");
      result[snakeCaseKey] = toSnakeCase(input[key]);
    }
  }

  return result;
}
