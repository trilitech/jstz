const KEY = "counter";

const handler = () => {
  let a = [];
  a[1].foo();
  let counter = Kv.get(KEY);
  console.log(`Counter: ${counter}`);
  if (counter === null) {
    counter = 0;
  } else {
    counter++;
  }
  Kv.set(KEY, counter);
  return new Response();
};

export default handler;
