const quantifier = /\{2,4\}/;
const classed = /[{\\["']/;
const template = /{{\s*(\w+)\s*}}/g;

function strip(text: string): string {
  return text.replace(template, "($1)");
}
