const query = `
{
  "id": ${id},
  "name": "${name}"
}
`;

function rows(): string[] {
  const parts = ["a", "b"];



  return parts.map((p) => `${p}: ${query.length}`);
}

let id = 7;
let name = "ada";
