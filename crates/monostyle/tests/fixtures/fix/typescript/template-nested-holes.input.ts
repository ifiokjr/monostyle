const object = `${ {a: 1, b: 2} } done`;
const nested = `${["x", "y"].map((v) => `${v}!`).join(", ")}`;
const quoted = `he said "${"hi"}" and {left}`;



console.log(object, nested, quoted);
