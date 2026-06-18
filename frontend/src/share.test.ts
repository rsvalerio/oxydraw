import { describe, expect, it } from "vitest";

// Imported from the editor-free module where the parser lives (share.ts re-exports it but pulls in
// the editor bundle, which can't load in a plain node test environment).
import { parseShareFragment } from "./fragment";

describe("parseShareFragment", () => {
  it("parses a valid #json=<id>,<key> fragment", () => {
    expect(parseShareFragment("#json=abc123,KEYZZZ")).toEqual({ id: "abc123", key: "KEYZZZ" });
  });

  it("rejects a fragment with the wrong prefix", () => {
    expect(parseShareFragment("#room=abc123,KEYZZZ")).toBeNull();
    expect(parseShareFragment("#abc123,KEYZZZ")).toBeNull();
    expect(parseShareFragment("")).toBeNull();
  });

  it("rejects a fragment missing the key", () => {
    expect(parseShareFragment("#json=abc123")).toBeNull();
    expect(parseShareFragment("#json=abc123,")).toBeNull();
  });

  it("rejects a fragment missing the id", () => {
    expect(parseShareFragment("#json=,KEYZZZ")).toBeNull();
  });

  it("rejects an empty fragment body", () => {
    expect(parseShareFragment("#json=")).toBeNull();
    expect(parseShareFragment("#json=,")).toBeNull();
  });
});
