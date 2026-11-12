// Signs Clerk-shaped session tokens with a fresh RS256 key and asks the Worker (port 8789) to
// verify them. Run: node check.mjs > .dev.vars once (prints the key), start the Worker, then
// node check.mjs --run.

import { createPrivateKey, generateKeyPairSync, sign } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";

const BASE = "http://localhost:8789";
if (!process.argv.includes("--run")) {
  const { privateKey, publicKey } = generateKeyPairSync("rsa", { modulusLength: 2048 });
  writeFileSync("spike-key.pem", privateKey.export({ type: "pkcs8", format: "pem" }));
  const pem = publicKey.export({ type: "spki", format: "pem" }).replaceAll("\n", "\\n");
  console.log(`CLERK_JWT_KEY="${pem}"\nAPP_ORIGIN=http://localhost:8787`);
  process.exit(0);
}
const key = createPrivateKey(readFileSync("spike-key.pem"));
const b64 = (v) => Buffer.from(JSON.stringify(v)).toString("base64url");
const jwt = (claims) => {
  const head = `${b64({ alg: "RS256", typ: "JWT", kid: "ins_spike" })}.${b64(claims)}`;
  return `${head}.${sign("RSA-SHA256", Buffer.from(head), key).toString("base64url")}`;
};
const now = Math.floor(Date.now() / 1000);
const good = { sub: "user_spike", sid: "sess_spike", iat: now, nbf: now - 5, exp: now + 60, azp: "http://localhost:8787", iss: "https://clerk.example.com" };
const ask = async (token) => (await fetch(`${BASE}/api/me`, { headers: { authorization: `Bearer ${token}` } })).json();
let failed = 0;
const check = (name, ok, detail) => {
  failed += ok ? 0 : 1;
  console.log(`${ok ? "ok  " : "FAIL"} ${name} (${JSON.stringify(detail)})`);
};
let r = await ask(jwt(good));
check("a valid session token verifies on the Worker", r.userId === "user_spike", r);
r = await ask(jwt({ ...good, exp: now - 10 }));
check("an expired token is refused", r.error !== undefined, r);
r = await ask(jwt({ ...good, azp: "https://evil.example" }));
check("a token for another origin is refused", r.error !== undefined, r);
process.exit(failed);
