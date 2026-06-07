import { describe, expect, test } from "bun:test"

import {
  decodeServerFrame,
  encodeClientFrame,
  encodeOutputEvent,
} from "../src/brokerProtocol"

describe("brokerProtocol", () => {
  test("round trips input bytes through base64 frames", () => {
    const encoded = encodeClientFrame({
      version: 1,
      type: "input",
      terminalId: "00000000-0000-0000-0000-000000000601",
      data: new Uint8Array([0, 159, 146, 150, 255]),
    })

    expect(encoded).toContain('"data_base64":"AJ+Slv8="')
  })

  test("decodes output event bytes from base64 frames", () => {
    const decoded = decodeServerFrame(
      encodeOutputEvent({
        terminalId: "00000000-0000-0000-0000-000000000602",
        seq: 3,
        data: new Uint8Array([104, 105]),
      }),
    )

    expect(decoded).toEqual({
      version: 1,
      type: "output",
      terminalId: "00000000-0000-0000-0000-000000000602",
      seq: 3,
      data: new Uint8Array([104, 105]),
    })
  })
})
