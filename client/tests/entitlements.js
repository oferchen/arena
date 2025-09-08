const assert = require("assert");

function mock_entitlements(expected) {
  globalThis.fetch = async (url) => {
    assert.strictEqual(url, expected);
    return new Response(JSON.stringify({ entitlements: ["duck_hunt"] }), {
      status: 200,
      headers: { "Content-Type": "application/json" },
    });
  };
}

module.exports = { mock_entitlements };
