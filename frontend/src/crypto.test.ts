import { describe, expect, it } from "vitest";

import { decrypt, encrypt, exportKey, generateKey, importKey } from "./crypto";

const utf8 = (s: string) => new TextEncoder().encode(s);
const fromUtf8 = (b: Uint8Array) => new TextDecoder().decode(b);

describe("crypto", () => {
  it("round-trips encrypt → decrypt", async () => {
    const key = await generateKey();
    const plaintext = utf8("the room key never reaches the server");
    const blob = await encrypt(key, plaintext);
    const recovered = await decrypt(key, blob);
    expect(fromUtf8(recovered)).toBe("the room key never reaches the server");
  });

  it("round-trips arbitrary binary, including high bytes", async () => {
    const key = await generateKey();
    const bytes = new Uint8Array(256);
    for (let i = 0; i < bytes.length; i++) {
      bytes[i] = i; // every byte value 0..255
    }
    const recovered = await decrypt(key, await encrypt(key, bytes));
    expect(Array.from(recovered)).toEqual(Array.from(bytes));
  });

  it("uses a fresh IV per encryption", async () => {
    const key = await generateKey();
    const plaintext = utf8("identical plaintext");
    const a = await encrypt(key, plaintext);
    const b = await encrypt(key, plaintext);
    // The 12-byte IV prefix must differ, so the ciphertext does too (no nonce reuse).
    expect(Array.from(a.subarray(0, 12))).not.toEqual(Array.from(b.subarray(0, 12)));
    expect(Array.from(a)).not.toEqual(Array.from(b));
  });

  it("fails to decrypt with the wrong key", async () => {
    const a = await generateKey();
    const b = await generateKey();
    const blob = await encrypt(a, utf8("secret"));
    await expect(decrypt(b, blob)).rejects.toBeDefined();
  });

  it("round-trips a key through URL-safe base64 export/import", async () => {
    const key = await generateKey();
    const encoded = await exportKey(key);
    // base64url alphabet only: no '+', '/', or '=' padding that would corrupt a URL fragment.
    expect(encoded).toMatch(/^[A-Za-z0-9_-]+$/);

    // A key re-imported from the encoded form must decrypt what the original encrypted, proving
    // the base64url round-trip preserved the raw key bytes exactly.
    const reimported = await importKey(encoded);
    const blob = await encrypt(key, utf8("cross-key payload"));
    expect(fromUtf8(await decrypt(reimported, blob))).toBe("cross-key payload");
  });
});
