import { Buffer } from "node:buffer"

export type ClientFrame = Readonly<{
  version: 1
  type: "input"
  terminalId: string
  data: Uint8Array
}>

export type ServerFrame = Readonly<{
  version: 1
  type: "output"
  terminalId: string
  seq: number
  data: Uint8Array
}>

export type OutputEventInput = Readonly<{
  terminalId: string
  seq: number
  data: Uint8Array
}>

export function encodeClientFrame(frame: ClientFrame): string {
  switch (frame.type) {
    case "input":
      return `${JSON.stringify({
        version: frame.version,
        type: frame.type,
        terminal_id: frame.terminalId,
        data_base64: encodeBase64(frame.data),
      })}\n`
  }
}

export function encodeOutputEvent(input: OutputEventInput): string {
  return `${JSON.stringify({
    version: 1,
    type: "output",
    terminal_id: input.terminalId,
    seq: input.seq,
    data_base64: encodeBase64(input.data),
  })}\n`
}

export function decodeServerFrame(raw: string): ServerFrame {
  const parsed: unknown = JSON.parse(raw)
  if (typeof parsed !== "object" || parsed === null) {
    throw new Error("protocol frame must be an object")
  }
  const version = readNumber(parsed, "version")
  const type = readString(parsed, "type")
  if (version !== 1 || type !== "output") {
    throw new Error("unsupported protocol frame")
  }
  return {
    version,
    type,
    terminalId: requireString(parsed, "terminal_id"),
    seq: requireNumber(parsed, "seq"),
    data: decodeBase64(requireString(parsed, "data_base64")),
  }
}

function encodeBase64(data: Uint8Array): string {
  return Buffer.from(data).toString("base64")
}

function decodeBase64(data: string): Uint8Array {
  return new Uint8Array(Buffer.from(data, "base64"))
}

function requireString(record: object, key: string): string {
  const value = readString(record, key)
  if (value === undefined) {
    throw new Error(`missing protocol string field ${key}`)
  }
  return value
}

function requireNumber(record: object, key: string): number {
  const value = readNumber(record, key)
  if (value === undefined) {
    throw new Error(`missing protocol number field ${key}`)
  }
  return value
}

function readString(record: object, key: string): string | undefined {
  const value: unknown = Reflect.get(record, key)
  return typeof value === "string" ? value : undefined
}

function readNumber(record: object, key: string): number | undefined {
  const value: unknown = Reflect.get(record, key)
  return typeof value === "number" ? value : undefined
}
