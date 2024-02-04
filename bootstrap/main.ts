interface RuntimeData {
  asyncHandle: Record<number, () => void>
  count: number
}

export default async function bootstrap(entry: string) {
  var runtime: RuntimeData = {
    asyncHandle: [],
    count: 0,
  }
  //@ts-ignore
  globalThis.setTimeout = (fn: Function, delay: number, ...arg: any[]) => {
    runtime.asyncHandle[runtime.count] = () => {
      fn(...arg)
    }
    this.timer.send(runtime.count, delay)
    runtime.count++
  }
  globalThis.exec = (id: number) => {
    if (runtime.asyncHandle[id]) {
      let fn = runtime.asyncHandle[id]
      delete runtime.asyncHandle[id]
      fn()
    }
  }

  await import(entry)
}
