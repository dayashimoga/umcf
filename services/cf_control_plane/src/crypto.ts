// Cryptographic signature verification and anti-replay protection for UMCF Control Plane.
// Strictly enforces zero-placeholder and default-deny invariants.

const BASE58_ALPHABET = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

/** Decodes a Base58btc encoded string into a Uint8Array */
export function decodeBase58(input: string): Uint8Array {
  const bytes: number[] = [0];
  for (let i = 0; i < input.length; i++) {
    const char = input[i];
    const value = BASE58_ALPHABET.indexOf(char);
    if (value === -1) {
      throw new Error(`Invalid Base58 character: ${char}`);
    }
    for (let j = 0; j < bytes.length; j++) {
      bytes[j] = (bytes[j] ?? 0) * 58;
    }
    bytes[0] = (bytes[0] ?? 0) + value;
    let carry = 0;
    for (let j = 0; j < bytes.length; j++) {
      const byteVal = (bytes[j] ?? 0) + carry;
      bytes[j] = byteVal & 0xff;
      carry = byteVal >> 8;
    }
    while (carry > 0) {
      bytes.push(carry & 0xff);
      carry >>= 8;
    }
  }

  // Handle leading zeros in base58
  for (let i = 0; i < input.length && input[i] === "1"; i++) {
    bytes.push(0);
  }

  return new Uint8Array(bytes.reverse());
}

/** Converts a hex string into a Uint8Array */
export function hexToBytes(hex: string): Uint8Array {
  const cleanHex = hex.trim().toLowerCase();
  if (cleanHex.length % 2 !== 0) {
    throw new Error(`Invalid hex length: ${cleanHex.length}`);
  }
  const bytes = new Uint8Array(cleanHex.length / 2);
  for (let i = 0; i < cleanHex.length; i += 2) {
    bytes[i / 2] = parseInt(cleanHex.slice(i, i + 2), 16);
  }
  return bytes;
}

/** Converts a Uint8Array into a lowercase hex string */
export function bytesToHex(bytes: Uint8Array): string {
  let hex = "";
  for (let i = 0; i < bytes.length; i++) {
    hex += (bytes[i] ?? 0).toString(16).padStart(2, "0");
  }
  return hex;
}

/** Extracts the raw 32-byte Ed25519 public key from a DID or explicit hex string */
export function extractEd25519PublicKey(did: string, explicitHexPk?: string | null): Uint8Array {
  if (explicitHexPk && explicitHexPk.length === 64) {
    return hexToBytes(explicitHexPk);
  }

  if (did.startsWith("did:key:z6Mk")) {
    const raw = decodeBase58(did.slice(8));
    // Multicodec Ed25519 prefix: 0xed, 0x01 followed by 32 public key bytes
    if (raw.length >= 34 && raw[0] === 0xed && raw[1] === 0x01) {
      return raw.slice(2, 34);
    }
    if (raw.length === 32) {
      return raw;
    }
  }

  throw new Error(
    `Cannot extract Ed25519 public key from DID '${did}'. Provide X-UMCF-Public-Key header with 64 hex characters.`
  );
}

/** Maximum allowed clock drift between client and server (60 seconds) */
export const MAX_REPLAY_SKEW_MS = 60_000;

export interface AuthValidationResult {
  valid: boolean;
  did?: string;
  error?: string;
}

/**
 * Verifies request authenticity via Ed25519 digital signature and timestamp freshness.
 * Target format: `${method.toUpperCase()}:${pathname}:${timestampMs}:${bodySha256Hex}`
 */
export async function verifyRequestSignature(
  method: string,
  pathname: string,
  headers: Headers,
  bodyBytes: Uint8Array,
  nowMs: number = Date.now()
): Promise<AuthValidationResult> {
  const did = headers.get("x-umcf-did") ?? headers.get("X-UMCF-DID");
  const timestampStr = headers.get("x-umcf-timestamp") ?? headers.get("X-UMCF-Timestamp");
  const signatureHex = headers.get("x-umcf-signature") ?? headers.get("X-UMCF-Signature");
  const explicitPk = headers.get("x-umcf-public-key") ?? headers.get("X-UMCF-Public-Key");

  if (!did || !timestampStr || !signatureHex) {
    return {
      valid: false,
      error: "Missing mandatory authentication headers: X-UMCF-DID, X-UMCF-Timestamp, X-UMCF-Signature"
    };
  }

  const timestampMs = parseInt(timestampStr, 10);
  if (isNaN(timestampMs)) {
    return { valid: false, error: "Invalid X-UMCF-Timestamp format; must be integer milliseconds" };
  }

  // 1. Anti-Replay: Clock drift check
  if (Math.abs(nowMs - timestampMs) > MAX_REPLAY_SKEW_MS) {
    return {
      valid: false,
      error: `Timestamp expired: request timestamp ${timestampMs} ms skews from server time ${nowMs} ms by > ${MAX_REPLAY_SKEW_MS} ms`
    };
  }

  // 2. Extract public key
  let pkBytes: Uint8Array;
  try {
    pkBytes = extractEd25519PublicKey(did, explicitPk);
  } catch (e: any) {
    return { valid: false, error: e?.message ?? "Failed to extract public key" };
  }

  // 3. Extract signature bytes
  let sigBytes: Uint8Array;
  try {
    sigBytes = hexToBytes(signatureHex);
    if (sigBytes.length !== 64) {
      return { valid: false, error: `Invalid signature byte length: expected 64, got ${sigBytes.length}` };
    }
  } catch {
    return { valid: false, error: "Invalid hex signature string" };
  }

  // 4. Compute body SHA-256 digest
  const hashBuffer = await crypto.subtle.digest("SHA-256", bodyBytes);
  const bodySha256Hex = bytesToHex(new Uint8Array(hashBuffer));

  // 5. Construct canonical sign target
  const canonicalPayload = `${method.toUpperCase()}:${pathname}:${timestampMs}:${bodySha256Hex}`;
  const dataBytes = new TextEncoder().encode(canonicalPayload);
  const domainDataBytes = new TextEncoder().encode(`UMCF-SIG-v1:${canonicalPayload}`);

  // 6. Verify via WebCrypto
  try {
    let cryptoKey: CryptoKey;
    try {
      cryptoKey = await crypto.subtle.importKey(
        "raw",
        pkBytes,
        { name: "Ed25519" },
        false,
        ["verify"]
      );
    } catch {
      // Cloudflare Worker alias
      cryptoKey = await crypto.subtle.importKey(
        "raw",
        pkBytes,
        { name: "NODE-ED25519" },
        false,
        ["verify"]
      );
    }

    let isValid = await crypto.subtle.verify(
      cryptoKey.algorithm.name,
      cryptoKey,
      sigBytes,
      dataBytes
    );

    if (!isValid) {
      isValid = await crypto.subtle.verify(
        cryptoKey.algorithm.name,
        cryptoKey,
        sigBytes,
        domainDataBytes
      );
    }

    if (!isValid) {
      return { valid: false, error: "Ed25519 cryptographic signature verification failed" };
    }

    return { valid: true, did };
  } catch (err: any) {
    return { valid: false, error: `WebCrypto verification exception: ${err?.message ?? err}` };
  }
}

