const makeServiceWorkerEnv = require("service-worker-mock");

function setup_indexeddb_error() {
  Object.assign(globalThis, makeServiceWorkerEnv());
  globalThis.window = globalThis;
  globalThis.navigator = {
    storage: {
      getDirectory: () => Promise.reject("opfs disabled"),
    },
  };
  delete globalThis.indexedDB;
}

module.exports = { setup_indexeddb_error };
