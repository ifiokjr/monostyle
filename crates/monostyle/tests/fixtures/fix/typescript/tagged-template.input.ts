function sql(strings: TemplateStringsArray, ...values: unknown[]): string {
  return strings.join("?");
}

const table = "users";
const query = sql`
  SELECT {kept} FROM ${table}
  WHERE id = ${42}
`;



export { query };
