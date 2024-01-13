interface RuntimeData {
  asyncHandle: Record<number, () => void>
  count: number
}

var runtimeData: RuntimeData = {
  asyncHandle: [],
  count: 0,
}

export default async function bootstrap(entry: string) {
  //@ts-ignore
  globalThis.setTimeout = (fn: Function, delay: number, ...arg: any[]) => {
    runtimeData.asyncHandle[runtimeData.count] = () => {
      fn(...arg)
    }
    this.timer.send(runtimeData.count, delay)
    runtimeData.count++
  }
  globalThis.exec = (id: number) => {
    if (runtimeData.asyncHandle[id]) {
      let fn = runtimeData.asyncHandle[id]
      delete runtimeData.asyncHandle[id]
      fn()
    }
  }

  await import(entry)
}
