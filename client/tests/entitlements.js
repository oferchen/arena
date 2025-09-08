function mock_entitlements() {
  globalThis.fetch = async () => {
    return new Response(JSON.stringify({ entitlements: ["duck_hunt"] }), {
      status: 200,
      headers: { "Content-Type": "application/json" },
    });
  };
}

module.exports = { mock_entitlements };
