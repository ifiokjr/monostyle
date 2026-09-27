const query = `
{
  "id": ${id},
  "flags": ["a", "b"]
}
`;

function rows() {
  const keys = Object.keys({ a: 1 });



  return keys.map((k) => `${k}: ${query.length}`);
}

let id = 7;
