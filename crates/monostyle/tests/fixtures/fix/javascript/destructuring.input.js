const { a, b: { c }, ...rest } = source;
const [first, second, ...others] = list;

function swap({ x, y }) {
  return { x: y, y: x };
}



let source = { a: 1, b: { c: 2 }, d: 3 };
let list = [1, 2, 3, 4];
