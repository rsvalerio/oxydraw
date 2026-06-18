import { describe, expect, it } from "vitest";

import { parseBroadcastPayload, parseRoomFragment } from "./protocol";

describe("parseRoomFragment", () => {
  it("parses a valid #room=<roomId>,<key> fragment", () => {
    expect(parseRoomFragment("#room=room42,KEYZZZ")).toEqual({ roomId: "room42", key: "KEYZZZ" });
  });

  it("rejects a fragment with the wrong prefix", () => {
    expect(parseRoomFragment("#json=room42,KEYZZZ")).toBeNull();
    expect(parseRoomFragment("#room42,KEYZZZ")).toBeNull();
    expect(parseRoomFragment("")).toBeNull();
  });

  it("rejects a fragment missing the key", () => {
    expect(parseRoomFragment("#room=room42")).toBeNull();
    expect(parseRoomFragment("#room=room42,")).toBeNull();
  });

  it("rejects a fragment missing the roomId", () => {
    expect(parseRoomFragment("#room=,KEYZZZ")).toBeNull();
  });

  it("rejects an empty fragment body", () => {
    expect(parseRoomFragment("#room=")).toBeNull();
    expect(parseRoomFragment("#room=,")).toBeNull();
  });
});

describe("parseBroadcastPayload", () => {
  const pointer = { x: 1, y: 2, tool: "pointer" as const };
  const validMouse = {
    type: "MOUSE_LOCATION",
    payload: {
      socketId: "sid-1",
      pointer,
      button: "down",
      selectedElementIds: { e1: true },
      username: "Ada",
    },
  };

  it("accepts a well-formed SCENE_INIT / SCENE_UPDATE frame", () => {
    expect(parseBroadcastPayload({ type: "SCENE_INIT", payload: { elements: [] } })).toEqual({
      type: "SCENE_INIT",
      payload: { elements: [] },
    });
    expect(parseBroadcastPayload({ type: "SCENE_UPDATE", payload: { elements: [] } })).toEqual({
      type: "SCENE_UPDATE",
      payload: { elements: [] },
    });
  });

  it("accepts a well-formed MOUSE_LOCATION frame", () => {
    expect(parseBroadcastPayload(validMouse)).toEqual(validMouse);
  });

  it("rejects non-object / null input", () => {
    expect(parseBroadcastPayload(null)).toBeNull();
    expect(parseBroadcastPayload("MOUSE_LOCATION")).toBeNull();
    expect(parseBroadcastPayload(42)).toBeNull();
  });

  it("rejects an unknown or missing type", () => {
    expect(parseBroadcastPayload({ type: "EVIL", payload: {} })).toBeNull();
    expect(parseBroadcastPayload({ payload: { elements: [] } })).toBeNull();
  });

  it("rejects a missing or malformed payload", () => {
    expect(parseBroadcastPayload({ type: "SCENE_INIT" })).toBeNull();
    expect(parseBroadcastPayload({ type: "SCENE_INIT", payload: null })).toBeNull();
  });

  it("rejects SCENE_* frames whose elements is not an array", () => {
    expect(parseBroadcastPayload({ type: "SCENE_UPDATE", payload: { elements: "nope" } })).toBeNull();
    expect(parseBroadcastPayload({ type: "SCENE_UPDATE", payload: {} })).toBeNull();
  });

  it("rejects MOUSE_LOCATION frames with malformed fields", () => {
    const bad = (override: Record<string, unknown>) =>
      parseBroadcastPayload({ ...validMouse, payload: { ...validMouse.payload, ...override } });
    expect(bad({ socketId: 1 })).toBeNull();
    expect(bad({ username: null })).toBeNull();
    expect(bad({ button: "left" })).toBeNull();
    expect(bad({ pointer: { x: 1, y: 2 } })).toBeNull();
    expect(bad({ pointer: { x: "1", y: 2, tool: "pointer" } })).toBeNull();
    expect(bad({ pointer: { x: 1, y: 2, tool: "wand" } })).toBeNull();
    expect(bad({ selectedElementIds: "all" })).toBeNull();
  });
});
