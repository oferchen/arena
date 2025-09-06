# WebFPS Hub

WebFPS Hub is a centralized platform for hosting and developing browser-based first-person shooter projects. It combines a modular client with a lightweight server so developers can prototype and share FPS experiences entirely on the web.

## Goals

- Showcase FPS gameplay that runs in any modern browser.
- Provide a reference server that handles matchmaking and game rooms.
- Grow a community that contributes new maps, weapons, and modes.

## Setup

1. **Clone the repository**

   ```bash
   git clone https://example.com/webfps-hub.git
   cd webfps-hub
   ```

2. **Install dependencies**

   ```bash
   npm install
   ```

### Client Build

Compile the browser client:

```bash
npm run build
```

The production bundle is emitted to `dist/`.

### Server Run

Start the development server:

```bash
npm start
```

The server listens on `http://localhost:3000` by default.

### Contribution Guidelines

- Open an issue to discuss significant changes before starting work.
- Run linting and tests with `npm test` before submitting.
- Create pull requests with clear descriptions of your changes.

