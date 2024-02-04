setTimeout(
  (...arg: number[]) => {
    console.log("Hello Timeout", ...arg);
  },
  100,
  1,
  2,
  3
);

setTimeout((...arg: number[]) => {
  console.log("Timeout 2", /^bin$/g);
}, 10);

export default {
  copy() {
    console.log("1121");
  },
};
