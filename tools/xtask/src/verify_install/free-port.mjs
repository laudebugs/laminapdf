// `cargo xtask verify-install`: copied next to chromium.mjs and workerd.mjs by xtask.
import { createServer } from "node:net";

/** A free TCP port on 127.0.0.1 (the OS picks one, then the listener closes). */
export const freePort = () =>
  new Promise((resolve, reject) => {
    const server = createServer();
    server.on("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const { port } = server.address();
      server.close(() => resolve(port));
    });
  });
