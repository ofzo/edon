setTimeout(
  (...arg: number[]) => {
    console.log("Hello Timeout", ...arg);
  },
  1000,
  1,
  2,
  3,
);
console.log("Welcome Timeout!");
setTimeout((...arg: number[]) => {
  console.log("Timeout 2", ...arg, /^bin$/g);
}, 100);

export default {
  copy() {
    console.log("1121");
  },
};

export function* x() {}
