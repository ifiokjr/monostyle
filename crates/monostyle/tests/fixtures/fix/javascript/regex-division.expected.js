const pattern = /ab+c/gi;
const ratio = total / count;
const normalized = value / 2 / scale;
const flagged = check(input, /[{}]/);

let total = 4;
let count = 2;
let value = 8;
let scale = 4;

function check(s, r) { return r.test(s); }
