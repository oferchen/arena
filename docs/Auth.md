# Authentication Flow

The server issues a guest session when a client connects without a token. A unique identifier is stored in the `players` table and returned as a session cookie.

To upgrade to a registered account, clients request a one-time passcode via `/auth/request` supplying an email address. The server rate limits requests and sends the code via email. The code and a salted hash of the email are held temporarily in memory.

The client verifies the code with `/auth/verify`. On success, the server returns a new session token and sets it as a cookie. Clients store this token and replace the guest identifier.
