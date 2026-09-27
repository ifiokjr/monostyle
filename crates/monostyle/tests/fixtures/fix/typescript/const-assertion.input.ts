const palette = {
  red: "#f00",
  nested: { light: "#f99" },
} as const;

function pick(key: keyof typeof palette): string {
  return palette[key];
}
