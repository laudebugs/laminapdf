// A same-origin page for the browser checks: WebAuthn needs a page on the relying party's origin.
export const PAGE = `<!doctype html><meta charset="utf-8"><title>auth spike</title>
<script>
const api = (path, body) =>
  fetch(path, body === undefined ? {} : { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify(body) })
    .then(async (r) => ({ status: r.status, body: await r.json().catch(() => null) }));
window.registerPasskey = async () => {
  const options = await api("/api/auth/passkey/generate-register-options");
  const credential = await navigator.credentials.create({ publicKey: PublicKeyCredential.parseCreationOptionsFromJSON(options.body) });
  return api("/api/auth/passkey/verify-registration", { response: credential.toJSON(), name: "spike" });
};
window.signInWithPasskey = async () => {
  const options = await api("/api/auth/passkey/generate-authenticate-options");
  const credential = await navigator.credentials.get({ publicKey: PublicKeyCredential.parseRequestOptionsFromJSON(options.body) });
  return api("/api/auth/passkey/verify-authentication", { response: credential.toJSON() });
};
window.api = api;
</script>`;
