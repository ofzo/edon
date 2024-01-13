import "https://deno.land/std@0.182.0/examples/welcome.ts";
import "https://deno.land/std@0.182.0/examples/welcome.ts";

const res = await import("./test1.ts");
console.log("res", res);
try {
  const text = await res.json();
  console.log("🚀 text", text);
  test.copy();
} catch (err) {
  console.error("err => ", err);
}
